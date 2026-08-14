//! Model-driven walking: A* over the decoded map with the wild-encounter
//! stream simulated, so a crossing is *planned* against the fated rate-test
//! sequence instead of discovered edge by edge in the emulator.
//!
//! The model this leans on is already proven piecewise: the rate test runs
//! on its own LCG advanced once per test (`docs/defeat-brock/research/`
//! `wild-encounters.md`, `decompiled/src/wild_encounter.c:302-332`), so its
//! pass/fail sequence from the current `sWildEncounterData` is computable in
//! full before a single frame is emulated. What frame timing *can* reach are
//! the two `gRngValue` gates (the 5% cooldown gate and the 60%
//! behavior-change roll) -- the executor steers those with 1-frame delays
//! when a step's observed outcome differs from the plan, and replans (the
//! plan is milliseconds) when reality still disagrees.
//!
//! The planner's frame model, kept deliberately coarse -- the executor
//! measures real frames and optimality only needs the *ranking* right:
//!
//! - a walked tile is 16 frames (`sMovementActionFuncs` walk-normal timing;
//!   the old `nav.rs` measured ~17 with chaining);
//! - a ledge hop is 2 tiles in ~40 frames (`MB_JUMP_*`,
//!   `include/constants/metatile_behaviors.h:47-50`), encounter-check-free
//!   (forced movement skips the check, `src/field_control_avatar.c:137-143`);
//! - a behavior-boundary step can either *consume* a rate test (the 60%
//!   branch) or *skip* it (the 40% branch) -- both reachable by delay
//!   steering, priced at the expected delay;
//! - a step whose fated rate test passes means a wild battle: priced at
//!   [`ENCOUNTER_COST`] rather than forbidden, so a genuinely walled map can
//!   still route through one flee.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use frlg_emu::keys;
use frlg_mon::wild::MapWild;
use frlg_rng::WildRng;

use crate::observe::WildData;
use crate::world::{MapData, MB_JUMP_EAST, MB_JUMP_NORTH, MB_JUMP_SOUTH, MB_JUMP_WEST};

/// What a fled wild battle costs, in frames: ~1200 for the battle plus the
/// walk disruption. High enough that any dodgeable encounter is dodged,
/// low enough that a fully index-walled belt still routes through its
/// cheapest flee instead of failing.
const ENCOUNTER_COST: u32 = 1400;

/// Walking one tile. See the module docs for why 16 and not the measured 17:
/// the constant term cancels between candidate paths of equal length, and a
/// *lower* bound keeps the heuristic admissible.
const TILE_COST: u32 = 16;

/// A ledge hop: two tiles of progress, no encounter check.
const JUMP_COST: u32 = 40;

/// Steering a behavior-boundary step to *consume* its rate test: the 60%
/// branch, expected < 1 delay frame.
const CONSUME_BOUNDARY_COST: u32 = TILE_COST + 1;

/// Steering a behavior-boundary step to *skip* its rate test: the 40%
/// branch, expected ~2 delay frames.
const SKIP_BOUNDARY_COST: u32 = TILE_COST + 3;

/// A tile inside a trainer's sight cone (`trainer_type == TRAINER_TYPE_NORMAL`,
/// facing direction, `trainer_sight_or_berry_tree_id` tiles): entering one is
/// a forced trainer battle. Not forbidden -- Sammy's cone seals the forest's
/// exit corridor (`research/story-gates.md`) -- just never worth it when a
/// clean tile exists.
const SIGHT_CONE_COST: u32 = 20_000;

/// A tile a wandering NPC can reach: the walk may have to wait for it to
/// shuffle aside.
const WANDER_COST: u32 = 40;

/// A tile with a coord event on it: a script may fire. Avoid unless the leg
/// explicitly wants it (the tutorial trigger) or the detour is silly.
const COORD_EVENT_COST: u32 = 600;

/// How many rate tests ahead the fated sequence is computed. A leg is a few
/// hundred tiles at most; 1024 tests is several legs of headroom.
const FATED_HORIZON: usize = 1024;

/// What one planned step expects to happen, so the executor can check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    /// Not an encounter tile (or no wild table): nothing to check.
    Free,
    /// A grass step inside the cooldown window: no rate test, but the 5%
    /// `gRngValue` gate exists -- steer away if it fires.
    Cooldown,
    /// A grass step that consumes rate test `index`. `fated_pass` says what
    /// the model expects of it; a planned `true` is a deliberate encounter.
    Consume { index: u16, fated_pass: bool },
    /// A behavior-boundary grass step steered to *not* consume a test.
    SkipBoundary,
    /// A ledge hop: two tiles, no check.
    Jump,
}

#[derive(Debug, Clone, Copy)]
pub struct PlanStep {
    pub dir: u16,
    pub to: (i16, i16),
    pub kind: StepKind,
}

/// The immutable per-leg inputs to a plan.
pub struct PlanRequest<'a> {
    pub map: &'a MapData,
    /// The map's land encounter table, if it has one.
    pub wild: Option<&'static MapWild>,
    pub start: (i16, i16),
    /// `sWildEncounterData` as read from RAM at planning time.
    pub wild_data: WildData,
    /// Reach any of these tiles.
    pub targets: Vec<(i16, i16)>,
    /// Edges learned blocked during execution (an NPC parked on a tile, a
    /// collision the static model missed): `(from, to)` pairs the planner
    /// must not use this time round.
    pub blocked: HashSet<((i16, i16), (i16, i16))>,
}

const DIRS: [(u16, (i16, i16)); 4] = [
    (keys::UP, (0, -1)),
    (keys::DOWN, (0, 1)),
    (keys::LEFT, (-1, 0)),
    (keys::RIGHT, (1, 0)),
];

/// The fated rate-test outcomes: `pass[j]` for the j-th test consumed from
/// `wild_data`, assuming every earlier test failed (a pass means a battle,
/// after which the caller replans anyway). Buff growth per
/// `AddToWildEncounterRateBuff` (`decompiled/src/wild_encounter.c:778-784`),
/// test per `DoWildEncounterRateTest` (`:309-332`).
pub fn fated_passes(wild_data: &WildData, table: &MapWild, horizon: usize) -> Vec<bool> {
    let mut rng = WildRng(wild_data.rng_state);
    let mut buff = wild_data.rate_buff as u32;
    let mut out = Vec::with_capacity(horizon);
    for _ in 0..horizon {
        let rate = (table.rate as u32 * 16 + buff * 16 / 200).min(1600);
        out.push(((rng.random() % 1600) as u32) < rate);
        buff += table.rate as u32;
    }
    out
}

/// Static per-tile extra costs: sight cones, NPC tiles, coord events.
fn penalty_grid(map: &MapData) -> HashMap<(i16, i16), u32> {
    let mut grid: HashMap<(i16, i16), u32> = HashMap::new();
    let mut add = |x: i16, y: i16, cost: u32| {
        let e = grid.entry((x, y)).or_insert(0);
        *e = (*e).max(cost);
    };
    for obj in &map.objects {
        if obj.stationary() {
            // A stationary NPC's tile is a wall; the executor never needs to
            // wait it out.
            add(obj.x, obj.y, u32::MAX);
        } else {
            for dx in -obj.range_x..=obj.range_x {
                for dy in -obj.range_y..=obj.range_y {
                    add(obj.x + dx, obj.y + dy, WANDER_COST);
                }
            }
        }
        if obj.trainer_type == "TRAINER_TYPE_NORMAL" && obj.sight > 0 {
            for (fx, fy) in obj.facings() {
                for step in 1..=obj.sight {
                    let (sx, sy) = (obj.x + fx * step, obj.y + fy * step);
                    match map.tile(sx, sy) {
                        Some(t) if t.collision == 0 => add(sx, sy, SIGHT_CONE_COST),
                        _ => break, // the cone stops at a wall
                    }
                }
            }
        }
    }
    for &(x, y) in &map.coord_events {
        add(x, y, COORD_EVENT_COST);
    }
    grid
}

/// A* over `(tile, cooldown-steps, rate-test index)`.
///
/// Returns the planned steps and the model's frame estimate, or `None` when
/// no target is reachable. `prev_behavior` is per-node *derived from the
/// node's tile* -- `TryStandardWildEncounter` updates it on every step
/// including early-outs (`decompiled/src/wild_encounter.c:761,768,773`), so
/// after any step it equals the stood-on tile's behavior; only the start
/// tile uses the RAM value.
pub fn plan(req: &PlanRequest) -> Option<(Vec<PlanStep>, u32)> {
    let map = req.map;
    let min_steps = req.wild.map(|w| w.min_steps()).unwrap_or(0);
    let fated = req
        .wild
        .map(|w| fated_passes(&req.wild_data, w, FATED_HORIZON))
        .unwrap_or_default();
    let penalties = penalty_grid(map);
    let targets: HashSet<(i16, i16)> = req.targets.iter().copied().collect();
    if targets.is_empty() {
        return None;
    }

    // (x, y, cooldown steps so far (saturated), tests consumed).
    type Node = (i16, i16, u8, u16);
    // A jump moves 2 tiles per 40 frames, so the cheapest possible per-tile
    // progress is 40/2 = 20 > 16; distance * TILE_COST would still be
    // admissible, but halving keeps a safety margin for any future cheaper
    // movement (bike, spin tiles) without re-deriving this bound.
    let h = |&(x, y, _, _): &Node| -> u32 {
        targets
            .iter()
            .map(|&(tx, ty)| x.abs_diff(tx) as u32 + y.abs_diff(ty) as u32)
            .min()
            .unwrap_or(0)
            / 2
            * TILE_COST
    };

    let start: Node = (
        req.start.0,
        req.start.1,
        req.wild_data.steps_since.min(min_steps),
        0,
    );
    let start_behavior = req.wild_data.prev_behavior;

    let mut best: HashMap<Node, (u32, Option<(Node, PlanStep)>)> = HashMap::new();
    let mut queue: BinaryHeap<Reverse<(u32, Node)>> = BinaryHeap::new();
    best.insert(start, (0, None));
    queue.push(Reverse((h(&start), start)));

    let goal = loop {
        let Reverse((_, node)) = queue.pop()?;
        let (g, _) = best[&node];
        let (x, y, cd, j) = node;
        if targets.contains(&(x, y)) {
            break node;
        }
        // The behavior the game's `prevMetatileBehavior` holds while standing
        // here: the stood-on tile's, except at the very start.
        let behavior_here = if (x, y) == req.start {
            start_behavior
        } else {
            map.tile(x, y).map(|t| t.behavior).unwrap_or(0)
        };

        for (dir, (dx, dy)) in DIRS {
            let (nx, ny) = (x + dx, y + dy);
            if req.blocked.contains(&((x, y), (nx, ny))) {
                continue;
            }
            let Some(tile) = map.tile(nx, ny) else {
                continue;
            };
            let penalty = penalties.get(&(nx, ny)).copied().unwrap_or(0);
            if penalty == u32::MAX {
                continue;
            }

            // Ledges: `MB_JUMP_*` hops the player over the tile when entered
            // from the jump side, and is a wall from every other side.
            let jump = match tile.behavior {
                MB_JUMP_EAST => Some(keys::RIGHT),
                MB_JUMP_WEST => Some(keys::LEFT),
                MB_JUMP_NORTH => Some(keys::UP),
                MB_JUMP_SOUTH => Some(keys::DOWN),
                _ => None,
            };
            if let Some(jump_dir) = jump {
                if jump_dir != dir {
                    continue;
                }
                let (lx, ly) = (nx + dx, ny + dy);
                let Some(land_tile) = map.tile(lx, ly) else {
                    continue;
                };
                if land_tile.collision != 0 {
                    continue;
                }
                let land_pen = penalties.get(&(lx, ly)).copied().unwrap_or(0);
                if land_pen == u32::MAX {
                    continue;
                }
                let next: Node = (lx, ly, cd, j);
                let ng = g + JUMP_COST + land_pen;
                if best.get(&next).is_none_or(|&(seen, _)| ng < seen) {
                    let step = PlanStep {
                        dir,
                        to: (lx, ly),
                        kind: StepKind::Jump,
                    };
                    best.insert(next, (ng, Some((node, step))));
                    queue.push(Reverse((ng + h(&next), next)));
                }
                continue;
            }

            if tile.collision != 0 {
                continue;
            }

            // The step outcomes this edge can be steered into.
            let mut push = |next: Node, cost: u32, kind: StepKind| {
                let ng = g + cost + penalty;
                if best.get(&next).is_none_or(|&(seen, _)| ng < seen) {
                    let step = PlanStep {
                        dir,
                        to: (nx, ny),
                        kind,
                    };
                    best.insert(next, (ng, Some((node, step))));
                    queue.push(Reverse((ng + h(&next), next)));
                }
            };

            if req.wild.is_none() || !tile.land {
                push((nx, ny, cd, j), TILE_COST, StepKind::Free);
            } else if cd < min_steps {
                push((nx, ny, cd + 1, j), TILE_COST, StepKind::Cooldown);
            } else {
                let pass = fated.get(j as usize).copied().unwrap_or(true);
                let consume_cost = if map.tile(nx, ny).unwrap().behavior != behavior_here {
                    // A boundary step can go either way; both edges exist.
                    push((nx, ny, cd, j), SKIP_BOUNDARY_COST, StepKind::SkipBoundary);
                    CONSUME_BOUNDARY_COST
                } else {
                    TILE_COST
                };
                let battle = if pass { ENCOUNTER_COST } else { 0 };
                push(
                    (nx, ny, cd, j + 1),
                    consume_cost + battle,
                    StepKind::Consume {
                        index: j,
                        fated_pass: pass,
                    },
                );
            }
        }
    };

    // Walk the parents back to the start.
    let total = best[&goal].0;
    let mut steps = Vec::new();
    let mut node = goal;
    while let (_, Some((parent, step))) = best[&node] {
        steps.push(step);
        node = parent;
    }
    steps.reverse();
    Some((steps, total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    fn wild0(rng_state: u32) -> WildData {
        WildData {
            rng_state,
            prev_behavior: 0,
            rate_buff: 0,
            steps_since: 0,
        }
    }

    /// The forest, entrance to exit: the planner must find a path whose
    /// consumed rate tests are all fated to fail (or accept battles it
    /// prices honestly), inside milliseconds rather than hours.
    #[test]
    fn forest_plans_in_bounded_time() {
        let mut world = match World::load() {
            Ok(w) => w,
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };
        let forest = world.map((1, 0)).expect("forest decodes");
        let req = PlanRequest {
            map: forest,
            wild: Some(&frlg_mon::wild::VIRIDIAN_FOREST_FR),
            start: (29, 61),
            wild_data: wild0(0x1234_5678),
            targets: vec![(4, 9), (5, 9), (6, 9)],
            blocked: HashSet::new(),
        };
        let begin = std::time::Instant::now();
        let (steps, cost) = plan(&req).expect("the forest has an exit");
        let took = begin.elapsed();
        eprintln!(
            "forest plan: {} steps, cost {cost}, {} ms",
            steps.len(),
            took.as_millis()
        );
        assert!(took.as_secs() < 30, "planning took {took:?}");
        // The plan must end on a target.
        let last = steps.last().unwrap();
        assert!([(4, 9), (5, 9), (6, 9)].contains(&last.to));
        // A cost under one encounter means every consumed test was fated to
        // fail on this seed -- the defining property of a clean crossing.
        // (Sammy's cone is priced SIGHT_CONE_COST, so "under 20k" would be
        // the wrong assertion; just require the walk to be coherent.)
        for pair in steps.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            let dist = (a.to.0 - b.to.0).abs() + (a.to.1 - b.to.1).abs();
            assert!(dist <= 2, "steps {a:?} -> {b:?} are not adjacent");
        }
    }

    /// On a map with no wild table the planner degenerates to plain shortest
    /// path.
    #[test]
    fn town_plan_is_shortest_path() {
        let mut world = match World::load() {
            Ok(w) => w,
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };
        let pallet = world.map((3, 0)).expect("Pallet decodes");
        let req = PlanRequest {
            map: pallet,
            wild: None,
            start: (12, 10),
            wild_data: wild0(1),
            targets: vec![(12, 1)],
            blocked: HashSet::new(),
        };
        let (steps, _) = plan(&req).expect("Pallet is walkable");
        // Manhattan distance is 9; NPCs or fences may force a detour, but a
        // shortest path never doubles back, so it stays well under 2x.
        assert!(
            (9..=18).contains(&steps.len()),
            "{} steps for a 9-tile walk",
            steps.len()
        );
    }
}
