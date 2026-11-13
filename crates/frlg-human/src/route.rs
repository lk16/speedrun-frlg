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
        /// Prize money (`battle_script_commands.c:5373`).
        money: u32,
        /// The mons with IVs and moves, for the battle simulation.
        full: Vec<crate::data::PartyMon>,
        /// AI bits and heal items (`trainers.h` `.aiFlags`, `.items`).
        ai: u32,
        heals: u32,
    },
    /// Several real parties fought back to back (trainers of the given maps, hardest first).
    Parties {
        parties: Vec<Vec<(u32, usize)>>,
        money: u32,
        fulls: Vec<Vec<crate::data::PartyMon>>,
        ais: Vec<u32>,
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
    /// Teleport with an Abra if one was caught on the way, else walk back (run speed).
    TeleportOrWalk { frames: u32, tiles: u32 },
    /// Master Ball on Mewtwo.
    Mewtwo,
    /// A gift mon or a one-off wild battle.
    Special { special: Special },
    /// A Poké Mart stop: buy balls with the money on hand.
    Mart { mart: crate::params::Mart },
    /// Teach a TM or tutor move (battle_sim).
    Teach { teach: crate::params::Teach },
    /// Buy X items (battle_sim).
    XBuy { xbuy: crate::params::XBuy },
    /// Pick up an Elixir-class item (battle_sim).
    Elixir { pickup: crate::params::ElixirPickup },
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
    skip: &'a [String],
    group_n: &'a std::collections::HashMap<String, u32>,
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
        self.wander_area(name, site, Area::Land, steps, era);
    }

    fn wander_area(&mut self, name: &str, site: &str, area: Area, steps: u32, era: Era) {
        self.push(
            name,
            Src::Guess,
            0,
            Kind::Wander {
                enc: Encounters {
                    site: site.into(),
                    area,
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
        let f = self.dex.fight(id);
        self.push(
            name,
            Src::Guess,
            8,
            Kind::Trainer {
                party: f.party,
                intro,
                adv,
                money: f.money,
                full: f.full,
                ai: f.ai,
                heals: f.heals,
            },
        );
    }

    fn group(&mut self, name: &str, n: u32, mons: u32, level: u32, yield_: u32) {
        if self.skip.iter().any(|s| s == name) {
            return;
        }
        let n = *self.group_n.get(name).unwrap_or(&n);
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

    /// Trainers the sight-cone audit (`human-run/tools/forced.py`) says the leg cannot avoid:
    /// `always` all fought, and from each `(k, options)` the `k` weakest options (the human walks
    /// into the cheapest of the interchangeable ones).
    fn forced(&mut self, name: &str, always: &[&str], alts: &[(usize, &[&str])]) {
        let fight = |l: &&str| self.dex.fight(self.dex.script_trainer(l));
        let mut fights: Vec<crate::data::Fight> = always.iter().map(fight).collect();
        for (k, opts) in alts {
            let mut o: Vec<_> = opts.iter().map(fight).collect();
            o.sort_by_key(|f| f.party.iter().map(|x| x.0).sum::<u32>());
            fights.extend(o.into_iter().take(*k));
        }
        fights.sort_by_key(|f| std::cmp::Reverse(f.party.iter().map(|x| x.0).sum::<u32>()));
        let n = fights.len() as u32;
        let money = fights.iter().map(|f| f.money).sum();
        let parties = fights.iter().map(|f| f.party.clone()).collect();
        let ais = fights.iter().map(|f| f.ai).collect();
        let fulls = fights.into_iter().map(|f| f.full).collect();
        self.push(
            name,
            Src::Tiles,
            6 * n,
            Kind::Parties {
                parties,
                money,
                fulls,
                ais,
            },
        );
    }

    /// The `n` hardest trainers on the given maps. `n` is a routing assumption (which ones the
    /// route cannot walk around), the parties are the decomp's.
    fn maps_group(&mut self, name: &str, maps: &[&str], n: u32) {
        if self.skip.iter().any(|s| s == name) {
            return;
        }
        let n = *self.group_n.get(name).unwrap_or(&n) as usize;
        let fights: Vec<_> = self.dex.map_parties(maps).into_iter().take(n).collect();
        let money = fights.iter().map(|f| f.money).sum();
        let parties = fights.iter().map(|f| f.party.clone()).collect();
        let ais = fights.iter().map(|f| f.ai).collect();
        let fulls = fights.into_iter().map(|f| f.full).collect();
        self.push(
            name,
            Src::Tiles,
            6 * n as u32,
            Kind::Parties {
                parties,
                money,
                fulls,
                ais,
            },
        );
    }

    fn split(&mut self) {
        self.segs.last_mut().unwrap().split = true;
    }
}

fn erika_leg(b: &mut B<'_>, p: &Params, e: Era) {
    b.walk("erika-walk", 146, e, None, Src::Tiles);
    if p.audited_forced {
        b.forced(
            "celadon-gym-trainers",
            &[],
            &[(
                2,
                &[
                    "CeladonCity_Gym_EventScript_Bridget",
                    "CeladonCity_Gym_EventScript_Kay",
                    "CeladonCity_Gym_EventScript_Lisa",
                    "CeladonCity_Gym_EventScript_Lori",
                    "CeladonCity_Gym_EventScript_Mary",
                    "CeladonCity_Gym_EventScript_Tamia",
                    "CeladonCity_Gym_EventScript_Tina",
                ],
            )],
        );
    } else {
        b.group("celadon-gym-trainers", 2, 2, 25, 75);
    }
    b.trainer("erika", "TRAINER_LEADER_ERIKA", 600);
    b.text("erika-badge", 600, Src::Guess);
    b.split();
}

pub fn build(dex: &Dex, p: &Params) -> Vec<Seg> {
    // The rival's trainer ids are named after the *rival's* starter, which is the one that beats
    // the player's: `decompiled/data/maps/PalletTown_ProfessorOaksLab/scripts.inc:303-306`
    // ("Names below refer to the Rival's mon (e.g. for RivalBattleSquirtle, player has Charmander)").
    let sfx = match p.starter.as_str() {
        "squirtle" => "BULBASAUR",
        "bulbasaur" => "CHARMANDER",
        "charmander" => "SQUIRTLE",
        s => panic!("unknown starter {s}"),
    };
    let tn = |s: &str| format!("{s}_{sfx}");
    let mut b = B {
        segs: vec![],
        dex,
        skip: &p.skip_groups,
        group_n: &p.group_n,
        era: Era::Walk,
    };

    // Grass steps: the path table (`pathenc.py`: shortest path in tiles, fewest grass tiles among the
    // shortest; `avoid` = a path that pays up to 50 tiles per grass tile) or the original guess.
    let g = |short: u32, avoid: u32, guess: u32| {
        if !p.enc_path {
            guess
        } else if p.avoid_grass {
            avoid
        } else {
            short
        }
    };
    let enc =
        |site: &'static str, short: u32, avoid: u32, guess: u32| -> Option<(&'static str, u32)> {
            let n = g(short, avoid, guess);
            (n > 0).then_some((site, n))
        };
    // Extra tiles walked to stay out of grass.
    let detour = |b: &mut B, name: &str, tiles: u32| {
        if p.enc_path && p.avoid_grass && tiles > 0 {
            b.walk(name, tiles, Era::Walk, None, Src::Tiles);
        }
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
    detour(&mut b, "route1-detour", 16);
    b.wander("route1-encounters", "MAP_ROUTE1", g(22, 20, 50));
    b.text("parcel", 1278, Src::Ledger);
    b.text("deliver", 4422, Src::Ledger);
    b.text("tutorial", 5551, Src::Ledger);
    b.text("to-forest", 1161, Src::Ledger);
    detour(&mut b, "route2-detour", 4);
    b.wander("route2-encounters", "MAP_ROUTE2", g(5, 0, 60));
    b.text("forest", 6708, Src::Ledger);
    detour(&mut b, "forest-detour", 20);
    b.wander("forest-encounters", "MAP_VIRIDIAN_FOREST", g(81, 48, 100));
    b.text("to-pewter", 1531, Src::Ledger);
    if p.audited_forced {
        b.forced(
            "forest-trainers",
            &["ViridianForest_EventScript_Sammy"],
            &[],
        );
    } else {
        b.maps_group("forest-trainers", &["ViridianForest"], 3);
    }
    b.trainer_adv("brock", "TRAINER_LEADER_BROCK", 600, 1.4);
    b.split();
    b.era = Era::Run;

    // 2. Pewter to Cerulean: 492 tiles (distances.json pewter-gym_to_cerulean-pc), split by guess.
    b.walk(
        "route3",
        120,
        b.era,
        enc("MAP_ROUTE3", 0, 0, 50),
        Src::Guess,
    );
    if p.audited_forced {
        b.forced(
            "route3-trainers",
            &["Route3_EventScript_Colton", "Route3_EventScript_James"],
            &[(1, &["Route3_EventScript_Greg", "Route3_EventScript_Sally"])],
        );
    } else {
        b.group("route3-trainers", 4, 2, 9, 55);
    }
    b.walk(
        "mt-moon-1f",
        90,
        b.era,
        enc("MAP_MT_MOON_1F", 65, 65, 90),
        Src::Guess,
    );
    b.walk(
        "mt-moon-b1f",
        100,
        b.era,
        enc("MAP_MT_MOON_B1F", 13, 13, 100),
        Src::Guess,
    );
    b.walk(
        "mt-moon-b2f",
        100,
        b.era,
        enc("MAP_MT_MOON_B2F", 116, 116, 100),
        Src::Guess,
    );
    if p.audited_forced {
        b.forced(
            "mt-moon-miguel",
            &[
                "MtMoon_B2F_EventScript_BattleMiguel",
                "MtMoon_B2F_EventScript_Grunt1",
            ],
            &[],
        );
    } else {
        b.group("mt-moon-trainers", 5, 2, 11, 60);
        b.maps_group("mt-moon-miguel", &["MtMoon_B2F"], 1);
    }
    b.walk("route4", 82, b.era, enc("MAP_ROUTE4", 0, 0, 30), Src::Guess);
    b.text("cerulean-heal", 500, Src::Guess);
    if p.early_dig {
        // The robbed house's Rocket is visible from the first Cerulean arrival
        // (`data/maps/CeruleanCity/map.json:67` hides him behind
        // FLAG_HIDE_CERULEAN_ROCKET, which nothing in the decomp ever sets), so TM28
        // Dig can be fetched before the rival (`CeruleanCity/scripts.inc:164-187`).
        b.trainer("dig-grunt", "TRAINER_TEAM_ROCKET_GRUNT_5", 600);
        b.text("tm28-dig", 900, Src::Guess);
    }

    // 3. Cerulean: rival, Nugget Bridge, Bill, Misty.
    b.trainer("rival-cerulean", &tn("TRAINER_RIVAL_CERULEAN"), 700);
    b.walk(
        "nugget-bridge",
        100,
        b.era,
        enc("MAP_ROUTE24", 0, 0, 60),
        Src::Guess,
    );
    if p.audited_forced {
        b.forced(
            "nugget-bridge-trainers",
            &[
                "Route24_EventScript_Ethan",
                "Route24_EventScript_Reli",
                "Route24_EventScript_Timmy",
                "Route24_EventScript_Ali",
                "Route24_EventScript_Cale",
                "Route24_EventScript_BattleRocket",
            ],
            &[],
        );
    } else {
        b.maps_group("nugget-bridge-trainers", &["Route24"], 6);
    }
    b.walk("bill", 56, b.era, enc("MAP_ROUTE25", 0, 0, 40), Src::Guess);
    if p.audited_forced {
        b.forced(
            "route25-trainers",
            &["Route25_EventScript_Haley", "Route25_EventScript_Kelsey"],
            &[(
                2,
                &[
                    "Route25_EventScript_Flint",
                    "Route25_EventScript_Franklin",
                    "Route25_EventScript_Nob",
                    "Route25_EventScript_Wayne",
                ],
            )],
        );
    }
    b.text("bill-cottage", 2400, Src::Guess);
    if p.teleport_abra {
        b.push(
            "teleport-cerulean",
            Src::Guess,
            15,
            Kind::TeleportOrWalk {
                frames: 700,
                tiles: 156,
            },
        );
    } else {
        b.walk("bill-return", 156, b.era, None, Src::Tiles);
    }
    b.walk("misty-walk", 26, b.era, None, Src::Tiles);
    if p.audited_forced {
        b.forced(
            "cerulean-gym-trainer",
            &["CeruleanCity_Gym_EventScript_Diana"],
            &[],
        );
    } else {
        b.group("cerulean-gym-trainer", 1, 2, 16, 65);
    }
    b.trainer("misty", "TRAINER_LEADER_MISTY", 600);
    b.text("misty-badge", 600, Src::Guess);
    b.split();

    // 4. Vermilion: S.S. Anne and Surge, no Vermilion heal.
    b.walk(
        "route5-6",
        260,
        b.era,
        enc("MAP_ROUTE5", 6, 0, 40),
        Src::Tiles,
    );
    detour(&mut b, "route5-6-detour", 21);
    b.wander("route6-encounters", "MAP_ROUTE6", g(3, 0, 40));
    if p.audited_forced {
        b.forced("route6-trainers", &["Route6_EventScript_Jeff"], &[]);
    }
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
        b.push(
            "teleport-cerulean-2",
            Src::Guess,
            15,
            Kind::TeleportOrWalk {
                frames: 700,
                tiles: 260,
            },
        );
    } else {
        b.walk("vermilion-return", 260, b.era, None, Src::Tiles);
    }

    // 5. Bike, Dig, Rock Tunnel, Lavender.
    if p.bike {
        b.walk("bike-shop", 40, b.era, None, Src::Tiles);
        b.text("bike-redeem", 1200, Src::Guess);
        b.era = Era::Bike;
    }
    if !p.early_dig {
        // The Rocket who stole TM28 (`CeruleanCity/scripts.inc:172`).
        b.trainer("dig-grunt", "TRAINER_TEAM_ROCKET_GRUNT_5", 600);
        b.text("tm28-dig", 900, Src::Guess);
    }
    let e = b.era;
    b.walk("route9", 60, e, enc("MAP_ROUTE9", 0, 0, 30), Src::Guess);
    b.walk(
        "route10-north",
        20,
        e,
        enc("MAP_ROUTE10", 0, 0, 10),
        Src::Guess,
    );
    b.walk(
        "rock-tunnel",
        138,
        e,
        enc("MAP_ROCK_TUNNEL_1F", 252, 252, 138),
        Src::Guess,
    );
    if p.audited_forced {
        b.forced(
            "rock-tunnel-trainers",
            &[
                "Route9_EventScript_Alicia",
                "Route9_EventScript_Conner",
                "RockTunnel_1F_EventScript_Ashton",
                "RockTunnel_1F_EventScript_Dana",
                "RockTunnel_B1F_EventScript_Dudley",
                "RockTunnel_B1F_EventScript_Martha",
                "RockTunnel_B1F_EventScript_Sofia",
                "RockTunnel_B1F_EventScript_Winston",
            ],
            &[
                (
                    1,
                    &[
                        "RockTunnel_1F_EventScript_Lenny",
                        "RockTunnel_1F_EventScript_Oliver",
                    ],
                ),
                (
                    1,
                    &[
                        "RockTunnel_B1F_EventScript_Allen",
                        "RockTunnel_B1F_EventScript_Eric",
                    ],
                ),
            ],
        );
    } else {
        b.maps_group(
            "rock-tunnel-trainers",
            &["RockTunnel_1F", "RockTunnel_B1F"],
            7,
        );
    }
    b.text("lavender-heal", 500, Src::Guess);

    // 6. Celadon: Tea, Hideout, Erika, HM02.
    b.walk("route8-7", 227, e, enc("MAP_ROUTE8", 0, 0, 40), Src::Tiles);
    if p.audited_forced {
        b.forced(
            "route8-trainers",
            &[],
            &[(1, &["Route8_EventScript_Julia", "Route8_EventScript_Rich"])],
        );
    }
    b.walk("celadon-tea", 40, e, None, Src::Tiles);
    b.text("tea", 600, Src::Guess);
    b.walk("hideout-walk", 526, e, None, Src::Guess);
    b.maps_group(
        "hideout-grunts",
        &[
            "RocketHideout_B1F",
            "RocketHideout_B2F",
            "RocketHideout_B3F",
            "RocketHideout_B4F",
        ],
        8,
    );
    b.trainer("hideout-giovanni", "TRAINER_BOSS_GIOVANNI", 700);
    b.text("scope-and-dig", 1800, Src::Guess);
    if !p.erika_after_tower {
        erika_leg(&mut b, p, e);
    }
    b.walk("hm02-trip", 218, e, None, Src::Tiles);
    b.text("hm02-fly", 800, Src::Guess);

    // 7. Pokémon Tower and the flute.
    b.text("fly-lavender", 700, Src::Guess);
    b.walk("tower-walk", 215, e, None, Src::Guess);
    b.wander("tower-encounters", "MAP_POKEMON_TOWER_5F", g(108, 108, 80));
    b.trainer("rival-tower", &tn("TRAINER_RIVAL_POKEMON_TOWER"), 700);
    if p.audited_forced {
        b.forced(
            "tower-channelers",
            &[
                "PokemonTower_6F_EventScript_Emilia",
                "PokemonTower_6F_EventScript_Jennifer",
            ],
            &[(
                1,
                &[
                    "PokemonTower_5F_EventScript_Karina",
                    "PokemonTower_5F_EventScript_Tammy",
                ],
            )],
        );
    }
    b.maps_group("tower-rockets", &["PokemonTower_7F"], 3);
    b.text("fuji-flute", 1500, Src::Guess);

    // 8. Cycling Road to Fuchsia, Safari, HM03 and HM04, Koga.
    b.text("fly-celadon", 700, Src::Guess);
    if p.erika_after_tower {
        // Erika postponed past the tower: the carry is two levels up and usually a
        // Blastoise by now, and the fly lands in Celadon anyway (walk re-used; Guess).
        let e = b.era;
        erika_leg(&mut b, p, e);
    }
    b.walk(
        "cycling-road",
        374,
        e,
        enc("MAP_ROUTE16", 0, 0, 60),
        Src::Tiles,
    );
    b.text("snorlax", 1500, Src::Guess);
    // The rod's house trip (guess: walking there and back plus the Fishing Guru's text).
    match p.rod.as_str() {
        "old" => {
            b.text("old-rod", 900, Src::Guess);
        }
        "good" => {
            b.text("good-rod", 1500, Src::Guess);
        }
        "super" => {
            b.text("super-rod", 2400, Src::Guess);
        }
        _ => {}
    }
    b.walk("safari-trip", 198, e, None, Src::Tiles);
    b.text("safari-fee-and-hms", 2400, Src::Guess);
    b.walk("safari-walk", 460, Era::Run, None, Src::Tiles);
    b.walk("warden", 18, e, None, Src::Tiles);
    b.text("warden-strength", 1400, Src::Guess);
    b.walk("koga-walk", 119, e, None, Src::Guess);
    if p.audited_forced {
        b.forced(
            "fuchsia-gym-trainers",
            &["FuchsiaCity_Gym_EventScript_Kirk"],
            &[],
        );
    } else {
        b.group("fuchsia-gym-trainers", 3, 2, 34, 75);
    }
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
    // The audit reaches Sabrina through the teleport pads without crossing any cone.
    if !p.audited_forced {
        b.group("saffron-gym-trainers", 2, 2, 36, 75);
    }
    b.trainer("sabrina", "TRAINER_LEADER_SABRINA", 600);
    b.text("sabrina-badge", 600, Src::Guess);
    b.split();

    // 10. Cinnabar: Mansion, Blaine, and the forced trip to One Island.
    b.text("fly-pallet", 700, Src::Guess);
    b.walk("surf-cinnabar", 146, Era::Surf, None, Src::Tiles);
    if p.enc_path {
        // Surfing Route 21: 47 + 42 water-encounter steps (`pathenc.py`, pallet_to_cinnabar-pc).
        b.wander_area(
            "surf-route21-n",
            "MAP_ROUTE21_NORTH",
            Area::Water,
            g(47, 38, 47),
            Era::Surf,
        );
        b.wander_area(
            "surf-route21-s",
            "MAP_ROUTE21_SOUTH",
            Area::Water,
            g(36, 38, 36),
            Era::Surf,
        );
    }
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
    if p.audited_forced {
        b.forced(
            "viridian-gym-trainers",
            &["ViridianCity_Gym_EventScript_Takashi"],
            &[],
        );
    } else {
        b.group("viridian-gym-trainers", 3, 2, 42, 80);
    }
    b.trainer("giovanni", "TRAINER_LEADER_GIOVANNI", 600);
    b.text("giovanni-badge", 600, Src::Guess);
    b.split();

    // 12. League approach.
    b.walk("route22", 113, e, enc("MAP_ROUTE22", 10, 9, 20), Src::Tiles);
    b.trainer("rival-route22", &tn("TRAINER_RIVAL_ROUTE22_LATE"), 700);
    b.walk("route23", 167, Era::Surf, None, Src::Tiles);
    if p.enc_path {
        // 39 water-encounter steps on Route 23 (`pathenc.py`, route22-gate_to_victoryroad).
        b.wander_area(
            "route23-water",
            "MAP_ROUTE23",
            Area::Water,
            g(39, 31, 39),
            Era::Surf,
        );
    }
    b.text("badge-gates", 900, Src::Guess);
    b.walk(
        "victory-road",
        255,
        e,
        enc("MAP_VICTORY_ROAD_1F", 91, 91, 150),
        Src::Guess,
    );
    // The audit finds no trainer on Victory Road the route cannot walk around.
    if !p.audited_forced {
        b.maps_group(
            "victory-road-trainers",
            &["VictoryRoad_1F", "VictoryRoad_2F", "VictoryRoad_3F"],
            8,
        );
    }
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

    for name in &p.extra_splits {
        let seg = b
            .segs
            .iter_mut()
            .find(|s| s.name == *name)
            .unwrap_or_else(|| panic!("extra split {name} matches no segment"));
        seg.split = true;
    }

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
    for g in p.grinds.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == g.after)
            .unwrap_or_else(|| panic!("grind anchor {} matches no segment", g.after));
        b.segs.insert(
            at + 1,
            Seg {
                name: format!("grind@{}", g.after),
                src: Src::Guess,
                inputs: 0,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Grind {
                    battles: g.battles,
                    level: g.level,
                    yield_: g.yield_,
                },
            },
        );
    }
    for m in p.marts.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == m.after)
            .unwrap_or_else(|| panic!("mart anchor {} matches no segment", m.after));
        b.segs.insert(
            at + 1,
            Seg {
                name: format!("mart@{}", m.after),
                src: Src::Guess,
                inputs: 10,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Mart { mart: m.clone() },
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
    for t in p.tms.iter().rev() {
        dex.move_id(&t.mv);
        let at = b
            .segs
            .iter()
            .position(|s| s.name == t.after)
            .unwrap_or_else(|| panic!("tm anchor {} matches no segment", t.after));
        b.segs.insert(
            at + 1,
            Seg {
                name: format!(
                    "tm-{}@{}",
                    t.mv.trim_start_matches("MOVE_").to_lowercase(),
                    t.after
                ),
                src: Src::Guess,
                inputs: 10,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Teach { teach: t.clone() },
            },
        );
    }
    for h in p.extra_heals.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == *h)
            .unwrap_or_else(|| panic!("heal anchor {h} matches no segment"));
        // The name contains "heal": the sim restores HP, PP, status and fodder there.
        b.segs.insert(
            at + 1,
            Seg {
                name: format!("center-heal@{h}"),
                src: Src::Guess,
                inputs: 8,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Text {
                    frames: p.extra_heal_frames,
                },
            },
        );
    }
    for e in p.elixirs.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == e.after)
            .unwrap_or_else(|| panic!("elixir anchor {} matches no segment", e.after));
        b.segs.insert(
            at + 1,
            Seg {
                name: format!("elixir-{}@{}", e.kind, e.after),
                src: Src::Guess,
                inputs: 4,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Elixir { pickup: e.clone() },
            },
        );
    }
    for x in p.xbuys.iter().rev() {
        let at = b
            .segs
            .iter()
            .position(|s| s.name == x.after)
            .unwrap_or_else(|| panic!("xbuy anchor {} matches no segment", x.after));
        b.segs.insert(
            at + 1,
            Seg {
                name: format!("xbuy@{}", x.after),
                src: Src::Guess,
                inputs: 12,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::XBuy { xbuy: x.clone() },
            },
        );
    }
    b.segs
}
