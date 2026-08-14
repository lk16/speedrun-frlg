//! Seed screening: rank `seed_delay` candidates by *modeled* walk cost
//! without building them.
//!
//! A full build spends most of its wall clock on battle delay searches, but
//! what distinguishes seeds most is the wild stream -- and the wild stream's
//! effect on the route is computable: boot ~650 frames to the point where
//! `SeedWildEncounterRng` has run twice (`decompiled/src/intro.c:1004`,
//! `src/title_screen.c:737`; both inside the `01-boot` segment), read
//! `sWildEncounterData.rngState`, and run the planner over the route's grass
//! crossings in sequence, threading the consumed-test count from one
//! crossing into the next. The sum of plan costs ranks the seeds; battle
//! stream luck (measured ±600 frames across seeds) is invisible here, which
//! is why the scan picks a short list to *build*, not a winner.
//!
//! The crossing list mirrors the defeat-brock walking legs on their
//! encounter maps. Entry tiles are the route's habitual entry points --
//! approximate is fine: every seed is scored on the same shapes, so the
//! ranking is fair even where a tile is a step off the build's true entry.

use std::path::Path;

use frlg_emu::SymbolTable;
use frlg_mon::wild::MapWild;

use crate::observe::{Observer, WildData};
use crate::plan::{self, PlanRequest, StepKind};
use crate::record::{Recorder, RouteError};
use crate::segments::{self, Starter, Tuning, Version};
use crate::world::World;

/// One grass crossing of the route: which map, where the walk enters, and
/// which tiles end it.
struct Crossing {
    name: &'static str,
    map: (u8, u8),
    start: (i16, i16),
    targets: Vec<(i16, i16)>,
}

/// The defeat-brock crossings in route order (maps and tiles as in
/// `brock.rs`; Route 1 is crossed three times).
fn crossings() -> Vec<Crossing> {
    use crate::brock::{ROUTE1, ROUTE2, VIRIDIAN_FOREST};
    vec![
        Crossing {
            name: "r1-north",
            map: ROUTE1,
            start: (12, 39),
            targets: vec![(10, 0), (11, 0), (12, 0), (13, 0)],
        },
        Crossing {
            name: "r1-south",
            map: ROUTE1,
            start: (12, 0),
            targets: vec![(12, 39), (13, 39)],
        },
        Crossing {
            name: "r1-north2",
            map: ROUTE1,
            start: (12, 39),
            targets: vec![(10, 0), (11, 0), (12, 0), (13, 0)],
        },
        Crossing {
            name: "route2-south",
            map: ROUTE2,
            // Viridian's north exit (x 19..23) lands at Route 2 x 7..11:
            // the connection carries offset -12 (`data/maps/Route2/
            // map.json`, connections). The old (16,54) start sat in the
            // east corridor, which a cut tree at (16,62) walls off from
            // the forest side -- the planner rightly said unreachable,
            // and the scan silently scored this crossing as free.
            start: (9, 79),
            targets: vec![(5, 51), (6, 51)],
        },
        Crossing {
            name: "forest",
            map: VIRIDIAN_FOREST,
            start: (29, 61),
            targets: vec![(4, 9), (5, 9), (6, 9)],
        },
        Crossing {
            name: "route2-north",
            map: ROUTE2,
            start: (7, 2),
            targets: vec![(8, 0), (9, 0), (10, 0), (11, 0)],
        },
    ]
}

/// What one seed scored.
pub struct SeedScore {
    pub seed_delay: usize,
    /// The wild LCG state captured at the end of `01-boot`.
    pub wild_state: u32,
    /// Summed plan cost over the crossings (frames, model units).
    pub walk_cost: u32,
    /// Fated encounters the plans accepted (flees the build would fight).
    pub encounters: u32,
    /// Per-crossing `(name, cost, encounters)`.
    pub detail: Vec<(&'static str, u32, u32)>,
}

/// Boot one seed to the end of `01-boot` and return the captured wild state.
fn wild_state_for_seed(
    rom: &Path,
    obs: &Observer,
    version: Version,
    seed_delay: usize,
) -> Result<u32, RouteError> {
    let tuning = Tuning {
        seed_delay,
        ..Tuning::default()
    };
    let mut rec = Recorder::from_reset(rom)?;
    // Starter is irrelevant before the lab; segment 0 is the boot.
    let segs = segments::all(version, Starter::Squirtle, tuning);
    (segs[0].run)(&mut rec, obs)?;
    Ok(obs.wild_data(rec.emu()).rng_state)
}

/// Score one seed: plan every crossing in order, threading the stream.
pub fn score_seed(
    world: &mut World,
    version: Version,
    seed_delay: usize,
    wild_state: u32,
) -> SeedScore {
    let mut rng_state = wild_state;
    let mut walk_cost = 0u32;
    let mut encounters = 0u32;
    let mut detail = Vec::new();

    for crossing in crossings() {
        let table: Option<&'static MapWild> = crate::brock::wild_table(crossing.map, version);
        let Ok(map) = world.map(crossing.map) else {
            continue;
        };
        let req = PlanRequest {
            map,
            wild: table,
            start: crossing.start,
            wild_data: WildData {
                rng_state,
                // Map entry resets the modifiers
                // (`decompiled/src/overworld.c:764,799`).
                prev_behavior: 0,
                rate_buff: 0,
                steps_since: 0,
            },
            targets: crossing.targets,
            blocked: Default::default(),
        };
        let Some((steps, cost)) = plan::plan(&req) else {
            detail.push((crossing.name, u32::MAX, 0));
            continue;
        };
        let consumed = steps
            .iter()
            .filter(|s| matches!(s.kind, StepKind::Consume { .. }))
            .count() as u32;
        let fated = steps
            .iter()
            .filter(|s| {
                matches!(
                    s.kind,
                    StepKind::Consume {
                        fated_pass: true,
                        ..
                    }
                )
            })
            .count() as u32;
        // Advance the stream by the tests this crossing consumes; the next
        // crossing starts on the far side of them.
        let mut rng = frlg_rng::WildRng(rng_state);
        for _ in 0..consumed {
            rng.random();
        }
        rng_state = rng.0;
        walk_cost += cost;
        encounters += fated;
        detail.push((crossing.name, cost, fated));
    }

    SeedScore {
        seed_delay,
        wild_state,
        walk_cost,
        encounters,
        detail,
    }
}

/// Scan a range of seed delays: boot each, score each, return sorted
/// cheapest-first.
pub fn scan(
    rom: &Path,
    sym: &Path,
    seeds: impl IntoIterator<Item = usize>,
    mut progress: impl FnMut(&SeedScore),
) -> Result<Vec<SeedScore>, RouteError> {
    let text_err = |what: String| RouteError::Timeout {
        what,
        budget: 0,
        frames: 0,
    };
    let syms = SymbolTable::load(sym).map_err(|e| text_err(format!("loading {sym:?}: {e}")))?;
    let obs = Observer::new(syms).map_err(text_err)?;
    let version = Version::of_rom(rom)
        .ok()
        .flatten()
        .unwrap_or(Version::FireRed);
    let mut world = World::load().map_err(text_err)?;

    let mut scores = Vec::new();
    for seed in seeds {
        let wild_state = wild_state_for_seed(rom, &obs, version, seed)?;
        let score = score_seed(&mut world, version, seed, wild_state);
        progress(&score);
        scores.push(score);
    }
    scores.sort_by_key(|s| s.walk_cost);
    Ok(scores)
}
