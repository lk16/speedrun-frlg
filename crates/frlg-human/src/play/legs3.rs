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
            eprintln!("reset after {i} frames: {}", s.obs.callback2_name(s.hf.rec.emu()));
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
    mva(s, "Hall of Fame 1 -> continue", t0, "hall-of-fame-1", "fly-pallet-2");
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
        return Err(PlayError::Fatal("no National Dex after the lab scene".into()));
    }
    let n = super::dex::caught_count(s);
    s.event(
        "decision",
        format!("National Dex received ({n} caught)"),
        json!({"caught": n}),
    );
    mva(s, "continue -> National Dex", t0, "fly-pallet-2", "oak-national-dex");
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
            return Err(PlayError::Death("soft-lock: the National Dex scenes".into()));
        }
        s.hf.step(0)?;
    }
}

#[allow(dead_code)]
fn unused(s: &mut Session) -> PResult<()> {
    let _ = (flag(s, "FLAG_SYS_GAME_CLEAR"), have(s, "ITEM_RUBY"), hm_opts());
    let _ = (Ram::new(&s.obs), Opts::default());
    let _ = go as fn(&mut Session, &str, i16, i16, &Opts) -> PResult<()>;
    let _ = go_face as fn(&mut Session, &str, i16, i16, &Opts) -> PResult<()>;
    let _ = go_map as fn(&mut Session, &str, &Opts) -> PResult<()>;
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
    ]
}
