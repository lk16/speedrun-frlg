#![allow(clippy::type_complexity)]
//! Walking as a human does it: plan a tile path from the static map data the player can see (the
//! decomp's collision, ledges, warps, connections, the people standing around), then hold the d-pad
//! along it, and deal with whatever happens on the way -- a wild encounter, a trainer who spots
//! the player, a script, an NPC that steps into the path -- when it happens.
//!
//! This deliberately does **not** use `frlg_route::nav`: nav's search expands nodes in the emulator
//! and treats battle-interrupted edges as blocked, which is foresight of the encounter RNG that a
//! human does not have. Here the plan is fixed before the step and the encounters fall where they
//! fall.
//!
//! The planner is a port of `bin/frlg-mapgraph`'s movement model (collision, warp tiles enterable
//! despite collision, water only when surfing, directed ledges, connections stitched by offset),
//! with live object positions read from `gObjectEvents` on the current map and objects hidden by
//! their `flag` (`FlagGet`, `decompiled/src/event_data.c`) left out on every map.
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use frlg_emu::keys;
use frlg_route::world::{Tile, World};
use frlg_route::Feed;
use serde_json::json;

use super::names::names;
use super::prims::{is_free, settle};
use super::session::{PResult, PlayError, Session};

pub type MapId = (u8, u8);

/// Water behaviors a surfer can cross (`decompiled/include/constants/metatile_behaviors.h`
/// 0x10-0x22 water group; the set `bin/frlg-mapgraph` uses).
pub const MB_WATER: [u16; 9] = [0x10, 0x11, 0x12, 0x13, 0x15, 0x19, 0x1A, 0x1B, 0x22];
/// Warp tiles are enterable despite their collision bit: `MB_CAVE_DOOR` 0x60 .. 0x6F except
/// `MB_LAVARIDGE_1F_WARP` 0x68 (`metatile_behaviors.h`).
fn spin_dir(b: u16) -> Option<(i16, i16)> {
    match b {
        0x54 => Some((1, 0)),
        0x55 => Some((-1, 0)),
        0x56 => Some((0, -1)),
        0x57 => Some((0, 1)),
        _ => None,
    }
}

pub fn is_warp_tile(b: u16) -> bool {
    (0x60..0x70).contains(&b) && b != 0x68
}
/// `MB_EAST/WEST/NORTH/SOUTH_ARROW_WARP` 0x62-0x65: the warp fires when the player presses the
/// arrow's direction while standing on it.
/// Side stairs `MB_UP_RIGHT/UP_LEFT/DOWN_RIGHT/DOWN_LEFT_STAIR_WARP` 0x6C-0x6F fire the same way
/// on EAST / WEST (`TryArrowWarp`, `IsDirectionalStairWarpMetatileBehavior`,
/// `decompiled/src/field_control_avatar.c:825-940`).
fn arrow_dir(b: u16) -> Option<u16> {
    match b {
        0x62 | 0x6C | 0x6E => Some(keys::RIGHT),
        0x63 | 0x6D | 0x6F => Some(keys::LEFT),
        0x64 => Some(keys::UP),
        0x65 => Some(keys::DOWN),
        _ => None,
    }
}
/// Ledges (`MB_JUMP_EAST` .. `MB_JUMP_SOUTH`, 0x38-0x3B): hop direction.
pub fn ledge(b: u16) -> Option<(i16, i16)> {
    match b {
        0x38 => Some((1, 0)),
        0x39 => Some((-1, 0)),
        0x3A => Some((0, -1)),
        0x3B => Some((0, 1)),
        _ => None,
    }
}

#[derive(Clone, Debug)]
pub struct ObjInfo {
    pub x: i16,
    pub y: i16,
    pub local_id: u8,
    pub movement: String,
    pub script: String,
    pub graphics: String,
    /// Hidden while this flag is set (0 = never hidden).
    pub flag: u16,
    pub trainer: bool,
    pub sight: i16,
    /// The trainer's beaten flag (`TRAINER_FLAGS_START` + its `TRAINER_*` id from the first
    /// `trainerbattle` of its script), 0 if unknown: a beaten trainer's sight no longer triggers.
    pub beaten: u16,
}

impl ObjInfo {
    fn stationary(&self) -> bool {
        let m = self.movement.as_str();
        m.contains("FACE")
            || m == "MOVEMENT_TYPE_NONE"
            || m.contains("ROTATE")
            || m.contains("BERRY")
            || m.contains("LOOK_AROUND")
            || self.graphics.contains("ITEM_BALL")
    }
    pub fn is_cut_tree(&self) -> bool {
        self.graphics.contains("CUT_TREE")
    }
    pub fn is_boulder(&self) -> bool {
        self.graphics.contains("PUSHABLE_BOULDER")
    }
    fn facing(&self) -> Option<(i16, i16)> {
        match self.movement.as_str() {
            "MOVEMENT_TYPE_FACE_UP" => Some((0, -1)),
            "MOVEMENT_TYPE_FACE_DOWN" => Some((0, 1)),
            "MOVEMENT_TYPE_FACE_LEFT" => Some((-1, 0)),
            "MOVEMENT_TYPE_FACE_RIGHT" => Some((1, 0)),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct MapInfo {
    pub name: String,
    pub id: String,
    pub w: i16,
    pub h: i16,
    tiles: Vec<Tile>,
    /// (x, y, destination map, destination warp index).
    pub warps: Vec<(i16, i16, Option<MapId>, usize)>,
    /// (direction, offset, map).
    pub conns: Vec<(String, i16, MapId)>,
    pub objects: Vec<ObjInfo>,
    /// `allow_cycling` of the map's `map.json` (the Bicycle can be ridden here).
    pub bike: bool,
}

impl MapInfo {
    pub fn tile(&self, x: i16, y: i16) -> Option<Tile> {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return None;
        }
        Some(self.tiles[(x + y * self.w) as usize])
    }

    pub fn set_collision(&mut self, x: i16, y: i16, blocked: bool) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            let w = self.w;
            self.tiles[(x + y * w) as usize].collision = u8::from(blocked);
        }
    }
}

/// The whole overworld, loaded once.
pub struct Graph {
    pub by_id: HashMap<String, MapId>,
    pub maps: HashMap<MapId, MapInfo>,
}

fn decomp_root() -> std::path::PathBuf {
    for v in ["FRLG_DECOMP", "FRLG_DECOMP_RO"] {
        if let Some(p) = std::env::var_os(v) {
            let p = std::path::PathBuf::from(p);
            if p.join("data/maps/map_groups.json").exists() {
                return p;
            }
        }
    }
    panic!("no decomp checkout")
}

impl Graph {
    pub fn load(world: &mut World) -> Result<Self, String> {
        let root = decomp_root();
        let n = names();
        let mut raw: Vec<(MapId, serde_json::Value)> = vec![];
        let mut by_id = HashMap::new();
        for g in 0..64u8 {
            if world.map_name((g, 0)).is_none() {
                break;
            }
            for m in 0..255u8 {
                let Some(name) = world.map_name((g, m)).map(str::to_string) else {
                    break;
                };
                let path = root.join(format!("data/maps/{name}/map.json"));
                let Ok(text) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let j: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("{name}: {e}"))?;
                if let Some(id) = j["id"].as_str() {
                    by_id.insert(id.to_string(), (g, m));
                }
                raw.push(((g, m), j));
            }
        }
        let int = |v: &serde_json::Value| -> i64 {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0)
        };
        let mut maps = HashMap::new();
        for (mid, j) in raw {
            let Ok(md) = world.map(mid) else {
                continue;
            };
            let (w, h) = (md.width as i16, md.height as i16);
            let mut tiles = Vec::with_capacity(md.width * md.height);
            for y in 0..h {
                for x in 0..w {
                    tiles.push(md.tile(x, y).unwrap());
                }
            }
            let name = md.name.clone();
            let warps = j["warp_events"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|wv| {
                    (
                        int(&wv["x"]) as i16,
                        int(&wv["y"]) as i16,
                        wv["dest_map"].as_str().and_then(|d| by_id.get(d).copied()),
                        int(&wv["dest_warp_id"]) as usize,
                    )
                })
                .collect();
            let conns = j["connections"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|c| {
                    Some((
                        c["direction"].as_str()?.to_string(),
                        int(&c["offset"]) as i16,
                        *by_id.get(c["map"].as_str()?)?,
                    ))
                })
                .collect();
            let scripts =
                std::fs::read_to_string(root.join(format!("data/maps/{name}/scripts.inc")))
                    .unwrap_or_default();
            let tflags = n.get("TRAINER_FLAGS_START").unwrap_or(0);
            let beaten_of = |script: &str| -> u16 {
                let mut inside = false;
                for line in scripts.lines() {
                    let l = line.trim();
                    if let Some(lbl) = l.strip_suffix("::") {
                        inside = lbl == script;
                        continue;
                    }
                    if inside && l.starts_with("trainerbattle") {
                        let t = l
                            .split_whitespace()
                            .nth(1)
                            .unwrap_or("")
                            .trim_end_matches(',');
                        return n.get(t).map_or(0, |id| tflags + id);
                    }
                }
                0
            };
            let objects = j["object_events"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
                .map(|(i, o)| {
                    let s = |k: &str| o[k].as_str().unwrap_or_default().to_string();
                    let flag = s("flag");
                    ObjInfo {
                        x: int(&o["x"]) as i16,
                        y: int(&o["y"]) as i16,
                        // A named `local_id` (`LOCALID_*`) is `#define`d to the object's index + 1
                        // (`decompiled/tools/mapjson/mapjson.cpp:415-416`), like an unnamed one.
                        local_id: o
                            .get("local_id")
                            .map(int)
                            .filter(|&v| v > 0)
                            .unwrap_or(i as i64 + 1) as u8,
                        movement: s("movement_type"),
                        script: s("script"),
                        graphics: s("graphics_id"),
                        flag: if flag.starts_with("FLAG_") {
                            n.get(&flag).unwrap_or(0)
                        } else {
                            0
                        },
                        trainer: s("trainer_type") == "TRAINER_TYPE_NORMAL",
                        sight: int(&o["trainer_sight_or_berry_tree_id"]) as i16,
                        beaten: if s("trainer_type") == "TRAINER_TYPE_NORMAL" {
                            beaten_of(&s("script"))
                        } else {
                            0
                        },
                    }
                })
                .collect();
            maps.insert(
                mid,
                MapInfo {
                    name,
                    id: j["id"].as_str().unwrap_or_default().to_string(),
                    w,
                    h,
                    tiles,
                    warps,
                    conns,
                    objects,
                    bike: j["allow_cycling"].as_bool().unwrap_or(false),
                },
            );
        }
        Ok(Graph { by_id, maps })
    }

    pub fn id(&self, map: &str) -> MapId {
        *self
            .by_id
            .get(map)
            .unwrap_or_else(|| panic!("unknown map {map}"))
    }
}

/// Planner switches.
#[derive(Clone, Default)]
pub struct Opts {
    /// May cut trees on the way (Cut is usable).
    pub cut: bool,
    /// May start surfing (Surf is usable).
    pub surf: bool,
    /// Objects whose script contains one of these are ignored (they will have moved by then).
    pub ignore: Vec<&'static str>,
    /// Maps the path must not enter.
    pub avoid: Vec<&'static str>,
    /// Tiles a script has opened since the layout was drawn (`setmetatile` barriers: gym beams,
    /// doors), as (map, x, y): walkable floor whatever the layout says.
    pub open: Vec<(&'static str, i16, i16)>,
    /// Collision of other maps as a script will have left it when the player gets there (the
    /// Mansion's switch doors): (map, x, y, blocked). The current map's live grid wins.
    pub patch: Vec<(&'static str, i16, i16, bool)>,
    /// Fail with a `Death("blocked: ...")` at once when the live view has no path (the caller
    /// knows a way to reset the blocker, e.g. leaving and re-entering the map).
    pub strict: bool,
    /// Unbeaten trainers' sight lines are walls (the first plan of every walk; the second, if
    /// the first finds no way, only prices them).
    pub cone_walls: bool,
}

pub type Node = (MapId, i16, i16);

/// What the planner knows of the live world when it plans.
struct View {
    cur_map: MapId,
    live: Vec<(i16, i16, u8)>,
    surfing: bool,
    flags: HashMap<u16, bool>,
    temp_block: HashSet<Node>,
    /// The current map as the game holds it now (collision patched from `VMap`).
    cur_live: Option<MapInfo>,
    /// Where the player stands and its `currentElevation`.
    player: Option<(Node, u8)>,
}

/// Cost of one step and the human's tie-breakers, in hundredths of a step.
const STEP: u32 = 100;
const TURN: u32 = 3;
const GRASS: u32 = 25;
const SIGHT: u32 = 300;
const CUT_COST: u32 = 400;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    /// Walk/hop to the next node holding `dir`.
    Step(u16),
    /// Cut the tree in direction `dir`, then step there.
    Cut(u16),
    /// Start surfing in direction `dir`.
    Surf(u16),
    /// On a warp tile: the warp takes the player to the next node.
    Warp,
}

pub struct Plan {
    pub nodes: Vec<Node>,
    pub moves: Vec<Move>,
}

impl Graph {
    fn blocked(
        &self,
        view: &View,
        mid: MapId,
        opts: &Opts,
    ) -> (
        HashSet<(i16, i16)>,
        HashSet<(i16, i16)>,
        HashSet<(i16, i16)>,
    ) {
        // (blocked, cut trees, trainer sight tiles)
        let mut b = HashSet::new();
        let mut trees = HashSet::new();
        let mut sight = HashSet::new();
        let Some(m) = self.maps.get(&mid) else {
            return (b, trees, sight);
        };
        let live_ids: HashSet<u8> = if mid == view.cur_map {
            view.live.iter().map(|l| l.2).collect()
        } else {
            HashSet::new()
        };
        for o in &m.objects {
            if o.flag != 0 && view.flags.get(&o.flag).copied().unwrap_or(false) {
                continue;
            }
            if opts.ignore.iter().any(|t| o.script.contains(t)) {
                continue;
            }
            if o.is_cut_tree() {
                trees.insert((o.x, o.y));
                continue;
            }
            if mid == view.cur_map && live_ids.contains(&o.local_id) {
                continue; // the live position below is the truth
            }
            if o.stationary() {
                b.insert((o.x, o.y));
            }
            let beaten = o.beaten != 0 && view.flags.get(&o.beaten).copied().unwrap_or(false);
            if o.trainer && o.sight > 0 && !beaten {
                let dirs: Vec<(i16, i16)> = match o.facing() {
                    Some(d) => vec![d],
                    None => vec![(0, -1), (0, 1), (-1, 0), (1, 0)],
                };
                for (dx, dy) in dirs {
                    for k in 1..=o.sight {
                        sight.insert((o.x + dx * k, o.y + dy * k));
                    }
                }
            }
        }
        if mid == view.cur_map {
            for &(x, y, id) in &view.live {
                let tree = m
                    .objects
                    .iter()
                    .any(|o| o.local_id == id && o.is_cut_tree());
                let ignored = m
                    .objects
                    .iter()
                    .any(|o| o.local_id == id && opts.ignore.iter().any(|t| o.script.contains(t)));
                if tree {
                    trees.insert((x, y));
                } else if !ignored {
                    b.insert((x, y));
                }
            }
        }
        (b, trees, sight)
    }

    fn passable(&self, m: &MapInfo, x: i16, y: i16, water_ok: bool) -> Option<bool> {
        // Some(is_water) when enterable.
        let t = m.tile(x, y)?;
        if is_warp_tile(t.behavior) {
            return Some(false);
        }
        if t.collision != 0 {
            return None;
        }
        if MB_WATER.contains(&t.behavior) {
            return water_ok.then_some(true);
        }
        if ledge(t.behavior).is_some() {
            return None;
        }
        Some(false)
    }

    /// Dijkstra over (map, x, y, direction) from `start` to any node satisfying `goal`.
    fn plan(
        &self,
        view: &View,
        start: Node,
        goal: &dyn Fn(&Node) -> bool,
        opts: &Opts,
    ) -> Option<Plan> {
        let mut cache: HashMap<
            MapId,
            (
                HashSet<(i16, i16)>,
                HashSet<(i16, i16)>,
                HashSet<(i16, i16)>,
            ),
        > = HashMap::new();
        let avoid: HashSet<MapId> = opts
            .avoid
            .iter()
            .filter_map(|a| self.by_id.get(*a).copied())
            .collect();
        let open: HashSet<Node> = opts
            .open
            .iter()
            .filter_map(|(m, x, y)| self.by_id.get(*m).map(|id| (*id, *x, *y)))
            .collect();
        let mut patched: HashMap<MapId, MapInfo> = HashMap::new();
        for &(pm, x, y, b) in &opts.patch {
            let Some(&id) = self.by_id.get(pm) else {
                continue;
            };
            if id == view.cur_map && view.cur_live.is_some() {
                continue;
            }
            let Some(base) = self.maps.get(&id) else {
                continue;
            };
            let m = patched.entry(id).or_insert_with(|| base.clone());
            if x >= 0 && y >= 0 && x < m.w && y < m.h {
                let w = m.w;
                m.tiles[(x + y * w) as usize].collision = u8::from(b);
            }
        }
        // state: node + last direction index (4 = none) + surfing
        type St = (Node, u8, bool);
        let mut dist: HashMap<St, u32> = HashMap::new();
        let mut prev: HashMap<St, (St, Move)> = HashMap::new();
        let mut q = BinaryHeap::new();
        // The dir field also carries the player's elevation in its high nibble
        // (`ObjectEventUpdateElevation` / `IsElevationMismatchAt`, `decompiled/src/event_object_movement.c`):
        // moving onto a tile of another elevation (not 0, not 15) is blocked on foot.
        let z0 = self
            .maps
            .get(&start.0)
            .and_then(|m| m.tile(start.1, start.2))
            .map_or(0, |t| if t.elevation == 15 { 0 } else { t.elevation });
        let z0 = match view.player {
            Some((n, z)) if n == start && z % 15 != 0 => z,
            _ => z0,
        };
        let s0: St = (start, 4 | (z0 << 4), view.surfing);
        dist.insert(s0, 0);
        q.push(Reverse((0u32, s0)));
        let dirs = [
            (keys::UP, 0i16, -1i16),
            (keys::DOWN, 0, 1),
            (keys::LEFT, -1, 0),
            (keys::RIGHT, 1, 0),
        ];
        let mut found = None;
        let mut expanded = 0usize;
        while let Some(Reverse((d, st))) = q.pop() {
            if dist.get(&st).is_some_and(|&b| b < d) {
                continue;
            }
            let (node, ldz, surfing) = st;
            let (ld, z) = (ldz & 0xF, ldz >> 4);
            if goal(&node) {
                found = Some(st);
                break;
            }
            expanded += 1;
            if expanded > 400_000 {
                break;
            }
            let (mid, x, y) = node;
            let Some(m) = (if mid == view.cur_map {
                view.cur_live.as_ref()
            } else {
                None
            })
            .or_else(|| patched.get(&mid))
            .or_else(|| self.maps.get(&mid)) else {
                continue;
            };
            let (blk, trees, sight) = cache
                .entry(mid)
                .or_insert_with(|| self.blocked(view, mid, opts))
                .clone();
            let push = |q: &mut BinaryHeap<Reverse<(u32, St)>>,
                        dist: &mut HashMap<St, u32>,
                        prev: &mut HashMap<St, (St, Move)>,
                        ns: St,
                        cost: u32,
                        mv: Move| {
                if avoid.contains(&ns.0 .0) || view.temp_block.contains(&ns.0) {
                    return;
                }
                let nd = d + cost;
                if dist.get(&ns).is_none_or(|&b| nd < b) {
                    dist.insert(ns, nd);
                    prev.insert(ns, (st, mv));
                    q.push(Reverse((nd, ns)));
                }
            };
            // A warp under the player's feet. A warp event only fires on a warp behavior
            // (`TryStartWarpEventScript` / `TryArrowWarp`, `field_control_avatar.c:825-866`): the
            // plain tiles of a three-wide exit mat around the one arrow tile do nothing.
            // Standing where the player started on a non-arrow warp (a Silph pad, arrived on by a
            // warp): it only fires on stepping onto it (`TryStartWarpEventScript` runs on a step),
            // so it has to be left and re-entered.
            let warp_here = m.tile(x, y).is_some_and(|t| {
                arrow_dir(t.behavior).is_some() || (is_warp_tile(t.behavior) && st != s0)
            });
            if let Some(&(_, _, Some(dm), wid)) =
                m.warps.iter().find(|w| warp_here && (w.0, w.1) == (x, y))
            {
                if let Some(dmap) = self.maps.get(&dm) {
                    if let Some(&(dx, dy, _, _)) = dmap.warps.get(wid) {
                        let wz = dmap.tile(dx, dy).map_or(0, |t| {
                            if t.elevation == 15 {
                                0
                            } else {
                                t.elevation
                            }
                        });
                        push(
                            &mut q,
                            &mut dist,
                            &mut prev,
                            ((dm, dx, dy), 4 | (wz << 4), false),
                            STEP,
                            Move::Warp,
                        );
                    }
                }
            }
            let here_e = m.tile(x, y).map_or(0, |t| t.elevation);
            // Elevation after stepping onto a tile of elevation `e` (unchanged across 15).
            let nz = |e: u8| -> u8 {
                if e == 15 || here_e == 15 {
                    z
                } else {
                    e
                }
            };
            let mismatch = |e: u8| z != 0 && e != 0 && e != 15 && e != z;
            for (di, &(key, dx, dy)) in dirs.iter().enumerate() {
                let turn = if ld != 4 && ld as usize != di {
                    TURN
                } else {
                    0
                };
                let (nx, ny) = (x + dx, y + dy);
                // Off the edge: a connection.
                if nx < 0 || ny < 0 || nx >= m.w || ny >= m.h {
                    let want = match (dx, dy) {
                        (0, -1) => "up",
                        (0, 1) => "down",
                        (-1, 0) => "left",
                        _ => "right",
                    };
                    for (cd, off, om) in &m.conns {
                        if cd != want {
                            continue;
                        }
                        let Some(o) = self.maps.get(om) else { continue };
                        let (tx, ty) = match want {
                            "up" => (x - off, o.h - 1),
                            "down" => (x - off, 0),
                            "left" => (o.w - 1, y - off),
                            _ => (0, y - off),
                        };
                        if let Some(w) = self.passable(o, tx, ty, surfing) {
                            let e = o.tile(tx, ty).map_or(0, |t| t.elevation);
                            push(
                                &mut q,
                                &mut dist,
                                &mut prev,
                                ((*om, tx, ty), di as u8 | (nz(e) << 4), w),
                                STEP + turn,
                                Move::Step(key),
                            );
                        }
                    }
                    continue;
                }
                let Some(t) = m.tile(nx, ny) else { continue };
                // Spin tiles (`MB_SPIN_RIGHT/LEFT/UP/DOWN` 0x54-0x57, `MB_STOP_SPINNING` 0x58,
                // `decompiled/include/constants/metatile_behaviors.h:66-70`): stepping on one
                // slides the player its way, turning on every further spin tile, until a stop tile
                // or something solid (`field_player_avatar.c` forced movement). The node is where
                // the slide ends.
                if spin_dir(t.behavior).is_some() && !surfing && !blk.contains(&(nx, ny)) {
                    let (mut cx, mut cy) = (nx, ny);
                    let mut d = spin_dir(t.behavior).unwrap();
                    let mut n = 0;
                    loop {
                        n += 1;
                        if n > 64 {
                            break;
                        }
                        let (tx, ty) = (cx + d.0, cy + d.1);
                        let free = self.passable(m, tx, ty, false) == Some(false)
                            && !blk.contains(&(tx, ty));
                        if !free {
                            break;
                        }
                        cx = tx;
                        cy = ty;
                        let b = m.tile(cx, cy).map_or(0, |t| t.behavior);
                        if b == 0x58 {
                            break;
                        }
                        if let Some(nd) = spin_dir(b) {
                            d = nd;
                        }
                    }
                    if n <= 64 {
                        push(
                            &mut q,
                            &mut dist,
                            &mut prev,
                            ((mid, cx, cy), di as u8 | (z << 4), false),
                            STEP * (1 + n as u32 / 2) + turn,
                            Move::Step(key),
                        );
                    }
                    continue;
                }
                if let Some(hop) = ledge(t.behavior) {
                    if hop == (dx, dy) && !surfing {
                        let (lx, ly) = (nx + dx, ny + dy);
                        if self.passable(m, lx, ly, false) == Some(false)
                            && !blk.contains(&(lx, ly))
                        {
                            let e = m.tile(lx, ly).map_or(0, |t| t.elevation);
                            push(
                                &mut q,
                                &mut dist,
                                &mut prev,
                                ((mid, lx, ly), di as u8 | (nz(e) << 4), false),
                                2 * STEP + turn,
                                Move::Step(key),
                            );
                        }
                    }
                    continue;
                }
                if trees.contains(&(nx, ny)) {
                    if opts.cut && !surfing && !mismatch(t.elevation) {
                        push(
                            &mut q,
                            &mut dist,
                            &mut prev,
                            ((mid, nx, ny), di as u8 | (nz(t.elevation) << 4), false),
                            STEP + CUT_COST + turn,
                            Move::Cut(key),
                        );
                    }
                    continue;
                }
                if blk.contains(&(nx, ny)) {
                    continue;
                }
                let opened = open.contains(&(mid, nx, ny));
                let Some(water) = self
                    .passable(m, nx, ny, surfing || opts.surf)
                    .or(opened.then_some(false))
                else {
                    continue;
                };
                // On foot onto land: the elevation has to match. Surfing onto land is a dismount
                // (`COLLISION_ELEVATION_MISMATCH` -> `COLLISION_STOP_SURFING`,
                // `decompiled/src/field_player_avatar.c`), and Surf starts from any shore.
                if !surfing && !water && !opened && mismatch(t.elevation) {
                    continue;
                }
                let new_z = if water {
                    0
                } else if surfing {
                    t.elevation % 15
                } else {
                    nz(t.elevation)
                };
                let mut cost = STEP + turn;
                if t.land {
                    cost += GRASS;
                }
                if sight.contains(&(nx, ny)) {
                    // An unbeaten trainer's sight line costs about a battle (~60 steps of
                    // walking) when asked, so only a shorter detour avoids the fight.
                    cost += if opts.cone_walls { 60 * STEP } else { SIGHT };
                }
                let mv = if water && !surfing {
                    cost += CUT_COST;
                    Move::Surf(key)
                } else {
                    Move::Step(key)
                };
                push(
                    &mut q,
                    &mut dist,
                    &mut prev,
                    ((mid, nx, ny), di as u8 | (new_z << 4), water),
                    cost,
                    mv,
                );
            }
        }
        if found.is_none() && std::env::var_os("FRLG_PLAN_DEBUG").is_some() {
            let mut per: HashMap<MapId, usize> = HashMap::new();
            for k in dist.keys() {
                *per.entry(k.0 .0).or_default() += 1;
            }
            for (m, n) in per {
                eprintln!(
                    "plan: reached {n} states on {}",
                    self.maps.get(&m).map_or("?", |x| x.name.as_str())
                );
            }
        }
        let end = found?;
        let mut nodes = vec![end.0];
        let mut moves = vec![];
        let mut cur = end;
        while let Some(&(p, mv)) = prev.get(&cur) {
            moves.push(mv);
            nodes.push(p.0);
            cur = p;
        }
        nodes.reverse();
        moves.reverse();
        Some(Plan { nodes, moves })
    }
}

/// `gPlayerAvatar.flags` bits (`decompiled/include/global.fieldmap.h:301-308`).
pub const AVATAR_MACH_BIKE: u8 = 1 << 1;
pub const AVATAR_ACRO_BIKE: u8 = 1 << 2;
pub const AVATAR_SURFING: u8 = 1 << 3;

pub fn here(s: &mut Session) -> Option<Node> {
    let emu = s.hf.rec.emu();
    let m = s.obs.map(emu)?;
    let p = s.obs.pos(emu)?;
    Some((m, p.0, p.1))
}

fn view(s: &mut Session, temp_block: &HashSet<Node>) -> View {
    let cur_map = here(s).map(|n| n.0).unwrap_or((0, 0));
    let live: Vec<(i16, i16, u8)> = {
        let ram = super::ram::Ram::new(&s.obs);
        let emu = s.hf.rec.emu();
        ram.objects(emu)
            .iter()
            .map(|o| (o.x, o.y, o.local_id))
            .collect()
    };
    let emu = s.hf.rec.emu();
    let surfing = s.obs.player_avatar_flags(emu) & AVATAR_SURFING != 0;
    let mut flags = HashMap::new();
    let g = s.graph();
    let fl: Vec<u16> = g
        .maps
        .values()
        .flat_map(|m| m.objects.iter().flat_map(|o| [o.flag, o.beaten]))
        .filter(|&f| f != 0)
        .collect();
    for f in fl {
        let v = s.obs.flag(s.hf.rec.emu(), f).unwrap_or(false);
        flags.insert(f, v);
    }
    View {
        cur_map,
        live,
        surfing,
        flags,
        temp_block: temp_block.clone(),
        cur_live: live_map(s, cur_map),
        player: here(s).map(|n| {
            (
                n,
                super::ram::Ram::new(&s.obs).player_elevation(s.hf.rec.emu()),
            )
        }),
    }
}

/// The current map with its collision bits read from the live grid: `VMap.map[(x + 7) +
/// VMap.Xsize * (y + 7)]`, collision in bits 10-11 (`decompiled/src/fieldmap.c:17,57-59,107-124`,
/// `struct BackupMapLayout` in `include/global.fieldmap.h:103-108`). Scripts' `setmetatile`
/// (gym beams, opened doors) write there, so this sees barriers the layout file does not.
pub fn live_map(s: &mut Session, cur: MapId) -> Option<MapInfo> {
    let g = s.graph();
    let mut m = g.maps.get(&cur)?.clone();
    let vmap = s.sym("VMap");
    let emu = s.hf.rec.emu();
    let xsize = emu.read32(vmap) as i32;
    let ptr = emu.read32(vmap + 8);
    if !(0x0200_0000..0x0204_0000).contains(&ptr) || xsize != m.w as i32 + 15 {
        return None;
    }
    let row = (m.w as u32) * 2;
    for y in 0..m.h {
        let a = ptr + (((y as i32 + 7) * xsize + 7) * 2) as u32;
        let bytes = emu.read_bytes(a, row);
        for x in 0..m.w {
            let v = u16::from_le_bytes([bytes[x as usize * 2], bytes[x as usize * 2 + 1]]);
            let tile = &mut m.tiles[(x + y * m.w) as usize];
            tile.collision = ((v >> 10) & 3) as u8;
            tile.elevation = (v >> 12) as u8;
        }
    }
    Some(m)
}

/// Whether to hold B (run) while walking: on foot only (`PLAYER_AVATAR_FLAG_ON_FOOT`), and the
/// Running Shoes are in hand (`FLAG_SYS_B_DASH`, `decompiled/include/constants/flags.h`).
fn run_bit(s: &mut Session) -> u16 {
    let emu = s.hf.rec.emu();
    let f = s.obs.player_avatar_flags(emu);
    let dash = s
        .obs
        .flag(emu, names().get("FLAG_SYS_B_DASH").unwrap_or(0))
        .unwrap_or(false);
    if f & (AVATAR_MACH_BIKE | AVATAR_ACRO_BIKE | AVATAR_SURFING) == 0 && dash {
        keys::B
    } else {
        0
    }
}

/// Outcome of following a plan for a while.
enum Follow {
    Arrived,
    /// Something interrupted (battle, script, deviation): plan again.
    Replan,
    /// Bumped into something unexpected at this node.
    Bumped(Node),
}

/// Walk to any node satisfying `goal`, re-planning after every interruption.
pub fn go_to(
    s: &mut Session,
    what: &str,
    goal: &dyn Fn(&Node) -> bool,
    opts: &Opts,
) -> PResult<()> {
    let start_frame = s.frame();
    let mut temp_block: HashSet<Node> = HashSet::new();
    let mut bumps = 0;
    let mut plans = 0;
    let mut bike_tried: Option<MapId> = None;
    loop {
        settle(s)?;
        let Some(cur) = here(s) else {
            return Err(PlayError::Fatal("no save block while walking".into()));
        };
        if goal(&cur) {
            break;
        }
        // On the bike wherever the map allows it (`allow_cycling`) and the walk is more than a
        // few tiles: the human gets on with the registered SELECT (once per map per walk).
        if bike_tried != Some(cur.0)
            && s.graph().maps.get(&cur.0).is_some_and(|m| m.bike)
            && super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), names().id("ITEM_BICYCLE")) > 0
            && s.obs.player_avatar_flags(s.hf.rec.emu())
                & (AVATAR_MACH_BIKE | AVATAR_ACRO_BIKE | AVATAR_SURFING)
                == 0
            && plan_len(s, None, goal, opts).is_some_and(|n| n > 6)
        {
            bike_tried = Some(cur.0);
            let _ = super::items::mount_bike(s)?;
        }
        plans += 1;
        if plans > 60 {
            return Err(PlayError::Death(format!(
                "soft-lock: {what}: 60 re-plans without arriving"
            )));
        }
        let v = view(s, &temp_block);
        // Around every unbeaten trainer's sight line if there is a way, else through (a forced
        // fight: `human-run/tools/forced.py` audits which ones the plan cannot avoid).
        let walls = Opts {
            cone_walls: true,
            ..opts.clone()
        };
        let live = s
            .graph()
            .plan(&v, cur, goal, &walls)
            .or_else(|| s.graph().plan(&v, cur, goal, opts));
        if live.is_none() && opts.strict {
            return Err(PlayError::Death(format!(
                "blocked: {what}: no path around live objects"
            )));
        }
        let plan = live.or_else(|| {
            // Which live input closed the way? Try the layout's own collision, then no people.
            let mut v2 = view(s, &temp_block);
            v2.cur_live = None;
            let p = s.graph().plan(&v2, cur, goal, opts);
            if p.is_some() {
                eprintln!("walk: {what}: no path on the live grid, one on the layout's");
                return p;
            }
            v2.live.clear();
            let p = s.graph().plan(&v2, cur, goal, opts);
            if p.is_some() {
                eprintln!("walk: {what}: no path around the live objects, one through them");
            }
            p
        });
        let Some(plan) = plan else {
            if !temp_block.is_empty() {
                temp_block.clear();
                continue;
            }
            return Err(PlayError::Fatal(format!(
                "{what}: no path from {cur:?} ({})",
                s.graph().maps.get(&cur.0).map_or("?", |m| m.name.as_str())
            )));
        };
        match follow(s, &plan)? {
            Follow::Arrived => {}
            Follow::Replan => {}
            Follow::Bumped(n) => {
                bumps += 1;
                let mn = s
                    .graph()
                    .maps
                    .get(&n.0)
                    .map_or("?".to_string(), |m| m.name.clone());
                s.event(
                    "warn",
                    format!(
                        "bumped into something at {mn} ({}, {}); re-planning",
                        n.1, n.2
                    ),
                    json!({"map": mn, "x": n.1, "y": n.2}),
                );
                temp_block.insert(n);
                if bumps > 40 {
                    return Err(PlayError::Death(format!("soft-lock: {what}: 40 bumps")));
                }
            }
        }
    }
    let _ = start_frame;
    Ok(())
}

/// Length in steps of the path the planner would take now from where the player stands to a node
/// satisfying `goal` (no input fed), or from `from` if given (then the live view of the current
/// map still applies to the current map only).
pub fn plan_len(
    s: &mut Session,
    from: Option<Node>,
    goal: &dyn Fn(&Node) -> bool,
    opts: &Opts,
) -> Option<usize> {
    let cur = from.or_else(|| here(s))?;
    let mut v = view(s, &HashSet::new());
    if from.is_some() {
        // A hypothetical start: the current map as `opts.patch` will leave it, not as it is now.
        v.cur_live = None;
    }
    s.graph().plan(&v, cur, goal, opts).map(|p| p.moves.len())
}

/// The nodes of the path the planner would take from `from` (the current map as `opts.patch`
/// leaves it, not the live grid) to a node satisfying `goal`.
pub fn plan_nodes(
    s: &mut Session,
    from: Node,
    goal: &dyn Fn(&Node) -> bool,
    opts: &Opts,
) -> Option<Vec<Node>> {
    let mut v = view(s, &HashSet::new());
    v.cur_live = None;
    s.graph().plan(&v, from, goal, opts).map(|p| p.nodes)
}

/// Walk to `(map, x, y)`.
pub fn go(s: &mut Session, map: &str, x: i16, y: i16, opts: &Opts) -> PResult<()> {
    let mid = s.graph().id(map);
    go_to(
        s,
        &format!("{map} ({x}, {y})"),
        &move |n: &Node| *n == (mid, x, y),
        opts,
    )
}

/// Walk until standing anywhere on `map`.
pub fn go_map(s: &mut Session, map: &str, opts: &Opts) -> PResult<()> {
    let mid = s.graph().id(map);
    go_to(s, map, &move |n: &Node| n.0 == mid, opts)
}

/// Walk next to `(x, y)` on `map` and face it (to talk to someone or use something there).
pub fn go_face(s: &mut Session, map: &str, x: i16, y: i16, opts: &Opts) -> PResult<()> {
    let mid = s.graph().id(map);
    go_to(
        s,
        &format!("next to {map} ({x}, {y})"),
        &move |n: &Node| n.0 == mid && (n.1 - x).abs() + (n.2 - y).abs() == 1,
        opts,
    )?;
    let cur = here(s).unwrap();
    let dir = match (x - cur.1, y - cur.2) {
        (0, -1) => keys::UP,
        (0, 1) => keys::DOWN,
        (-1, 0) => keys::LEFT,
        _ => keys::RIGHT,
    };
    face(s, dir)
}

/// `DIR_*` for a d-pad key (`decompiled/include/constants/global.h:110-113`).
fn dir_code(key: u16) -> u8 {
    match key {
        keys::DOWN => 1,
        keys::UP => 2,
        keys::LEFT => 3,
        _ => 4,
    }
}

/// Turn in place to face `dir` (a tap shorter than a step turns without walking).
pub fn face(s: &mut Session, dir: u16) -> PResult<()> {
    for _ in 0..6 {
        let f = super::ram::Ram::new(&s.obs).player_facing(s.hf.rec.emu());
        if f == dir_code(dir) {
            return Ok(());
        }
        s.hf.late()?;
        s.hf.step(dir)?;
        s.hf.idle(8)?;
        settle(s)?;
    }
    Err(PlayError::Death("soft-lock: could not turn".into()))
}

const STEP_BUDGET: usize = 48;

fn follow(s: &mut Session, plan: &Plan) -> PResult<Follow> {
    let mut last_key = 0u16;
    for (i, mv) in plan.moves.iter().enumerate() {
        let from = plan.nodes[i];
        let to = plan.nodes[i + 1];
        if here(s) != Some(from) {
            return Ok(Follow::Replan);
        }
        match *mv {
            Move::Cut(dir) | Move::Surf(dir) => {
                face(s, dir)?;
                s.hf.press(keys::A)?;
                settle(s)?;
                let what = if matches!(mv, Move::Cut(_)) {
                    "cut a tree"
                } else {
                    "started surfing"
                };
                s.event(
                    "decision",
                    format!("{what} at ({}, {})", to.1, to.2),
                    json!({}),
                );
                return Ok(Follow::Replan);
            }
            Move::Warp => {
                // Standing on the warp: doors and stairs fire on arrival; arrow warps need the
                // arrow pressed; otherwise keep holding the way we came.
                let beh = s
                    .graph()
                    .maps
                    .get(&from.0)
                    .and_then(|m| m.tile(from.1, from.2))
                    .map_or(0, |t| t.behavior);
                let key =
                    arrow_dir(beh).unwrap_or(if last_key != 0 { last_key } else { keys::DOWN });
                let mut fed = 0;
                loop {
                    let n = here(s);
                    if n.is_some_and(|n| n.0 != from.0) || s.obs.in_battle(s.hf.rec.emu()) {
                        break;
                    }
                    if fed > 90 {
                        return Ok(Follow::Bumped(to));
                    }
                    let held = if s.obs.player_can_step(s.hf.rec.emu()) {
                        key
                    } else {
                        0
                    };
                    s.hf.step(held)?;
                    fed += 1;
                }
                settle(s)?;
                return Ok(Follow::Replan);
            }
            Move::Step(dir) => {
                if dir != last_key && last_key != 0 {
                    // A change of direction: the human's thumb is late by U{0..N}.
                    s.hf.late()?;
                } else if last_key == 0 {
                    s.hf.late()?;
                }
                last_key = dir;
                let mut fed = 0;
                loop {
                    let emu = s.hf.rec.emu();
                    if s.obs.in_battle(emu) {
                        settle(s)?;
                        return Ok(Follow::Replan);
                    }
                    let n = here(s);
                    if n != Some(from) {
                        if n == Some(to) {
                            break;
                        }
                        // A map change through a connection or warp, or a forced move.
                        settle(s)?;
                        return Ok(Follow::Replan);
                    }
                    let emu = s.hf.rec.emu();
                    if s.obs.field_controls_locked(emu) || (fed > 2 && s.obs.prevent_step(emu)) {
                        settle(s)?;
                        return Ok(Follow::Replan);
                    }
                    if fed > STEP_BUDGET {
                        return Ok(Follow::Bumped(to));
                    }
                    let b = run_bit(s);
                    s.hf.step(dir | b)?;
                    fed += 1;
                }
            }
        }
    }
    // Let the last step land.
    for _ in 0..40 {
        if is_free(s) {
            break;
        }
        if s.obs.in_battle(s.hf.rec.emu()) || s.obs.field_controls_locked(s.hf.rec.emu()) {
            settle(s)?;
            break;
        }
        s.hf.step(0)?;
    }
    Ok(Follow::Arrived)
}
