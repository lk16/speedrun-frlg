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

fn fly_lavender(s: &mut Session) -> PResult<()> {
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
            name: "fly-lavender",
            run: fly_lavender,
        },
    ]
}
