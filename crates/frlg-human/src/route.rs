//! The route as a sequence of segments, built from plan parameters.
//!
//! Every frame constant carries a `Src`:
//! - `Ledger`: measured on tier 1 and tier-2 accepted, `route/defeat-brock/ledger.json` (TAS speed;
//!   the human multiplier is applied on top).
//! - `Tiles`: a tile count from `docs/glitchless-run/distances.json` (static BFS floor).
//! - `Guess`: invented here. Nothing about it is measured or cited.

use crate::data::{Area, Dex};
use crate::params::{Hunt, Params, Special};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Src {
    Ledger,
    Tiles,
    Guess,
}

/// Frames per tile, `decompiled/src/event_object_movement.c:158-162,8866-8931` as cited by
/// `docs/glitchless-run/distances.json`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Era {
    Walk,
    Run,
    Bike,
    Surf,
}

impl Era {
    pub fn frames_per_tile(self) -> f64 {
        match self {
            Era::Walk => 16.0,
            Era::Run => 8.0,
            Era::Bike => 6.0,
            Era::Surf => 8.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Encounters {
    pub site: String,
    pub area: Area,
    /// Steps on encounter tiles (grass, cave floor or water).
    pub steps: u32,
}

#[derive(Clone, Debug)]
pub enum Kind {
    /// Text, menus and cutscenes: a floor in frames, scaled by `text_factor`.
    Text { frames: u32 },
    /// Walking: tiles at the era's speed, plus wild encounters on `enc`.
    Walk {
        tiles: u32,
        era: Era,
        enc: Option<Encounters>,
    },
    /// Only the random encounters of a stretch whose walking is already counted elsewhere.
    Wander { enc: Encounters, era: Era },
    /// A trainer with a real party from the decomp.
    Trainer {
        /// (level, species index)
        party: Vec<(u32, usize)>,
        intro: u32,
        /// Type-matchup multiplier on the player's effective level (guess).
        adv: f64,
    },
    /// Trainers we only know in aggregate (a guess).
    Group {
        n: u32,
        mons: u32,
        level: u32,
        yield_: u32,
    },
    /// Stand in the encounter tiles until the new targets are caught or the budget runs out.
    Hunt { hunt: Hunt, era: Era },
    /// Wild fights for exp.
    Grind {
        battles: u32,
        level: u32,
        yield_: u32,
    },
    /// Oak's rating: fewer than 60 Kanto species caught triggers the `fill` hunts.
    OakCheck,
    /// Buy stones (Celadon 4F) and evolve whatever evolves by them.
    Stones,
    /// Master Ball on Mewtwo.
    Mewtwo,
    /// A gift mon or a one-off wild battle.
    Special { special: Special },
}

#[derive(Clone, Debug)]
pub struct Seg {
    pub name: String,
    pub src: Src,
    /// Mashed or menu inputs, each late by U{0..jitter_n}.
    pub inputs: u32,
    /// The human checks the reset rule after this segment.
    pub split: bool,
    /// Counted on the wall clock but not on the run timer (before New Game).
    pub off_timer: bool,
    /// The starter's stats are read after this segment.
    pub starter_check: bool,
    pub kind: Kind,
}

struct B<'a> {
    segs: Vec<Seg>,
    dex: &'a Dex,
    era: Era,
}

impl<'a> B<'a> {
    fn push(&mut self, name: &str, src: Src, inputs: u32, kind: Kind) -> &mut Seg {
        self.segs.push(Seg {
            name: name.to_string(),
            src,
            inputs,
            split: false,
            off_timer: false,
            starter_check: false,
            kind,
        });
        self.segs.last_mut().unwrap()
    }

    fn text(&mut self, name: &str, frames: u32, src: Src) -> &mut Seg {
        self.push(name, src, frames / 45, Kind::Text { frames })
    }

    fn walk(&mut self, name: &str, tiles: u32, era: Era, enc: Option<(&str, u32)>, src: Src) {
        let enc = enc.map(|(s, steps)| Encounters {
            site: s.to_string(),
            area: Area::Land,
            steps,
        });
        self.push(name, src, tiles / 10, Kind::Walk { tiles, era, enc });
    }

    fn wander(&mut self, name: &str, site: &str, steps: u32) {
        let era = self.era;
        self.push(
            name,
            Src::Guess,
            0,
            Kind::Wander {
                enc: Encounters {
                    site: site.into(),
                    area: Area::Land,
                    steps,
                },
                era,
            },
        );
    }

    fn trainer(&mut self, name: &str, id: &str, intro: u32) {
        self.trainer_adv(name, id, intro, 1.0);
    }

    fn trainer_adv(&mut self, name: &str, id: &str, intro: u32, adv: f64) {
        let party = self.dex.trainer(id).to_vec();
        self.push(name, Src::Guess, 8, Kind::Trainer { party, intro, adv });
    }

    fn group(&mut self, name: &str, n: u32, mons: u32, level: u32, yield_: u32) {
        self.push(
            name,
            Src::Guess,
            6 * n,
            Kind::Group {
                n,
                mons,
                level,
                yield_,
            },
        );
    }

    fn split(&mut self) {
        self.segs.last_mut().unwrap().split = true;
    }
}

pub fn build(dex: &Dex, p: &Params) -> Vec<Seg> {
    assert_eq!(p.starter, "squirtle", "only the Squirtle line is wired up");
    let sfx = "SQUIRTLE";
    let tn = |s: &str| format!("{s}_{sfx}");
    let mut b = B {
        segs: vec![],
        dex,
        era: Era::Walk,
    };

    // 1. Power-on to Brock beaten. Non-battle segments are the defeat-brock ledger's frames
    // (TAS speed). Its battles are re-modelled by the trainer model instead.
    b.text("boot", 646, Src::Ledger).off_timer = true;
    b.text("intro-oak", 1565, Src::Ledger);
    b.text("names", 1045, Src::Ledger);
    b.text("options", 197, Src::Ledger);
    b.text("house", 432, Src::Ledger);
    b.text("to-lab", 1211, Src::Ledger);
    b.text("starter", 1722, Src::Ledger).starter_check = true;
    b.trainer("rival-lab", &tn("TRAINER_RIVAL_OAKS_LAB"), 354);
    b.text("exit-lab", 864, Src::Ledger);
    b.text("to-viridian", 2671, Src::Ledger);
    b.wander("route1-encounters", "MAP_ROUTE1", 50);
    b.text("parcel", 1278, Src::Ledger);
    b.text("deliver", 4422, Src::Ledger);
    b.text("tutorial", 5551, Src::Ledger);
    b.text("to-forest", 1161, Src::Ledger);
    b.wander("route2-encounters", "MAP_ROUTE2", 60);
    b.text("forest", 6708, Src::Ledger);
    b.wander("forest-encounters", "MAP_VIRIDIAN_FOREST", 100);
    b.text("to-pewter", 1531, Src::Ledger);
    b.group("forest-trainers", 3, 2, 8, 50);
    b.trainer_adv("brock", "TRAINER_LEADER_BROCK", 600, 1.4);
    b.split();
    b.era = Era::Run;

    // 2. Pewter to Cerulean: 492 tiles (distances.json pewter-gym_to_cerulean-pc), split by guess.
    b.walk("route3", 120, b.era, Some(("MAP_ROUTE3", 50)), Src::Guess);
    b.group("route3-trainers", 4, 2, 9, 55);
    b.walk(
        "mt-moon-1f",
        90,
        b.era,
        Some(("MAP_MT_MOON_1F", 90)),
        Src::Guess,
    );
    b.walk(
        "mt-moon-b1f",
        100,
        b.era,
        Some(("MAP_MT_MOON_B1F", 100)),
        Src::Guess,
    );
    b.walk(
        "mt-moon-b2f",
        100,
        b.era,
        Some(("MAP_MT_MOON_B2F", 100)),
        Src::Guess,
    );
    b.group("mt-moon-trainers", 5, 2, 11, 60);
    b.group("mt-moon-miguel", 1, 3, 12, 60);
    b.walk("route4", 82, b.era, Some(("MAP_ROUTE4", 30)), Src::Guess);
    b.text("cerulean-heal", 500, Src::Guess);

    // 3. Cerulean: rival, Nugget Bridge, Bill, Misty.
    b.trainer("rival-cerulean", &tn("TRAINER_RIVAL_CERULEAN"), 700);
    b.walk(
        "nugget-bridge",
        100,
        b.era,
        Some(("MAP_ROUTE24", 60)),
        Src::Guess,
    );
    b.group("nugget-bridge-trainers", 5, 2, 13, 65);
    b.walk("bill", 56, b.era, Some(("MAP_ROUTE25", 40)), Src::Guess);
    b.text("bill-cottage", 2400, Src::Guess);
    if p.teleport_abra {
        b.text("teleport-cerulean", 700, Src::Guess);
    } else {
        b.walk("bill-return", 156, b.era, None, Src::Tiles);
    }
    b.walk("misty-walk", 26, b.era, None, Src::Tiles);
    b.group("cerulean-gym-trainer", 1, 2, 16, 65);
    b.trainer("misty", "TRAINER_LEADER_MISTY", 600);
    b.text("misty-badge", 600, Src::Guess);
    b.split();

    // 4. Vermilion: S.S. Anne and Surge, no Vermilion heal.
    b.walk("route5-6", 260, b.era, Some(("MAP_ROUTE5", 40)), Src::Tiles);
    b.wander("route6-encounters", "MAP_ROUTE6", 40);
    if p.bike {
        b.walk("fan-club", 60, b.era, None, Src::Tiles);
        b.text("bike-voucher", 900, Src::Guess);
    }
    b.walk("ss-anne", 210, b.era, None, Src::Guess);
    b.trainer("rival-ss-anne", &tn("TRAINER_RIVAL_SS_ANNE"), 700);
    b.text("captain-hm01", 1500, Src::Guess);
    b.walk("surge-walk", 156, b.era, None, Src::Guess);
    b.group("surge-gym-trainers", 2, 2, 20, 65);
    b.trainer("surge", "TRAINER_LEADER_LT_SURGE", 600);
    b.text("surge-badge", 600, Src::Guess);
    b.split();
    if p.teleport_abra {
        b.text("teleport-cerulean-2", 700, Src::Guess);
    } else {
        b.walk("vermilion-return", 260, b.era, None, Src::Tiles);
    }

    // 5. Bike, Dig, Rock Tunnel, Lavender.
    if p.bike {
        b.walk("bike-shop", 40, b.era, None, Src::Tiles);
        b.text("bike-redeem", 1200, Src::Guess);
        b.era = Era::Bike;
    }
    b.group("dig-grunt", 1, 2, 17, 70);
    b.text("tm28-dig", 900, Src::Guess);
    let e = b.era;
    b.walk("route9", 60, e, Some(("MAP_ROUTE9", 30)), Src::Guess);
    b.walk(
        "route10-north",
        20,
        e,
        Some(("MAP_ROUTE10", 10)),
        Src::Guess,
    );
    b.walk(
        "rock-tunnel",
        138,
        e,
        Some(("MAP_ROCK_TUNNEL_1F", 138)),
        Src::Guess,
    );
    b.group("rock-tunnel-trainers", 7, 2, 21, 70);
    b.text("lavender-heal", 500, Src::Guess);

    // 6. Celadon: Tea, Hideout, Erika, HM02.
    b.walk("route8-7", 227, e, Some(("MAP_ROUTE8", 40)), Src::Tiles);
    b.walk("celadon-tea", 40, e, None, Src::Tiles);
    b.text("tea", 600, Src::Guess);
    b.walk("hideout-walk", 526, e, None, Src::Guess);
    b.group("hideout-grunts", 8, 2, 24, 70);
    b.trainer("hideout-giovanni", "TRAINER_BOSS_GIOVANNI", 700);
    b.text("scope-and-dig", 1800, Src::Guess);
    b.walk("erika-walk", 146, e, None, Src::Tiles);
    b.group("celadon-gym-trainers", 2, 2, 25, 75);
    b.trainer("erika", "TRAINER_LEADER_ERIKA", 600);
    b.text("erika-badge", 600, Src::Guess);
    b.split();
    b.walk("hm02-trip", 218, e, None, Src::Tiles);
    b.text("hm02-fly", 800, Src::Guess);

    // 7. Pokémon Tower and the flute.
    b.text("fly-lavender", 700, Src::Guess);
    b.walk("tower-walk", 215, e, None, Src::Guess);
    b.wander("tower-encounters", "MAP_POKEMON_TOWER_5F", 80);
    b.trainer("rival-tower", &tn("TRAINER_RIVAL_POKEMON_TOWER"), 700);
    b.group("tower-rockets", 3, 2, 25, 70);
    b.text("fuji-flute", 1500, Src::Guess);

    // 8. Cycling Road to Fuchsia, Safari, HM03 and HM04, Koga.
    b.text("fly-celadon", 700, Src::Guess);
    b.walk(
        "cycling-road",
        374,
        e,
        Some(("MAP_ROUTE16", 60)),
        Src::Tiles,
    );
    b.text("snorlax", 1500, Src::Guess);
    b.walk("safari-trip", 198, e, None, Src::Tiles);
    b.text("safari-fee-and-hms", 2400, Src::Guess);
    b.walk("safari-walk", 460, Era::Run, None, Src::Tiles);
    b.walk("warden", 18, e, None, Src::Tiles);
    b.text("warden-strength", 1400, Src::Guess);
    b.walk("koga-walk", 119, e, None, Src::Guess);
    b.group("fuchsia-gym-trainers", 3, 2, 34, 75);
    b.trainer("koga", "TRAINER_LEADER_KOGA", 600);
    b.text("koga-badge", 600, Src::Guess);
    b.split();

    // 9. Silph Co. and Sabrina.
    b.text("fly-celadon-2", 700, Src::Guess);
    b.walk("silph-walk", 479, e, None, Src::Guess);
    b.trainer("rival-silph", &tn("TRAINER_RIVAL_SILPH"), 700);
    b.group("silph-grunts", 4, 2, 32, 70);
    b.trainer("silph-giovanni", "TRAINER_BOSS_GIOVANNI_2", 700);
    b.text("master-ball", 1800, Src::Guess);
    b.walk("sabrina-walk", 123, e, None, Src::Guess);
    b.group("saffron-gym-trainers", 2, 2, 36, 75);
    b.trainer("sabrina", "TRAINER_LEADER_SABRINA", 600);
    b.text("sabrina-badge", 600, Src::Guess);
    b.split();

    // 10. Cinnabar: Mansion, Blaine, and the forced trip to One Island.
    b.text("fly-pallet", 700, Src::Guess);
    b.walk("surf-cinnabar", 146, Era::Surf, None, Src::Tiles);
    b.walk(
        "mansion",
        400,
        Era::Run,
        Some(("MAP_POKEMON_MANSION_1F", 120)),
        Src::Guess,
    );
    b.group("mansion-trainers", 3, 2, 36, 75);
    b.text("secret-key", 900, Src::Guess);
    b.walk("blaine-walk", 34, Era::Run, None, Src::Tiles);
    b.group("blaine-quiz", 2, 2, 38, 75);
    b.trainer("blaine", "TRAINER_LEADER_BLAINE", 600);
    b.text("blaine-badge-and-sevii-scene", 5000, Src::Guess);
    b.split();

    // 11. Viridian Gym.
    b.text("fly-viridian", 700, Src::Guess);
    b.walk("viridian-gym-walk", 55, e, None, Src::Tiles);
    b.group("viridian-gym-trainers", 3, 2, 42, 80);
    b.trainer("giovanni", "TRAINER_LEADER_GIOVANNI", 600);
    b.text("giovanni-badge", 600, Src::Guess);
    b.split();

    // 12. League approach.
    b.walk("route22", 113, e, Some(("MAP_ROUTE22", 20)), Src::Tiles);
    b.trainer("rival-route22", &tn("TRAINER_RIVAL_ROUTE22_LATE"), 700);
    b.walk("route23", 167, Era::Surf, None, Src::Tiles);
    b.text("badge-gates", 900, Src::Guess);
    b.walk(
        "victory-road",
        255,
        e,
        Some(("MAP_VICTORY_ROAD_1F", 150)),
        Src::Guess,
    );
    b.group("victory-road-trainers", 8, 2, 42, 80);
    b.text("indigo-heal", 500, Src::Guess);
    if p.grind_pre_e4 > 0 {
        b.push(
            "grind-pre-e4",
            Src::Guess,
            0,
            Kind::Grind {
                battles: p.grind_pre_e4,
                level: p.grind_level,
                yield_: p.grind_yield,
            },
        );
    }
    b.split();

    // 13. First Elite Four, Champion and Hall of Fame.
    for (nm, id) in [
        ("lorelei", "TRAINER_ELITE_FOUR_LORELEI".to_string()),
        ("bruno", "TRAINER_ELITE_FOUR_BRUNO".to_string()),
        ("agatha", "TRAINER_ELITE_FOUR_AGATHA".to_string()),
        ("lance", "TRAINER_ELITE_FOUR_LANCE".to_string()),
        ("champion", tn("TRAINER_CHAMPION_FIRST")),
    ] {
        b.trainer(nm, &id, 900);
    }
    b.text("hall-of-fame-1", 5400, Src::Guess);
    b.split();

    // 14. Post-game: National Dex, Celio's chain, Rocket Warehouse.
    b.text("fly-pallet-2", 700, Src::Guess);
    b.walk("pallet-to-oak", 30, e, None, Src::Tiles);
    b.push("oak-dex-check", Src::Guess, 0, Kind::OakCheck);
    b.text("oak-national-dex", 2500, Src::Guess);
    b.split();
    if !p.stones.is_empty() {
        b.push("stones", Src::Guess, 0, Kind::Stones);
    }
    b.text("to-one-island", 3000, Src::Guess);
    b.text("celio-ruby-request", 1500, Src::Guess);
    b.walk("mt-ember", 300, Era::Run, None, Src::Guess);
    b.group("mt-ember-trainers", 3, 3, 38, 75);
    b.walk("mt-ember-back", 150, Era::Run, None, Src::Guess);
    b.text("celio-rainbow-pass", 2800, Src::Guess);
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
            450,
            Era::Run,
            Some(("MAP_CERULEAN_CAVE_1F", 150)),
            Src::Guess,
        );
        b.push("mewtwo", Src::Guess, 10, Kind::Mewtwo);
    }
    if p.grind_pre_rematch > 0 {
        b.push(
            "grind-pre-rematch",
            Src::Guess,
            0,
            Kind::Grind {
                battles: p.grind_pre_rematch,
                level: p.grind_level,
                yield_: p.grind_yield,
            },
        );
    }

    // 15. Rematches: Elite Four, Champion, second Hall of Fame (timer ends on the black frame).
    b.text("fly-indigo", 700, Src::Guess);
    b.text("league-entry", 700, Src::Guess);
    for (nm, id) in [
        ("lorelei-2", "TRAINER_ELITE_FOUR_LORELEI_2".to_string()),
        ("bruno-2", "TRAINER_ELITE_FOUR_BRUNO_2".to_string()),
        ("agatha-2", "TRAINER_ELITE_FOUR_AGATHA_2".to_string()),
        ("lance-2", "TRAINER_ELITE_FOUR_LANCE_2".to_string()),
        ("champion-2", tn("TRAINER_CHAMPION_REMATCH")),
    ] {
        b.trainer(nm, &id, 900);
    }
    b.text("hall-of-fame-2", 2400, Src::Guess);

    // Hunts go in right after the segment they are anchored to.
    for h in p.hunts.iter().rev() {
        assert!(dex.has_site(&h.site), "hunt site {} unknown", h.site);
        let at = b
            .segs
            .iter()
            .position(|s| s.name == h.after)
            .unwrap_or_else(|| panic!("hunt anchor {} matches no segment", h.after));
        let label = format!(
            "hunt-{}-{}@{}",
            h.site.trim_start_matches("MAP_").to_lowercase(),
            h.area,
            h.after
        );
        let era = if b.segs[..=at].iter().any(|s| s.name == "bike-redeem") {
            Era::Bike
        } else {
            Era::Run
        };
        b.segs.insert(
            at + 1,
            Seg {
                name: label,
                src: Src::Guess,
                inputs: 0,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Hunt {
                    hunt: h.clone(),
                    era,
                },
            },
        );
    }
    for sp in p.specials.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == sp.after)
            .unwrap_or_else(|| panic!("special anchor {} matches no segment", sp.after));
        for n in &sp.species {
            dex.id(n);
        }
        b.segs.insert(
            at + 1,
            Seg {
                name: format!(
                    "{}-{}@{}",
                    sp.kind,
                    sp.species.join("+").replace("SPECIES_", "").to_lowercase(),
                    sp.after
                ),
                src: Src::Guess,
                inputs: sp.frames / 60,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Special {
                    special: sp.clone(),
                },
            },
        );
    }
    b.segs
}
