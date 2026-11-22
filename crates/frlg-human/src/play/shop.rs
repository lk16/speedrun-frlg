//! Marts, the Pokémon Storage PC, and items used from the bag outside battle, driven by cursor
//! positions read from RAM the way a human reads them off the screen.
//!
//! # API (for the route stages)
//!
//! ```ignore
//! use frlg_human::play::shop;
//! // Buy at a mart: walk in, talk to the clerk across the counter, BUY, each item with its quantity
//! // (capped by money and 99), out. Returns what was bought.
//! let got = shop::buy(s, "MAP_FUCHSIA_CITY_MART", None, &[("ITEM_GREAT_BALL", 20)])?;
//! // A department-store floor with two clerks: say which one (its map position).
//! shop::buy(s, "MAP_CELADON_CITY_DEPARTMENT_STORE_4F", Some((3, 13)), &[("ITEM_THUNDER_STONE", 1)])?;
//! // Pokémon Center PC: deposit party slots, withdraw species from the boxes.
//! shop::pc(s, "MAP_FUCHSIA_CITY_POKEMON_CENTER_1F", &[PcOp::Deposit(5), PcOp::Withdraw(species_id)])?;
//! // Items from the bag on the field.
//! shop::use_item(s, "ITEM_REPEL")?;                       // Repel, Max Repel, ...
//! shop::use_item_on(s, "ITEM_THUNDER_STONE", slot)?;      // stones, Rare Candy: evolution scene included
//! ```
//!
//! Every purchase, deposit and item use is logged as a `decision` event with the money, quantity
//! and price behind it.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::items::{close_to_field, list_goto, list_index, open_bag};
use super::menu;
use super::names::{id, names};
use super::prims::{is_free, press_until, settle, wait_until};
use super::ram::{read_mon, Ram};
use super::session::{PResult, PlayError, Session};
use super::walk::{face, go, go_face, here, Opts};

/// `MB_COUNTER` (`decompiled/include/constants/metatile_behaviors.h:91`): the player talks to the
/// person behind it from the other side (`decompiled/src/field_control_avatar.c:412-440`).
const MB_COUNTER: u16 = 0x80;
/// `MB_PC` (`metatile_behaviors.h:94`): the storage PC of a Pokémon Center (`EventScript_PC`,
/// `decompiled/data/scripts/pc.inc`).
const MB_PC: u16 = 0x83;

/// Walk up to the person at `(x, y)` of `map` -- next to them, or across a counter -- face them
/// and press A. Leaves their script running.
pub fn talk_at(s: &mut Session, map: &str, x: i16, y: i16) -> PResult<()> {
    let g = s.graph();
    let mid = g.id(map);
    let m = &g.maps[&mid];
    // Across a counter: (x, y) + 2 d, the counter tile at (x, y) + d.
    let across: Vec<(i16, i16, u16)> = [
        (0, 1, keys::UP),
        (0, -1, keys::DOWN),
        (1, 0, keys::LEFT),
        (-1, 0, keys::RIGHT),
    ]
    .iter()
    .filter(|&&(dx, dy, _)| {
        m.tile(x + dx, y + dy)
            .is_some_and(|t| t.behavior == MB_COUNTER)
            && m.tile(x + 2 * dx, y + 2 * dy)
                .is_some_and(|t| t.collision == 0)
    })
    .map(|&(dx, dy, k)| (x + 2 * dx, y + 2 * dy, k))
    .collect();
    if let Some(&(sx, sy, k)) = across.first() {
        go(s, map, sx, sy, &Opts::default())?;
        face(s, k)?;
    } else {
        go_face(s, map, x, y, &Opts::default())?;
    }
    s.hf.press(keys::A)?;
    for _ in 0..12 {
        s.hf.step(0)?;
        if s.obs.field_controls_locked(s.hf.rec.emu()) {
            return Ok(());
        }
    }
    Err(PlayError::Death(format!(
        "soft-lock: talking to ({x}, {y}) of {map} started nothing"
    )))
}

/// An item's mart price: `gItems[id].price` (`struct Item`, `decompiled/include/item.h:8-24`:
/// 14-byte name, `u16 itemId`, `u16 price` at 0x10; 44 bytes per item).
pub fn price(s: &mut Session, item: u16) -> u32 {
    let base = s.sym("gItems");
    s.hf.rec.emu().read16(base + item as u32 * 44 + 0x10) as u32
}

/// `sShopData` (`decompiled/src/shop.c:55-70`): `const u16 *itemList` at 4, `u16 itemCount` at
/// 0x10. The buy list shows the items in this order, then CANCEL.
fn mart_items(s: &mut Session) -> Vec<u16> {
    let a = s.sym("sShopData");
    let emu = s.hf.rec.emu();
    let list = emu.read32(a + 4);
    let n = emu.read16(a + 0x10).min(64) as u32;
    (0..n).map(|i| emu.read16(list + 2 * i)).collect()
}

/// The quantity on the "How many?" box: the shop task's `tItemCount` = `data[1]`
/// (`shop.c:33`; task data at 8 in `struct Task`, `decompiled/include/task.h:15`).
fn buy_quantity(s: &mut Session) -> Option<i16> {
    let g = s.sym("gTasks");
    let f = s.sym("Task_BuyHowManyDialogueHandleInput");
    let emu = s.hf.rec.emu();
    (0..16u32).find_map(|i| {
        let b = g + i * 40;
        (emu.read8(b + 4) != 0 && emu.read32(b) & !1 == f).then(|| emu.read16(b + 10) as i16)
    })
}

fn buy_list_up(s: &mut Session) -> bool {
    menu::task(s, "Task_BuyMenu") && !super::prims::fade_active(s) && list_index(s).is_some()
}

/// One line of a purchase.
#[derive(Clone, Debug)]
pub struct Bought {
    pub item: String,
    pub qty: u16,
    pub cost: u32,
}

/// Buy `want` (item constant, quantity) at the mart on `map`: talk to its clerk (the one at
/// `clerk`, else the only `OBJ_EVENT_GFX_CLERK` there), BUY, and per item: the cursor to its row
/// (`ListMenu` cursor from RAM), the quantity by d-pad (RIGHT +10, UP +1, DOWN -1:
/// `AdjustQuantityAccordingToDPadInput`, `decompiled/src/menu_helpers.c:169`), A, YES, and the
/// "Here you go" text. The quantity is capped by money (`Task_BuyHowManyDialogueInit`,
/// `shop.c`: `money / price`, at most 99). Items the mart does not sell are logged and skipped.
pub fn buy(
    s: &mut Session,
    map: &str,
    clerk: Option<(i16, i16)>,
    want: &[(&str, u16)],
) -> PResult<Vec<Bought>> {
    let (cx, cy) = match clerk {
        Some(p) => p,
        None => {
            let g = s.graph();
            let m = &g.maps[&g.id(map)];
            m.objects
                .iter()
                .find(|o| o.graphics.contains("CLERK"))
                .map(|o| (o.x, o.y))
                .ok_or_else(|| PlayError::Fatal(format!("no clerk on {map}")))?
        }
    };
    let money0 = Ram::new(&s.obs).money(s.hf.rec.emu());
    talk_at(s, map, cx, cy)?;
    // "Hi there! May I help you?" then BUY / SELL / SEE YA (`Task_ShopMenu`, `shop.c:256`).
    press_until(s, "the shop menu", keys::A, 3000, |s| {
        menu::task(s, "Task_ShopMenu")
    })?;
    menu::choose(s, 0)?;
    wait_until(s, "the buy list", 600, buy_list_up)?;
    let stock = mart_items(s);
    let mut out = vec![];
    for &(name, qty) in want {
        let item = id(name);
        let Some(row) = stock.iter().position(|&i| i == item) else {
            s.event(
                "model-vs-actual",
                format!("{map} does not sell {name}"),
                json!({"item": name, "stock": stock.iter().map(|&i| names().item(i)).collect::<Vec<_>>()}),
            );
            continue;
        };
        let p = price(s, item);
        let money = Ram::new(&s.obs).money(s.hf.rec.emu());
        let can = if p == 0 { 99 } else { (money / p).min(99) } as u16;
        let n = qty.min(can);
        if n == 0 {
            s.event(
                "model-vs-actual",
                format!("cannot afford {name} (¥{p}, have ¥{money})"),
                json!({"item": name, "price": p, "money": money, "want": qty}),
            );
            continue;
        }
        if n < qty {
            s.event(
                "model-vs-actual",
                format!("money for {n} of the {qty} {name} wanted"),
                json!({"item": name, "price": p, "money": money, "want": qty, "qty": n}),
            );
        }
        list_goto(s, row as u16)?;
        // A until the list lets go of the input (a press on a frame the list skips is lost).
        press_until(s, "the item chosen", keys::A, 300, |s| {
            !menu::task(s, "Task_BuyMenu")
        })?;
        // "X? Certainly. How many?" prints, then the quantity box takes input.
        wait_until(s, "the quantity box", 600, |s| buy_quantity(s).is_some())?;
        for _ in 0..40 {
            let Some(cur) = buy_quantity(s) else {
                return Err(PlayError::Death("soft-lock: quantity box closed".into()));
            };
            let d = n as i16 - cur;
            if d == 0 {
                break;
            }
            let k = if d >= 10 {
                keys::RIGHT
            } else if d > 0 {
                keys::UP
            } else if d <= -10 {
                keys::LEFT
            } else {
                keys::DOWN
            };
            s.hf.press(k)?;
            s.hf.idle(1)?;
        }
        s.hf.press(keys::A)?;
        // "X, and you want N. That will be ¥. OK?" -> YES (`Task_CallYesOrNoCallback`,
        // `menu_helpers.c:47`, cursor on YES).
        wait_until(s, "the purchase yes/no", 600, |s| {
            menu::task(s, "Task_CallYesOrNoCallback")
        })?;
        menu::answer(s, true)?;
        // "Here you go, thank you!" waits for A or B (`Task_ReturnToItemListAfterItemPurchase`).
        press_until(s, "back to the buy list", keys::A, 900, buy_list_up)?;
        let after = Ram::new(&s.obs).money(s.hf.rec.emu());
        let have = Ram::new(&s.obs).count(s.hf.rec.emu(), item);
        s.event(
            "decision",
            format!(
                "bought {n} {} for ¥{} (¥{after} left)",
                name.trim_start_matches("ITEM_"),
                money - after
            ),
            json!({"item": name, "qty": n, "price": p, "cost": money - after, "money": after, "in_bag": have}),
        );
        out.push(Bought {
            item: name.to_string(),
            qty: n,
            cost: money - after,
        });
    }
    // CANCEL out of the list (B), SEE YA on the shop menu (B), the clerk's goodbye.
    press_until(s, "the shop menu again", keys::B, 900, |s| {
        menu::task(s, "Task_ShopMenu")
    })?;
    s.hf.idle(4)?;
    press_until(s, "out of the shop menu", keys::B, 300, |s| {
        !menu::task(s, "Task_ShopMenu")
    })?;
    settle(s)?;
    let spent = money0 - Ram::new(&s.obs).money(s.hf.rec.emu());
    s.event(
        "decision",
        format!("left {map}: spent ¥{spent}"),
        json!({"map": map, "spent": spent}),
    );
    Ok(out)
}

/// One PC operation.
#[derive(Clone, Copy, Debug)]
pub enum PcOp {
    /// Deposit the mon in this party slot.
    Deposit(usize),
    /// Withdraw the first boxed mon of this species (game species id).
    Withdraw(u16),
}

/// `gPokemonStoragePtr` -> `struct PokemonStorage` (`decompiled/include/pokemon_storage_system.h:44-50`):
/// `u8 currentBox`, then `struct BoxPokemon boxes[14][30]` at 4 (80 bytes each).
fn storage(s: &mut Session) -> u32 {
    let p = s.sym("gPokemonStoragePtr");
    s.hf.rec.emu().read32(p)
}

/// Species in box `b` (30 slots, 0 = empty).
pub fn box_species(s: &mut Session, b: u32) -> Vec<u16> {
    let st = storage(s);
    let emu = s.hf.rec.emu();
    (0..30u32)
        .map(|i| {
            let a = st + 4 + (b * 30 + i) * 80;
            // An empty slot is all zero (`hasSpecies` clear): personality 0, otId 0.
            if emu.read32(a) == 0 && emu.read32(a + 4) == 0 {
                0
            } else {
                read_mon(emu, a).species
            }
        })
        .collect()
}

fn current_box(s: &mut Session) -> u32 {
    let st = storage(s);
    s.hf.rec.emu().read8(st) as u32
}

/// `gStorage->state` (`struct PokemonStorageSystemData`, `decompiled/include/pokemon_storage_system_internal.h:277-279`:
/// `u8 state` first): the storage screen takes input in `Task_PokeStorageMain` state 0.
fn storage_ready(s: &mut Session, task: &str) -> bool {
    menu::task(s, task) && !super::prims::fade_active(s) && storage_state(s) == 0
}

/// `gStorage->state`, 0xFF while the storage screen is not allocated.
fn storage_state(s: &mut Session) -> u8 {
    let p = s.sym("gStorage");
    let emu = s.hf.rec.emu();
    let g = emu.read32(p);
    if (0x0200_0000..0x0204_0000).contains(&g) {
        emu.read8(g)
    } else {
        0xFF
    }
}

/// `sCursorArea`, `sCursorPosition` (`decompiled/src/pokemon_storage_system_data.c:16-17`):
/// area 0 box, 1 party, 2 box title, 3 buttons (`pokemon_storage_system_internal.h:114-120`).
fn storage_cursor(s: &mut Session) -> (i8, i8) {
    let a = s.sym("sCursorArea");
    let p = s.sym("sCursorPosition");
    let emu = s.hf.rec.emu();
    (emu.read8(a) as i8, emu.read8(p) as i8)
}

/// The storage screen's selection menu for the mon under the cursor takes input
/// (`Task_OnSelectedMon` state 2, `decompiled/src/pokemon_storage_system_tasks.c:935-960`).
fn selection_menu_up(s: &mut Session) -> bool {
    menu::task(s, "Task_OnSelectedMon") && storage_state(s) == 2
}

/// Open the Pokémon Storage System on the PC of `center_map` with PC main-menu row `row`
/// (`Task_PCMainMenu`, `decompiled/src/pokemon_storage_system_menu.c:241`: WITHDRAW 0, DEPOSIT 1,
/// MOVE 2, MOVE ITEMS 3, SEE YA 4).
fn open_storage(s: &mut Session, center_map: &str, row: i8) -> PResult<()> {
    if !menu::task(s, "Task_PCMainMenu") {
        let g = s.graph();
        let mid = g.id(center_map);
        let m = &g.maps[&mid];
        let pc = (0..m.h)
            .flat_map(|y| (0..m.w).map(move |x| (x, y)))
            .find(|&(x, y)| m.tile(x, y).is_some_and(|t| t.behavior == MB_PC))
            .ok_or_else(|| PlayError::Fatal(format!("no PC on {center_map}")))?;
        // The PC sits in the back wall: use it from the tile below, facing up.
        go(s, center_map, pc.0, pc.1 + 1, &Opts::default())?;
        face(s, keys::UP)?;
        s.hf.press(keys::A)?;
        // "Booted up the PC." -> "Which PC should be accessed?" (`CreatePCMenu`,
        // `decompiled/src/script_menu.c:977`, a multichoice with the storage PC first).
        // A mashed press can land on the list as it opens: row 0 is what the human wants anyway.
        press_until(s, "the PC list", keys::A, 3000, |s| {
            menu::multichoice_up(s) || menu::task(s, "Task_PCMainMenu")
        })?;
        if menu::multichoice_up(s) {
            menu::choose(s, 0)?;
        }
        press_until(s, "the storage menu", keys::A, 3000, |s| {
            menu::task(s, "Task_PCMainMenu")
        })?;
    }
    menu::choose(s, row)
}

/// Leave the storage screen, the PC menu and the PC (B on each: "Continue Box operations?" is
/// answered NO by B, `Task_OnBPressed`, `pokemon_storage_system_tasks.c:1988`).
fn close_pc(s: &mut Session) -> PResult<()> {
    for _ in 0..40 {
        if is_free(s) {
            return Ok(());
        }
        s.hf.press(keys::B)?;
        s.hf.idle(12)?;
    }
    settle(s)
}

/// Deposit and withdraw at the storage PC of a Pokémon Center (walks there first).
pub fn pc(s: &mut Session, center_map: &str, ops: &[PcOp]) -> PResult<()> {
    let mut opened: Option<i8> = None;
    for &op in ops {
        let row = match op {
            PcOp::Withdraw(_) => 0,
            PcOp::Deposit(_) => 1,
        };
        if opened != Some(row) {
            if opened.is_some() {
                // Back to the PC main menu: B out of the box screen, NO to "Continue?".
                press_until(s, "the PC main menu", keys::B, 900, |s| {
                    menu::task(s, "Task_PCMainMenu")
                })?;
            }
            open_storage(s, center_map, row)?;
            wait_until(s, "the storage screen", 900, |s| {
                storage_ready(s, "Task_PokeStorageMain")
            })?;
            s.hf.idle(8)?;
            // `gStorage->boxOption` (offset 1): OPTION_WITHDRAW 0, OPTION_DEPOSIT 1.
            let g = s.sym("gStorage");
            let emu = s.hf.rec.emu();
            let gp = emu.read32(g);
            let opt = emu.read8(gp + 1) as i8;
            if opt != row {
                return Err(PlayError::Death(format!(
                    "storage opened with option {opt}, wanted {row}"
                )));
            }
            opened = Some(row);
        }
        match op {
            PcOp::Deposit(slot) => deposit(s, slot)?,
            PcOp::Withdraw(sp) => withdraw(s, sp)?,
        }
    }
    close_pc(s)
}

/// In the deposit screen: the party cursor to `slot` (UP/DOWN wrap through CANCEL at 6,
/// `HandleInput_InParty`, `pokemon_storage_system_data.c:1448`), A, STORE (row 0), the box chooser
/// (`Task_DepositMenu`, `pokemon_storage_system_tasks.c:1189`): the first box with room from the
/// one offered (`sChooseBoxMenu->curBox`, `struct ChooseBoxMenu` at 0x244,
/// `pokemon_storage_system_internal.h:228-242`).
fn deposit(s: &mut Session, slot: usize) -> PResult<()> {
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let Some(mon) = party.get(slot).cloned() else {
        return Err(PlayError::Fatal(format!("no party slot {slot} to deposit")));
    };
    for _ in 0..10 {
        let (_, pos) = storage_cursor(s);
        if pos == slot as i8 {
            break;
        }
        let k = if (slot as i8) > pos {
            keys::DOWN
        } else {
            keys::UP
        };
        s.hf.press(k)?;
        s.hf.idle(6)?;
    }
    s.hf.press(keys::A)?;
    wait_until(s, "the mon's storage menu", 300, selection_menu_up)?;
    menu::choose(s, 0)?; // STORE
    wait_until(s, "the box chooser", 300, |s| {
        menu::task(s, "Task_DepositMenu") && storage_state(s) == 1
    })?;
    s.hf.idle(4)?;
    let menu_ptr = {
        let p = s.sym("sChooseBoxMenu");
        s.hf.rec.emu().read32(p)
    };
    let cur = s.hf.rec.emu().read8(menu_ptr + 0x244) as u32;
    let target = (0..14u32)
        .map(|k| (cur + k) % 14)
        .find(|&b| box_species(s, b).contains(&0))
        .ok_or_else(|| PlayError::Fatal("every box is full".into()))?;
    for _ in 0..14 {
        let c = s.hf.rec.emu().read8(menu_ptr + 0x244) as u32;
        if c == target {
            break;
        }
        s.hf.press(keys::RIGHT)?;
        s.hf.idle(4)?;
    }
    if target != cur {
        s.event(
            "decision",
            format!("box {} is full: depositing in box {}", cur + 1, target + 1),
            json!({"from_box": cur, "box": target}),
        );
    }
    s.hf.press(keys::A)?;
    wait_until(s, "the deposit", 600, |s| {
        storage_ready(s, "Task_PokeStorageMain")
    })?;
    s.hf.idle(8)?;
    let n = super::names::names();
    s.event(
        "decision",
        format!(
            "deposited {} Lv{} in box {}",
            n.species(mon.species).trim_start_matches("SPECIES_"),
            mon.level,
            target + 1
        ),
        json!({"species": n.species(mon.species), "slot": slot, "box": target}),
    );
    Ok(())
}

/// In the withdraw screen (cursor in the box, `CURSOR_AREA_IN_BOX`, 6 x 5): find the species,
/// scroll to its box from the box title (UP from the top row, then LEFT/RIGHT,
/// `HandleInput_BoxTitle`), the cursor to its slot, A, WITHDRAW (row 0).
fn withdraw(s: &mut Session, species: u16) -> PResult<()> {
    let n = super::names::names();
    let cur = current_box(s);
    let found = (0..14u32)
        .map(|k| (cur + k) % 14)
        .find_map(|b| {
            box_species(s, b)
                .iter()
                .position(|&x| x == species)
                .map(|p| (b, p as i8))
        })
        .ok_or_else(|| PlayError::Fatal(format!("{} is in no box", n.species(species))))?;
    let (b, pos) = found;
    if b != cur {
        // Up to the box title, then scroll.
        for _ in 0..8 {
            if storage_cursor(s).0 == 2 {
                break;
            }
            s.hf.press(keys::UP)?;
            s.hf.idle(6)?;
        }
        for _ in 0..14 {
            if current_box(s) == b {
                break;
            }
            s.hf.press(keys::RIGHT)?;
            wait_until(s, "the box scroll", 120, |s| {
                storage_ready(s, "Task_PokeStorageMain")
            })?;
            s.hf.idle(4)?;
        }
        s.hf.press(keys::DOWN)?;
        s.hf.idle(6)?;
    }
    for _ in 0..16 {
        let (area, p) = storage_cursor(s);
        if area == 0 && p == pos {
            break;
        }
        let (cx, cy) = if area == 0 { (p % 6, p / 6) } else { (0, -1) };
        let (tx, ty) = (pos % 6, pos / 6);
        let k = if cy < ty {
            keys::DOWN
        } else if cy > ty {
            keys::UP
        } else if cx < tx {
            keys::RIGHT
        } else {
            keys::LEFT
        };
        s.hf.press(k)?;
        s.hf.idle(6)?;
    }
    s.hf.press(keys::A)?;
    wait_until(s, "the mon's storage menu", 300, selection_menu_up)?;
    menu::choose(s, 0)?; // WITHDRAW
    wait_until(s, "the withdraw", 600, |s| {
        storage_ready(s, "Task_PokeStorageMain")
    })?;
    s.hf.idle(8)?;
    s.event(
        "decision",
        format!(
            "withdrew {} from box {}",
            n.species(species).trim_start_matches("SPECIES_"),
            b + 1
        ),
        json!({"species": n.species(species), "box": b, "pos": pos}),
    );
    Ok(())
}

/// Use `item` from the bag on the field (Repel, Max Repel, an Escape Rope...): Bag -> item -> USE,
/// its message, back out.
pub fn use_item(s: &mut Session, item: &str) -> PResult<()> {
    let it = id(item);
    let before = Ram::new(&s.obs).count(s.hf.rec.emu(), it);
    if before == 0 {
        return Err(PlayError::Fatal(format!("{item} is not in the bag")));
    }
    open_bag(s)?;
    super::items::bag_use(s, it, 0)?;
    // The item's message prints in the bag, then the bag takes input again (or the field returns).
    let start = s.hf.fed();
    loop {
        if is_free(s) || menu::task(s, "Task_BagMenu_HandleInput") && !super::prims::fade_active(s)
        {
            break;
        }
        if s.hf.fed() - start > 2000 {
            return Err(PlayError::Death(format!("soft-lock: using {item}")));
        }
        s.hf.press(keys::A)?;
        s.hf.idle(2)?;
    }
    close_to_field(s)?;
    let after = Ram::new(&s.obs).count(s.hf.rec.emu(), it);
    s.event(
        "decision",
        format!("used {} ({after} left)", item.trim_start_matches("ITEM_")),
        json!({"item": item, "left": after, "used": before.saturating_sub(after)}),
    );
    Ok(())
}

/// Use `item` from the bag on party slot `slot` (an evolution stone, a Rare Candy): Bag -> item ->
/// USE -> the party menu -> the mon. An evolution scene is mashed with A only (B stops it:
/// `decompiled/src/evolution_scene.c`). Returns the mon's species afterwards.
pub fn use_item_on(s: &mut Session, item: &str, slot: usize) -> PResult<u16> {
    let it = id(item);
    let ram_party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let before = ram_party
        .get(slot)
        .ok_or_else(|| PlayError::Fatal(format!("no party slot {slot}")))?
        .clone();
    open_bag(s)?;
    super::items::bag_use(s, it, 0)?;
    super::items::party_choose(s, slot as i8)?;
    // Evolution scene (A only), the "grew to Lv" / "evolved" text, back to the bag or the field.
    let start = s.hf.fed();
    loop {
        let emu = s.hf.rec.emu();
        let evo = s.obs.callback2_is(emu, "CB2_EvolutionSceneUpdate");
        if !evo
            && (is_free(s)
                || (menu::task(s, "Task_BagMenu_HandleInput") && !super::prims::fade_active(s)))
        {
            break;
        }
        if s.hf.fed() - start > 6000 {
            return Err(PlayError::Death(format!(
                "soft-lock: using {item} on slot {slot}"
            )));
        }
        // A move-learn prompt after a candy level: decline (the carry's moves are the battle
        // bridge's business; a dex mon keeps what it has).
        if menu::yesno_up(s) {
            menu::answer(s, false)?;
            continue;
        }
        s.hf.press(keys::A)?;
        s.hf.idle(2)?;
    }
    close_to_field(s)?;
    let after = Ram::new(&s.obs).party(s.hf.rec.emu());
    let now = after.get(slot).map_or(0, |m| m.species);
    let n = names();
    s.event(
        "decision",
        format!(
            "used {} on {}: now {}",
            item.trim_start_matches("ITEM_"),
            n.species(before.species).trim_start_matches("SPECIES_"),
            n.species(now).trim_start_matches("SPECIES_")
        ),
        json!({"item": item, "slot": slot, "before": n.species(before.species), "after": n.species(now)}),
    );
    let _ = here(s);
    Ok(now)
}
