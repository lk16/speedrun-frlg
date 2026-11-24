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
        /// Of `heals`, the Full Heals (no HP; `Params::ai_cure_items`).
        cures: u32,
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
    /// An evolution banked as a dex entry (stone, candy or lead swap).
    Evolve { evo: crate::params::Evolve },
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

pub struct B<'a> {
    pub(crate) segs: Vec<Seg>,
    pub(crate) dex: &'a Dex,
    pub(crate) skip: &'a [String],
    pub(crate) group_n: &'a std::collections::HashMap<String, u32>,
    pub(crate) era: Era,
    /// Optional groups the plan skips, each mapped to the segment that precedes where it would
    /// have been, so anchors naming a skipped group still resolve (to the point just before it).
    pub(crate) skipped: std::collections::HashMap<String, String>,
}

impl<'a> B<'a> {
    /// Index of the segment a plan anchor names: the segment itself, or, for a skipped group,
    /// the segment before it.
    pub(crate) fn find(&self, name: &str) -> Option<usize> {
        let by = |n: &str| self.segs.iter().position(|s| s.name == n);
        by(name).or_else(|| self.skipped.get(name).and_then(|prev| by(prev)))
    }

    pub(crate) fn push(&mut self, name: &str, src: Src, inputs: u32, kind: Kind) -> &mut Seg {
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

    pub(crate) fn text(&mut self, name: &str, frames: u32, src: Src) -> &mut Seg {
        self.push(name, src, frames / 45, Kind::Text { frames })
    }

    pub(crate) fn walk(
        &mut self,
        name: &str,
        tiles: u32,
        era: Era,
        enc: Option<(&str, u32)>,
        src: Src,
    ) {
        let enc = enc.map(|(s, steps)| Encounters {
            site: s.to_string(),
            area: Area::Land,
            steps,
        });
        self.push(name, src, tiles / 10, Kind::Walk { tiles, era, enc });
    }

    pub(crate) fn wander(&mut self, name: &str, site: &str, steps: u32) {
        let era = self.era;
        self.wander_area(name, site, Area::Land, steps, era);
    }

    pub(crate) fn wander_area(&mut self, name: &str, site: &str, area: Area, steps: u32, era: Era) {
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

    pub(crate) fn trainer(&mut self, name: &str, id: &str, intro: u32) {
        self.trainer_adv(name, id, intro, 1.0);
    }

    pub(crate) fn trainer_adv(&mut self, name: &str, id: &str, intro: u32, adv: f64) {
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
                cures: f.cures,
            },
        );
    }

    pub(crate) fn group(&mut self, name: &str, n: u32, mons: u32, level: u32, yield_: u32) {
        if self.skip.iter().any(|s| s == name) {
            if let Some(prev) = self.segs.last() {
                self.skipped.insert(name.to_string(), prev.name.clone());
            }
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
    pub(crate) fn forced(&mut self, name: &str, always: &[&str], alts: &[(usize, &[&str])]) {
        let fight = |l: &&str| self.dex.fight(self.dex.script_trainer(l));
        let mut fights: Vec<crate::data::Fight> = always.iter().map(fight).collect();
        for (k, opts) in alts {
            let mut o: Vec<_> = opts.iter().map(fight).collect();
            o.sort_by_key(|f| f.party.iter().map(|x| x.0).sum::<u32>());
            fights.extend(o.into_iter().take(*k));
        }
        fights.sort_by_key(|f| std::cmp::Reverse(f.party.iter().map(|x| x.0).sum::<u32>()));
        // `group_n` may cap the fights of a forced list (the hardest are kept): the WR run's window
        // times are too short for every cone-forced trainer to be fought.
        if let Some(&cap) = self.group_n.get(name) {
            fights.truncate(cap as usize);
        }
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

    /// `Params::fight_lists`: replace a `Parties`/`Group` segment's fights with the given trainer ids, or
    /// insert them as a new `<seg>+fights` segment after any other segment. An empty list on a `Trainer`
    /// segment removes that fight.
    pub(crate) fn apply_fight_lists(
        &mut self,
        lists: &std::collections::BTreeMap<String, Vec<String>>,
    ) -> Result<(), String> {
        for (seg, ids) in lists {
            let i = self
                .segs
                .iter()
                .position(|s| s.name == *seg)
                .ok_or_else(|| format!("fight_lists: no segment {seg}"))?;
            let fights: Vec<crate::data::Fight> = ids
                .iter()
                .map(|id| {
                    if self.dex.has_trainer(id) {
                        Ok(self.dex.fight(id))
                    } else {
                        Err(format!("fight_lists: unknown trainer {id}"))
                    }
                })
                .collect::<Result<_, _>>()?;
            let n = fights.len() as u32;
            let kind = Kind::Parties {
                money: fights.iter().map(|f| f.money).sum(),
                parties: fights.iter().map(|f| f.party.clone()).collect(),
                ais: fights.iter().map(|f| f.ai).collect(),
                fulls: fights.into_iter().map(|f| f.full).collect(),
            };
            // An empty list on a single-trainer segment drops that fight (a trainer the runner walks
            // around, e.g. Aroma Lady Nikki on the Bond Bridge in the WR video).
            let drop_trainer = n == 0 && matches!(self.segs[i].kind, Kind::Trainer { .. });
            if drop_trainer
                || matches!(self.segs[i].kind, Kind::Parties { .. } | Kind::Group { .. })
            {
                self.segs[i].kind = kind;
                self.segs[i].inputs = 6 * n;
            } else {
                self.segs.insert(
                    i + 1,
                    Seg {
                        name: format!("{seg}+fights"),
                        src: Src::Tiles,
                        inputs: 6 * n,
                        split: false,
                        off_timer: false,
                        starter_check: false,
                        kind,
                    },
                );
            }
        }
        Ok(())
    }

    /// The `n` hardest trainers on the given maps. `n` is a routing assumption (which ones the
    /// route cannot walk around), the parties are the decomp's.
    pub(crate) fn maps_group(&mut self, name: &str, maps: &[&str], n: u32) {
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

    pub(crate) fn split(&mut self) {
        self.segs.last_mut().unwrap().split = true;
    }
}

/// MODEL CHANGE `Params::indoor_speed`: `tiles` of a walk at era `e` of which `inside` lie on maps
/// whose header forbids the bike (and, for `Era::Walk`, running too), priced as the equivalent number of
/// tiles at `e` (`decompiled/src/bike.c:245-259`; `data/maps/<map>/map.json` `allow_cycling`,
/// `allow_running`, `map_type`). Off: the plain tile count.
pub(crate) fn indoor(p: &Params, tiles: u32, inside: u32, inside_era: Era, e: Era) -> u32 {
    let f = inside_era.frames_per_tile() / e.frames_per_tile();
    if !p.indoor_speed || f <= 1.0 {
        return tiles;
    }
    let inside = inside.min(tiles);
    tiles - inside + (inside as f64 * f).round() as u32
}

pub(crate) fn erika_leg(b: &mut B<'_>, p: &Params, e: Era) {
    // Celadon Gym: no bike, no running (`CeladonCity_Gym/map.json`); ~40 of the tiles inside (guess).
    let t = indoor(p, 146, 40, Era::Walk, e);
    b.walk("erika-walk", t, e, None, Src::Tiles);
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

/// Closures `build` shares with the route-order functions (grass-step table, encounter and detour
/// helpers, the rival's trainer-id suffix).
pub struct Helpers<'h> {
    pub g: &'h dyn Fn(u32, u32, u32) -> u32,
    pub enc: &'h dyn Fn(&'static str, u32, u32, u32) -> Option<(&'static str, u32)>,
    pub detour: &'h dyn Fn(&mut B, &str, u32),
    pub tn: &'h dyn Fn(&str) -> String,
}

/// The original order (Brock, Misty, Surge, Rock Tunnel, Celadon, ..., the Hall of Fame, then the
/// dex fill and the post-game).
#[allow(unused_variables)]
pub(crate) fn leg_pewter_cerulean(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
    if p.early_dig && !p.no_dig {
        // The robbed house's Rocket is visible from the first Cerulean arrival
        // (`data/maps/CeruleanCity/map.json:67` hides him behind
        // FLAG_HIDE_CERULEAN_ROCKET, which nothing in the decomp ever sets), so TM28
        // Dig can be fetched before the rival (`CeruleanCity/scripts.inc:164-187`).
        b.trainer("dig-grunt", "TRAINER_TEAM_ROCKET_GRUNT_5", 600);
        b.text("tm28-dig", 900, Src::Guess);
    }
}

#[allow(unused_variables)]
pub(crate) fn leg_nugget_bill(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_bill_return(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_misty(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_ssanne(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 4. Vermilion: S.S. Anne and Surge, no Vermilion heal.
    b.walk(
        "route5-6",
        260,
        b.era,
        enc("MAP_ROUTE5", 6, 0, 40),
        Src::Tiles,
    );
    detour(b, "route5-6-detour", 21);
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
}

#[allow(unused_variables)]
pub(crate) fn leg_surge(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // Vermilion Gym: no bike, no running (`VermilionCity_Gym/map.json`); 60 tiles inside, as played
    // (cans 32, to Surge 8, out 20; calibration 2026-11-22).
    let t = indoor(p, 156, 60, Era::Walk, b.era);
    b.walk("surge-walk", t, b.era, None, Src::Guess);
    if p.audit2 > 0 {
        // Tucker (7,8), FACE_LEFT sight 3, covers (4..6,8), the only way up to the barrier gap
        // (`VermilionCity_Gym/map.json`; `forced.py` with the gap opened: Tucker forced).
        let alts: &[(usize, &[&str])] = if p.audit2 > 1 {
            &[(
                1,
                &[
                    "VermilionCity_Gym_EventScript_Baily",
                    "VermilionCity_Gym_EventScript_Dwayne",
                ],
            )]
        } else {
            &[]
        };
        b.forced(
            "surge-gym-trainers",
            &["VermilionCity_Gym_EventScript_Tucker"],
            alts,
        );
    } else {
        b.group("surge-gym-trainers", 2, 2, 20, 65);
    }
    b.trainer("surge", "TRAINER_LEADER_LT_SURGE", 600);
    b.text("surge-badge", 600, Src::Guess);
    b.split();
}

#[allow(unused_variables)]
pub(crate) fn leg_vermilion_return(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_bike(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 5. Bike, Dig, Rock Tunnel, Lavender.
    if p.bike {
        b.walk("bike-shop", 40, b.era, None, Src::Tiles);
        b.text("bike-redeem", 1200, Src::Guess);
        b.era = Era::Bike;
    }
    if !p.early_dig && !p.no_dig {
        // The Rocket who stole TM28 (`CeruleanCity/scripts.inc:172`).
        b.trainer("dig-grunt", "TRAINER_TEAM_ROCKET_GRUNT_5", 600);
        b.text("tm28-dig", 900, Src::Guess);
    }
}

#[allow(unused_variables)]
pub(crate) fn leg_rock_tunnel(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_celadon(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
    // Rocket Hideout: running, no bike (`RocketHideout_B1F/map.json`); the Game Corner neither
    // (`CeladonCity_GameCorner/map.json`, ~26 tiles, played 17 one way). Guess for the split.
    let t = indoor(p, 526, 500, Era::Run, e);
    let t = indoor(p, t, 26, Era::Walk, e);
    b.walk("hideout-walk", t, e, None, Src::Guess);
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
}

#[allow(unused_variables)]
pub(crate) fn leg_hm02(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    b.walk("hm02-trip", 218, e, None, Src::Tiles);
    b.text("hm02-fly", 800, Src::Guess);
}

/// `Params::fly_badge_gate` in the `wr` order with Surge after the tower: the legs between the Route 16
/// house and Surge cannot use Fly.
pub(crate) fn fly_gated(p: &Params) -> bool {
    p.fly_badge_gate && p.route_style == "wr" && !p.variant("surge_early")
}

#[allow(unused_variables)]
pub(crate) fn leg_tower(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 7. Pokémon Tower and the flute.
    if fly_gated(p) {
        // MODEL CHANGE `fly_badge_gate`: no Thunder Badge yet, so no Fly (`party_menu.c:3926`): Route 16
        // house -> Celadon -> Route 7 -> Saffron -> Route 8 -> Lavender, 321 tiles (`frlg-mapgraph path`).
        b.walk("walk-lavender", 321, e, None, Src::Tiles);
    } else {
        b.text("fly-lavender", 700, Src::Guess);
    }
    // Pokemon Tower: no bike, no running (`PokemonTower_*/map.json`); 160 of the tiles inside (played:
    // 154 tower tiles to the 7F, calibration 2026-11-22).
    let t = indoor(p, 215, 160, Era::Walk, e);
    b.walk("tower-walk", t, e, None, Src::Guess);
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
    if p.marowak_fight {
        // MODEL CHANGE `marowak_fight`: the Silph Scope reveals the ghost and it must be beaten (fleeing
        // replays the scene): `PokemonTower_6F/scripts.inc:9` `setwildbattle SPECIES_MAROWAK, 30`.
        b.push(
            "tower-marowak",
            Src::Guess,
            10,
            Kind::Special {
                special: crate::params::Special {
                    after: String::new(),
                    kind: "fight".into(),
                    species: vec!["SPECIES_MAROWAK".into()],
                    level: 30,
                    frames: 400,
                    ..Default::default()
                },
            },
        );
    }
    b.maps_group("tower-rockets", &["PokemonTower_7F"], 3);
    b.text("fuji-flute", 1500, Src::Guess);
}

#[allow(unused_variables)]
pub(crate) fn leg_cycling(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 8. Cycling Road to Fuchsia, Safari, HM03 and HM04, Koga.
    b.text("fly-celadon", 700, Src::Guess);
    if (p.erika_after_tower && p.route_style != "wr") || p.variant("erika_cycling") {
        // Erika postponed past the tower: the carry is two levels up and usually a
        // Blastoise by now, and the fly lands in Celadon anyway (walk re-used; Guess).
        let e = b.era;
        erika_leg(b, p, e);
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
}

#[allow(unused_variables)]
pub(crate) fn leg_safari(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    b.walk("safari-trip", 198, e, None, Src::Tiles);
    b.text("safari-fee-and-hms", 2400, Src::Guess);
    b.walk("safari-walk", 460, Era::Run, None, Src::Tiles);
}

#[allow(unused_variables)]
pub(crate) fn leg_warden(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    b.walk("warden", 18, e, None, Src::Tiles);
    b.text("warden-strength", 1400, Src::Guess);
}

#[allow(unused_variables)]
pub(crate) fn leg_koga(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // Fuchsia Gym: no bike, no running (`FuchsiaCity_Gym/map.json`); 51 tiles inside, as played.
    let t = indoor(p, 119, 51, Era::Walk, e);
    b.walk("koga-walk", t, e, None, Src::Guess);
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
}

#[allow(unused_variables)]
pub(crate) fn leg_silph(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 9. Silph Co. and Sabrina.
    b.text("fly-celadon-2", 700, Src::Guess);
    leg_silph_walk(b, p, h);
}

/// Silph and Sabrina from Celadon (the fly is the caller's).
#[allow(unused_variables)]
pub(crate) fn leg_silph_walk(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // Silph Co.: running, no bike (`SilphCo_*F/map.json`); ~420 tiles inside (guess for the split).
    let t = indoor(p, 479, 420, Era::Run, e);
    b.walk("silph-walk", t, e, None, Src::Guess);
    b.trainer("rival-silph", &tn("TRAINER_RIVAL_SILPH"), 700);
    if p.audit2 > 0 {
        // 11F: the only way from the warp pad's landing to the Giovanni trigger passes (2,18),
        // inside Grunt2's FACE_UP sight-5 cone; three card-key doors on the path
        // (`silph_audit.py`: 5F D2, 3F D2, 11F D), each a fanfare + message.
        let alts: &[(usize, &[&str])] = if p.audit2 > 1 {
            &[(1, &["SilphCo_5F_EventScript_Grunt1"])]
        } else {
            &[]
        };
        b.text("silph-card-doors", 600, Src::Guess);
        b.forced("silph-grunts", &["SilphCo_11F_EventScript_Grunt2"], alts);
    } else {
        b.group("silph-grunts", 4, 2, 32, 70);
    }
    b.trainer("silph-giovanni", "TRAINER_BOSS_GIOVANNI_2", 700);
    b.text("master-ball", 1800, Src::Guess);
    // Saffron Gym: no bike, no running (`SaffronCity_Gym/map.json`); ~100 tiles inside (guess).
    let t = indoor(p, 123, 100, Era::Walk, e);
    b.walk("sabrina-walk", t, e, None, Src::Guess);
    // The audit reaches Sabrina through the teleport pads without crossing any cone.
    if !p.audited_forced {
        b.group("saffron-gym-trainers", 2, 2, 36, 75);
    }
    b.trainer("sabrina", "TRAINER_LEADER_SABRINA", 600);
    b.text("sabrina-badge", 600, Src::Guess);
    b.split();
}

#[allow(unused_variables)]
pub(crate) fn leg_mansion_blaine(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
    // Cinnabar Gym: no running (`CinnabarIsland_Gym/map.json`); ~30 tiles inside (guess).
    let t = indoor(p, 34, 30, Era::Walk, Era::Run);
    b.walk("blaine-walk", t, Era::Run, None, Src::Tiles);
    if p.audit2 > 0 {
        // Six quiz statues, one door each, chained to Blaine (`CinnabarIsland_Gym/scripts.inc`
        // OpenDoor1-6); every trainer has sight 0, so a correct answer costs text only.
        // Guess: ~450 frames per question.
        b.text("blaine-quiz-text", 2700, Src::Guess);
    } else {
        b.group("blaine-quiz", 2, 2, 38, 75);
    }
    b.trainer("blaine", "TRAINER_LEADER_BLAINE", 600);
}

#[allow(unused_variables)]
pub(crate) fn leg_sevii(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // The forced first Sevii trip. Previously one 5000-frame guess; audited 2026-11-13 against
    // the scripts (journal of that date): sailing home needs VAR_MAP_SCENE_CINNABAR_ISLAND >= 4
    // (data/scripts/seagallop.inc:4), reached only through Bill's sail-home scene, which needs
    // the meteorite delivered (TwoIsland_JoyfulGameCorner/scripts.inc:149-152), which needs
    // Lostelle rescued (ThreeIsland_BerryForest/scripts.inc:24-39, forced wild HYPNO Lv30), which
    // needs the Three Island biker wall cleared: four forced fights (ThreeIsland/scripts.inc:195-260)
    // and two unavoidable Bond Bridge trainers on the way. Tile counts are the audit's BFS (+-10%).
    // Scene-text frames are guesses; walking, fights and grass are modelled pieces.
    b.text("sevii-bill-boat", 2600, Src::Guess); // Bill scene, boat, harbor, scripted walk to the PC
    b.text("celio-tri-pass", 2600, Src::Guess); // meteorite, Tri-Pass, town-map scene
    b.walk("one-island-out", 33, Era::Run, None, Src::Tiles); // PC -> harbor, sailor
    b.text("sail-to-two", 900, Src::Guess);
    b.walk("two-island-in", 40, Era::Run, None, Src::Tiles);
    b.text("corner-lostelle-scene", 1200, Src::Guess); // forced first-visit Game Corner scene
    b.walk("two-island-out", 40, Era::Run, None, Src::Tiles);
    b.text("sail-to-three", 900, Src::Guess);
    b.walk("three-island-town", 56, Era::Run, None, Src::Tiles); // port + town to the biker wall
    b.trainer("three-island-goon-1", "TRAINER_BIKER_GOON", 300);
    b.trainer("three-island-goon-2", "TRAINER_BIKER_GOON_2", 300);
    b.trainer("three-island-goon-3", "TRAINER_BIKER_GOON_3", 300);
    b.trainer("three-island-paxton", "TRAINER_CUE_BALL_PAXTON", 400);
    b.walk(
        "bond-bridge",
        125,
        Era::Run,
        // The old order's site name matches no table (its Bond Bridge walk has no encounters); the
        // WR order uses the real map (`wild_encounters.json`, `MAP_THREE_ISLAND_BOND_BRIDGE`).
        Some((
            if p.route_style == "wr" {
                "MAP_THREE_ISLAND_BOND_BRIDGE"
            } else {
                "MAP_BOND_BRIDGE"
            },
            g(34, 34, 34),
        )),
        Src::Guess,
    );
    b.trainer("bond-bridge-nikki", "TRAINER_AROMA_LADY_NIKKI", 300);
    b.trainer("bond-bridge-violet", "TRAINER_AROMA_LADY_VIOLET", 300);
    b.walk(
        "berry-forest",
        197,
        Era::Run,
        Some(("MAP_THREE_ISLAND_BERRY_FOREST", g(55, 55, 55))),
        Src::Guess,
    );
    // Lostelle's forced wild Hypno Lv30: intro plus a flee. Catching it instead is a plan
    // special (kind "static", SPECIES_HYPNO, after "berry-forest"); that slightly double-counts
    // this flee cost, which is within the guess error.
    b.text("lostelle-hypno", 950, Src::Guess);
    // Rescue warps straight to the Game Corner; delivery hands over the free Moon Stone.
    b.text("deliver-meteorite", 1400, Src::Guess);
    b.walk("two-island-out-2", 40, Era::Run, None, Src::Tiles);
    b.text("sail-to-one", 900, Src::Guess);
    b.walk("one-island-in", 18, Era::Run, None, Src::Tiles);
    b.text("blaine-badge-and-sevii-scene", 2800, Src::Guess); // Bill sail-home + Cinnabar return
    b.split();
}

#[allow(unused_variables)]
pub(crate) fn leg_viridian(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 11. Viridian Gym.
    b.text("fly-viridian", 700, Src::Guess);
    // Viridian Gym: no bike, no running (`ViridianCity_Gym/map.json`); ~45 tiles inside (guess).
    let t = indoor(p, 55, 45, Era::Walk, e);
    b.walk("viridian-gym-walk", t, e, None, Src::Tiles);
    if p.audited_forced {
        let always: &[&str] = if p.viridian_warren {
            &[
                "ViridianCity_Gym_EventScript_Takashi",
                "ViridianCity_Gym_EventScript_Warren",
            ]
        } else {
            &["ViridianCity_Gym_EventScript_Takashi"]
        };
        b.forced("viridian-gym-trainers", always, &[]);
    } else {
        b.group("viridian-gym-trainers", 3, 2, 42, 80);
    }
    b.trainer("giovanni", "TRAINER_LEADER_GIOVANNI", 600);
    b.text("giovanni-badge", 600, Src::Guess);
    b.split();
}

#[allow(unused_variables)]
pub(crate) fn leg_league_approach(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_e4_1(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
}

#[allow(unused_variables)]
pub(crate) fn leg_postgame(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
    // 14. Post-game: National Dex, Celio's chain, Rocket Warehouse.
    // Stones must evolve BEFORE Oak counts the dex — they used to sit after the check,
    // where they could never contribute an entry (fill audit, 2026-11-13).
    if !p.stones.is_empty() {
        b.push("stones", Src::Guess, 0, Kind::Stones);
    }
    b.text("fly-pallet-2", 700, Src::Guess);
    b.walk("pallet-to-oak", 30, e, None, Src::Tiles);
    b.push("oak-dex-check", Src::Guess, 0, Kind::OakCheck);
    b.text("oak-national-dex", 2500, Src::Guess);
    b.split();
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
    } else if let Some(prev) = b.segs.last().map(|s| s.name.clone()) {
        // The Mewtwo leg is absent: anchors on it fall back to the segment before it.
        for n in ["fly-cerulean", "cerulean-cave", "mewtwo"] {
            b.skipped.insert(n.to_string(), prev.clone());
        }
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
}

#[allow(unused_variables)]
pub(crate) fn leg_e4_2(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let e = b.era;
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
        if p.league_transit {
            // Seconds between rooms from the WR's split windows minus its fight durations
            // (Bruno-2 124-92, Agatha-2 113-98, Lance-2 154-110, Champion-2 222-196).
            let t: u32 = match nm {
                "bruno-2" => 1920,
                "agatha-2" => 900,
                "lance-2" => 2640,
                "champion-2" => 1560,
                _ => 0,
            };
            if t > 0 {
                b.text(
                    &format!("e4-transit-{nm}"),
                    t.saturating_sub(p.transit_outro),
                    Src::Guess,
                );
            }
        }
        b.trainer(nm, &id, 900);
    }
    // The WR's End split (Champion 2 -> black frame) is 73 s against the model's 43 s.
    b.text(
        "hall-of-fame-2",
        if p.league_transit { 4200 } else { 2400 },
        Src::Guess,
    );
}

fn old_order(b: &mut B<'_>, p: &Params, h: &Helpers<'_>) {
    let (g, enc, detour, tn) = (h.g, h.enc, h.detour, h.tn);
    let _ = (g, enc, detour, tn);
    leg_pewter_cerulean(b, p, h);
    leg_nugget_bill(b, p, h);
    leg_bill_return(b, p, h);
    leg_misty(b, p, h);
    leg_ssanne(b, p, h);
    leg_surge(b, p, h);
    leg_vermilion_return(b, p, h);
    leg_bike(b, p, h);
    leg_rock_tunnel(b, p, h);
    leg_celadon(b, p, h);
    let e = b.era;
    if !p.erika_after_tower {
        erika_leg(b, p, e);
    }
    leg_hm02(b, p, h);
    leg_tower(b, p, h);
    leg_cycling(b, p, h);
    leg_safari(b, p, h);
    leg_warden(b, p, h);
    leg_koga(b, p, h);
    leg_silph(b, p, h);
    leg_mansion_blaine(b, p, h);
    leg_sevii(b, p, h);
    leg_viridian(b, p, h);
    leg_league_approach(b, p, h);
    leg_e4_1(b, p, h);
    leg_postgame(b, p, h);
    leg_e4_2(b, p, h);
}

pub fn build(dex: &Dex, p: &Params) -> Result<Vec<Seg>, String> {
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
        skipped: Default::default(),
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
    // 6708 in the ledger includes the forced Sammy fight (2706f), which "forest-trainers"
    // charges separately (prebrock audit 2026-11-13): 6708 - 2706.
    b.text("forest", 4002, Src::Ledger);
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
    if p.route_style == "wr" {
        // Camper Liam stands in the gym (`decompiled/data/maps/PewterCity_Gym/scripts.inc:38-41`); the
        // WR run's Squirtle went from Lv8 to Lv11 on him before Brock (video fight analysis).
        b.forced(
            "pewter-gym-trainer",
            &["PewterCity_Gym_EventScript_Liam"],
            &[],
        );
    }
    b.trainer_adv("brock", "TRAINER_LEADER_BROCK", 600, 1.4);
    b.split();
    b.era = Era::Run;

    let helpers = Helpers {
        g: &g,
        enc: &enc,
        detour: &detour,
        tn: &tn,
    };
    if p.route_style == "wr" {
        crate::route_wr::wr_order(&mut b, p, &helpers);
    } else {
        old_order(&mut b, p, &helpers);
    }
    b.apply_fight_lists(&p.fight_lists)?;

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
        let Some(at) = b.find(&h.after) else {
            if p.skip_powerplant && h.after == "powerplant-out" {
                continue;
            }
            return Err(format!("hunt anchor {} matches no segment", h.after));
        };
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
            .find(&g.after)
            .ok_or_else(|| format!("grind anchor {} matches no segment", g.after))?;
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
            .find(&m.after)
            .ok_or_else(|| format!("mart anchor {} matches no segment", m.after))?;
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
        let Some(at) = b.find(&sp.after) else {
            if p.skip_powerplant && sp.after == "powerplant-out" {
                continue;
            }
            return Err(format!("special anchor {} matches no segment", sp.after));
        };
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
    for x in p.extra_fights.iter().rev() {
        let at = b
            .find(&x.after)
            .ok_or_else(|| format!("extra fight anchor {} matches no segment", x.after))?;
        let scripts: Vec<&str> = x.scripts.iter().map(|s| s.as_str()).collect();
        b.forced(&x.name, &scripts, &[]);
        let seg = b.segs.pop().expect("forced pushes a segment");
        b.segs.insert(at + 1, seg);
    }
    for ev in p.evolutions.iter().rev() {
        dex.id(&ev.species);
        let Some(at) = b.find(&ev.after) else {
            if p.skip_powerplant && ev.after == "powerplant-out" {
                continue;
            }
            return Err(format!("evolution anchor {} matches no segment", ev.after));
        };
        b.segs.insert(
            at + 1,
            Seg {
                name: format!(
                    "evo-{}@{}",
                    ev.species.trim_start_matches("SPECIES_").to_lowercase(),
                    ev.after
                ),
                src: Src::Guess,
                inputs: 6,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Evolve { evo: ev.clone() },
            },
        );
    }
    for t in p.tms.iter().rev() {
        // A teach aimed at Mewtwo means nothing when the plan does not catch it.
        if t.mewtwo && !p.mewtwo {
            continue;
        }
        dex.move_id(&t.mv);
        let at = b.find(&t.after).ok_or_else(|| {
            let why = if t.after == "tm28-dig" && p.no_dig {
                " (the plan sets no_dig, so TM28 is never fetched)"
            } else {
                ""
            };
            format!("tm anchor {} matches no segment{why}", t.after)
        })?;
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
    // PLAN `mt_moon_heal` (review item 18): the Route 4 Pokémon Center at Mt. Moon's entrance. Not a
    // `center-heal@` anchor, so the Torrent phase does not walk past it.
    if p.mt_moon_heal {
        let at = b
            .find("route3-trainers")
            .ok_or("mt_moon_heal: no route3-trainers segment")?;
        b.segs.insert(
            at + 1,
            Seg {
                name: "mtmoon-heal@route3-trainers".into(),
                src: Src::Guess,
                inputs: 8,
                split: false,
                off_timer: false,
                starter_check: false,
                kind: Kind::Text {
                    frames: p.mt_moon_heal_frames,
                },
            },
        );
    }
    for h in p.extra_heals.iter().rev() {
        let at = b
            .find(h)
            .ok_or_else(|| format!("heal anchor {h} matches no segment"))?;
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
        let Some(at) = b.find(&e.after) else {
            if p.skip_powerplant && e.after == "powerplant-out" {
                continue;
            }
            return Err(format!("elixir anchor {} matches no segment", e.after));
        };
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
            .find(&x.after)
            .ok_or_else(|| format!("xbuy anchor {} matches no segment", x.after))?;
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
    // FIT `Params::guess_text_scale`: the guessed text lumps before the League (up to `indigo-heal`, the
    // Start E4R1 split) scaled; the League and post-game lumps keep their own WR-window fits
    // (`league_transit`, `postgame_calib`).
    if p.guess_text_scale > 0.0 {
        let end = b
            .segs
            .iter()
            .position(|s| s.name == "indigo-heal")
            .unwrap_or(b.segs.len());
        for s in b.segs[..end].iter_mut() {
            if s.src == Src::Guess {
                if let Kind::Text { frames } = &mut s.kind {
                    *frames = (*frames as f64 * p.guess_text_scale).round() as u32;
                }
            }
        }
    }
    Ok(b.segs)
}
