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
}

/// Where a walk is trying to get to.
pub enum Goal {
    /// A tile on a map. The only goal that enables the A* heuristic, since it
    /// is the only one that knows how far away it is.
    Tile { map: (u8, u8), x: i16, y: i16 },
    /// Any tile on a map -- "get through that door", where which side of the
    /// doormat you land on is the game's business.
    AnyOn { map: (u8, u8) },
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

    pub fn when(pred: impl FnMut(&mut Emu) -> bool + 'static) -> Self {
        Goal::Pred(Box::new(pred))
    }
}

fn place(obs: &Observer, emu: &mut Emu) -> Option<Place> {
    Some(Place {
        map: obs.map(emu)?,
        pos: obs.pos(emu)?,
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
fn take_edge(emu: &mut Emu, obs: &Observer, dir: u16, budget: usize) -> Option<Vec<u16>> {
    let start = place(obs, emu)?;
    let mut fed = Vec::new();
    for _ in 0..budget {
        emu.step(dir);
        fed.push(dir);

        // A script took the player (a trigger fired). Let it run, holding
        // nothing, so the direction does not leak into whatever it opens.
        if obs.prevent_step(emu) {
            settle(emu, obs, &mut fed, budget);
            return Some(fed);
        }

        match place(obs, emu) {
            Some(now) if now != start => {
                // A map change means a warp and a fade; the next edge would
                // otherwise hold a direction into a black screen.
                if now.map != start.map {
                    settle(emu, obs, &mut fed, budget);
                }
                return Some(fed);
            }
            _ => {}
        }
    }
    None
}

/// Hold nothing until the player is standing free again.
fn settle(emu: &mut Emu, obs: &Observer, fed: &mut Vec<u16>, budget: usize) {
    for _ in 0..budget {
        if obs.player_can_step(emu) {
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

    let target = match &goal {
        Goal::Tile { map, x, y } => Some((*map, *x, *y)),
        _ => None,
    };
    let heuristic = move |p: &Place| -> usize {
        match target {
            Some((map, x, y)) if map == p.map => {
                (p.pos.0.abs_diff(x) as usize + p.pos.1.abs_diff(y) as usize) * MIN_FRAMES_PER_TILE
            }
            _ => 0,
        }
    };
    let mut reached = |emu: &mut Emu, p: &Place| -> bool {
        match &mut goal {
            Goal::Tile { map, x, y } => *map == p.map && (*x, *y) == p.pos,
            Goal::AnyOn { map } => *map == p.map,
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
    while let Some(Reverse(node)) = queue.pop() {
        let Some(state) = states.remove(&node.key) else {
            continue; // already expanded through a cheaper path
        };
        let (cost, inputs) = best[&node.key].clone();
        expanded += 1;
        if expanded > max_nodes {
            break;
        }

        for dir in DIRECTIONS {
            emu.load_state(&state)?;
            let Some(fed) = take_edge(emu, obs, dir, EDGE_BUDGET) else {
                continue;
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
