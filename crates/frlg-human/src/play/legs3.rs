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
use super::walk::{go, go_face, go_map, go_to, here, MapId, Node, Opts};

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
        fill_to_60(s)?;
        if super::dex::caught_count(s) < 60 {
            return Err(PlayError::Fatal("still fewer than 60 caught".into()));
        }
        leave_to_fly(s, "MAP_PALLET_TOWN")?;
        if here(s).is_none_or(|n| n.0 != s.graph().id("MAP_PALLET_TOWN")) {
            super::items::fly(s, "MAPSEC_PALLET_TOWN")?;
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

/// Is the map outdoors (Fly works: `MAP_TYPE_TOWN/CITY/ROUTE/OCEAN_ROUTE`, `map.json`
/// `map_type`; `Overworld_MapTypeAllowsTeleportAndFly`, `decompiled/src/overworld.c`)?
fn outdoors(s: &mut Session) -> bool {
    let Some(n) = here(s) else { return false };
    let name = s.graph().maps[&n.0].name.clone();
    std::fs::read_to_string(decomp().join(format!("data/maps/{name}/map.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| j["map_type"].as_str().map(str::to_string))
        .is_some_and(|t| {
            matches!(
                t.as_str(),
                "MAP_TYPE_TOWN" | "MAP_TYPE_CITY" | "MAP_TYPE_ROUTE" | "MAP_TYPE_OCEAN_ROUTE"
            )
        })
}

/// `MAP_FOO_BAR` -> its `data/maps/<Folder>` (the folder whose `map.json` has that id).
fn map_folder(s: &Session, id: &str) -> Option<String> {
    let g = s.graph();
    g.by_id.get(id).map(|m| g.maps[m].name.clone())
}

/// The fly point nearest a hunt site (`human-run/data/flydist.json`, `HEAL_LOCATION_*`) as
/// (heal location map, its `MAPSEC_*`).
fn fly_point(s: &Session, site: &str, area: &str) -> Option<(String, String)> {
    let fd: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("human-run/data/flydist.json").ok()?).ok()?;
    let e = fd.get(format!("{site}|{area}"))?;
    let hl = e["from"].as_str()?.to_string();
    let heal: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(decomp().join("src/data/heal_locations.json")).ok()?,
    )
    .ok()?;
    let map = heal["heal_locations"]
        .as_array()?
        .iter()
        .find(|h| h["id"] == hl.as_str())?["map"]
        .as_str()?
        .to_string();
    let folder = map_folder(s, &map)?;
    let j: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(decomp().join(format!("data/maps/{folder}/map.json"))).ok()?,
    )
    .ok()?;
    Some((map, j["region_map_section"].as_str()?.to_string()))
}

/// How many uncaught Kanto species a site's area offers, and their share of the encounters.
fn site_new(s: &mut Session, site: &str, area: &str) -> (u32, f64) {
    let a = match area {
        "water" => crate::data::Area::Water,
        _ => crate::data::Area::Land,
    };
    let lg = s.plan.params.version == "LG";
    let Some((_, slots, w)) = s.plan.dex.area(site, a, lg, "none") else {
        return (0, 0.0);
    };
    let owned = super::dex::owned_set(s);
    let mut seen = std::collections::HashSet::new();
    let mut share = 0.0;
    let tot: u32 = w.iter().sum::<u32>().max(1);
    for (i, sl) in slots.iter().enumerate() {
        let sp = &s.plan.dex.species[sl.species];
        if sp.dex == 0 || sp.dex > 151 || owned[sp.dex as usize] {
            continue;
        }
        share += w.get(i).copied().unwrap_or(0) as f64 / tot as f64;
        seen.insert(sl.species);
    }
    (seen.len() as u32, share)
}

/// Not in the model's run: the plan's 60 for Oak's check are caught before the League
/// (`route_wr.rs`: "All 60 were caught before the League"), but the played route reached the Hall
/// of Fame far short of it, and the plan's `fill` hunts are never anchored (`dex::dex_hook` runs
/// `hunts`, not `fill`). The human catches up here: Great Balls at the Celadon store (2F,
/// `CeladonCity_DepartmentStore_2F/scripts.inc`), the plan's `fill` sites in order, then the Kanto
/// site with the most uncaught species (greedy), each reached by flying to its nearest Fly point
/// (`human-run/data/flydist.json`), until 60 are caught. Every hunt is the dex module's
/// (`dex::hunt`: `sim::wants_catch`, the plan's ball rules).
fn fill_to_60(s: &mut Session) -> PResult<()> {
    let t0 = s.timer().unwrap_or(0.0);
    let start_n = super::dex::caught_count(s);
    let mut tried: Vec<String> = vec![];
    let fill: Vec<crate::params::Hunt> = s.plan.params.fill.clone();
    for round in 0..40 {
        let n = super::dex::caught_count(s);
        if n >= 60 {
            break;
        }
        let balls =
            have(s, "ITEM_GREAT_BALL") + have(s, "ITEM_POKE_BALL") + have(s, "ITEM_ULTRA_BALL");
        if balls < 8 {
            let want = (2 * (60 - n) as u16).clamp(12, 50);
            let money = Ram::new(&s.obs).money(s.hf.rec.emu());
            // Keep the League's Full Restores (`league_heals`) and four Escape Ropes.
            let keep = s.plan.params.league_heals as u32 * 3000 + 4 * 550;
            let afford = (money.saturating_sub(keep) / 600) as u16;
            let k = want.saturating_sub(balls).min(afford);
            if k == 0 {
                return Err(PlayError::Fatal(format!(
                    "dex fill: {n} caught, no balls and no money for them"
                )));
            }
            leave_to_fly(s, "MAP_CELADON_CITY")?;
            super::items::fly(s, "MAPSEC_CELADON_CITY")?;
            super::shop::buy(
                s,
                "MAP_CELADON_CITY_DEPARTMENT_STORE_2F",
                Some((1, 8)),
                &[("ITEM_GREAT_BALL", k)],
            )?;
        }
        let mut next = None;
        for h in &fill {
            if !tried.contains(&h.site) && site_new(s, &h.site, &h.area).0 > 0 {
                next = Some(h.clone());
                break;
            }
        }
        let next = match next {
            Some(h) => Some(h),
            None => greedy_site(s, &tried),
        };
        let Some(mut h) = next else {
            return Err(PlayError::Fatal(format!(
                "dex fill: {n} caught, no site left"
            )));
        };
        tried.push(h.site.clone());
        if h.steps == 0 {
            h.steps = 120;
        }
        if let Some((map, sec)) = fly_point(s, &h.site, &h.area) {
            let fl = format!("FLAG_WORLD_MAP_{}", map.trim_start_matches("MAP_"));
            let known = super::names::names().get(&fl).is_some();
            if (!known || flag(s, &fl)) && region_cell(&sec).is_some_and(|c| c.0 == 0) {
                leave_to_fly(s, &map)?;
                super::items::fly(s, &sec)?;
            }
        }
        let (k, share) = site_new(s, &h.site, &h.area);
        s.event(
            "decision",
            format!(
                "dex fill {round}: {} ({k} uncaught species, {:.0}% of encounters), {n} caught",
                h.site.trim_start_matches("MAP_"),
                share * 100.0
            ),
            json!({"site": h.site, "new_species": k, "share": share, "caught": n}),
        );
        super::dex::hunt(s, &h)?;
    }
    let n = super::dex::caught_count(s);
    s.event(
        "model-vs-actual",
        format!(
            "dex fill after the Hall of Fame: {start_n} -> {n} caught in {:.0}s (the model has none here)",
            s.timer().unwrap_or(0.0) - t0
        ),
        json!({"from": start_n, "to": n, "seconds": s.timer().unwrap_or(0.0) - t0}),
    );
    if n < 60 {
        return Err(PlayError::Fatal(format!("dex fill stopped at {n} caught")));
    }
    Ok(())
}

/// The not-yet-tried Kanto site (land or water, `flydist.json`) with the most uncaught species;
/// ties go to the larger encounter share, then the shorter walk from its Fly point.
fn greedy_site(s: &mut Session, tried: &[String]) -> Option<crate::params::Hunt> {
    let fd: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string("human-run/data/flydist.json").ok()?).ok()?;
    let mut best: Option<(u32, f64, i64, String, String)> = None;
    for (key, v) in fd.as_object()? {
        let Some((site, area)) = key.split_once('|') else {
            continue;
        };
        if area == "fish" || tried.iter().any(|t| t == site) || site.contains("SAFARI") {
            continue;
        }
        if !s.graph().by_id.contains_key(site) {
            continue;
        }
        // Kanto only: no Fly across regions, and the hunt walks from the Fly point.
        if !fly_point(s, site, area)
            .is_some_and(|(_, sec)| region_cell(&sec).is_some_and(|c| c.0 == 0))
        {
            continue;
        }
        let (k, share) = site_new(s, site, area);
        if k == 0 {
            continue;
        }
        let tiles = v["tiles"].as_i64().unwrap_or(999);
        let better = match &best {
            None => true,
            Some(b) => (k, share, -tiles) > (b.0, b.1, b.2),
        };
        if better {
            best = Some((k, share, -tiles, site.to_string(), area.to_string()));
        }
    }
    let (_, _, _, site, area) = best?;
    Some(crate::params::Hunt {
        site,
        area,
        steps: 120,
        ..Default::default()
    })
}

/// Get somewhere Fly works: outdoors already, else an Escape Rope if the bag has one, else walk
/// to `town` (out of the cave or building the last hunt ended in).
fn leave_to_fly(s: &mut Session, town: &str) -> PResult<()> {
    if outdoors(s) {
        return Ok(());
    }
    if have(s, "ITEM_ESCAPE_ROPE") > 0 {
        let from = here(s);
        super::shop::use_item(s, "ITEM_ESCAPE_ROPE")?;
        super::legs2::settle_until(s, "the Escape Rope", 3000, move |s| here(s) != from)?;
        if outdoors(s) {
            return Ok(());
        }
    }
    // Walk out to the nearest outdoor map (not all the way to `town`: a cave hunt ends far from
    // it), then Fly.
    let _ = town;
    let g = s.graph();
    let out: std::collections::HashSet<MapId> = g
        .maps
        .iter()
        .filter(|(_, m)| map_is_outdoors(&m.name))
        .map(|(id, _)| *id)
        .collect();
    go_to(
        s,
        "outdoors",
        &move |n: &Node| out.contains(&n.0),
        &hm_opts(),
    )
}

/// `map_type` of a map folder allows Fly (see [`outdoors`]).
fn map_is_outdoors(name: &str) -> bool {
    std::fs::read_to_string(decomp().join(format!("data/maps/{name}/map.json")))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|j| j["map_type"].as_str().map(str::to_string))
        .is_some_and(|t| {
            matches!(
                t.as_str(),
                "MAP_TYPE_TOWN" | "MAP_TYPE_CITY" | "MAP_TYPE_ROUTE" | "MAP_TYPE_OCEAN_ROUTE"
            )
        })
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
        // Surf may be needed to reach the harbor (Four Island's mart and cave side).
        go_map(s, HARBORS[from as usize], &o)?;
        talk_to(s, HARBORS[from as usize], 2)?;
    }
    // "Where do you want to sail?" is `message` + `waitmessage`: the list opens by itself, and a
    // mashed A would take its first row (`OneIsland_Harbor/scripts.inc` `Sailor`).
    wait_until(s, "the Seagallop list", 900, menu::multichoice_up)?;
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

// ----------------------------------------------------------------------------------------------
// Icefall Cave: ice that slides, thin ice that breaks, a waterfall.

const MB_WATERFALL: u16 = 0x13;
const MB_ICE: u16 = 0x23;
/// Icefall 1F's thin ice: a step on it cracks it, a step on the cracked tile breaks it and the
/// player falls to B1F at the same coordinates (`sIcefallCaveIceCoords`,
/// `IcefallCaveIcePerStepCallback`, `decompiled/src/field_tasks.c:51-62,172-245`;
/// `FourIsland_IcefallCave_1F_EventScript_FallDownHole`: `warphole MAP_FOUR_ISLAND_ICEFALL_CAVE_B1F`).
/// The cracks are temp flags (`FlagGet(i + 1)`), cleared on every map load.
const THIN_ICE: [(i16, i16); 9] = [
    (8, 3),
    (10, 5),
    (15, 5),
    (8, 9),
    (9, 9),
    (16, 9),
    (8, 10),
    (9, 10),
    (8, 14),
];

/// One planned move: the direction pressed and where the player stands after it (slides,
/// falls and warps included).
type IceMove = (u16, Node);

/// Dijkstra (in moves) over (map, x, y, cracked thin ice) across the maps `maps`: walls are
/// collision and map objects; warp tiles move to the destination warp; ice (`MB_ICE`) slides
/// on in the same direction until the next tile is blocked (`ForcedMovement_Slip` for
/// `MetatileBehavior_IsIce_2`, `decompiled/src/field_player_avatar.c:229`); water needs Surf and a
/// waterfall is climbed northward only (`GetInteractedWaterScript`,
/// `decompiled/src/field_control_avatar.c:600-615`).
fn ice_plan(
    s: &Session,
    maps: &[&str],
    start: Node,
    goal: &dyn Fn(&Node) -> bool,
) -> Option<Vec<IceMove>> {
    use std::collections::{BinaryHeap, HashMap};
    let g = s.graph();
    let ids: Vec<MapId> = maps.iter().map(|m| g.id(m)).collect();
    let f1 = g.id("MAP_FOUR_ISLAND_ICEFALL_CAVE_1F");
    let b1 = g.id("MAP_FOUR_ISLAND_ICEFALL_CAVE_B1F");
    let passable = |mid: MapId, x: i16, y: i16, key: u16| -> bool {
        let Some(m) = g.maps.get(&mid) else {
            return false;
        };
        let Some(t) = m.tile(x, y) else { return false };
        if m.objects.iter().any(|o| (o.x, o.y) == (x, y)) {
            return false;
        }
        if super::walk::is_warp_tile(t.behavior) {
            return true;
        }
        if t.collision != 0 {
            return false;
        }
        t.behavior != MB_WATERFALL || key == keys::UP
    };
    let warp_dest = |mid: MapId, x: i16, y: i16| -> Option<Node> {
        let m = g.maps.get(&mid)?;
        let w = m.warps.iter().find(|w| (w.0, w.1) == (x, y))?;
        let dm = w.2?;
        if !ids.contains(&dm) {
            return None;
        }
        let dw = g.maps.get(&dm)?.warps.get(w.3)?;
        Some((dm, dw.0, dw.1))
    };
    type St = (Node, u16);
    let s0: St = (start, 0);
    let mut dist: HashMap<St, u32> = HashMap::from([(s0, 0)]);
    let mut prev: HashMap<St, (St, u16)> = HashMap::new();
    let mut pq = BinaryHeap::from([std::cmp::Reverse((0u32, s0))]);
    let mut end = None;
    while let Some(std::cmp::Reverse((c, st))) = pq.pop() {
        if dist.get(&st).is_some_and(|&d| d < c) {
            continue;
        }
        if goal(&st.0) {
            end = Some(st);
            break;
        }
        let ((mid, x, y), cracked) = st;
        for (key, dx, dy) in [
            (keys::UP, 0i16, -1i16),
            (keys::DOWN, 0, 1),
            (keys::LEFT, -1, 0),
            (keys::RIGHT, 1, 0),
        ] {
            let (mut px, mut py) = (x + dx, y + dy);
            if !passable(mid, px, py, key) {
                continue;
            }
            let m = &g.maps[&mid];
            while m.tile(px, py).is_some_and(|t| t.behavior == MB_ICE)
                && passable(mid, px + dx, py + dy, key)
            {
                px += dx;
                py += dy;
            }
            let next: St = if mid == f1 && THIN_ICE.contains(&(px, py)) {
                let i = THIN_ICE.iter().position(|&p| p == (px, py)).unwrap();
                if cracked >> i & 1 != 0 {
                    ((b1, px, py), 0)
                } else {
                    ((mid, px, py), cracked | 1 << i)
                }
            } else if let Some(d) = warp_dest(mid, px, py) {
                (d, 0)
            } else {
                ((mid, px, py), cracked)
            };
            let nc = c + 1;
            if dist.get(&next).is_none_or(|&d| nc < d) {
                dist.insert(next, nc);
                prev.insert(next, (st, key));
                pq.push(std::cmp::Reverse((nc, next)));
            }
        }
    }
    let mut cur = end?;
    let mut out = vec![];
    while let Some(&(p, key)) = prev.get(&cur) {
        out.push((key, cur.0));
        cur = p;
    }
    out.reverse();
    Some(out)
}

/// Play an [`ice_plan`]: each move is one press of its direction, held until the player leaves
/// the tile, then settled (slides, falls, warps and battles run out). Water: A to Surf when
/// stepping off land, A at a waterfall to climb it (both scripts' yes/no default to YES). The
/// player's node is matched against the plan after every move (a slide or a climb can pass
/// several planned nodes).
fn ice_follow(s: &mut Session, plan: &[IceMove]) -> PResult<()> {
    let mut i = 0;
    let mut guard = 0;
    while i < plan.len() {
        guard += 1;
        if guard > plan.len() * 3 + 20 {
            return Err(PlayError::Death("soft-lock: the ice route".into()));
        }
        settle(s)?;
        let cur = here(s).ok_or("no position")?;
        let (key, to) = plan[i];
        let (dx, dy) = match key {
            keys::UP => (0, -1),
            keys::DOWN => (0, 1),
            keys::LEFT => (-1, 0),
            _ => (1, 0),
        };
        let g = s.graph();
        let t = g
            .maps
            .get(&cur.0)
            .and_then(|m| m.tile(cur.1 + dx, cur.2 + dy));
        let surfing = s.obs.player_avatar_flags(s.hf.rec.emu()) & super::walk::AVATAR_SURFING != 0;
        let beh = t.map_or(0, |t| t.behavior);
        if beh == MB_WATERFALL || (!surfing && super::walk::MB_WATER.contains(&beh)) {
            super::walk::face(s, key)?;
            s.hf.press(keys::A)?;
            settle(s)?;
        } else {
            // Hold the direction until the player leaves the tile or a script takes over (the
            // thin-ice fall locks the player on the tile it breaks under).
            let mut fed = 0;
            let mut held = 0;
            while here(s) == Some(cur) && held < 3 {
                if fed > 60 {
                    return Err(PlayError::Death(format!(
                        "soft-lock: ice route bump at {cur:?}"
                    )));
                }
                if super::prims::is_free(s) {
                    s.hf.step(key)?;
                    held = 0;
                } else {
                    s.hf.step(0)?;
                    held += 1;
                }
                fed += 1;
            }
            s.hf.step(0)?;
        }
        settle(s)?;
        let mut now = here(s).ok_or("no position")?;
        if std::env::var_os("FRLG_PG_DEBUG").is_some() {
            let lb = super::walk::live_map(s, now.0)
                .and_then(|m| m.tile(now.1, now.2))
                .map(|t| t.behavior);
            eprintln!("ice: {cur:?} key {key} -> {now:?} (planned {to:?}) live behavior {lb:x?}");
        }
        if now != to && to.0 != cur.0 && now.0 == cur.0 {
            // A fall or a warp that has not started yet (the broken ice's on-frame script).
            let dest = to.0;
            let start = s.frame();
            while s.frame() - start < 300 && here(s).is_none_or(|n| n.0 != dest) {
                if super::prims::is_free(s) {
                    s.hf.step(0)?;
                } else {
                    settle(s)?;
                }
            }
            settle(s)?;
            now = here(s).ok_or("no position")?;
        }
        if now == to {
            i += 1;
            continue;
        }
        if now == cur {
            // Only turned (a tap on a new direction turns in place): press again.
            continue;
        }
        match plan[i..].iter().position(|m| m.1 == now) {
            Some(k) => i += k + 1,
            None => {
                return Err(PlayError::Death(format!(
                    "ice route: at {now:?}, planned {to:?}"
                )))
            }
        }
    }
    Ok(())
}

/// Plan and play an Icefall route to `goal`.
fn ice_go(s: &mut Session, what: &str, goal: &dyn Fn(&Node) -> bool) -> PResult<()> {
    const MAPS: [&str; 4] = [
        "MAP_FOUR_ISLAND_ICEFALL_CAVE_ENTRANCE",
        "MAP_FOUR_ISLAND_ICEFALL_CAVE_1F",
        "MAP_FOUR_ISLAND_ICEFALL_CAVE_B1F",
        "MAP_FOUR_ISLAND_ICEFALL_CAVE_BACK",
    ];
    settle(s)?;
    let cur = here(s).ok_or("no position")?;
    let plan = ice_plan(s, &MAPS, cur, goal)
        .ok_or_else(|| PlayError::Fatal(format!("{what}: no ice route from {cur:?}")))?;
    s.event(
        "decision",
        format!("Icefall Cave: {} moves to {what}", plan.len()),
        json!({"moves": plan.len()}),
    );
    ice_follow(s, &plan)
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

fn party_slot(s: &mut Session, species: &str) -> Option<usize> {
    let sp = super::names::id(species);
    Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .position(|m| m.species == sp)
}

/// Escape Rope from the bag: back to the last outdoor escape point
/// (`decompiled/src/item_use.c` `ItemUseOutOfBattle_EscapeRope`, `SetEscapeWarp`).
fn escape_rope(s: &mut Session, outside: &str) -> PResult<()> {
    let from = here(s);
    super::shop::use_item(s, "ITEM_ESCAPE_ROPE")?;
    let out = s.graph().id(outside);
    super::legs2::settle_until(s, "the Escape Rope", 3000, move |s| {
        here(s).is_some_and(|n| n.0 == out)
    })?;
    s.event(
        "decision",
        format!("Escape Rope out to {}", outside.trim_start_matches("MAP_")),
        json!({"from": from.map(|n| [n.1, n.2])}),
    );
    Ok(())
}

/// Escape Ropes bought at the Four Island Mart (`FourIsland_Mart/scripts.inc` sells
/// `ITEM_ESCAPE_ROPE`): Icefall Cave's HM07 pocket and the Back room, the Dotted Hole, Cerulean
/// Cave (`rope_cerulean_cave`).
const ROPES: u16 = 4;

/// "sail-to-four", "icefall-cave-walk", "icefall-grunt", "icefall-ice-slides", "icefall-cave".
/// The Back room (Lorelei's scene, the forced grunt: `LoreleiRocketsScene` on the coord row y 18,
/// `TRAINER_TEAM_ROCKET_GRUNT_45`; it hides the Ruin Valley scientist that stands on the Dotted
/// Hole's Cut spot, `FLAG_HIDE_RUIN_VALLEY_SCIENTIST`,
/// `decompiled/data/maps/FourIsland_IcefallCave_Back/scripts.inc`) is behind the Entrance's
/// waterfall (rows 14-20, `MB_WATERFALL`, `LAYOUT_FOUR_ISLAND_ICEFALL_CAVE_ENTRANCE`): its upper
/// door (9, 7) leads to 1F (2, 8) and the Back door (3, 6) (`map.json` warps). Waterfall is HM07,
/// the item ball at 1F (12, 16) (`FourIsland_IcefallCave_1F/map.json`), in a pocket reached only
/// by falling through 1F's thin ice to B1F and up its (15, 16) ladder, and left only by an
/// Escape Rope ([`ice_plan`] finds no way back on foot). Lapras learns HM07 (`tmhm_learnsets.h`).
fn icefall(s: &mut Session) -> PResult<()> {
    s.seg = "sail-to-four".into();
    let t0 = s.timer().unwrap_or(0.0);
    seagallop(s, 1, 4)?;
    let n = ROPES.saturating_sub(have(s, "ITEM_ESCAPE_ROPE"));
    if n > 0 {
        super::shop::buy(s, "MAP_FOUR_ISLAND_MART", None, &[("ITEM_ESCAPE_ROPE", n)])?;
    }
    s.seg = "icefall-cave-walk".into();
    let f1 = s.graph().id("MAP_FOUR_ISLAND_ICEFALL_CAVE_1F");
    if have(s, "ITEM_HM07") == 0 {
        go_map(s, "MAP_FOUR_ISLAND_ICEFALL_CAVE_1F", &hm_opts())?;
        s.seg = "icefall-ice-slides".into();
        ice_go(s, "the HM07 ball", &move |n: &Node| {
            n.0 == f1 && (n.1 - 12).abs() + (n.2 - 16).abs() == 1
        })?;
        let cur = here(s).ok_or("no position")?;
        let key = match (12 - cur.1, 16 - cur.2) {
            (0, 1) => keys::DOWN,
            (0, -1) => keys::UP,
            (1, 0) => keys::RIGHT,
            _ => keys::LEFT,
        };
        super::walk::face(s, key)?;
        s.hf.press(keys::A)?;
        settle(s)?;
        if have(s, "ITEM_HM07") == 0 {
            return Err(PlayError::Fatal("no HM07 after the ball".into()));
        }
        s.event(
            "model-vs-actual",
            "Icefall: the Back room is behind a waterfall; HM07 (1F pocket) and an Escape Rope \
             are not in the model",
            json!({"segment": "icefall-cave-walk"}),
        );
        let slot = party_slot(s, "SPECIES_LAPRAS").ok_or("no Lapras for HM07")?;
        super::items::teach_tmhm(s, "ITEM_HM07", slot)?;
        escape_rope(s, "MAP_FOUR_ISLAND")?;
    }
    s.seg = "icefall-cave-walk".into();
    go_map(s, "MAP_FOUR_ISLAND_ICEFALL_CAVE_ENTRANCE", &hm_opts())?;
    let back = s.graph().id("MAP_FOUR_ISLAND_ICEFALL_CAVE_BACK");
    s.seg = "icefall-grunt".into();
    ice_go(s, "the Back room", &move |n: &Node| n.0 == back)?;
    // Up the room to the scene row (the scene walks the player on from there).
    go(
        s,
        "MAP_FOUR_ISLAND_ICEFALL_CAVE_BACK",
        12,
        19,
        &Opts::default(),
    )?;
    let mut fed = 0;
    while super::prims::is_free(s) && fed < 60 {
        s.hf.step(keys::UP)?;
        fed += 1;
    }
    super::legs2::settle_until(s, "Lorelei's scene", 20_000, |s| {
        var(s, "VAR_MAP_SCENE_ICEFALL_CAVE_BACK") >= 1
    })?;
    s.seg = "icefall-cave".into();
    escape_rope(s, "MAP_FOUR_ISLAND")?;
    mva(s, "Icefall Cave", t0, "celio-rainbow-pass", "icefall-cave");
    split(s, "Icefall", "icefall-cave")
}

/// Not in the model (`route_wr.rs` goes from Icefall straight to the Five Island Meadow): the
/// warehouse door opens only with `FLAG_LEARNED_YES_NAH_CHANSEY`
/// (`FiveIsland_Meadow_EventScript_WarehouseDoor`, `decompiled/data/maps/FiveIsland_Meadow/scripts.inc:9-26`),
/// set by the thief's scene when the Sapphire is found in the Dotted Hole
/// (`SixIsland_DottedHole_SapphireRoom_EventScript_Sapphire`, the same flag). The Dotted Hole's door
/// in Ruin Valley opens with Cut used from (24, 25) facing north (`CutMoveRuinValleyCheck`,
/// `decompiled/src/field_specials.c:2296-2316`); the scientist who stood there left after
/// Lorelei's scene. The hole's floors are plain warps (`SixIsland_DottedHole_*/map.json`).
fn dotted_hole(s: &mut Session) -> PResult<()> {
    s.seg = "sapphire-and-sail-back".into();
    let t0 = s.timer().unwrap_or(0.0);
    seagallop(s, 4, 6)?;
    s.event(
        "model-vs-actual",
        "the Rocket Warehouse needs the thief's password from the Dotted Hole (Six Island) \
         first; the model has no Six Island trip before the warehouse",
        json!({"cite": "decompiled/data/maps/FiveIsland_Meadow/scripts.inc:9-26"}),
    );
    if !flag(s, "FLAG_USED_CUT_ON_RUIN_VALLEY_BRAILLE") {
        go(s, "MAP_SIX_ISLAND_RUIN_VALLEY", 24, 25, &hm_opts())?;
        super::walk::face(s, keys::UP)?;
        super::items::use_field_move(s, "MOVE_CUT")?;
        settle(s)?;
        if !flag(s, "FLAG_USED_CUT_ON_RUIN_VALLEY_BRAILLE") {
            return Err(PlayError::Fatal("the Dotted Hole door did not open".into()));
        }
        s.event(
            "decision",
            "Cut on the Ruin Valley braille: the Dotted Hole opened",
            json!({}),
        );
    }
    // The door's open metatile is set by the script (`CutMoveOpenDottedHoleDoor`); the planner
    // reads the layout's closed door: walk in by hand.
    take_warp(s, "MAP_SIX_ISLAND_RUIN_VALLEY", 24, 25, keys::UP)?;
    go_face(
        s,
        "MAP_SIX_ISLAND_DOTTED_HOLE_SAPPHIRE_ROOM",
        7,
        7,
        &hm_opts(),
    )?;
    s.hf.press(keys::A)?;
    settle(s)?;
    if !flag(s, "FLAG_LEARNED_YES_NAH_CHANSEY") {
        return Err(PlayError::Fatal(
            "no warehouse password after the Sapphire scene".into(),
        ));
    }
    s.event(
        "decision",
        "Dotted Hole: the thief took the Sapphire and gave the password away",
        json!({}),
    );
    escape_rope(s, "MAP_SIX_ISLAND_RUIN_VALLEY")?;
    fly_to(s, "MAPSEC_SIX_ISLAND")?;
    mva(
        s,
        "Six Island and the Dotted Hole (not in the model)",
        t0,
        "icefall-cave",
        "icefall-cave",
    );
    Ok(())
}

/// "five-island-approach", "meadow-grunt", "five-island-resort", the warehouse fights,
/// "warehouse-gideon": Seagallop to Five Island, the Meadow, the door at (12, 21) (the password
/// opens it: `FLAG_UNLOCKED_ROCKET_WAREHOUSE`), then Gideon at (27, 4); the admins and grunts on the
/// way fight where their sight lines or the coord trigger (row y 11, `Admin2Trigger`) force them.
/// Beating Gideon gives the Sapphire (`FiveIsland_RocketWarehouse_EventScript_DefeatedGideon`:
/// `FLAG_RECOVERED_SAPPHIRE`, `decompiled/data/maps/FiveIsland_RocketWarehouse/scripts.inc`).
fn warehouse(s: &mut Session) -> PResult<()> {
    s.seg = "five-island-approach".into();
    let t0 = s.timer().unwrap_or(0.0);
    seagallop(s, 6, 5)?;
    if !flag(s, "FLAG_UNLOCKED_ROCKET_WAREHOUSE") {
        go_face(s, "MAP_FIVE_ISLAND_MEADOW", 12, 21, &hm_opts())?;
        s.hf.press(keys::A)?;
        settle(s)?;
        if !flag(s, "FLAG_UNLOCKED_ROCKET_WAREHOUSE") {
            return Err(PlayError::Fatal("the warehouse door stayed shut".into()));
        }
    }
    s.seg = "warehouse-interior".into();
    take_warp(s, "MAP_FIVE_ISLAND_MEADOW", 12, 22, keys::UP)?;
    s.seg = "warehouse-gideon".into();
    talk_script(s, "MAP_FIVE_ISLAND_ROCKET_WAREHOUSE", "EventScript_Gideon")?;
    if !flag(s, "FLAG_RECOVERED_SAPPHIRE") {
        return Err(PlayError::Fatal("no Sapphire after Gideon".into()));
    }
    s.event("decision", "beat Gideon: the Sapphire", json!({}));
    mva(
        s,
        "Five Island and the warehouse",
        t0,
        "icefall-cave",
        "warehouse-gideon",
    );
    split(s, "Warehouse", "warehouse-gideon")
}

/// "celio-network-machine": Seagallop to One Island, Celio takes the Sapphire
/// (`GiveCelioSapphire`: `FLAG_SYS_CAN_LINK_WITH_RS` -- the Elite Four's rematch switch,
/// `PokemonLeague_LoreleisRoom/scripts.inc:57-62` -- and `FLAG_HIDE_CERULEAN_CAVE_GUARD`,
/// `OneIsland_PokemonCenter_1F/scripts.inc:244-282`).
fn celio_sapphire(s: &mut Session) -> PResult<()> {
    s.seg = "celio-network-machine".into();
    let t0 = s.timer().unwrap_or(0.0);
    seagallop(s, 5, 1)?;
    talk_script(s, CELIO_PC, "EventScript_Celio")?;
    if !flag(s, "FLAG_SYS_CAN_LINK_WITH_RS") {
        return Err(PlayError::Fatal("Celio did not take the Sapphire".into()));
    }
    s.event(
        "decision",
        "Celio took the Sapphire: the network is up, the League upgraded",
        json!({}),
    );
    mva(
        s,
        "Sapphire to Celio",
        t0,
        "warehouse-gideon",
        "celio-network-machine",
    );
    split(s, "Sevii done", "celio-network-machine")
}

/// Swap party slots `a` and `b` (party menu: the mon, SWITCH -- after SUMMARY and the mon's field
/// moves, `SetPartyMonFieldSelectionActions`, `decompiled/src/party_menu.c:2963-2988` -- then the
/// other mon).
fn party_swap(s: &mut Session, a: usize, b: usize) -> PResult<()> {
    let mon = Ram::new(&s.obs).party(s.hf.rec.emu())[a].clone();
    let fm: Vec<u16> = [
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
    ]
    .iter()
    .map(|m| super::names::id(m))
    .collect();
    let switch_row = 1 + mon.moves.iter().filter(|m| fm.contains(m)).count() as i8;
    menu::open_start(s)?;
    let r = super::items::start_row(s, super::items::STARTMENU_POKEMON)?;
    menu::start_menu_pick(s, r)?;
    super::items::party_choose(s, a as i8)?;
    wait_until(s, "the mon's menu", 200, |s| {
        menu::task(s, "Task_HandleSelectionMenuInput")
    })?;
    menu::choose(s, switch_row)?;
    // Switch mode moves the second cursor, `gPartyMenu.slotId2` (`struct PartyMenu`,
    // `decompiled/include/party_menu.h:8-20`: offset 10).
    wait_until(s, "the switch cursor", 200, |s| {
        menu::task(s, "Task_HandleChooseMonInput")
    })?;
    let pm = s.sym("gPartyMenu");
    let mut moved = false;
    for _ in 0..12 {
        let cur = s.hf.rec.emu().read8(pm + 10) as i8;
        if cur == b as i8 {
            s.hf.press(keys::A)?;
            moved = true;
            break;
        }
        s.hf.press(if (b as i8) > cur {
            keys::DOWN
        } else {
            keys::UP
        })?;
        s.hf.idle(1)?;
    }
    if !moved {
        return Err(PlayError::Death("soft-lock: the switch cursor".into()));
    }
    let sp = mon.species;
    wait_until(s, "the switch", 300, move |s| {
        Ram::new(&s.obs).party(s.hf.rec.emu())[b].species == sp
            && menu::task(s, "Task_HandleChooseMonInput")
    })?;
    super::items::close_to_field(s)?;
    settle(s)?;
    s.event(
        "decision",
        format!(
            "party: {} to slot {b}",
            super::names::names()
                .species(sp)
                .trim_start_matches("SPECIES_")
        ),
        json!({"from": a, "to": b}),
    );
    Ok(())
}

/// "fly-cerulean", "cerulean-cave", "mewtwo" (`mewtwo: true`): Seagallop to Vermilion (no flying
/// across regions), fly to Cerulean, deposit the Weedle so Mewtwo joins the party, the cave (the
/// guard left with the Sapphire), Mewtwo at B1F (7, 12) (`CeruleanCave_B1F/map.json`,
/// `CeruleanCave_B1F_EventScript_Mewtwo`) with the Master Ball, the Escape Rope out
/// (`rope_cerulean_cave`), the plan's Mewtwo TMs (`tms` with `mewtwo: true`: Fire Blast,
/// Earthquake, Calm Mind; `replace_slot` picks the forgotten move), and Mewtwo to the front: after
/// the catch the sim's fighter is Mewtwo (`mewtwo_pp`, `sim.rs`).
fn mewtwo(s: &mut Session) -> PResult<()> {
    s.seg = "fly-cerulean".into();
    let t0 = s.timer().unwrap_or(0.0);
    let mw = super::names::id("SPECIES_MEWTWO");
    if party_slot(s, "SPECIES_MEWTWO").is_none() {
        go_map(s, "MAP_ONE_ISLAND_HARBOR", &hm_opts())?;
        seagallop(s, 1, 0)?;
        super::items::fly(s, "MAPSEC_CERULEAN_CITY")?;
        let party = Ram::new(&s.obs).party(s.hf.rec.emu());
        if party.len() >= 6 {
            let slot = party_slot(s, "SPECIES_WEEDLE").unwrap_or(2);
            super::shop::pc(
                s,
                "MAP_CERULEAN_CITY_POKEMON_CENTER_1F",
                &[super::shop::PcOp::Deposit(slot)],
            )?;
        }
        s.seg = "cerulean-cave".into();
        go_face(s, "MAP_CERULEAN_CAVE_B1F", 7, 12, &hm_opts())?;
        s.seg = "mewtwo".into();
        s.wild_plan = super::battle::WildPlan::Master;
        s.hf.press(keys::A)?;
        settle(s)?;
        s.wild_plan = super::battle::WildPlan::Flee;
        if party_slot(s, "SPECIES_MEWTWO").is_none() {
            return Err(PlayError::Death(
                "Mewtwo not in the party after the battle".into(),
            ));
        }
        escape_rope(s, "MAP_CERULEAN_CITY")?;
    }
    for tm in s.plan.params.tms.clone() {
        if !tm.mewtwo {
            continue;
        }
        let mv = super::names::id(&tm.mv);
        let slot = party_slot(s, "SPECIES_MEWTWO").ok_or("no Mewtwo")?;
        let m = Ram::new(&s.obs).party(s.hf.rec.emu())[slot].clone();
        if m.knows(mv).is_some() {
            continue;
        }
        let Some(item) = tm_item_for(&tm.mv) else {
            s.event(
                "model-vs-actual",
                format!("no TM item for {}", tm.mv),
                json!({"move": tm.mv}),
            );
            continue;
        };
        if have(s, &item) == 0 {
            s.event(
                "model-vs-actual",
                format!("{item} not in the bag: {} not taught", tm.mv),
                json!({"move": tm.mv}),
            );
            continue;
        }
        super::items::teach_tmhm(s, &item, slot)?;
    }
    let slot = party_slot(s, "SPECIES_MEWTWO").ok_or("no Mewtwo")?;
    if slot != 0 {
        party_swap(s, slot, 0)?;
    }
    let m = Ram::new(&s.obs).party(s.hf.rec.emu())[0].clone();
    s.event(
        "decision",
        format!("Mewtwo Lv{} leads", m.level),
        json!({"species": mw, "moves": m.moves.iter().map(|x| super::names::names().mv(*x)).collect::<Vec<_>>(),
               "hp": m.max_hp}),
    );
    mva(s, "Mewtwo", t0, "celio-network-machine", "mewtwo");
    split(s, "Mewtwo", "mewtwo")
}

/// "fly-indigo", "league-entry": fly to Indigo Plateau, heal, top the League heals up (the same
/// Full Restores as the first round, `legs2::indigo_mart`).
fn league2_entry(s: &mut Session) -> PResult<()> {
    s.seg = "fly-indigo".into();
    let t0 = s.timer().unwrap_or(0.0);
    super::items::fly(s, "MAPSEC_INDIGO_PLATEAU")?;
    s.seg = "league-entry".into();
    center_heal(s, "MAP_INDIGO_PLATEAU_POKEMON_CENTER_1F")?;
    super::legs2::indigo_mart(s)?;
    mva(s, "to the League", t0, "mewtwo", "league-entry");
    Ok(())
}

/// Attacking PP (moves with power) of a party mon, weighted by nothing: the number of attacks left.
fn attack_pp(s: &Session, m: &super::ram::Mon) -> u32 {
    m.moves
        .iter()
        .zip(m.pp.iter())
        .filter(|(&mv, _)| {
            mv != 0
                && super::battle::move_index(s.plan.dex, mv)
                    .is_some_and(|i| s.plan.dex.moves[i].power > 0)
        })
        .map(|(_, &pp)| pp as u32)
        .sum()
}

/// Between League rooms (no center, and this run has no Elixir left): when the lead has fewer
/// than 8 attacks left, the party mon with the most leads instead. Not in the model: its Mewtwo
/// keeps its PP topped up with the plan's Max Elixirs and PP Ups (`mewtwo_pp`,
/// `apply_pp_ups`), which the played route never picked up; Mewtwo reached Lance 2 with Psychic
/// and Fire Blast at 0 and lost 12 of 12 branches using Calm Mind only.
fn pp_lead(s: &mut Session) -> PResult<()> {
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let lead = attack_pp(s, &party[0]);
    if lead >= 8 {
        return Ok(());
    }
    let best = (1..party.len())
        .filter(|&i| party[i].hp > 0 && party[i].level >= 40)
        .max_by_key(|&i| attack_pp(s, &party[i]));
    let Some(b) = best else { return Ok(()) };
    let pp_b = attack_pp(s, &party[b]);
    if pp_b <= lead {
        return Ok(());
    }
    s.event(
        "model-vs-actual",
        format!(
            "lead has {lead} attacks left (no Elixir in the bag): {} leads ({pp_b} attacks)",
            super::names::names()
                .species(party[b].species)
                .trim_start_matches("SPECIES_")
        ),
        json!({"lead_pp": lead, "new_lead_pp": pp_b, "slot": b}),
    );
    party_swap(s, b, 0)
}

/// Put the first party mon of `species` in front (if it can fight).
fn lead_species(s: &mut Session, species: &str) -> PResult<()> {
    let Some(i) = party_slot(s, species) else {
        return Ok(());
    };
    let m = Ram::new(&s.obs).party(s.hf.rec.emu())[i].clone();
    if i == 0 || m.hp == 0 || attack_pp(s, &m) < 8 {
        return Ok(());
    }
    s.event(
        "model-vs-actual",
        format!(
            "{} leads this room (Mewtwo's PP is the round's budget: 25 attacks, no Elixirs)",
            species.trim_start_matches("SPECIES_")
        ),
        json!({"species": species}),
    );
    party_swap(s, i, 0)
}

/// Lorelei 2 and Bruno 2 go to the carry, so Mewtwo's 25 attacks (Psychic 10, Fire Blast 5,
/// Earthquake 10) last for Agatha, Lance and the Champion; the sim's Mewtwo has the plan's PP Ups
/// and Max Elixirs for all five, which the played route never collected.
fn lorelei2(s: &mut Session) -> PResult<()> {
    lead_species(s, "SPECIES_BLASTOISE")?;
    pp_lead(s)?;
    super::legs2::e4_member(
        s,
        "MAP_POKEMON_LEAGUE_LORELEIS_ROOM",
        "lorelei-2",
        6,
        5,
        "FLAG_DEFEATED_LORELEI",
    )
}

fn bruno2(s: &mut Session) -> PResult<()> {
    lead_species(s, "SPECIES_BLASTOISE")?;
    pp_lead(s)?;
    super::legs2::e4_member(
        s,
        "MAP_POKEMON_LEAGUE_BRUNOS_ROOM",
        "bruno-2",
        6,
        5,
        "FLAG_DEFEATED_BRUNO",
    )
}

fn agatha2(s: &mut Session) -> PResult<()> {
    lead_species(s, "SPECIES_MEWTWO")?;
    pp_lead(s)?;
    super::legs2::e4_member(
        s,
        "MAP_POKEMON_LEAGUE_AGATHAS_ROOM",
        "agatha-2",
        6,
        5,
        "FLAG_DEFEATED_AGATHA",
    )
}

fn lance2(s: &mut Session) -> PResult<()> {
    pp_lead(s)?;
    super::legs2::e4_member(
        s,
        "MAP_POKEMON_LEAGUE_LANCES_ROOM",
        "lance-2",
        6,
        8,
        "FLAG_DEFEATED_LANCE",
    )
}

/// "champion-2": the rematch Champion (`TRAINER_CHAMPION_REMATCH_*`; the room's scene as in the
/// first round, `PokemonLeague_ChampionsRoom/scripts.inc`), then into the Hall of Fame room, whose
/// scene ends in `special EnterHallOfFame` (`PokemonLeague_HallOfFame/scripts.inc`). Ends on the
/// first frame of the Hall of Fame screen (`CB2_DoHallOfFameScreen`).
fn champion2(s: &mut Session) -> PResult<()> {
    s.seg = "champion-2".into();
    pp_lead(s)?;
    let room = s.graph().id("MAP_POKEMON_LEAGUE_CHAMPIONS_ROOM");
    let hof = s.graph().id("MAP_POKEMON_LEAGUE_HALL_OF_FAME");
    super::legs2::league_prep(s)?;
    super::legs2::e4_door(s)?;
    go_to(
        s,
        "the Champion's room",
        &move |n: &Node| n.0 == room || n.0 == hof,
        &Opts::default(),
    )?;
    let start = s.frame();
    let mut split_done = false;
    loop {
        if super::prims::in_hall_of_fame(s) {
            break;
        }
        if s.frame() - start > 60_000 {
            return Err(PlayError::Death(
                "soft-lock: no Hall of Fame after the Champion".into(),
            ));
        }
        if s.obs.in_battle(s.hf.rec.emu()) {
            super::fallback_battle::battle(s)?;
            continue;
        }
        if !split_done && flag(s, "FLAG_DEFEATED_CHAMP") {
            split_done = true;
            let t = s.timer().unwrap_or(0.0);
            let p50 = s.model.p50("champion-2");
            s.event(
                "split",
                format!(
                    "champion-2: {t:.0}s vs model p50 {:.0}s ({:+.0}s)",
                    p50.unwrap_or(f64::NAN),
                    t - p50.unwrap_or(f64::NAN)
                ),
                json!({"actual": t, "p50": p50, "name": "champion-2", "seg": "champion-2"}),
            );
        }
        if super::prims::is_free(s) {
            if here(s).is_some_and(|n| n.0 == room) {
                take_warp(s, "MAP_POKEMON_LEAGUE_CHAMPIONS_ROOM", 6, 3, keys::UP)?;
                continue;
            }
            s.hf.step(0)?;
            continue;
        }
        s.hf.late()?;
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
    s.seg = "hall-of-fame-2".into();
    s.event(
        "decision",
        "the second Hall of Fame begins",
        json!({"cite": "decompiled/src/hall_of_fame.c:361"}),
    );
    Ok(())
}

/// Is the whole screen black (every pixel's R, G and B zero)?
fn screen_black(s: &mut Session) -> bool {
    let px = s.hf.rec.emu().screen_rgba();
    px.chunks_exact(4)
        .all(|p| p[0] == 0 && p[1] == 0 && p[2] == 0)
}

/// "hall-of-fame-2": the timer stops on the first full black frame at the end of the second Hall
/// of Fame sequence (the category rule, `human-run/PROMPT.md`). The sequence waits for A on the
/// "LEAGUE CHAMPION" text (`Task_Hof_ExitOnKeyPressed`), then fades every palette to black
/// (`Task_Hof_HandlePaletteOnExit`: `BeginNormalPaletteFade(PALETTES_ALL, 8, 0, 16, RGB_BLACK)`)
/// and tears down when the fade is done (`Task_Hof_HandleExit`), `decompiled/src/hall_of_fame.c:665-697`.
/// The finish frame is the first frame after that A whose picture is entirely black.
fn hall_of_fame2(s: &mut Session) -> PResult<()> {
    s.seg = "hall-of-fame-2".into();
    wait_until(s, "the LEAGUE CHAMPION text", 20_000, |s| {
        menu::task(s, "Task_Hof_ExitOnKeyPressed")
    })?;
    s.hf.press(keys::A)?;
    let start = s.frame();
    while !screen_black(s) {
        if s.frame() - start > 600 {
            return Err(PlayError::Fatal(
                "no black frame after the Hall of Fame A".into(),
            ));
        }
        s.hf.step(0)?;
    }
    let t = s.timer().unwrap_or(0.0);
    let p50 = s.model.p50("hall-of-fame-2");
    let q = s.model.index("hall-of-fame-2").map(|i| s.model.q[i]);
    s.event(
        "split",
        format!(
            "FINISH (first black frame of the second Hall of Fame): {} ({t:.1}s) vs model p50 {:.0}s ({:+.0}s); WR 12325s ({:+.0}s)",
            hms(t),
            p50.unwrap_or(f64::NAN),
            t - p50.unwrap_or(f64::NAN),
            t - 12325.0
        ),
        json!({"actual": t, "p50": p50, "p10": q.map(|q| q.0), "p90": q.map(|q| q.2),
               "name": "Finish", "seg": "hall-of-fame-2", "final": true, "timer_stopped": true,
               "frame": s.frame(), "cite": "decompiled/src/hall_of_fame.c:665-697"}),
    );
    Ok(())
}

/// The `ITEM_TMxx` that teaches `mv` (`sTMHMMoves`, `items::tm_move`).
fn tm_item_for(mv: &str) -> Option<String> {
    let want = super::names::id(mv);
    (1..=50)
        .map(|i| format!("ITEM_TM{i:02}"))
        .find(|it| super::items::tm_move(super::names::id(it)) == Some(want))
}

fn hms(t: f64) -> String {
    let t = t.max(0.0) as u64;
    format!("{}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60)
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
        Stage {
            name: "icefall",
            run: icefall,
        },
        Stage {
            name: "dotted-hole",
            run: dotted_hole,
        },
        Stage {
            name: "warehouse",
            run: warehouse,
        },
        Stage {
            name: "celio-sapphire",
            run: celio_sapphire,
        },
        Stage {
            name: "mewtwo",
            run: mewtwo,
        },
        Stage {
            name: "league2-entry",
            run: league2_entry,
        },
        Stage {
            name: "lorelei-2",
            run: lorelei2,
        },
        Stage {
            name: "bruno-2",
            run: bruno2,
        },
        Stage {
            name: "agatha-2",
            run: agatha2,
        },
        Stage {
            name: "lance-2",
            run: lance2,
        },
        Stage {
            name: "champion-2",
            run: champion2,
        },
        Stage {
            name: "hall-of-fame-2",
            run: hall_of_fame2,
        },
    ]
}
