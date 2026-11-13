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
    for (nm, id) in [
        ("lorelei", "TRAINER_ELITE_FOUR_LORELEI".to_string()),
        ("bruno", "TRAINER_ELITE_FOUR_BRUNO".to_string()),
        ("agatha", "TRAINER_ELITE_FOUR_AGATHA".to_string()),
        ("lance", "TRAINER_ELITE_FOUR_LANCE".to_string()),
        ("champion", tn("TRAINER_CHAMPION_FIRST")),
    ] {
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
    b.text("celio-rainbow-pass", 2800, Src::Guess);
    // Four Island's Icefall Cave: the Lorelei scene and one forced grunt
    // (`decompiled/data/maps/FourIsland_IcefallCave_Back/scripts.inc`, `LoreleiRocketsScene`).
    b.text("sail-to-four", 900, Src::Guess);
    b.walk("icefall-cave-walk", 250, Era::Run, None, Src::Guess);
    b.trainer("icefall-grunt", "TRAINER_TEAM_ROCKET_GRUNT_45", 500);
    b.text("icefall-cave", 1500, Src::Guess);
    b.walk("five-island-approach", 250, Era::Run, None, Src::Guess);
    b.trainer("warehouse-grunt-1", "TRAINER_TEAM_ROCKET_GRUNT_47", 500);
    b.trainer("warehouse-admin-1", "TRAINER_TEAM_ROCKET_ADMIN", 700);
    b.trainer("warehouse-admin-2", "TRAINER_TEAM_ROCKET_ADMIN_2", 700);
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
