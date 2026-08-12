//! Overworld movement, searched in the emulator rather than reasoned about.
//!
//! Walking FireRed by hand means knowing the collision map, the warp table, the
//! turn-in-place rule and how many frames a tile costs. All four are already
//! encoded in the ROM, so this asks the ROM: from the current savestate, hold a
//! direction, see where the player ends up, and search over the results.
//!
//! The search is Dijkstra on frame cost -- with an optional Manhattan
//! admissible heuristic when the goal is a known tile -- keyed on
//! `(map, x, y)`. Facing is deliberately *not* in the key: an edge holds its
//! direction until the player actually moves, so the turn-in-place frames are
//! already inside the edge cost, and including facing would quadruple the
//! search for a distinction the cost model already makes.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use frlg_emu::{keys, Emu, SaveState};

use crate::observe::Observer;
use crate::record::{Feed, Recorder, RouteError};

/// The four walking directions, in the decomp's key bits.
pub const DIRECTIONS: [u16; 4] = [keys::UP, keys::DOWN, keys::LEFT, keys::RIGHT];

/// The direction that undoes `dir`, 0 for anything that is not a plain
/// d-pad hold.
fn opposite(dir: u16) -> u16 {
    match dir {
        keys::UP => keys::DOWN,
        keys::DOWN => keys::UP,
        keys::LEFT => keys::RIGHT,
        keys::RIGHT => keys::LEFT,
        _ => 0,
    }
}

/// How many frames a single edge may take before it counts as blocked. A tile
/// step is ~16 frames and a warp's fade is well under this.
const EDGE_BUDGET: usize = 240;

/// Cheapest observed cost of one tile, used only as an A* heuristic scale. It
/// must stay a *lower* bound or the search stops being optimal; 8 is half a
/// walking step, which no movement in this game beats.
const MIN_FRAMES_PER_TILE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Place {
    pub map: (u8, u8),
    pub pos: (i16, i16),
    /// The wild-encounter decision state ([`Observer::wild_key`]). The same
    /// tile under a different encounter-rate-test index is a different node:
    /// the rate test's pass/fail sequence is indexed by how many tests have
    /// run (`docs/defeat-brock/research/wild-encounters.md`), so per-tile
    /// caching without this collapses every minimal path onto one shared
    /// outcome -- and one fated pass then walls off a whole grass belt that
    /// a one-step-longer path walks straight through. Constant on maps with
    /// no encounters, where nodes degenerate to plain tiles.
    pub wild: u64,
}

/// The `AnyOnVia` heuristic's frame cost per tile of distance to the exit
/// hint. Deliberately far *above* the true per-tile cost (~17 frames): with
/// the wild-encounter state multiplying nodes (see [`Place::wild`]), an
/// honest A* drowns in same-row alternates, so `AnyOnVia` runs essentially
/// greedy -- dive at the exit, back off only when blocked. The path is not
/// optimal and is not trying to be; optimality on grass is the later
/// model-driven search's job.
const VIA_SCALE: usize = 64;

/// What `AnyOnVia` charges a node that is on neither the goal map nor the
/// via map: worse than anywhere on the via map, so doors and backtracking
/// are the search's last resort rather than its first.
const OFF_ROUTE_PENALTY: usize = 100_000;

/// Where a walk is trying to get to.
pub enum Goal {
    /// A tile on a map. Enables the admissible A* heuristic.
    Tile { map: (u8, u8), x: i16, y: i16 },
    /// Any tile on a map -- "get through that door", where which side of the
    /// doormat you land on is the game's business. Plain Dijkstra.
    AnyOn { map: (u8, u8) },
    /// Any tile on `map`, guided by a hint: the walk currently stands on
    /// `via_map` and should leave it near `via` (a connection edge or warp,
    /// cited by the caller). **Not admissible, not optimal** -- [`VIA_SCALE`]
    /// overweights distance and off-route maps are penalized flat. This is
    /// the goal for "cross this grass map": directed enough that the
    /// wild-state node explosion (see [`Place::wild`]) stays a corridor
    /// rather than a flood.
    AnyOnVia {
        map: (u8, u8),
        via_map: (u8, u8),
        via: (i16, i16),
    },
    /// Anything else, checked against the emulator. Plain Dijkstra.
    Pred(Box<dyn FnMut(&mut Emu) -> bool>),
}

impl Goal {
    pub fn tile(map: (u8, u8), x: i16, y: i16) -> Self {
        Goal::Tile { map, x, y }
    }

    pub fn on_map(map: (u8, u8)) -> Self {
        Goal::AnyOn { map }
    }

    pub fn on_map_via(map: (u8, u8), via_map: (u8, u8), via: (i16, i16)) -> Self {
        Goal::AnyOnVia { map, via_map, via }
    }

    pub fn when(pred: impl FnMut(&mut Emu) -> bool + 'static) -> Self {
        Goal::Pred(Box::new(pred))
    }
}

fn place(obs: &Observer, emu: &mut Emu) -> Option<Place> {
    Some(Place {
        map: obs.map(emu)?,
        pos: obs.pos(emu)?,
        wild: obs.wild_key(emu),
    })
}

/// Hold `dir` until the player either arrives somewhere new, changes map, or a
/// script takes over. Returns the frames it fed, or `None` if the direction is
/// blocked (the emulator is then left mid-attempt and the caller must restore).
///
/// The edge ends the frame `gSaveBlock1Ptr->pos` changes, which is part way
/// through the step animation and *not* back at rest. That is deliberate: the
/// game keeps walking while a direction stays held, so ending at rest would
/// price every tile as if the player stopped and started again. Chaining edges
/// from mid-animation is exactly what holding the button does.
/// What holding a direction from a node did.
enum Edge {
    /// Arrived somewhere: the masks fed.
    Took(Vec<u16>),
    /// A wall (or nothing happened within the budget).
    Blocked,
    /// A battle started. The node this edge would create is a battle, not a
    /// place, and admitting it would let a cheap battle-bound path claim a
    /// tile's best-cost slot and wall it off from clean paths -- so the
    /// search treats it as blocked, and whoever *wants* the forced battle
    /// takes it outside the search (`brock::walk_fleeing`).
    Battle,
}

/// A wall shows itself quickly: turn-in-place plus a bump cycle is well
/// under this many frames, and the player reads as free-standing between
/// bumps. Warps and scripts keep the player busy (no `player_can_step`
/// window), so they get the full budget.
const WALL_BAIL: usize = 64;

fn take_edge(emu: &mut Emu, obs: &Observer, dir: u16, budget: usize) -> Edge {
    let Some(start) = place(obs, emu) else {
        return Edge::Blocked;
    };
    let mut fed = Vec::new();
    let mut saw_free = false;
    for frame in 0..budget {
        emu.step(dir);
        fed.push(dir);

        if obs.in_battle(emu) {
            return Edge::Battle;
        }

        // Bumping a wall: the position never changes but the player keeps
        // returning to a controllable stand between bump cycles. Bail early
        // rather than spending the whole budget -- wall probes are most of
        // what a search does.
        if frame >= 24 {
            saw_free = saw_free || obs.player_can_step(emu);
            if frame >= WALL_BAIL && saw_free {
                return Edge::Blocked;
            }
        }

        // A script took the player (a trigger fired). Let it run, holding
        // nothing, so the direction does not leak into whatever it opens.
        if obs.prevent_step(emu) {
            settle(emu, obs, &mut fed, budget);
            return if obs.in_battle(emu) {
                Edge::Battle
            } else {
                Edge::Took(fed)
            };
        }

        match place(obs, emu) {
            Some(now) if now != start => {
                // A map change means a warp and a fade; the next edge would
                // otherwise hold a direction into a black screen.
                if now.map != start.map {
                    settle(emu, obs, &mut fed, budget);
                    if obs.in_battle(emu) {
                        return Edge::Battle;
                    }
                }
                return Edge::Took(fed);
            }
            _ => {}
        }
    }
    Edge::Blocked
}

/// Hold nothing until the player is standing free again (or a battle owns
/// the screen, which the caller checks for).
fn settle(emu: &mut Emu, obs: &Observer, fed: &mut Vec<u16>, budget: usize) {
    for _ in 0..budget {
        if obs.player_can_step(emu) || obs.in_battle(emu) {
            return;
        }
        emu.step(0);
        fed.push(0);
    }
}

struct Node {
    cost: usize,
    key: Place,
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}
impl Eq for Node {}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cost.cmp(&other.cost)
    }
}

/// The result of a search: the inputs that get there, and what they cost.
pub struct Path {
    pub inputs: Vec<u16>,
    pub frames: usize,
}

/// Search for a path from the recorder's current position, then walk it.
///
/// The recorder is left exactly where the winning path puts it, with those
/// frames appended to its log -- the search itself costs the route nothing,
/// because it runs on a savestate and is rewound.
pub fn walk_to(
    rec: &mut Recorder,
    obs: &Observer,
    goal: Goal,
    max_nodes: usize,
) -> Result<usize, RouteError> {
    let start_state = rec.save_state()?;
    let path = search(rec.emu(), obs, &start_state, goal, max_nodes)?;
    rec.emu().load_state(&start_state)?;
    let frames = path.frames;
    for keys in path.inputs {
        rec.step(keys)?;
    }
    Ok(frames)
}

/// Dijkstra/A* from `start`, leaving `emu` on the goal state when it succeeds.
pub fn search(
    emu: &mut Emu,
    obs: &Observer,
    start: &SaveState,
    goal: Goal,
    max_nodes: usize,
) -> Result<Path, RouteError> {
    match search_best_effort(emu, obs, start, goal, max_nodes)? {
        (path, true) => Ok(path),
        (_, false) => Err(RouteError::Timeout {
            what: format!("a path to the goal (search exhausted {max_nodes} nodes)"),
            budget: max_nodes,
            frames: 0,
        }),
    }
}

/// [`search`], but exhaustion is an answer rather than an error: returns the
/// path to the node *closest to the goal* (by the Tile heuristic, ties on
/// cost) and whether the goal itself was reached. This is what lets a walk
/// through forced encounters make progress: commit the closest approach,
/// deal with whatever stopped it there (a battle, usually), and search again.
pub fn search_best_effort(
    emu: &mut Emu,
    obs: &Observer,
    start: &SaveState,
    mut goal: Goal,
    max_nodes: usize,
) -> Result<(Path, bool), RouteError> {
    emu.load_state(start)?;
    let start_place = place(obs, emu).ok_or_else(|| RouteError::Timeout {
        what: "a save block to walk from".into(),
        budget: 0,
        frames: 0,
    })?;

    enum H {
        None,
        Tile((u8, u8), i16, i16),
        Via((u8, u8), (u8, u8), (i16, i16)),
    }
    let target = match &goal {
        Goal::Tile { map, x, y } => H::Tile(*map, *x, *y),
        Goal::AnyOnVia { map, via_map, via } => H::Via(*map, *via_map, *via),
        _ => H::None,
    };
    let manhattan = |p: &Place, x: i16, y: i16| -> usize {
        p.pos.0.abs_diff(x) as usize + p.pos.1.abs_diff(y) as usize
    };
    let heuristic = move |p: &Place| -> usize {
        match target {
            H::Tile(map, x, y) if map == p.map => manhattan(p, x, y) * MIN_FRAMES_PER_TILE,
            H::Tile(..) | H::None => 0,
            H::Via(goal_map, _, _) if p.map == goal_map => 0,
            H::Via(_, via_map, (x, y)) if p.map == via_map => manhattan(p, x, y) * VIA_SCALE,
            H::Via(..) => OFF_ROUTE_PENALTY,
        }
    };
    let mut reached = |emu: &mut Emu, p: &Place| -> bool {
        match &mut goal {
            Goal::Tile { map, x, y } => *map == p.map && (*x, *y) == p.pos,
            Goal::AnyOn { map } | Goal::AnyOnVia { map, .. } => *map == p.map,
            Goal::Pred(f) => f(emu),
        }
    };

    // Per-node savestates are what make the search cheap in emulated frames and
    // expensive in memory, so a state is dropped the moment its node is
    // expanded; only the frontier holds them.
    let mut states: HashMap<Place, SaveState> = HashMap::new();
    let mut best: HashMap<Place, (usize, Vec<u16>)> = HashMap::new();
    let mut queue: BinaryHeap<Reverse<Node>> = BinaryHeap::new();

    states.insert(start_place, start.clone());
    best.insert(start_place, (0, Vec::new()));
    queue.push(Reverse(Node {
        cost: heuristic(&start_place),
        key: start_place,
    }));

    let mut expanded = 0usize;
    let mut battle_blocked = 0usize;
    while let Some(Reverse(node)) = queue.pop() {
        let Some(state) = states.remove(&node.key) else {
            continue; // already expanded through a cheaper path
        };
        let (cost, inputs) = best[&node.key].clone();
        expanded += 1;
        if expanded > max_nodes {
            break;
        }
        if expanded % 500 == 0 && std::env::var_os("FRLG_NAV_DEBUG").is_some() {
            eprintln!(
                "nav: {expanded} expansions, frontier {}, at map {:?} pos {:?} cost {cost}",
                states.len(),
                node.key.map,
                node.key.pos
            );
        }

        // Never probe straight back the way this node was entered: the
        // reverse edge recreates the parent's tile (under a shifted wild
        // state at best), and index-shifting loops are an optimisation for
        // a later, model-driven search -- not worth a quarter of every
        // expansion here.
        let came_with = inputs.last().copied().unwrap_or(0);
        for dir in DIRECTIONS {
            if dir == opposite(came_with) {
                continue;
            }
            emu.load_state(&state)?;
            let fed = match take_edge(emu, obs, dir, EDGE_BUDGET) {
                Edge::Took(fed) => fed,
                Edge::Battle => {
                    battle_blocked += 1;
                    continue;
                }
                Edge::Blocked => continue,
            };
            let Some(next) = place(obs, emu) else {
                continue;
            };
            let next_cost = cost + fed.len();
            let mut next_inputs = inputs.clone();
            next_inputs.extend_from_slice(&fed);

            if reached(emu, &next) {
                return Ok((
                    Path {
                        frames: next_inputs.len(),
                        inputs: next_inputs,
                    },
                    true,
                ));
            }
            if best.get(&next).is_some_and(|(seen, _)| *seen <= next_cost) {
                continue;
            }
            best.insert(next, (next_cost, next_inputs));
            states.insert(next, emu.save_state()?);
            queue.push(Reverse(Node {
                cost: next_cost + heuristic(&next),
                key: next,
            }));
        }
    }

    if std::env::var_os("FRLG_NAV_DEBUG").is_some() {
        let mut per_map: HashMap<(u8, u8), (usize, i16, i16)> = HashMap::new();
        for p in best.keys() {
            let e = per_map.entry(p.map).or_insert((0, i16::MAX, i16::MIN));
            e.0 += 1;
            e.1 = e.1.min(p.pos.1);
            e.2 = e.2.max(p.pos.1);
        }
        eprintln!(
            "nav: exhausted after {expanded} expansions, {} places, {battle_blocked} edges battle-blocked; per map (count, min y, max y):",
            best.len()
        );
        for (map, (n, lo, hi)) in &per_map {
            eprintln!("  map {map:?}: {n} tiles, y {lo}..{hi}");
        }
    }
    // Exhausted without reaching the goal: hand back the closest approach.
    // Only meaningful when the heuristic is (a Tile goal); otherwise this is
    // the start itself and the caller learns only that no path was found.
    let closest = best
        .iter()
        .min_by_key(|(place, (cost, _))| (heuristic(place), *cost))
        .map(|(_, (cost, inputs))| (inputs.clone(), *cost));
    let (inputs, _cost) = closest.unwrap_or_default();
    Ok((
        Path {
            frames: inputs.len(),
            inputs,
        },
        false,
    ))
}
