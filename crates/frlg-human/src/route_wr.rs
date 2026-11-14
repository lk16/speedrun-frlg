//! The world record's route order (`route_style = "wr"`).
//!
//! Order and split boundaries are read off the WR run's LiveSplit and scene log
//! (`human-run/video-analysis/wr-3.25.25-main97.md`; external evidence, not a decomp citation):
//! Brock, Misty, Bill, Bike (S.S. Anne, voucher, Cerulean bike shop), Rock Tunnel, Celadon and the
//! Hideout (Giovanni 1), Tower and flute, Surge (folded into the Snorlax leg), Snorlax, Safari,
//! Koga, Power Plant, Mansion and Blaine, the first Sevii trip, Silph and Sabrina, Erika (after
//! Sabrina), the Warden's Strength and Giovanni 3, Route 22 and Victory Road with **all 60 dex
//! entries done before the League**, E4 round 1, then the post-game (Oak's dex check, Celio's
//! chain with Icefall Cave and the Rocket Warehouse, Cerulean Cave's Mewtwo) and round 2.
//!
//! The legs are the old order's own functions (`route.rs`, `leg_*`), so trainer lists and tile
//! counts stay cited in one place; only the order, the new connecting legs and the post-game differ.
//! Frame constants of the pieces added here are guesses unless they cite a decomp path.

use crate::params::Params;
use crate::route::*;

pub fn wr_order(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let tn = h.tn;

    // Brock is already built (prologue). Pewter -> Cerulean -> Misty first,
    leg_pewter_cerulean(b, p, h);
    if p.item_swaps {
        // Persim Berry for Misty's confusion every run, and the Mark-guy / potion menus
        // (`STRATEGY.md` 4, 6: T07 1:23:20, T06 2:55:15, 3:33:25). Frames: guesses (bag trips).
        b.text("persim-and-potions", 810, Src::Guess);
    }
    leg_misty(b, p, h);
    // then Nugget Bridge and Bill (the rival at Cerulean, `CeruleanCity/scripts.inc`, stands on the
    // bridge road, so it is fought on this leg), back through Cerulean, and Vermilion's S.S. Anne.
    leg_nugget_bill(b, p, h);
    b.split(); // "Bill"
    leg_bill_return(b, p, h);
    leg_ssanne(b, p, h);
    // Back to Cerulean's bike shop with the voucher; the Bike split is the redeemed bicycle.
    leg_vermilion_return(b, p, h);
    leg_bike(b, p, h);
    b.split(); // "Bike"

    // Rock Tunnel (Flash is assumed in the bag: the Route 2 aide's HM05 needs 10 species,
    // a guess carried from the old order), Lavender, Celadon, the Hideout.
    leg_rock_tunnel(b, p, h);
    leg_celadon(b, p, h);
    b.split(); // "Giovanni 1"
               // Cut tree, the Route 16 house's Fly, Pokemon Tower and the flute.
    leg_hm02(b, p, h);
    if p.item_swaps {
        // "take off the Persim and equip the black glasses in the teleport menu before Lavender
        // rival" (T06 3:05:09-3:07:09).
        b.text("black-glasses-swap", 400, Src::Guess);
    }
    leg_tower(b, p, h);
    // Surge is folded into this leg: fly to Vermilion, the gym, then to Celadon for Snorlax.
    b.text("fly-vermilion", 700, Src::Guess);
    leg_surge(b, p, h);
    leg_cycling(b, p, h);
    b.split(); // "Snorlax"

    // Safari, Koga.
    leg_safari(b, p, h);
    leg_koga(b, p, h);

    // Power Plant: fly to Lavender, surf to the plant (Route 10), catch, fly back out.
    if !p.skip_powerplant {
        b.text("fly-lavender-2", 700, Src::Guess);
        b.walk("powerplant-walk", 260, Era::Surf, None, Src::Guess);
        b.text("powerplant-out", 400, Src::Guess);
    }
    // Cinnabar: Mansion, Blaine.
    leg_mansion_blaine(b, p, h);
    b.text("blaine-badge", 600, Src::Guess);
    b.split(); // "Blaine"
    leg_sevii(b, p, h);
    b.split(); // "Sevii"
               // The WR run evolves Pikachu and the rest with stones right after the Sevii trip (Raichu at
               // 2:00 in the video): the stones bought in Celadon's store plus Two Island's Moon Stone.
               // The segment is the anchor of the trades and `evolutions` placed after it, so it exists even when the
               // plan's `stones` list is empty (it then costs nothing).
    b.push("stones", Src::Guess, 0, Kind::Stones);

    // Silph, Sabrina, then Erika (Celadon is a fly away), the Warden, Viridian.
    leg_silph(b, p, h);
    b.text("fly-celadon-3", 700, Src::Guess);
    let e = b.era;
    erika_leg(b, p, e);
    b.text("fly-fuchsia", 700, Src::Guess);
    leg_warden(b, p, h);
    leg_viridian(b, p, h);

    // Route 22, Victory Road, and the dex complete before the League.
    leg_league_approach(b, p, h);
    // Whatever is still missing from the 60 is hunted now (the plan's `fill` list), before Indigo:
    // there is no post-Hall-of-Fame fill phase in this order.
    let at = b
        .segs
        .iter()
        .position(|s| s.name == "victory-road")
        .expect("victory-road segment")
        + 1;
    b.segs.insert(
        at,
        Seg {
            name: "dex-fill".into(),
            src: Src::Guess,
            inputs: 0,
            split: false,
            off_timer: false,
            starter_check: false,
            kind: Kind::OakCheck,
        },
    );

    // First Elite Four. The Hall of Fame's ~50 s (dialogue, save, cards) is calibrated against the
    // WR video's Champion 1 split (walk in 12:18 -> save 13:08 in the E4R1 window).
    // Seconds between rooms: the WR split window minus the WR's measured fight duration
    // (Lorelei 173-148, Bruno 125-104, Agatha 110-88, Lance 166-120; the Champion's window is
    // covered by `hall-of-fame-1`). `league_transit` only.
    for (nm, id, transit) in [
        ("lorelei", "TRAINER_ELITE_FOUR_LORELEI".to_string(), 1500u32),
        ("bruno", "TRAINER_ELITE_FOUR_BRUNO".to_string(), 1260),
        ("agatha", "TRAINER_ELITE_FOUR_AGATHA".to_string(), 1320),
        ("lance", "TRAINER_ELITE_FOUR_LANCE".to_string(), 2760),
        ("champion", tn("TRAINER_CHAMPION_FIRST"), 0),
    ] {
        if p.league_transit && transit > 0 {
            b.text(
                &format!("e4-transit-{nm}"),
                transit.saturating_sub(p.transit_outro),
                Src::Guess,
            );
        }
        b.trainer(nm, &id, 900);
    }
    b.text("hall-of-fame-1", 3000, Src::Guess);
    b.split();

    // Post-game. Stones were used on the way (the plan lists them where they evolve).
    b.text("fly-pallet-2", 700, Src::Guess);
    b.walk("pallet-to-oak", 30, e, None, Src::Tiles);
    // All 60 were caught before the League; Oak only confirms it.
    b.text("oak-national-dex", 2500, Src::Guess);
    b.text("to-one-island", 3000, Src::Guess);
    b.text("celio-ruby-request", 1500, Src::Guess);
    if p.postgame_events {
        // The Ruby chain as the decomp lays it out (`postgame_events`). Kindle Road is a surf route:
        // 159 tiles from the One Island fly spot to the Mt. Ember exterior plus 21 to the cave door
        // (`bin/frlg-mapgraph` model with Surf and Cut; `tools/forced.py` finds no forced fight on it).
        // The cave door at (42, 39) is guarded by two Rocket grunts standing on (41, 40) and (42, 40);
        // once Celio has asked for the Ruby both must be beaten to get in
        // (`decompiled/data/maps/MtEmber_Exterior/scripts.inc` `Grunt1` / `Grunt2`, GRUNT_43 / 44).
        // The Ruby lies on B5F (`MtEmber_RubyPath_B5F_EventScript_Ruby`): 138 tiles each way through
        // five floors. Logan stands on the way to the summit, not to the Ruby, so he is not here.
        b.walk("kindle-road-in", 180, Era::Surf, None, Src::Tiles);
        b.trainer("mt-ember-grunt-1", "TRAINER_TEAM_ROCKET_GRUNT_43", 500);
        b.trainer("mt-ember-grunt-2", "TRAINER_TEAM_ROCKET_GRUNT_44", 500);
        b.walk("ruby-path", 276, Era::Run, None, Src::Tiles);
        b.walk("kindle-road-out", 180, Era::Surf, None, Src::Tiles);
    } else {
        b.walk("mt-ember", 300, Era::Run, None, Src::Guess);
        if p.audited_forced {
            b.forced(
                "mt-ember-trainers",
                &["MtEmber_Exterior_EventScript_Logan"],
                &[],
            );
        } else {
            b.group("mt-ember-trainers", 3, 3, 38, 75);
        }
        b.walk("mt-ember-back", 150, Era::Run, None, Src::Guess);
    }
    if p.postgame_calib {
        // Ruby at Celio at video 2:55 (5:35 after Champ 1) against the model's 3:37: the climb and
        // the Ruby Path. `wr-3.25.25-main97.md` scene log; 80 s (`postgame_events`: what the
        // events above leave of it, `ruby_text`).
        b.text(
            "mt-ember-ruby-climb",
            if p.postgame_events { p.ruby_text } else { 4800 },
            Src::Guess,
        );
    }
    b.text("celio-rainbow-pass", 2800, Src::Guess);
    // Four Island's Icefall Cave: the Lorelei scene and one forced grunt
    // (`decompiled/data/maps/FourIsland_IcefallCave_Back/scripts.inc`, `LoreleiRocketsScene`).
    b.text("sail-to-four", 900, Src::Guess);
    if p.postgame_events {
        // Four Island harbor door to the Back room 94 tiles, and 86 back to the harbor
        // (`frlg-mapgraph` model; ice tiles count as floor, the sliding is `icefall-ice-slides`).
        b.walk("icefall-cave-walk", 94, Era::Run, None, Src::Tiles);
    } else {
        b.walk("icefall-cave-walk", 250, Era::Run, None, Src::Guess);
    }
    b.trainer("icefall-grunt", "TRAINER_TEAM_ROCKET_GRUNT_45", 500);
    if p.postgame_calib {
        // The WR's Champ 1 -> Icefall split is 9:34 against the model's 7:31 (minus the 80 s above):
        // the ice-sliding rooms. 43 s.
        b.text("icefall-ice-slides", 2580, Src::Guess);
    }
    b.text("icefall-cave", 1500, Src::Guess);
    if p.postgame_events {
        b.walk("icefall-cave-out", 86, Era::Run, None, Src::Tiles);
        // Five Island harbor to the Meadow's Warehouse door: 82 tiles with one forced Rocket grunt
        // (`FiveIsland_Meadow_EventScript_Rocket2`, GRUNT_50, `forced.py`).
        b.walk("five-island-approach", 82, Era::Run, None, Src::Tiles);
        b.trainer("meadow-grunt", "TRAINER_TEAM_ROCKET_GRUNT_50", 500);
    } else {
        b.walk("five-island-approach", 250, Era::Run, None, Src::Guess);
    }
    if p.postgame_calib {
        // Icefall split 2:58:24 -> Warehouse entry at ~3:00:25 (scene log "Rocket Warehouse
        // 3:01-3:06" at video time = run + 35 s): ~2:00 against the model's 0:37, 100 s of it here.
        b.text(
            "five-island-resort",
            if p.postgame_events {
                p.resort_text
            } else {
                6000
            },
            Src::Guess,
        );
    }
    b.trainer("warehouse-grunt-1", "TRAINER_TEAM_ROCKET_GRUNT_47", 500);
    b.trainer("warehouse-admin-1", "TRAINER_TEAM_ROCKET_ADMIN", 700);
    b.trainer("warehouse-admin-2", "TRAINER_TEAM_ROCKET_ADMIN_2", 700);
    if p.postgame_calib {
        // The remainder of the 9:24 Icefall -> Warehouse split (the model's split ends at Gideon):
        // interior walking between the trainers. 120 s.
        b.text("warehouse-interior", 7200, Src::Guess);
    }
    b.trainer("warehouse-gideon", "TRAINER_SCIENTIST_GIDEON", 700);
    b.text("sapphire-and-sail-back", 3200, Src::Guess);
    b.text("celio-network-machine", 2400, Src::Guess);
    b.split();
    if p.mewtwo {
        b.text("fly-cerulean", 700, Src::Guess);
        b.walk(
            "cerulean-cave",
            if p.rope_cerulean_cave { 275 } else { 450 },
            Era::Run,
            Some((
                "MAP_CERULEAN_CAVE_1F",
                if p.rope_cerulean_cave { 95 } else { 150 },
            )),
            Src::Guess,
        );
        b.push("mewtwo", Src::Guess, 10, Kind::Mewtwo);
    }
    leg_e4_2(b, p, h);
}
