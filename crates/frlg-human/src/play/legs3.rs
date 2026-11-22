//! The post-game, from the first Hall of Fame to the first full black frame of the second one, in
//! the plan's order (`route_wr.rs` after `hall-of-fame-1`, `route.rs` `leg_e4_2`), with the gating
//! derived from the scripts (each stage cites its own):
//!
//! 1. Soft reset after the Hall of Fame save, CONTINUE (the continue warp is Pallet Town).
//! 2. Oak's rating scene in Pallet -> the lab -> the National Dex.
//! 3. One Island: Celio asks for the Ruby (needs the National Dex).
//! 4. Kindle Road, the two Mt. Ember grunts, the Ruby Path to the Ruby on B5F.
//! 5. Celio takes the Ruby -> the Rainbow Pass (Seagallop to islands 4-7).
//! 6. Four Island's Icefall Cave: Lorelei's scene (it removes the Ruin Valley scientist).
//! 7. Six Island: Cut on the Ruin Valley braille, the Dotted Hole, the Sapphire scene (the thief
//!    tells the warehouse password).
//! 8. Five Island: the Meadow, the Rocket Warehouse, Gideon -> the Sapphire.
//! 9. Celio takes the Sapphire: the upgraded League and Cerulean Cave open.
//! 10. Cerulean Cave's Mewtwo (`mewtwo: true`), Master Ball, the plan's TMs.
//! 11. The upgraded Elite Four and Champion, the second Hall of Fame up to the black frame.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::legs::{center_heal, split, take_warp, talk_to};
use super::legs2::{flag, have, hm_opts, var};
use super::menu;
use super::prims::{settle, wait_until};
use super::ram::Ram;
use super::session::{PResult, PlayError, Session, Stage};
use super::walk::{go, go_face, go_map, go_to, here, Node, Opts};

/// The model's no-reset clock (p50, seconds) after `seg`, or NaN.
fn p50(s: &Session, seg: &str) -> f64 {
    s.model.p50(seg).unwrap_or(f64::NAN)
}

/// A `model-vs-actual` line for a stretch: played seconds from `t0` against the model's p50
/// difference between the segments `from` and `to`.
fn mva(s: &mut Session, what: &str, t0: f64, from: &str, to: &str) {
    let t = s.timer().unwrap_or(0.0);
    let model = p50(s, to) - p50(s, from);
    s.event(
        "model-vs-actual",
        format!(
            "{what}: played {:.0}s vs model {:.0}s ({from} -> {to})",
            t - t0,
            model
        ),
        json!({"played": t - t0, "model": model, "from": from, "to": to}),
    );
}

/// The Hall of Fame task has saved: `Task_Hof_TrySaveData` calls `TrySavingData(SAVE_HALL_OF_FAME)`
/// (synchronous) and moves on to `Task_Hof_DelayAfterSave`, then the mon display
/// (`decompiled/src/hall_of_fame.c:463-476`).
fn hof_saved(s: &mut Session) -> bool {
    [
        "Task_Hof_DelayAfterSave",
        "Task_Hof_StartDisplayingMons",
        "Task_Hof_DisplayMon",
        "Task_Hof_PlayMonCryAndPrintInfo",
        "Task_Hof_TryDisplayAnotherMon",
    ]
    .iter()
    .any(|t| menu::task(s, t))
}

/// `gQuestLogState` is `QL_STATE_PLAYBACK` (2) or `QL_STATE_PLAYBACK_LAST` (3)
/// (`decompiled/include/constants/quest_log.h:5,23`).
fn quest_log_playing(s: &mut Session) -> bool {
    let a = s.sym("gQuestLogState");
    matches!(s.hf.rec.emu().read8(a), 2 | 3)
}

/// "hall-of-fame-1" -> "fly-pallet-2": the Hall of Fame saves the game before it shows the team
/// (`Task_Hof_InitTeamSaveData` -> `Task_Hof_TrySaveData`, `decompiled/src/hall_of_fame.c:428-468`),
/// after `EnterHallOfFame` set `FLAG_SYS_GAME_CLEAR` and the continue warp to Pallet Town
/// (`decompiled/src/post_battle_event_funcs.c:12-33`) and the room script called
/// `EventScript_SetDefeatedEliteFourFlagsVars` (`VAR_MAP_SCENE_PALLET_TOWN_OAK` 2,
/// `decompiled/data/scripts/hall_of_fame.inc:1-12`). Nothing after the save is needed, so the human
/// soft-resets (A+B+START+SELECT, `decompiled/src/main.c:182-190`) once "Saving..." is done and
/// skips the team display and the credits; then title, CONTINUE (the first main-menu row,
/// `decompiled/src/main_menu.c`), and B to end the quest log recap (`QL_HandleInput`,
/// `decompiled/src/quest_log.c:1003-1008`).
fn hof1_reset(s: &mut Session) -> PResult<()> {
    s.seg = "hall-of-fame-1".into();
    let t0 = s.timer().unwrap_or(0.0);
    let n = super::dex::caught_count(s);
    s.event(
        "decision",
        format!("Pokedex at the first Hall of Fame: {n} caught (Oak's check wants 60)"),
        json!({"caught": n, "need": 60, "cite": "decompiled/data/maps/PalletTown/scripts.inc:51-57"}),
    );
    wait_until(s, "the Hall of Fame save", 4000, hof_saved)?;
    s.hf.react()?;
    s.event(
        "decision",
        "Hall of Fame saved: soft reset (skips the team display and the credits)",
        json!({"cite": "decompiled/src/hall_of_fame.c:463-468"}),
    );
    let combo = keys::A | keys::B | keys::START | keys::SELECT;
    let mut seen = vec![];
    for _ in 0..4 {
        s.hf.step(combo)?;
        let m = s.sym("gMain");
        seen.push(s.hf.rec.emu().read16(m + 0x28));
        if !super::prims::in_hall_of_fame(s) {
            break;
        }
    }
    eprintln!("held raw during the combo: {seen:x?}");
    s.hf.step(0)?;
    for i in 0..600 {
        if !super::prims::in_hall_of_fame(s) {
            eprintln!(
                "reset after {i} frames: {}",
                s.obs.callback2_name(s.hf.rec.emu())
            );
            break;
        }
        s.hf.step(0)?;
    }
    if super::prims::in_hall_of_fame(s) {
        let a = s.sym("gSoftResetDisabled");
        let d = s.hf.rec.emu().read8(a);
        let m = s.sym("gMain");
        let held = s.hf.rec.emu().read16(m + 0x28);
        let raw = s.hf.rec.emu().read16(m + 0x2c);
        return Err(PlayError::Fatal(format!(
            "the soft reset did not happen (gSoftResetDisabled {d}, gMain+0x28 {held:#x} +0x2c {raw:#x})"
        )));
    }
    // Copyright, intro, title, main menu: A (each press late); CONTINUE is the first row.
    let obs = s.obs.clone();
    s.hf.human_mash_until("the continued game", keys::A, 4000, |emu| {
        obs.callback2_is(emu, "CB2_Overworld")
    })?;
    // The recap: B ends it.
    let start = s.frame();
    while s.frame() - start < 3000 {
        if quest_log_playing(s) {
            s.hf.press(keys::B)?;
            s.hf.idle(4)?;
            continue;
        }
        if super::prims::is_free(s) || s.obs.field_controls_locked(s.hf.rec.emu()) {
            break;
        }
        s.hf.step(0)?;
    }
    mva(
        s,
        "Hall of Fame 1 -> continue",
        t0,
        "hall-of-fame-1",
        "fly-pallet-2",
    );
    Ok(())
}

/// "pallet-to-oak" + "oak-national-dex": on the continue warp (Pallet Town (6, 8),
/// `decompiled/src/data/heal_locations.json`) Oak's rating scene runs on the first frame
/// (`PalletTown_OnFrame`, `VAR_MAP_SCENE_PALLET_TOWN_OAK` 2): it needs `FLAG_WORLD_MAP_ONE_ISLAND`
/// and `GetPokedexCount`'s caught count >= 60, then walks the player into the lab
/// (`decompiled/data/maps/PalletTown/scripts.inc:42-78`), where the rival scene ends in
/// `special EnableNationalPokedex` (`PalletTown_ProfessorOaksLab/scripts.inc:54-110`). With fewer
/// than 60 the scene ends early and the lab's Oak re-checks when talked to
/// (`PalletTown_ProfessorOaksLab_EventScript_TryStartNationalDexScene`, same file :904-911): the
/// plan's `fill` hunts are run first.
fn national_dex(s: &mut Session) -> PResult<()> {
    s.seg = "oak-national-dex".into();
    let t0 = s.timer().unwrap_or(0.0);
    let lab_done = |s: &mut Session| var(s, "VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB") >= 8;
    settle(s)?;
    if !lab_done(s) {
        settle_until_lab(s)?;
    }
    if !lab_done(s) {
        let n = super::dex::caught_count(s);
        s.event(
            "model-vs-actual",
            format!("Oak's check failed: {n} caught < 60; running the plan's fill hunts"),
            json!({"caught": n}),
        );
        if std::env::var_os("FRLG_PG_DEV_DEX").is_some() {
            dev_poke_dex(s, 60);
        }
        for h in s.plan.params.fill.clone() {
            if super::dex::caught_count(s) >= 60 {
                break;
            }
            let rep = super::dex::hunt(s, &h)?;
            s.event(
                "model-vs-actual",
                format!("fill hunt {}: {:?}", h.site, rep),
                json!({"site": h.site}),
            );
        }
        if super::dex::caught_count(s) < 60 {
            return Err(PlayError::Fatal("still fewer than 60 caught".into()));
        }
        talk_to(s, "MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB", oak_lab_id(s))?;
        settle_until_lab(s)?;
    }
    if !lab_done(s) {
        return Err(PlayError::Fatal(
            "no National Dex after the lab scene".into(),
        ));
    }
    let n = super::dex::caught_count(s);
    s.event(
        "decision",
        format!("National Dex received ({n} caught)"),
        json!({"caught": n}),
    );
    mva(
        s,
        "continue -> National Dex",
        t0,
        "fly-pallet-2",
        "oak-national-dex",
    );
    split(s, "National Dex", "oak-national-dex")
}

/// DEVELOPMENT ONLY (`FRLG_PG_DEV_DEX` set): mark Kanto species owned in RAM until `want` are, so
/// the stages after the National Dex can be developed from a run whose dex fill fell short. A run
/// made with it is not a run (logged as a `warn`); the run of record never sets it.
fn dev_poke_dex(s: &mut Session, want: u32) {
    let have = super::dex::owned_set(s);
    let mut n = have.iter().filter(|&&b| b).count() as u32;
    let sb2 = s.obs.save_block2(s.hf.rec.emu()).expect("save block 2");
    let sb1 = s.obs.save_block1(s.hf.rec.emu()).expect("save block 1");
    // owned + seen (`struct Pokedex` at SaveBlock2 + 0x18) and the anticheat copies `seen1` /
    // `seen2` (SaveBlock1 + 0x5F8 / 0x3A18, `decompiled/include/global.h:783,812`;
    // `DexScreen_GetSetPokedexFlag`, `decompiled/src/pokedex_screen.c:2248-2256`).
    let addrs = [sb2 + 0x28, sb2 + 0x5C, sb1 + 0x5F8, sb1 + 0x3A18];
    for (nat, &own) in have.iter().enumerate().take(152).skip(1) {
        if n >= want {
            break;
        }
        if own {
            continue;
        }
        let i = (nat - 1) as u32;
        for base in addrs {
            let a = base + i / 8;
            let v = s.hf.rec.emu().read8(a) | (1 << (i % 8));
            s.hf.rec.emu().write8(a, v);
        }
        n += 1;
    }
    s.event(
        "warn",
        format!("DEV: Pokedex owned flags poked to {n} (not a run)"),
        json!({"dev": true, "caught": n}),
    );
}

fn oak_lab_id(s: &Session) -> u8 {
    let g = s.graph();
    g.maps[&g.id("MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB")]
        .objects
        .iter()
        .find(|o| o.script.contains("ProfOak"))
        .map_or(1, |o| o.local_id)
}

/// Settle through the Pallet -> lab scenes: the walk into the lab is a warp inside a script, and the
/// lab's own scene starts on its first frame.
fn settle_until_lab(s: &mut Session) -> PResult<()> {
    let lab = s.graph().id("MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB");
    let start = s.frame();
    loop {
        settle(s)?;
        let in_lab = here(s).is_some_and(|n| n.0 == lab);
        if var(s, "VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB") >= 8
            || (!in_lab && var(s, "VAR_MAP_SCENE_PALLET_TOWN_OAK") >= 3 && s.frame() - start > 60)
        {
            return Ok(());
        }
        if s.frame() - start > 20_000 {
            return Err(PlayError::Death(
                "soft-lock: the National Dex scenes".into(),
            ));
        }
        s.hf.step(0)?;
    }
}

/// Is `trainer` (a `TRAINER_*` name) beaten: its flag `TRAINER_FLAGS_START` + id
/// (`decompiled/include/constants/flags.h`).
fn trainer_beaten(s: &mut Session, trainer: &str) -> bool {
    let n = super::names::names();
    let (Some(base), Some(id)) = (n.get("TRAINER_FLAGS_START"), n.get(trainer)) else {
        return false;
    };
    s.obs.flag(s.hf.rec.emu(), base + id).unwrap_or(false)
}

/// The local id of the object of `map` whose script contains `script` (`map.json`).
fn obj_id(s: &Session, map: &str, script: &str) -> PResult<u8> {
    let g = s.graph();
    g.maps[&g.id(map)]
        .objects
        .iter()
        .find(|o| o.script.contains(script))
        .map(|o| o.local_id)
        .ok_or_else(|| PlayError::Fatal(format!("no object {script} on {map}")))
}

/// Walk up to the object of `map` running `script`, talk, and settle the script.
fn talk_script(s: &mut Session, map: &str, script: &str) -> PResult<()> {
    let id = obj_id(s, map, script)?;
    talk_to(s, map, id)?;
    settle(s)
}

fn decomp() -> std::path::PathBuf {
    for v in ["FRLG_DECOMP", "FRLG_DECOMP_RO"] {
        if let Some(p) = std::env::var_os(v) {
            return std::path::PathBuf::from(p);
        }
    }
    std::path::PathBuf::from("decompiled")
}

/// The region map layouts in `REGIONMAP_*` order (`decompiled/src/region_map.c:29-34`: Kanto,
/// Sevii 1-3, Sevii 4-5, Sevii 6-7), `decompiled/src/data/region_map/region_map_layout_*.h`.
const LAYOUTS: [&str; 4] = [
    "region_map_layout_kanto.h",
    "region_map_layout_sevii_123.h",
    "region_map_layout_sevii_45.h",
    "region_map_layout_sevii_67.h",
];

/// (region, x, y) of the first `LAYER_MAP` cell holding `mapsec`.
fn region_cell(mapsec: &str) -> Option<(u8, i16, i16)> {
    for (r, f) in LAYOUTS.iter().enumerate() {
        let text = std::fs::read_to_string(decomp().join("src/data/region_map").join(f)).ok()?;
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
                return Some((r as u8, x as i16, y));
            }
            y += 1;
        }
    }
    None
}

/// The region the fly map opens on: the one holding the current map's `region_map_section`
/// (`GetRegionMapPlayerIsOn`, `decompiled/src/region_map.c`).
fn region_here(s: &mut Session) -> u8 {
    let Some(n) = here(s) else { return 0 };
    let name = s.graph().maps[&n.0].name.clone();
    let sec = std::fs::read_to_string(decomp().join(format!("data/maps/{name}/map.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| j["region_map_section"].as_str().map(str::to_string))
        .unwrap_or_default();
    // Dungeons (Mt. Ember, ...) are on the `LAYER_DUNGEON` layer only: any layer counts.
    LAYOUTS
        .iter()
        .position(|f| {
            std::fs::read_to_string(decomp().join("src/data/region_map").join(f)).is_ok_and(|t| {
                t.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .any(|w| w == sec)
            })
        })
        .unwrap_or(0) as u8
}

/// Fly to `mapsec`, which must be in the player's region: the fly map is `REGIONMAP_TYPE_FLY`,
/// which has no SWITCH button (`sRegionMapPermissions`, `decompiled/src/region_map.c:595-617`), and
/// it opens on the region the player is on. Kanto <-> Sevii and Sevii 1-3 <-> 4-5 <-> 6-7 are
/// Seagallop trips ([`seagallop`]).
fn fly_to(s: &mut Session, mapsec: &str) -> PResult<()> {
    let (region, tx, ty) =
        region_cell(mapsec).ok_or_else(|| PlayError::Fatal(format!("no map cell for {mapsec}")))?;
    let from_region = region_here(s);
    if region != from_region {
        return Err(PlayError::Fatal(format!(
            "fly to {mapsec}: region {region}, the player is on region {from_region}"
        )));
    }
    if region == 0 {
        return super::items::fly(s, mapsec);
    }
    let from = here(s);
    super::items::use_field_move(s, "MOVE_FLY")?;
    wait_until(s, "the fly map", 600, |s| {
        menu::task(s, "Task_FlyMap") && super::items::fly_cursor(s).is_some()
    })?;
    s.hf.idle(20)?;
    cursor_to(s, (tx, ty))?;
    s.hf.press(keys::A)?;
    let start = s.hf.fed();
    loop {
        let n = here(s);
        if n.is_some() && n.map(|n| n.0) != from.map(|f| f.0) && super::prims::is_free(s) {
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
        json!({"mapsec": mapsec, "region": region}),
    );
    Ok(())
}

/// Move the fly map cursor onto `target` (one square per press; the cursor glides).
fn cursor_to(s: &mut Session, target: (i16, i16)) -> PResult<()> {
    for _ in 0..80 {
        let Some(c) = super::items::fly_cursor(s) else {
            s.hf.step(0)?;
            continue;
        };
        if c == target {
            return Ok(());
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
        for _ in 0..30 {
            s.hf.step(0)?;
            if super::items::fly_cursor(s) != Some(c) {
                break;
            }
        }
        s.hf.idle(2)?;
    }
    Err(PlayError::Death(format!(
        "soft-lock: fly cursor never reached {target:?}"
    )))
}

/// `SEAGALLOP_*` (`decompiled/include/constants/seagallop.h`): Vermilion 0, One .. Seven 1 .. 7.
const HARBORS: [&str; 8] = [
    "MAP_VERMILION_CITY",
    "MAP_ONE_ISLAND_HARBOR",
    "MAP_TWO_ISLAND_HARBOR",
    "MAP_THREE_ISLAND_HARBOR",
    "MAP_FOUR_ISLAND_HARBOR",
    "MAP_FIVE_ISLAND_HARBOR",
    "MAP_SIX_ISLAND_HARBOR",
    "MAP_SEVEN_ISLAND_HARBOR",
];

/// (page, row) of `to` in the Rainbow Pass destination menu opened at `from`
/// (`DrawSeagallopDestinationMenu` / `GetSelectedSeagallopDestination`,
/// `decompiled/src/script_menu.c:1234-1330`): page 0 lists four ports from Vermilion on, skipping
/// the origin, then OTHER and EXIT; page 1 lists three from Five Island (from Five on: Four),
/// skipping the origin, then OTHER, EXIT.
fn seagallop_row(from: u8, to: u8) -> (u8, i8) {
    let page0: Vec<u8> = (0..8u8).filter(|&d| d != from).take(4).collect();
    if let Some(r) = page0.iter().position(|&d| d == to) {
        return (0, r as i8);
    }
    let first = if from < 5 { 5 } else { 4 };
    let page1: Vec<u8> = (first..8u8).filter(|&d| d != from).take(3).collect();
    (1, page1.iter().position(|&d| d == to).unwrap_or(0) as i8)
}

/// Sail the Seagallop from port `from` to port `to` (`SEAGALLOP_*`). In Vermilion the ferry sailor
/// (24, 33) is talked to from the north; the island harbors' sailor is object 2 at (8, 6)
/// (`*_Harbor/map.json`). With the Tri-Pass only (Celio's scene var 1-4) Vermilion offers One,
/// Two, Three (`MULTICHOICE_SEAGALLOP_123`, `VermilionCity/scripts.inc:156-166`); with the Rainbow
/// Pass every port uses the paged menu (`EventScript_SeviiDestinationsPage1`,
/// `decompiled/data/scripts/seagallop.inc`).
fn seagallop(s: &mut Session, from: u8, to: u8) -> PResult<()> {
    let o = hm_opts();
    if from == 0 {
        go(s, "MAP_VERMILION_CITY", 24, 32, &o)?;
        super::walk::face(s, keys::DOWN)?;
        s.hf.press(keys::A)?;
    } else {
        talk_to(s, HARBORS[from as usize], 2)?;
    }
    super::menu::text_until_multichoice(s)?;
    if celio_var(s) < 5 {
        let row = if from == 0 { to as i8 - 1 } else { -1 };
        if row < 0 {
            return Err(PlayError::Fatal(format!(
                "Seagallop {from} -> {to} before the Rainbow Pass"
            )));
        }
        menu::choose(s, row)?;
    } else {
        let (page, row) = seagallop_row(from, to);
        if page == 1 {
            menu::choose(s, 4)?;
            s.hf.idle(4)?;
            wait_until(s, "the second destination page", 300, menu::multichoice_up)?;
        }
        menu::choose(s, row)?;
    }
    let d = s.graph().id(HARBORS[to as usize]);
    super::legs2::settle_until(s, &format!("the Seagallop to {to}"), 8000, move |s| {
        here(s).is_some_and(|n| n.0 == d)
    })?;
    s.event(
        "decision",
        format!(
            "Seagallop to {}",
            HARBORS[to as usize]
                .trim_start_matches("MAP_")
                .trim_end_matches("_HARBOR")
        ),
        json!({"from": from, "to": to}),
    );
    Ok(())
}

/// Shortest push sequence (BFS over player and boulders, in player moves) that lets the player
/// stand on one of `goals` on the current map. A boulder moves one tile when walked into and the
/// tile behind it is plain floor of the same elevation with nothing on it
/// (`decompiled/src/field_player_avatar.c` `TryPushBoulder`; the same rules as
/// `legs2::boulder_plan`); Rock Smash rocks and people are walls (no Rock Smash in this party).
fn boulder_walk_plan(
    m: &super::walk::MapInfo,
    start: (i16, i16),
    z_player: u8,
    boulders: Vec<(i16, i16)>,
    walls: &std::collections::HashSet<(i16, i16)>,
    goals: &[(i16, i16)],
) -> Option<Vec<super::legs2::Push>> {
    use std::collections::{HashMap, VecDeque};
    let elev = |p: (i16, i16)| m.tile(p.0, p.1).map_or(0, |t| t.elevation);
    let z0 = match z_player % 15 {
        0 => elev(start) % 15,
        z => z,
    };
    let mismatch = |z: u8, e: u8| z != 0 && e != 0 && e != 15 && e != z;
    let floor = |p: (i16, i16)| -> bool {
        m.tile(p.0, p.1).is_some_and(|t| {
            t.collision == 0
                && !super::walk::MB_WATER.contains(&t.behavior)
                && super::walk::ledge(t.behavior).is_none()
                && !super::walk::is_warp_tile(t.behavior)
        }) && !walls.contains(&p)
    };
    type St = ((i16, i16), u8, Vec<(i16, i16)>);
    let mut b0 = boulders;
    b0.sort();
    let s0: St = (start, z0, b0);
    let mut prev: HashMap<St, Option<(St, Option<super::legs2::Push>)>> = HashMap::new();
    prev.insert(s0.clone(), None);
    let mut q = VecDeque::from([s0]);
    let mut found = None;
    while let Some(st) = q.pop_front() {
        if goals.contains(&st.0) {
            found = Some(st);
            break;
        }
        if prev.len() > 600_000 {
            break;
        }
        let (p, z, bs) = (st.0, st.1, st.2.clone());
        for d in [(0i16, -1i16), (0, 1), (-1, 0), (1, 0)] {
            let n = (p.0 + d.0, p.1 + d.1);
            let (next, push): (St, _) = if let Some(k) = bs.iter().position(|&b| b == n) {
                if mismatch(z, elev(n)) {
                    continue;
                }
                let t = (n.0 + d.0, n.1 + d.1);
                if !floor(t) || bs.contains(&t) || mismatch(z, elev(t)) {
                    continue;
                }
                let mut nb = bs.clone();
                nb[k] = t;
                nb.sort();
                ((p, z, nb), Some(super::legs2::Push { stand: p, dir: d }))
            } else {
                let goal = goals.contains(&n) && !walls.contains(&n);
                if !(floor(n) || goal) {
                    continue;
                }
                let e = elev(n);
                if mismatch(z, e) {
                    continue;
                }
                let nz = if e == 15 || elev(p) == 15 || e == 0 {
                    z
                } else {
                    e
                };
                ((n, nz, bs.clone()), None)
            };
            if prev.contains_key(&next) {
                continue;
            }
            prev.insert(next.clone(), Some((st.clone(), push)));
            q.push_back(next);
        }
    }
    let mut cur = found?;
    let mut pushes = vec![];
    while let Some(Some((p, push))) = prev.get(&cur).cloned() {
        if let Some(x) = push {
            pushes.push(x);
        }
        cur = p;
    }
    pushes.reverse();
    Some(pushes)
}

/// Walk to map `to` (a neighbour by warp of the current map), pushing Strength boulders out of the
/// way when the plain planner finds no path ([`boulder_walk_plan`], `legs2::do_pushes`).
fn boulder_go_map(s: &mut Session, to: &str) -> PResult<()> {
    let o = hm_opts();
    let dest = s.graph().id(to);
    if super::walk::plan_len(s, None, &move |n: &Node| n.0 == dest, &o).is_some() {
        return go_map(s, to, &o);
    }
    let cur = here(s).ok_or("no position")?;
    let g = s.graph();
    let minfo = g.maps.get(&cur.0).cloned().ok_or("no map")?;
    let m = super::walk::live_map(s, cur.0).unwrap_or_else(|| minfo.clone());
    let goals: Vec<(i16, i16)> = minfo
        .warps
        .iter()
        .filter(|w| w.2 == Some(dest))
        .map(|w| (w.0, w.1))
        .collect();
    let live = Ram::new(&s.obs).objects(s.hf.rec.emu());
    let is_boulder = |id: u8| {
        minfo
            .objects
            .iter()
            .any(|o| o.local_id == id && o.is_boulder())
    };
    let mut boulders: Vec<(i16, i16)> = live
        .iter()
        .filter(|o| is_boulder(o.local_id))
        .map(|o| (o.x, o.y))
        .collect();
    let mut walls: std::collections::HashSet<(i16, i16)> = live
        .iter()
        .filter(|o| !is_boulder(o.local_id) && o.local_id != 0xFF)
        .map(|o| (o.x, o.y))
        .collect();
    for o in &minfo.objects {
        let loaded = live.iter().any(|l| l.local_id == o.local_id);
        let hidden = o.flag != 0 && s.obs.flag(s.hf.rec.emu(), o.flag).unwrap_or(false);
        if loaded || hidden {
            continue;
        }
        if o.is_boulder() {
            boulders.push((o.x, o.y));
        } else {
            walls.insert((o.x, o.y));
        }
    }
    let zp = Ram::new(&s.obs).player_elevation(s.hf.rec.emu());
    let pushes = boulder_walk_plan(&m, (cur.1, cur.2), zp, boulders.clone(), &walls, &goals)
        .ok_or_else(|| {
            PlayError::Fatal(format!(
                "{}: no push sequence to {to} from {:?} (boulders {boulders:?})",
                minfo.name,
                (cur.1, cur.2)
            ))
        })?;
    s.event(
        "decision",
        format!(
            "{}: {} Strength pushes toward {}",
            minfo.name,
            pushes.len(),
            to.trim_start_matches("MAP_")
        ),
        json!({"pushes": pushes.len(),
               "stands": pushes.iter().map(|p| [p.stand.0, p.stand.1]).collect::<Vec<_>>()}),
    );
    super::legs2::do_pushes(s, cur.0, &pushes)?;
    go_map(s, to, &o)
}

const CELIO_PC: &str = "MAP_ONE_ISLAND_POKEMON_CENTER_1F";

fn celio_var(s: &mut Session) -> u16 {
    var(s, "VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F")
}

/// "to-one-island" + "celio-ruby-request": fly to One Island (its fly flag is set from the first
/// visit) and talk to Celio. With the National Dex he asks for the Ruby
/// (`OneIsland_PokemonCenter_1F_EventScript_Celio` -> `CelioRequestRuby`:
/// `VAR_MAP_SCENE_MT_EMBER_EXTERIOR` 1, `VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F` 4,
/// `decompiled/data/maps/OneIsland_PokemonCenter_1F/scripts.inc:198-216,342-353`).
fn celio_ruby(s: &mut Session) -> PResult<()> {
    s.seg = "to-one-island".into();
    let t0 = s.timer().unwrap_or(0.0);
    // Fly works outdoors only: out of the lab first. One Island is a Seagallop trip from
    // Vermilion (no flying between regions, see `fly_to`).
    go_map(s, "MAP_PALLET_TOWN", &Opts::default())?;
    super::items::fly(s, "MAPSEC_VERMILION_CITY")?;
    seagallop(s, 0, 1)?;
    s.event(
        "model-vs-actual",
        "to-one-island: Fly cannot cross regions (REGIONMAP_TYPE_FLY has no switch button): \
         fly Vermilion + Seagallop",
        json!({"segment": "to-one-island", "cite": "decompiled/src/region_map.c:595-617"}),
    );
    s.seg = "celio-ruby-request".into();
    talk_script(s, CELIO_PC, "EventScript_Celio")?;
    if celio_var(s) < 4 {
        return Err(PlayError::Fatal(format!(
            "Celio did not ask for the Ruby (scene var {})",
            celio_var(s)
        )));
    }
    s.event("decision", "Celio asked for the Ruby", json!({}));
    mva(
        s,
        "National Dex -> Celio's request",
        t0,
        "oak-national-dex",
        "celio-ruby-request",
    );
    Ok(())
}

/// "kindle-road-in", the two grunts, "ruby-path": out of the center, north up Kindle Road (Surf) to
/// Mt. Ember. The cave door (42, 39) opens once the scene var is >= 4 (`MtEmber_Exterior_OnLoad`);
/// the grunts on (41, 40) and (42, 40) battle when talked to with the var at 4 and leave once both
/// are beaten (`MtEmber_Exterior_EventScript_Grunt1/2`, `...DefeatedBothGrunts`); the password scene
/// on the row y 43 (`RocketPasswordScene`, coord events (37..39, 43)) plays on the way
/// (`decompiled/data/maps/MtEmber_Exterior/scripts.inc`, `map.json`). The Ruby is the object at
/// (7, 7) of B5F (`MtEmber_RubyPath_B5F/map.json`, `FLAG_GOT_RUBY`).
fn mt_ember(s: &mut Session) -> PResult<()> {
    s.seg = "kindle-road-in".into();
    let t0 = s.timer().unwrap_or(0.0);
    go_map(s, "MAP_MT_EMBER_EXTERIOR", &hm_opts())?;
    for (seg, script, tr) in [
        (
            "mt-ember-grunt-1",
            "EventScript_Grunt1",
            "TRAINER_TEAM_ROCKET_GRUNT_43",
        ),
        (
            "mt-ember-grunt-2",
            "EventScript_Grunt2",
            "TRAINER_TEAM_ROCKET_GRUNT_44",
        ),
    ] {
        if trainer_beaten(s, tr) {
            continue;
        }
        s.seg = seg.into();
        talk_script(s, "MAP_MT_EMBER_EXTERIOR", script)?;
    }
    s.seg = "ruby-path".into();
    // The door's metatile is swapped in by the map script (the layout has rock there), so the
    // planner does not see the warp: stand below it and walk up.
    take_warp(s, "MAP_MT_EMBER_EXTERIOR", 42, 40, keys::UP)?;
    for f in ["B1F", "B2F", "B3F", "B4F", "B5F"] {
        boulder_go_map(s, &format!("MAP_MT_EMBER_RUBY_PATH_{f}"))?;
    }
    go_face(s, "MAP_MT_EMBER_RUBY_PATH_B5F", 7, 7, &hm_opts())?;
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_GOT_RUBY") {
        return Err(PlayError::Fatal("no Ruby after the B5F talk".into()));
    }
    s.event("decision", "got the Ruby (Ruby Path B5F)", json!({}));
    mva(
        s,
        "Kindle Road + Ruby Path in",
        t0,
        "celio-ruby-request",
        "ruby-path",
    );
    Ok(())
}

/// Back to Celio: walk out of the Ruby Path to the Mt. Ember exterior and fly to One Island (the
/// model surfs Kindle Road back, "kindle-road-out"); give the Ruby, YES to the favour
/// (`GiveCelioRuby` / `GiveRainbowPass`: the Rainbow Pass, `FLAG_SYS_SEVII_MAP_4567`,
/// `VAR_MAP_SCENE_ONE_ISLAND_POKEMON_CENTER_1F` 5, `OneIsland_PokemonCenter_1F/scripts.inc:289-335`).
fn celio_rainbow(s: &mut Session) -> PResult<()> {
    s.seg = "kindle-road-out".into();
    let t0 = s.timer().unwrap_or(0.0);
    // The boulders are back where the map puts them (object positions reload with the map): the
    // way up is pushed again, floor by floor (no Escape Rope in the bag).
    for f in ["B4F", "B3F", "B2F", "B1F", "1F"] {
        boulder_go_map(s, &format!("MAP_MT_EMBER_RUBY_PATH_{f}"))?;
    }
    boulder_go_map(s, "MAP_MT_EMBER_EXTERIOR")?;
    fly_to(s, "MAPSEC_ONE_ISLAND")?;
    s.seg = "celio-rainbow-pass".into();
    talk_script(s, CELIO_PC, "EventScript_Celio")?;
    if celio_var(s) < 5 || have(s, "ITEM_RAINBOW_PASS") == 0 {
        return Err(PlayError::Fatal("no Rainbow Pass from Celio".into()));
    }
    s.event("decision", "Celio took the Ruby: Rainbow Pass", json!({}));
    mva(
        s,
        "Ruby -> Rainbow Pass",
        t0,
        "ruby-path",
        "celio-rainbow-pass",
    );
    split(s, "Ruby", "celio-rainbow-pass")
}

#[allow(dead_code)]
fn unused(s: &mut Session) -> PResult<()> {
    let _ = Ram::new(&s.obs);
    let _ = go as fn(&mut Session, &str, i16, i16, &Opts) -> PResult<()>;
    let _ = (center_heal, take_warp);
    let _ = go_to;
    let _: Option<Node> = None;
    Ok(())
}

pub fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "hof1-reset",
            run: hof1_reset,
        },
        Stage {
            name: "national-dex",
            run: national_dex,
        },
        Stage {
            name: "celio-ruby",
            run: celio_ruby,
        },
        Stage {
            name: "mt-ember",
            run: mt_ember,
        },
        Stage {
            name: "celio-rainbow",
            run: celio_rainbow,
        },
    ]
}
