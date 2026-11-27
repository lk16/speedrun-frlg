//! The route from Giovanni 1 on, as stages, in the plan's order (`route_wr.rs` `wr_order`, the
//! `route.rs` `leg_*` functions): `leg_hm02` (the Route 16 house's Fly), `leg_tower` (fly to
//! Lavender, the tower, the rival, Marowak, the Rockets, Mr. Fuji's flute), Surge (fly to
//! Vermilion), `leg_cycling` (fly to Celadon, Snorlax, Cycling Road) -> the "Snorlax" split, then
//! Safari and Koga.
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::menu;
use super::prims::{settle, wait_until};
use super::session::{PResult, Session, Stage};
use super::walk::{go, go_face, go_map, here, Opts};

fn noop(_s: &mut Session) -> PResult<()> {
    Ok(())
}

/// Stand on `(x, y)` of `map`, then hold `dir` until the map changes (an arrow warp, a dynamic
/// warp the planner cannot follow).
pub fn take_warp(s: &mut Session, map: &str, x: i16, y: i16, dir: u16) -> PResult<()> {
    go(s, map, x, y, &Opts::default())?;
    let from = s.graph().id(map);
    wait_until(s, "free to step", 300, super::prims::is_free)?;
    let mut fed = 0;
    while here(s).is_some_and(|n| n.0 == from) {
        if fed > 200 {
            return Err(super::session::PlayError::Death(format!(
                "soft-lock: warp at {map} ({x}, {y}) did not fire"
            )));
        }
        s.hf.step(dir)?;
        fed += 1;
    }
    settle(s)
}

/// Rocket Hideout B4F -> Celadon City: the elevator to B1F (the Lift Key is in the bag), the stairs
/// up to the Game Corner, out of the door. The elevator's floor list is the multichoice
/// `MULTICHOICE_ROCKET_HIDEOUT_ELEVATOR` with B1F first
/// (`decompiled/data/maps/RocketHideout_Elevator/scripts.inc`, `ToB1F` = `VAR_RESULT` 0); the
/// panel is the sign at (0, 2) of the elevator (`RocketHideout_Elevator/map.json`).
fn hideout_out(s: &mut Session) -> PResult<()> {
    let o = Opts::default();
    go_map(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", &o)?;
    go_face(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", 0, 2, &o)?;
    s.hf.press(keys::A)?;
    menu::text_until_multichoice(s)?;
    menu::choose(s, 0)?;
    settle(s)?;
    // Out of the elevator car: (2, 5) is its south arrow warp (behavior 0x65 in the layout), a
    // dynamic warp to the floor chosen
    // (`setdynamicwarp MAP_ROCKET_HIDEOUT_B1F, 255, 24, 25`).
    take_warp(s, "MAP_ROCKET_HIDEOUT_ELEVATOR", 2, 5, keys::DOWN)?;
    go_map(s, "MAP_CELADON_CITY", &o)?;
    s.event(
        "decision",
        "out of the Rocket Hideout by the elevator",
        json!({}),
    );
    Ok(())
}

/// Walk up to object `local_id` of `map` (wherever it is standing now), face it and press A; retry
/// when it wandered off before the press landed. Leaves the script running (the caller settles or
/// answers its menus).
pub fn talk_to(s: &mut Session, map: &str, local_id: u8) -> PResult<()> {
    let o = Opts::default();
    for _ in 0..20 {
        go_map(s, map, &o)?;
        let pos = {
            let ram = super::ram::Ram::new(&s.obs);
            ram.objects(s.hf.rec.emu())
                .iter()
                .find(|l| l.local_id == local_id)
                .map(|l| (l.x, l.y))
        };
        // Off screen (objects only spawn near the camera): head for where the map puts it.
        let pos = pos.or_else(|| {
            let g = s.graph();
            g.maps[&g.id(map)]
                .objects
                .iter()
                .find(|ob| ob.local_id == local_id)
                .map(|ob| (ob.x, ob.y))
        });
        let Some((x, y)) = pos else {
            return Err(super::session::PlayError::Fatal(format!(
                "object {local_id} of {map} is not on the map"
            )));
        };
        go_face(s, map, x, y, &o)?;
        s.hf.press(keys::A)?;
        for _ in 0..12 {
            s.hf.step(0)?;
            if s.obs.field_controls_locked(s.hf.rec.emu()) {
                return Ok(());
            }
        }
    }
    Err(super::session::PlayError::Death(format!(
        "soft-lock: could not talk to object {local_id} of {map}"
    )))
}

fn party_slot_of(s: &mut Session, species: &str) -> Option<usize> {
    let sp = super::names::id(species);
    let ram = super::ram::Ram::new(&s.obs);
    let party = ram.party(s.hf.rec.emu());
    party.iter().position(|m| m.species == sp)
}

/// `leg_hm02` (`route.rs`): "hm02-trip" (218 tiles) and "hm02-fly". The Route 16 house's woman
/// (object 1 of `Route16_House/map.json`) gives HM02 (`Route16_House/scripts.inc`,
/// `giveitem_msg ... ITEM_HM02`); the way there from Celadon is cut through the tree on Route 16
/// (Paras knows Cut). Fly is taught to the Pidgey (Pidgey learns HM02:
/// `decompiled/src/data/pokemon/tmhm_learnsets.h`).
fn hm02(s: &mut Session) -> PResult<()> {
    talk_to(s, "MAP_ROUTE16_HOUSE", 1)?;
    settle(s)?;
    let have = super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), super::names::id("ITEM_HM02"));
    if have == 0 {
        return Err(super::session::PlayError::Fatal(
            "no HM02 after the talk".into(),
        ));
    }
    s.event(
        "decision",
        "got HM02 (Fly) in the Route 16 house",
        json!({}),
    );
    let slot = party_slot_of(s, "SPECIES_PIDGEY")
        .ok_or_else(|| super::session::PlayError::Fatal("no Pidgey to teach Fly".into()))?;
    super::items::teach_tmhm(s, "ITEM_HM02", slot)?;
    Ok(())
}

fn hm02_walk(s: &mut Session) -> PResult<()> {
    let o = Opts {
        cut: true,
        ..Opts::default()
    };
    go_map(s, "MAP_ROUTE16_HOUSE", &o)
}

/// Log a split: the run timer against the model's no-reset p50 for the segment `seg` (the clock
/// after that segment, `Model::p50`), then the reset verdict the plan would give there (logged,
/// never acted on: `PROMPT.md`, *No resets in the review attempt*).
pub fn split(s: &mut Session, name: &str, seg: &str) -> PResult<()> {
    let t = s.timer().unwrap_or(0.0);
    let p50 = s.model.p50(seg);
    let q = s.model.index(seg).map(|i| s.model.q[i]);
    s.event(
        "split",
        match p50 {
            Some(p) => format!(
                "split {name}: {t:.0}s vs model p50 {p:.0}s ({:+.0}s)",
                t - p
            ),
            None => format!("split {name}: {t:.0}s (no model segment {seg})"),
        },
        json!({"actual": t, "p50": p50, "p10": q.map(|q| q.0), "p90": q.map(|q| q.2),
               "name": name, "seg": seg}),
    );
    if let Some((text, nums)) = s.model.verdict(seg, t) {
        s.event("reset-verdict", text, nums);
    }
    Ok(())
}

/// The plan flies from the Route 16 house to Lavender with the new HM02 (`leg_hm02`, then
/// `leg_tower`'s "fly-lavender"), and only later flies to Vermilion for Surge. Fly needs the
/// Thunder Badge: `FlagGet(FLAG_BADGE01_GET + fieldMove)` with `FIELD_MOVE_FLY` = 2
/// (`decompiled/src/party_menu.c:3926`, `include/constants/party_menu.h:36`), i.e. Surge's badge.
/// So Surge comes first, on foot: Celadon -> Saffron (the Tea opened the gates) -> Route 6 ->
/// Vermilion, cutting the tree in front of the gym.
fn to_vermilion(s: &mut Session) -> PResult<()> {
    s.event(
        "model-vs-actual",
        "model bug: the plan flies to Lavender before Surge, but Fly needs the Thunder Badge \
         (party_menu.c:3926); walking to Vermilion for Surge first, flying to Lavender after",
        json!({"segments": ["hm02-fly", "fly-lavender", "fly-vermilion"], "cite": "decompiled/src/party_menu.c:3926"}),
    );
    let o = Opts {
        cut: true,
        ..Opts::default()
    };
    go_map(s, "MAP_VERMILION_CITY_GYM", &o)
}

/// The trash-can switches (`decompiled/data/maps/VermilionCity_Gym/scripts.inc`): the first switch
/// is a random can (`SetVermilionTrashCans`, `decompiled/src/field_specials.c:552`), the second a
/// neighbour of it; a wrong second guess re-rolls both. The human checks cans nearest-first, and
/// after the first switch, its grid neighbours. Cans are the bg events at x 1..9 step 2, y 10..14
/// step 2, id = col + 5 * row + 1 (`VermilionCity_Gym/map.json`). Progress is read the way the
/// player sees it: the first lock's flag (`FOUND_FIRST_SWITCH` = `FLAG_TEMP_1`) and
/// `FLAG_FOUND_BOTH_VERMILION_GYM_SWITCHES`.
fn surge_cans(s: &mut Session) -> PResult<()> {
    let n = super::names::names();
    let both = n.id("FLAG_FOUND_BOTH_VERMILION_GYM_SWITCHES");
    let first = n.id("FLAG_TEMP_1");
    let map = "MAP_VERMILION_CITY_GYM";
    let can_xy = |id: i16| (1 + 2 * ((id - 1) % 5), 10 + 2 * ((id - 1) / 5));
    let mut checked = 0;
    let mut tried: Vec<i16> = vec![];
    let mut found_first: Option<i16> = None;
    while !s.obs.flag(s.hf.rec.emu(), both).unwrap_or(false) {
        if checked > 60 {
            return Err(super::session::PlayError::Death(
                "soft-lock: 60 trash cans without the locks".into(),
            ));
        }
        let cand: Vec<i16> = match found_first {
            Some(f) => {
                let (c, r) = ((f - 1) % 5, (f - 1) / 5);
                let mut v = vec![];
                if c < 4 {
                    v.push(f + 1);
                }
                if c > 0 {
                    v.push(f - 1);
                }
                if r < 2 {
                    v.push(f + 5);
                }
                if r > 0 {
                    v.push(f - 5);
                }
                v
            }
            None => (1..=15).filter(|c| !tried.contains(c)).collect(),
        };
        // Every can marked tried without the first switch (an A press that did not reach a can, e.g.
        // r2g-s31-full after Dwayne's fight): check them again, as a human would; `checked` bounds it.
        if cand.is_empty() {
            s.event(
                "warn",
                "trash cans: all 15 tried without the first switch: checking again",
                json!({"checked": checked}),
            );
            tried.clear();
            checked += 1;
            continue;
        }
        // Nearest can first.
        let me = here(s).ok_or("no position in the gym")?;
        let next = *cand
            .iter()
            .min_by_key(|&&c| {
                let (x, y) = can_xy(c);
                (x - me.1).abs() + (y - me.2).abs()
            })
            .unwrap();
        let (x, y) = can_xy(next);
        go_face(s, map, x, y, &Opts::default())?;
        let at = here(s);
        let facing = super::ram::Ram::new(&s.obs).player_facing(s.hf.rec.emu());
        // The walk's own `settle` mashes A, and arriving face to face with the can checks it: the
        // first switch can already be open here. Pressing again would be the "second" switch on the
        // same can, wrong, and reset both locks (`TrySwitchTwo`, `VermilionCity_Gym/scripts.inc:166-175`;
        // r2g-s31-full: switches 15/10, can 15 found by the walk, then reset, 31 cans).
        if found_first.is_none() && s.obs.flag(s.hf.rec.emu(), first).unwrap_or(false) {
            found_first = Some(next);
            tried.push(next);
            checked += 1;
            s.event(
                "decision",
                format!("first switch in can {next} (opened by the walk's A on arrival)"),
                json!({"can": next, "checked": checked}),
            );
            continue;
        }
        let f0 = s.frame();
        let pre = (
            // `VAR_TEMP_0` / `VAR_TEMP_1` = 0x4000 / 0x4001 (`include/constants/vars.h:7-9`).
            s.obs.var(s.hf.rec.emu(), 0x4000).unwrap_or(0),
            s.obs.var(s.hf.rec.emu(), 0x4001).unwrap_or(0),
            s.obs.flag(s.hf.rec.emu(), first).unwrap_or(false),
        );
        s.hf.press(keys::A)?;
        settle(s)?;
        checked += 1;
        s.event(
            "decision",
            format!(
                "trash can {next} at ({x}, {y}) checked from {at:?} facing {facing} ({} frames; switches {}/{} first {} before)",
                s.frame() - f0, pre.0, pre.1, pre.2
            ),
            json!({"can": next, "frames": s.frame() - f0, "switch1": pre.0, "switch2": pre.1, "first_before": pre.2}),
        );
        let f1 = s.obs.flag(s.hf.rec.emu(), first).unwrap_or(false);
        if found_first.is_none() {
            tried.push(next);
            if f1 {
                found_first = Some(next);
                s.event(
                    "decision",
                    format!("first switch in can {next}"),
                    json!({"can": next, "checked": checked}),
                );
            }
        } else if !f1 && !s.obs.flag(s.hf.rec.emu(), both).unwrap_or(false) {
            s.event(
                "decision",
                format!("can {next} was wrong: the locks reset"),
                json!({"can": next}),
            );
            found_first = None;
            tried.clear();
        }
    }
    s.event(
        "decision",
        format!("both locks open after {checked} cans"),
        json!({"cans": checked}),
    );
    Ok(())
}

/// `leg_surge`: Tucker (forced, `VermilionCity_Gym/map.json`) on the way, Lt. Surge at (5, 2).
fn surge(s: &mut Session) -> PResult<()> {
    s.seg = "surge-gym-trainers".into();
    // The beam barrier rows 6-7, x 3-7, are switched off by `SetBeamsOff` once both locks are
    // open (`VermilionCity_Gym/scripts.inc`).
    let mut o = Opts::default();
    for y in 6..=7 {
        for x in 3..=7 {
            o.open.push(("MAP_VERMILION_CITY_GYM", x, y));
        }
    }
    go_face(s, "MAP_VERMILION_CITY_GYM", 5, 2, &o)?;
    s.seg = "surge".into();
    s.hf.press(keys::A)?;
    settle(s)?;
    let badge = super::names::id("FLAG_BADGE03_GET");
    if !s.obs.flag(s.hf.rec.emu(), badge).unwrap_or(false) {
        return Err(super::session::PlayError::Fatal(
            "no Thunder Badge after Surge".into(),
        ));
    }
    split(s, "Surge", "surge-badge")
}

/// `leg_tower`: "tower-walk" to 2F, where the rival waits (`rival-tower`); his scene fires on the
/// coord events (16, 6) / (17, 5) next to him (`PokemonTower_2F/map.json`), which the plan's fight
/// needs, so the human walks onto (16, 6).
fn tower_rival(s: &mut Session) -> PResult<()> {
    let _ = super::items::mount_bike(s)?;
    go_map(s, "MAP_POKEMON_TOWER_1F", &Opts::default())?;
    s.seg = "rival-tower".into();
    go(s, "MAP_POKEMON_TOWER_2F", 16, 6, &Opts::default())?;
    settle(s)
}

/// 3F-6F with the forced channelers (`tower-channelers`: Emilia, Jennifer, one of Karina / Tammy),
/// up to the Marowak ghost's coord events (11, 15) / (12, 16) of 6F
/// (`PokemonTower_6F/scripts.inc` `MarowakGhost`). The ghost battle has to be won: the Silph
/// Scope reveals it and it blocks the stairs until beaten.
fn tower_marowak(s: &mut Session) -> PResult<()> {
    s.seg = "tower-channelers".into();
    go_map(s, "MAP_POKEMON_TOWER_6F", &Opts::default())?;
    // The ghost cannot be fled: running replays the scene on the next step. Fight it.
    s.seg = "tower-encounters".into();
    s.wild_plan = super::battle::WildPlan::Fight;
    go(s, "MAP_POKEMON_TOWER_6F", 12, 16, &Opts::default())?;
    s.wild_plan = super::battle::WildPlan::Flee;
    settle(s)?;
    let scene = super::names::names().get("VAR_MAP_SCENE_POKEMON_TOWER_6F");
    let v = scene.and_then(|v| s.obs.var(s.hf.rec.emu(), v));
    s.event(
        "decision",
        format!("Marowak scene var {v:?}"),
        json!({"var": v}),
    );
    Ok(())
}

/// 7F: the three Rockets (`tower-rockets`, `maps_group ... PokemonTower_7F, 3`), then Mr. Fuji at
/// (11, 4), who warps the player to the Volunteer House (`PokemonTower_7F/scripts.inc:13`) and
/// gives the Poké Flute there ("fuji-flute").
fn tower_fuji(s: &mut Session) -> PResult<()> {
    s.seg = "tower-rockets".into();
    s.wild_plan = super::battle::WildPlan::Flee;
    talk_to(s, "MAP_POKEMON_TOWER_7F", 1)?;
    settle(s)?;
    let flute = super::names::id("ITEM_POKE_FLUTE");
    // In the Volunteer House he has to be talked to (object 1,
    // `LavenderTown_VolunteerPokemonHouse/scripts.inc` `MrFuji`: `giveitem_msg ... ITEM_POKE_FLUTE`).
    if super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), flute) == 0 {
        talk_to(s, "MAP_LAVENDER_TOWN_VOLUNTEER_POKEMON_HOUSE", 1)?;
        settle(s)?;
    }
    if super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), flute) == 0 {
        return Err(super::session::PlayError::Fatal(
            "no Poke Flute after Fuji".into(),
        ));
    }
    s.event("decision", "got the Poke Flute from Mr. Fuji", json!({}));
    Ok(())
}

/// Heal at a Pokémon Center: walk in, stand two tiles below the nurse (the counter between),
/// face up, talk, and let the script run (its yes/no defaults to YES).
pub fn center_heal(s: &mut Session, center_map: &str) -> PResult<()> {
    // Cut usable (a gym behind Cut trees, e.g. Celadon's, has to be left the same way).
    let o = Opts {
        cut: true,
        ..Opts::default()
    };
    let g = s.graph();
    let mid = g.id(center_map);
    let nurse = g.maps[&mid]
        .objects
        .iter()
        .find(|ob| ob.graphics.contains("NURSE"))
        .map(|ob| (ob.x, ob.y))
        .ok_or("no nurse")?;
    go(s, center_map, nurse.0, nurse.1 + 2, &o)?;
    super::walk::face(s, keys::UP)?;
    s.hf.press(keys::A)?;
    settle(s)?;
    let party = super::ram::Ram::new(&s.obs).party(s.hf.rec.emu());
    s.event(
        "decision",
        format!("healed at {center_map}"),
        json!({"hp": party.iter().map(|m| (m.hp, m.max_hp)).collect::<Vec<_>>()}),
    );
    Ok(())
}

/// Not in the model: `leg_tower` has no heal between the Surge fight and the tower rival. Blastoise
/// arrives at about half HP and paralysed, and the first branch played into the tower lost to the
/// channelers' Gastly with every heal item spent on the rival; a human heals at Lavender's center,
/// next to the fly point.
fn lavender_heal(s: &mut Session) -> PResult<()> {
    s.event(
        "model-vs-actual",
        "model has no heal before the tower; healing at the Lavender center (Blastoise half HP, paralysed)",
        json!({"segment": "tower-walk"}),
    );
    center_heal(s, "MAP_LAVENDER_TOWN_POKEMON_CENTER_1F")?;
    super::items::set_battle_style_set(s)?;
    // v3: the first ball stop (plan 1413 `marts` walk-lavender: Great Balls to 15), so passing
    // catches (the tower's Gastly / Cubone) and the Power Plant hunt have balls.
    // PLAN `full_heals` at `walk-lavender` (honesty 4): Antidotes for Koga's poison in the same stop
    // (`LavenderTown_Mart/scripts.inc:32`, 100 each).
    let fh = s.plan.params.full_heals;
    let mut want = vec![("ITEM_GREAT_BALL", 15u16)];
    if fh > 0 && s.plan.params.full_heal_at == "walk-lavender" {
        want.push(("ITEM_ANTIDOTE", fh as u16));
    }
    super::shop::mart_topup(
        s,
        "MAP_LAVENDER_TOWN_MART",
        None,
        &want,
        "plan 1413 walk-lavender",
    )
}

fn fly_celadon(s: &mut Session) -> PResult<()> {
    go_map(s, "MAP_LAVENDER_TOWN", &Opts::default())?;
    super::items::fly(s, "MAPSEC_CELADON_CITY")
}

/// `leg_cycling` "snorlax": west out of Celadon on the bike to the Snorlax on Route 16 (31, 13),
/// the flute (`Route16/scripts.inc` `Snorlax`: yes/no, then `dowildbattle`), the battle fled.
fn snorlax(s: &mut Session) -> PResult<()> {
    let _ = super::items::mount_bike(s)?;
    s.seg = "snorlax".into();
    go_face(s, "MAP_ROUTE16", 31, 13, &Opts::default())?;
    s.hf.press(keys::A)?;
    settle(s)?;
    let hide = super::names::id("FLAG_HIDE_ROUTE_16_SNORLAX");
    if !s.obs.flag(s.hf.rec.emu(), hide).unwrap_or(false) {
        return Err(super::session::PlayError::Fatal(
            "Snorlax still there".into(),
        ));
    }
    s.event(
        "decision",
        "woke Snorlax with the Poke Flute and fled",
        json!({}),
    );
    Ok(())
}

/// "cycling-road" (374 tiles, `route.rs`): the gate, Cycling Road (Route 17) and Route 18 to
/// Fuchsia on the bike; the "Snorlax" WR split is taken on arrival.
fn cycling(s: &mut Session) -> PResult<()> {
    let _ = super::items::mount_bike(s)?;
    s.seg = "cycling-road".into();
    go_map(s, "MAP_FUCHSIA_CITY", &Opts::default())?;
    split(s, "Snorlax", "snorlax")
}

/// `leg_safari` "safari-trip" + "safari-fee-and-hms": into the Safari Zone gate; the entry
/// triggers at (3..5, 3) ask to join (yes, ¥500; `FuchsiaCity_SafariZone_Entrance/map.json`).
fn safari_trip(s: &mut Session) -> PResult<()> {
    let _ = super::items::mount_bike(s)?;
    go_map(s, "MAP_FUCHSIA_CITY_SAFARI_ZONE_ENTRANCE", &Opts::default())?;
    // Stand below the trigger row, step up onto it, and answer the fee's yes/no explicitly
    // (`AskEnterSafariZone`: a NO walks the player back down).
    go(
        s,
        "MAP_FUCHSIA_CITY_SAFARI_ZONE_ENTRANCE",
        4,
        4,
        &Opts::default(),
    )?;
    s.hf.late()?;
    let mut fed = 0;
    while !s.obs.field_controls_locked(s.hf.rec.emu()) {
        if fed > 120 {
            return Err(super::session::PlayError::Death(
                "soft-lock: Safari entry trigger did not fire".into(),
            ));
        }
        s.hf.step(if fed < 10 { keys::UP } else { 0 })?;
        fed += 1;
    }
    menu::text_until_yesno(s)?;
    menu::answer(s, true)?;
    settle(s)?;
    let center = s.graph().id("MAP_SAFARI_ZONE_CENTER");
    if here(s).map(|n| n.0) != Some(center) {
        return Err(super::session::PlayError::Fatal(
            "not in the Safari Zone after the fee".into(),
        ));
    }
    Ok(())
}

/// "safari-walk" with the plan's hunts in the East and West areas (`hunts` `after: safari-walk`,
/// on route): the Gold Teeth item ball at (28, 14) of the West area and the Secret House's HM03
/// (`SafariZone_West/map.json`, `SafariZone_SecretHouse/scripts.inc`). Wild Pokémon met on the way
/// are thrown at (`WildPlan::Catch`; the bridge plays the Safari ball/bait/rock policy).
fn safari_walk(s: &mut Session) -> PResult<()> {
    // Passing encounters: new species only (`dex::wild_plan`'s pass rule; the v2 run threw at
    // everything here and caught an owned Paras and a second Nidorina).
    s.wild_plan = super::battle::WildPlan::Flee;
    s.seg = "safari-walk".into();
    // The plan's two Safari hunts (`hunts` after safari-walk: East 90, West 100) in the plan's
    // order (v4, model honesty 3: East first; East adds Paras / Parasect / Pinsir, West then only
    // Venonat / Venomoth / Tauros). Center -> West has no walkable warp: the walk to the Secret
    // House (West) passes through East and North anyway (`bin/frlg-mapgraph`: 195 tiles), so the
    // East hunt is on the way. HM03 and the Gold Teeth are fetched before the first West hunt.
    // A hunt rolls twice per paced tile (`dex::hunt_loop`), so 90 + 100 rolls are ~95 Safari
    // steps; with the ~200-tile walk that stays far inside the zone's 600 (`safari_counters`).
    let hunts: Vec<crate::params::Hunt> = s
        .plan
        .params
        .hunts
        .iter()
        .filter(|h| h.after == "safari-walk")
        .cloned()
        .collect();
    let mut house = false;
    for h in &hunts {
        let inside =
            here(s).is_some_and(|n| s.graph().maps[&n.0].id.starts_with("MAP_SAFARI_ZONE"));
        if !inside && house {
            break;
        }
        if h.site == "MAP_SAFARI_ZONE_WEST" && !house {
            safari_house(s)?;
            house = true;
        }
        if !house {
            // Keep the steps the Secret House walk needs (~200 tiles from East, plus margin).
            let (_, steps) = super::dex::safari_counters(s);
            if steps < 320 {
                s.event(
                    "warn",
                    format!(
                        "Safari: {steps} steps left, skipping the {} hunt to keep the Secret House walk",
                        h.site.trim_start_matches("MAP_")
                    ),
                    json!({"steps": steps, "site": h.site}),
                );
                continue;
            }
        }
        super::dex::hunt(s, h)?;
    }
    if !house {
        safari_house(s)?;
    }
    // Out: walk back toward the gate; when the steps run out the game sends the player there.
    s.wild_plan = super::battle::WildPlan::Flee;
    go_map(s, "MAP_FUCHSIA_CITY", &Opts::default())
}

/// The Safari's Gold Teeth (`SafariZone_West/map.json` (28, 14)) and HM03 from the Secret House
/// (`SafariZone_SecretHouse/scripts.inc`).
fn safari_house(s: &mut Session) -> PResult<()> {
    let o = Opts::default();
    s.wild_plan = super::battle::WildPlan::Flee;
    s.seg = "safari-walk".into();
    go_face(s, "MAP_SAFARI_ZONE_WEST", 28, 14, &o)?;
    s.hf.press(keys::A)?;
    settle(s)?;
    talk_to(s, "MAP_SAFARI_ZONE_SECRET_HOUSE", 1)?;
    settle(s)?;
    let have = |s: &mut Session, it: &str| {
        super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), super::names::id(it)) > 0
    };
    let (teeth, hm03) = (have(s, "ITEM_GOLD_TEETH"), have(s, "ITEM_HM03"));
    s.event(
        "decision",
        format!("Safari: Gold Teeth {teeth}, HM03 {hm03}"),
        json!({"gold_teeth": teeth, "hm03": hm03}),
    );
    if !hm03 {
        return Err(super::session::PlayError::Fatal(
            "no HM03 from the Secret House".into(),
        ));
    }
    Ok(())
}

/// `leg_warden` (variant `warden_early`): the Warden takes the Gold Teeth and gives HM04
/// (`FuchsiaCity_WardensHouse/scripts.inc`).
fn warden(s: &mut Session) -> PResult<()> {
    talk_to(s, "MAP_FUCHSIA_CITY_WARDENS_HOUSE", 1)?;
    settle(s)?;
    let hm04 =
        super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), super::names::id("ITEM_HM04")) > 0;
    s.event(
        "decision",
        format!("Warden: HM04 {hm04}"),
        json!({"hm04": hm04}),
    );
    Ok(())
}

/// `leg_koga`: Kirk (forced, `fuchsia-gym-trainers`), Koga at (7, 13); the gym's invisible walls are
/// collision in the live grid.
fn koga(s: &mut Session) -> PResult<()> {
    // Not in the model (its heal is `center-heal@koga-badge`, after the fight): Blastoise comes
    // out of the Safari with 13/14/15 PP on its attacks and 98/130 HP, and the first branch lost
    // to Koga's Muk / Weezing. A human heals at Fuchsia's center, two screens from the gym.
    s.event(
        "model-vs-actual",
        "healing at the Fuchsia center before Koga (model heals after the badge); low PP after the Safari",
        json!({"segment": "koga-walk"}),
    );
    center_heal(s, "MAP_FUCHSIA_CITY_POKEMON_CENTER_1F")?;
    // Honesty 4: the model teaches HM03 Surf right after the Safari (`sim.rs`, "safari-walk"), so
    // its Koga is fought with Surf; v4 taught it after the badge (`teach-surf`) and fought Muk with
    // Water Gun in the slot (engine at the v4 stats: 0.61 win with Water Gun, 0.89 with Surf).
    // Teaching an HM needs no badge (`SafariZone_SecretHouse/scripts.inc` gives HM03).
    let surf = super::names::id("MOVE_SURF");
    if !super::ram::Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .any(|m| m.knows(surf).is_some())
    {
        super::items::teach_tmhm(s, "ITEM_HM03", 0)?;
    }
    // PLAN `full_heals` (honesty 4): Full Heals for Koga's Toxic / Sludge poison, at the Fuchsia mart
    // (`FuchsiaCity_Mart/scripts.inc:28`), two screens from the center and the gym.
    let fh = s.plan.params.full_heals;
    if fh > 0 && s.plan.params.full_heal_at == "koga-walk" {
        super::shop::mart_topup(
            s,
            "MAP_FUCHSIA_CITY_MART",
            None,
            &[("ITEM_FULL_HEAL", fh as u16)],
            "plan full_heals: Full Heals for Koga's poison",
        )?;
    }
    // v3: the Safari's catches evolve here with the stones in the bag (Moon Stone: Nidorina /
    // Nidorino), at the PC of the center the human is standing in.
    super::dexfin::errands(s, "MAP_FUCHSIA_CITY_POKEMON_CENTER_1F", None, 0)?;
    let _ = super::items::mount_bike(s)?;
    s.seg = "fuchsia-gym-trainers".into();
    go_map(s, "MAP_FUCHSIA_CITY_GYM", &Opts::default())?;
    go_face(s, "MAP_FUCHSIA_CITY_GYM", 7, 13, &Opts::default())?;
    s.seg = "koga".into();
    s.hf.press(keys::A)?;
    settle(s)?;
    let badge = super::names::id("FLAG_BADGE05_GET");
    if !s.obs.flag(s.hf.rec.emu(), badge).unwrap_or(false) {
        return Err(super::session::PlayError::Fatal(
            "no Soul Badge after Koga".into(),
        ));
    }
    split(s, "Koga", "koga-badge")?;
    // The model's own heal is after the badge (`center-heal@koga-badge`), but it skips it in the
    // Torrent phase: a heal after the badge lifts the carry out of the Torrent window for Blaine /
    // Sevii (model honesty 3, plan 1607: +2170 h, refuted). v4: no heal after the badge unless
    // the lead fainted (v3 dev: Blastoise fainted to Koga and the Power Plant was walked with
    // Nidoqueen in front); the in-battle potions keep it in the window.
    let lead = super::ram::Ram::new(&s.obs).party(s.hf.rec.emu())[0].clone();
    if lead.hp == 0 {
        center_heal(s, "MAP_FUCHSIA_CITY_POKEMON_CENTER_1F")?;
    }
    // v4: the Power Plant hunt (4 species) is next and the Route 10 center the human flies to has
    // no mart; v4's first pass reached it with no ball. Great Balls to 15 at the Fuchsia mart
    // (`FuchsiaCity_Mart/scripts.inc:24-31`), two screens from the gym. Not in plan 1413's
    // `marts` (its Lavender stop, before the Tower, is the model's last before the plant).
    super::shop::mart_topup(
        s,
        "MAP_FUCHSIA_CITY_MART",
        None,
        &[("ITEM_GREAT_BALL", 15)],
        "v4: balls for the Power Plant hunt",
    )?;
    Ok(())
}

/// The dex module's hook: the plan's evolutions, specials (gifts, trades, statics, buys) and hunts
/// anchored `after` the model segment `anchor`, played by `play/dex.rs` (`dex::dex_hook`).
pub fn dex_hook(s: &mut Session, anchor: &str) -> PResult<()> {
    super::dex::dex_hook(s, anchor)
}

fn fly_lavender(s: &mut Session) -> PResult<()> {
    // Fly works outdoors only ("Can't use that here."): out of the gym first.
    go_map(s, "MAP_VERMILION_CITY", &Opts::default())?;
    super::items::fly(s, "MAPSEC_LAVENDER_TOWN")
}

pub fn stages() -> Vec<Stage> {
    // Power-on to the Route 4 Center (`play/early.rs`, ends in `mt-moon-pc`), then the Center to
    // Giovanni 1 (`play/early2.rs`), then the rest.
    super::early::stages()
        .into_iter()
        .chain(super::early2::stages())
        .chain(rest())
        .collect()
}

fn rest() -> Vec<Stage> {
    vec![
        Stage {
            name: "giovanni1",
            run: noop,
        },
        Stage {
            name: "hideout-out",
            run: hideout_out,
        },
        Stage {
            name: "hm02-trip",
            run: hm02_walk,
        },
        Stage {
            name: "hm02",
            run: hm02,
        },
        Stage {
            name: "to-vermilion",
            run: to_vermilion,
        },
        Stage {
            name: "surge-cans",
            run: surge_cans,
        },
        Stage {
            name: "surge",
            run: surge,
        },
        Stage {
            name: "fly-lavender",
            run: fly_lavender,
        },
        Stage {
            name: "lavender-heal",
            run: lavender_heal,
        },
        Stage {
            name: "tower-rival",
            run: tower_rival,
        },
        Stage {
            name: "tower-marowak",
            run: tower_marowak,
        },
        Stage {
            name: "fuji-flute",
            run: tower_fuji,
        },
        Stage {
            name: "fly-celadon",
            run: fly_celadon,
        },
        Stage {
            name: "snorlax",
            run: snorlax,
        },
        Stage {
            name: "cycling-road",
            run: cycling,
        },
        Stage {
            name: "safari-trip",
            run: safari_trip,
        },
        Stage {
            name: "safari-walk",
            run: safari_walk,
        },
        Stage {
            name: "warden",
            run: warden,
        },
        Stage {
            name: "koga",
            run: koga,
        },
    ]
    .into_iter()
    .chain(super::legs2::stages())
    .chain(super::legs3::stages())
    .collect()
}
