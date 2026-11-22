//! The bag, the TM case, the party menu, field moves and Fly, driven by cursor positions read from
//! RAM the way a human reads them off the screen.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::menu;
use super::names::{id, names};
use super::prims::{is_free, settle, wait_until};
use super::ram::Ram;
use super::session::{PResult, PlayError, Session};

/// `struct Task` (`decompiled/include/task.h:15`): 40 bytes, `func` at 0, `isActive` at 4, `data`
/// at 8; 16 tasks in `gTasks`.
fn list_menu_task(s: &mut Session) -> Option<u32> {
    let g = s.sym("gTasks");
    let f = s.sym("ListMenuDummyTask");
    let emu = s.hf.rec.emu();
    (0..16u32).find_map(|i| {
        let b = g + i * 40;
        (emu.read8(b + 4) != 0 && emu.read32(b) & !1 == f).then_some(b + 8)
    })
}

/// The live list cursor: `struct ListMenu` lives in its task's data (`ListMenuInitInternal`,
/// `decompiled/src/list_menu.c:315`), `u16 cursorPos` at 0x18 and `u16 itemsAbove` at 0x1A after
/// the 0x18-byte template (`decompiled/include/list_menu.h:53-84`). Index = cursorPos + itemsAbove.
pub fn list_index(s: &mut Session) -> Option<(u16, u16)> {
    let d = list_menu_task(s)?;
    let emu = s.hf.rec.emu();
    let total = emu.read16(d + 0x0C);
    Some((emu.read16(d + 0x18) + emu.read16(d + 0x1A), total))
}

/// Move a list menu's cursor to row `want`, one human press per row.
pub fn list_goto(s: &mut Session, want: u16) -> PResult<()> {
    for _ in 0..80 {
        let Some((cur, _)) = list_index(s) else {
            s.hf.step(0)?;
            continue;
        };
        if cur == want {
            return Ok(());
        }
        s.hf.press(if want > cur { keys::DOWN } else { keys::UP })?;
        s.hf.idle(1)?;
    }
    Err(PlayError::Death(format!(
        "soft-lock: list cursor never reached row {want}"
    )))
}

/// `gBagMenuState` (`decompiled/include/item_menu.h:12-20`): `u16 pocket` at 6 (0 items, 1 key
/// items, 2 balls: `OPEN_BAG_*`).
fn bag_pocket(s: &mut Session) -> u16 {
    let a = s.sym("gBagMenuState");
    s.hf.rec.emu().read16(a + 6)
}

/// The bag takes input: its task runs and is not returning early for a palette fade or the window
/// animation (`Task_BagMenu_HandleInput`, `decompiled/src/item_menu.c:1037-1046`).
fn bag_input(s: &mut Session) -> bool {
    menu::task(s, "Task_BagMenu_HandleInput")
        && !super::prims::fade_active(s)
        && !menu::task(s, "Task_AnimateWin0v")
}

/// Start menu row of an action: `sStartMenuOrder[]` holds the `STARTMENU_*` ids in display order
/// (`decompiled/src/start_menu.c:43-66,213`).
fn start_row(s: &mut Session, action: u8) -> PResult<u8> {
    let order = s.sym("sStartMenuOrder");
    let n = s.sym("sNumStartMenuItems");
    let emu = s.hf.rec.emu();
    let count = emu.read8(n);
    (0..count)
        .find(|&i| emu.read8(order + i as u32) == action)
        .ok_or_else(|| PlayError::Fatal(format!("start menu has no action {action}")))
}
const STARTMENU_POKEMON: u8 = 1;
const STARTMENU_BAG: u8 = 2;

/// Open the bag from the field and wait for its list to take input.
pub fn open_bag(s: &mut Session) -> PResult<()> {
    menu::open_start(s)?;
    let row = start_row(s, STARTMENU_BAG)?;
    menu::start_menu_pick(s, row)?;
    wait_until(s, "the bag", 400, bag_input)
}

/// Back out of whatever menu is open to the free field (B, late by `U{0..N}` each).
pub fn close_to_field(s: &mut Session) -> PResult<()> {
    for _ in 0..30 {
        if is_free(s) {
            return Ok(());
        }
        if s.obs.callback2_is(s.hf.rec.emu(), "CB2_Overworld")
            && !s.obs.start_menu_taking_input(s.hf.rec.emu())
        {
            // Field, a script or a fade: settle handles it.
            return settle(s);
        }
        s.hf.press(keys::B)?;
        s.hf.idle(20)?;
    }
    settle(s)
}

/// Bag pocket (save order: 0 items, 1 key items, 2 balls, 3 TM/HM, 4 berries) holding `item`.
fn pocket_of(s: &mut Session, item: u16) -> Option<(usize, usize)> {
    let ram = Ram::new(&s.obs);
    let emu = s.hf.rec.emu();
    (0..3).find_map(|p| {
        ram.pocket(emu, p)
            .iter()
            .position(|&(i, _)| i == item)
            .map(|row| (p, row))
    })
}

/// In the open bag: switch to the pocket holding `item`, put the cursor on it, press A, and pick
/// `row` of its context menu (`sContextMenuItems_*`, `decompiled/src/item_menu.c:209-262`).
pub fn bag_use(s: &mut Session, item: u16, row: i8) -> PResult<()> {
    let (pocket, idx) = pocket_of(s, item)
        .ok_or_else(|| PlayError::Fatal(format!("{} is not in the bag", names().item(item))))?;
    for tries in 0.. {
        wait_until(s, "the bag", 200, bag_input)?;
        let cur = bag_pocket(s) as usize;
        if cur == pocket {
            break;
        }
        if tries > 8 {
            return Err(PlayError::Death("soft-lock: bag pocket".into()));
        }
        s.hf.press(if pocket > cur {
            keys::RIGHT
        } else {
            keys::LEFT
        })?;
        s.hf.idle(2)?;
    }
    list_goto(s, idx as u16)?;
    s.hf.press(keys::A)?;
    wait_until(s, "the item's context menu", 120, |s| {
        menu::task(s, "Task_FieldItemContextMenuHandleInput")
    })?;
    menu::choose(s, row)
}

/// `gSaveBlock1Ptr->registeredItem` (`decompiled/include/global.h:776`).
pub fn registered_item(s: &mut Session) -> u16 {
    let emu = s.hf.rec.emu();
    s.obs
        .save_block1(emu)
        .map_or(0, |sb| emu.read16(sb + 0x296))
}

/// Register the Bicycle to SELECT (key items context menu row 1 = REGISTER,
/// `item_menu.c:1400-1412`), once.
pub fn register_bike(s: &mut Session) -> PResult<()> {
    let bike = id("ITEM_BICYCLE");
    if registered_item(s) == bike {
        return Ok(());
    }
    open_bag(s)?;
    bag_use(s, bike, 1)?;
    s.hf.idle(10)?;
    close_to_field(s)?;
    s.event(
        "decision",
        "registered the Bicycle to SELECT",
        json!({"item": "ITEM_BICYCLE"}),
    );
    Ok(())
}

/// Get on the bike with SELECT (registered), unless already riding or not allowed here.
pub fn mount_bike(s: &mut Session) -> PResult<bool> {
    let flags = s.obs.player_avatar_flags(s.hf.rec.emu());
    if flags & (super::walk::AVATAR_MACH_BIKE | super::walk::AVATAR_ACRO_BIKE) != 0 {
        return Ok(true);
    }
    if flags & super::walk::AVATAR_SURFING != 0 {
        return Ok(false);
    }
    register_bike(s)?;
    settle(s)?;
    s.hf.press(keys::SELECT)?;
    s.hf.idle(4)?;
    settle(s)?;
    let on = s.obs.player_avatar_flags(s.hf.rec.emu())
        & (super::walk::AVATAR_MACH_BIKE | super::walk::AVATAR_ACRO_BIKE)
        != 0;
    Ok(on)
}

/// `gPartyMenu.slotId` (`decompiled/include/party_menu.h:8-20`: two pointers, a bitfield byte,
/// then `s8 slotId` at 9).
fn party_slot(s: &mut Session) -> i8 {
    let a = s.sym("gPartyMenu");
    s.hf.rec.emu().read8(a + 9) as i8
}

fn party_input(s: &mut Session) -> bool {
    menu::task(s, "Task_HandleChooseMonInput")
}

/// In the party menu: put the cursor on `slot` and press A.
pub fn party_choose(s: &mut Session, slot: i8) -> PResult<()> {
    wait_until(s, "the party menu", 400, party_input)?;
    for _ in 0..12 {
        let cur = party_slot(s);
        if cur == slot {
            s.hf.press(keys::A)?;
            return Ok(());
        }
        s.hf.press(if slot > cur { keys::DOWN } else { keys::UP })?;
        s.hf.idle(1)?;
    }
    Err(PlayError::Death("soft-lock: party cursor".into()))
}

/// The party slot of the first mon knowing `mv`.
pub fn slot_with_move(s: &mut Session, mv: &str) -> Option<usize> {
    let m = id(mv);
    let ram = Ram::new(&s.obs);
    let party = ram.party(s.hf.rec.emu());
    party.iter().position(|p| p.knows(m).is_some())
}

/// `sFieldMoves` (`decompiled/src/data/party_menu.h:1158-1162`); the selection menu lists
/// SUMMARY, then the mon's field moves in move-slot order, then SWITCH, ITEM, CANCEL
/// (`SetPartyMonFieldSelectionActions`, `party_menu.c:2963-2988`).
const FIELD_MOVES: [&str; 12] = [
    "MOVE_FLASH",
    "MOVE_CUT",
    "MOVE_FLY",
    "MOVE_STRENGTH",
    "MOVE_SURF",
    "MOVE_ROCK_SMASH",
    "MOVE_WATERFALL",
    "MOVE_TELEPORT",
    "MOVE_DIG",
    "MOVE_MILK_DRINK",
    "MOVE_SOFT_BOILED",
    "MOVE_SWEET_SCENT",
];

/// Open the party menu from the field, pick the mon knowing `mv`, and choose `mv` from its menu.
pub fn use_field_move(s: &mut Session, mv: &str) -> PResult<()> {
    let slot =
        slot_with_move(s, mv).ok_or_else(|| PlayError::Fatal(format!("nobody knows {mv}")))?;
    let ram = Ram::new(&s.obs);
    let mon = ram.party(s.hf.rec.emu())[slot].clone();
    let fm: Vec<u16> = FIELD_MOVES.iter().map(|m| id(m)).collect();
    let row = 1 + mon
        .moves
        .iter()
        .filter(|m| fm.contains(m))
        .position(|&m| m == id(mv))
        .unwrap() as i8;
    menu::open_start(s)?;
    let r = start_row(s, STARTMENU_POKEMON)?;
    menu::start_menu_pick(s, r)?;
    party_choose(s, slot as i8)?;
    wait_until(s, "the mon's menu", 200, |s| {
        menu::task(s, "Task_HandleSelectionMenuInput")
    })?;
    menu::choose(s, row)?;
    s.event(
        "decision",
        format!(
            "used {} from {}'s menu",
            mv.trim_start_matches("MOVE_"),
            names().species(mon.species).trim_start_matches("SPECIES_")
        ),
        json!({"move": mv, "slot": slot}),
    );
    Ok(())
}

/// Teach a TM/HM from the TM case to party slot `slot`: Bag -> TM CASE -> OPEN, the item, USE,
/// the yes/no, the mon (`decompiled/src/tm_case.c`).
pub fn teach_tmhm(s: &mut Session, item: &str, slot: usize) -> PResult<()> {
    let it = id(item);
    let ram = Ram::new(&s.obs);
    // The case sorts its pocket on opening, HMs first (`SortPocketAndPlaceHMsFirst`,
    // `decompiled/src/item.c:495`, called from `tm_case.c:515`): HMs by id, then TMs by id.
    let mut tms = ram.pocket(s.hf.rec.emu(), 3);
    let hm01 = id("ITEM_HM01");
    tms.sort_by_key(|&(i, _)| (i < hm01, i));
    let row = tms
        .iter()
        .position(|&(i, _)| i == it)
        .ok_or_else(|| PlayError::Fatal(format!("{item} not in the TM case")))?;
    open_bag(s)?;
    bag_use(s, id("ITEM_TM_CASE"), 0)?;
    wait_until(s, "the TM case", 400, |s| {
        menu::task(s, "Task_HandleListInput") && list_index(s).is_some()
    })?;
    s.hf.idle(4)?;
    list_goto(s, row as u16)?;
    s.hf.press(keys::A)?;
    wait_until(s, "the TM context menu", 200, |s| {
        menu::task(s, "Task_SelectedTMHM_Field")
    })?;
    menu::choose(s, 0)?; // USE
                         // "Booted up a TM/HM. It contained X. Teach X to a POKéMON?" -> yes, then the party menu.
    let start = s.hf.fed();
    loop {
        if party_input(s) {
            break;
        }
        if s.hf.fed() - start > 3000 {
            return Err(PlayError::Death("soft-lock: teaching menu".into()));
        }
        s.hf.press(keys::A)?;
        s.hf.idle(2)?;
    }
    party_choose(s, slot as i8)?;
    // "X learned Y!" and back to the TM case, then out.
    let start = s.hf.fed();
    loop {
        if menu::task(s, "Task_HandleListInput") && list_index(s).is_some() {
            break;
        }
        if s.hf.fed() - start > 3000 {
            return Err(PlayError::Death("soft-lock: after teaching".into()));
        }
        s.hf.press(keys::A)?;
        s.hf.idle(2)?;
    }
    close_to_field(s)?;
    let ram = Ram::new(&s.obs);
    let mon = ram.party(s.hf.rec.emu())[slot].clone();
    s.event(
        "decision",
        format!(
            "taught {} to {}",
            item.trim_start_matches("ITEM_"),
            names().species(mon.species).trim_start_matches("SPECIES_")
        ),
        json!({"item": item, "slot": slot, "moves": mon.moves.iter().map(|m| names().mv(*m)).collect::<Vec<_>>()}),
    );
    Ok(())
}

/// Fly map cursor (`struct MapCursor`, `decompiled/src/region_map.c:208-226`; `sMapCursor` is a
/// pointer): `s16 x, y` at 0 and 2.
fn fly_cursor(s: &mut Session) -> Option<(i16, i16)> {
    let p = s.sym("sMapCursor");
    let emu = s.hf.rec.emu();
    let ptr = emu.read32(p);
    (0x0200_0000..0x0204_0000)
        .contains(&ptr)
        .then(|| (emu.read16(ptr) as i16, emu.read16(ptr + 2) as i16))
}

/// The Kanto region map square of a `MAPSEC_*`: its first cell in `sRegionMapSections_Kanto`'s
/// `LAYER_MAP` (`decompiled/src/data/region_map/region_map_layout_kanto.h`, 22 x 15).
fn mapsec_cell(mapsec: &str) -> Option<(i16, i16)> {
    let root = std::env::var_os("FRLG_DECOMP_RO")
        .or_else(|| std::env::var_os("FRLG_DECOMP"))
        .map(std::path::PathBuf::from)?;
    let text =
        std::fs::read_to_string(root.join("src/data/region_map/region_map_layout_kanto.h")).ok()?;
    let mut y = 0i16;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("[LAYER_DUNGEON]") {
            break;
        }
        if !l.starts_with('{') || !l.contains("MAPSEC") {
            continue;
        }
        let cells: Vec<&str> = l
            .trim_matches(|c| c == '{' || c == '}' || c == ',')
            .split(',')
            .map(str::trim)
            .collect();
        if let Some(x) = cells.iter().position(|c| *c == mapsec) {
            return Some((x as i16, y));
        }
        y += 1;
    }
    None
}

/// Fly to `mapsec` (e.g. `MAPSEC_LAVENDER_TOWN`): party menu -> the Fly user -> FLY, move the map
/// cursor onto the town, A, and wait for the landing.
pub fn fly(s: &mut Session, mapsec: &str) -> PResult<()> {
    let target =
        mapsec_cell(mapsec).ok_or_else(|| PlayError::Fatal(format!("no map cell for {mapsec}")))?;
    let from = super::walk::here(s);
    use_field_move(s, "MOVE_FLY")?;
    wait_until(s, "the fly map", 600, |s| {
        menu::task(s, "Task_FlyMap") && fly_cursor(s).is_some()
    })?;
    s.hf.idle(20)?;
    for _ in 0..80 {
        let Some(c) = fly_cursor(s) else {
            s.hf.step(0)?;
            continue;
        };
        if c == target {
            break;
        }
        let key = if c.0 < target.0 {
            keys::RIGHT
        } else if c.0 > target.0 {
            keys::LEFT
        } else if c.1 < target.1 {
            keys::DOWN
        } else {
            keys::UP
        };
        s.hf.late()?;
        s.hf.step(key)?;
        // The cursor glides a square in a few frames.
        for _ in 0..30 {
            s.hf.step(0)?;
            if fly_cursor(s) != Some(c) {
                break;
            }
        }
        s.hf.idle(2)?;
    }
    s.hf.press(keys::A)?;
    let start = s.hf.fed();
    loop {
        let n = super::walk::here(s);
        if n.is_some() && n.map(|n| n.0) != from.map(|f| f.0) && is_free(s) {
            break;
        }
        if s.hf.fed() - start > 2000 {
            return Err(PlayError::Death(format!("soft-lock: fly to {mapsec}")));
        }
        s.hf.step(0)?;
    }
    settle(s)?;
    s.event(
        "decision",
        format!("flew to {}", mapsec.trim_start_matches("MAPSEC_")),
        json!({"mapsec": mapsec}),
    );
    Ok(())
}
