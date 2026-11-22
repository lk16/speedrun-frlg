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
        let Some((x, y)) = pos else {
            return Err(super::session::PlayError::Fatal(format!(
                "object {local_id} of {map} is not on screen"
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
        // Nearest can first.
        let me = here(s).unwrap();
        let next = *cand
            .iter()
            .min_by_key(|&&c| {
                let (x, y) = can_xy(c);
                (x - me.1).abs() + (y - me.2).abs()
            })
            .unwrap();
        let (x, y) = can_xy(next);
        go_face(s, map, x, y, &Opts::default())?;
        s.hf.press(keys::A)?;
        settle(s)?;
        checked += 1;
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

fn fly_lavender(s: &mut Session) -> PResult<()> {
    // Fly works outdoors only ("Can't use that here."): out of the gym first.
    go_map(s, "MAP_VERMILION_CITY", &Opts::default())?;
    super::items::fly(s, "MAPSEC_LAVENDER_TOWN")
}

pub fn stages() -> Vec<Stage> {
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
    ]
}
