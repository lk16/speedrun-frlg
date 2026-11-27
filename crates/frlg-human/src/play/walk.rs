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
use super::ram::LiveObject;
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
/// One-sided walls `MB_IMPASSABLE_EAST/WEST/NORTH/SOUTH/NORTHEAST/NORTHWEST/SOUTHEAST/SOUTHWEST`
/// 0x30-0x37 (`decompiled/include/constants/metatile_behaviors.h:39-46`): a step from `src` to `dst`
/// by `(dx, dy)` is refused when the source is blocked on the side it leaves by or the
/// destination on the side it enters by (`IsMetatileDirectionallyImpassable`,
/// `decompiled/src/event_object_movement.c:889-901,4889-4897`; `MetatileBehavior_Is*Blocked`,
/// `metatile_behavior.c:544-582`). Mt. Moon B2F row 14 is `MB_IMPASSABLE_NORTH`: not enterable
/// from row 13 (13 bumps per pass before this).
pub fn dir_blocked(src: u16, dst: u16, dx: i16, dy: i16) -> bool {
    let north = |b: u16| matches!(b, 0x32 | 0x34 | 0x35);
    let south = |b: u16| matches!(b, 0x33 | 0x36 | 0x37);
    let east = |b: u16| matches!(b, 0x30 | 0x34 | 0x36);
    let west = |b: u16| matches!(b, 0x31 | 0x35 | 0x37);
    match (dx, dy) {
        (0, 1) => south(src) || north(dst),
        (0, -1) => north(src) || south(dst),
        (-1, 0) => west(src) || east(dst),
        (1, 0) => east(src) || west(dst),
        _ => false,
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
    /// `movement_range_x` / `movement_range_y`: a wanderer stays within this box around its
    /// spawn tile (`IsCoordOutsideObjectEventMovementRange`, `decompiled/src/event_object_movement.c:4860-4885`).
    pub range_x: i16,
    pub range_y: i16,
}

/// Unit vector of a `DIR_*` code (`DIR_SOUTH` 1, `DIR_NORTH` 2, `DIR_WEST` 3, `DIR_EAST` 4,
/// `decompiled/include/constants/global.h`).
pub fn dir_vec(d: u8) -> Option<(i16, i16)> {
    match d {
        1 => Some((0, 1)),
        2 => Some((0, -1)),
        3 => Some((-1, 0)),
        4 => Some((1, 0)),
        _ => None,
    }
}

impl ObjInfo {
    /// The directions this object can face. `FACE_*` types hold one; the others turn among the
    /// directions their name lists (`WANDER_UP_AND_DOWN`: north and south, `FACE_DOWN_AND_LEFT`:
    /// south and west, the movement-type tables of `decompiled/src/event_object_movement.c`), and
    /// `LOOK_AROUND`, `WANDER_AROUND`, `ROTATE_*` and the rest turn every way.
    /// Offsets from the spawn tile where this object can stand: a wanderer's movement-range box
    /// (`WANDER_UP_AND_DOWN` only up and down, `WANDER_LEFT_AND_RIGHT` only sideways,
    /// `decompiled/src/event_object_movement.c` `MovementType_Wander*`); everyone else stays put.
    pub fn offsets(&self) -> Vec<(i16, i16)> {
        let m = self.movement.as_str();
        let (rx, ry) = if m.contains("WANDER_UP_AND_DOWN") || m.contains("WANDER_DOWN_AND_UP") {
            (0, self.range_y)
        } else if m.contains("WANDER_LEFT_AND_RIGHT") || m.contains("WANDER_RIGHT_AND_LEFT") {
            (self.range_x, 0)
        } else if m.contains("WANDER") || m.contains("WALK_") {
            (self.range_x, self.range_y)
        } else {
            (0, 0)
        };
        let mut v = vec![];
        for dx in -rx..=rx {
            for dy in -ry..=ry {
                v.push((dx, dy));
            }
        }
        v
    }

    /// Every (tile, facing) this object can have, spawned at `(x0, y0)`. A `WALK_SEQUENCE_A_B_C_D`
    /// object walks direction A until the next tile is outside its range, then B, C, D, and starts
    /// over at its spawn (`MoveNextDirectionInSequence`, `decompiled/src/event_object_movement.c:3909-3935`),
    /// so only the rim of its box with one facing per side; an obstacle stops it (it walks in
    /// place). Everyone else: any offset of [`Self::offsets`] with any of [`Self::facings`].
    /// The four directions of a `WALK_SEQUENCE_A_B_C_D` object, in order.
    pub fn sequence_dirs(&self) -> Option<Vec<(i16, i16)>> {
        let seq = self.movement.strip_prefix("MOVEMENT_TYPE_WALK_SEQUENCE_")?;
        let dirs: Vec<(i16, i16)> = seq
            .split('_')
            .filter_map(|w| match w {
                "UP" => Some((0, -1)),
                "DOWN" => Some((0, 1)),
                "LEFT" => Some((-1, 0)),
                "RIGHT" => Some((1, 0)),
                _ => None,
            })
            .collect();
        (dirs.len() == 4).then_some(dirs)
    }

    pub fn poses(&self, m: &MapInfo, x0: i16, y0: i16) -> Vec<((i16, i16), (i16, i16))> {
        {
            if let Some(dirs) = self.sequence_dirs() {
                let out_of_range = |x: i16, y: i16| {
                    (self.range_x != 0 && (x - x0).abs() > self.range_x)
                        || (self.range_y != 0 && (y - y0).abs() > self.range_y)
                };
                let (mut x, mut y, mut k) = (x0, y0, 0usize);
                let mut v = vec![];
                for _ in 0..256 {
                    if k == 3 && (x, y) == (x0, y0) {
                        break;
                    }
                    let mut d = dirs[k];
                    if out_of_range(x + d.0, y + d.1) && k < 3 {
                        k += 1;
                        d = dirs[k];
                    }
                    v.push(((x, y), d));
                    let free = m.tile(x + d.0, y + d.1).is_some_and(|t| t.collision == 0)
                        && !out_of_range(x + d.0, y + d.1);
                    if !free {
                        break;
                    }
                    x += d.0;
                    y += d.1;
                    v.push(((x, y), d));
                }
                return v;
            }
        }
        let mut v = vec![];
        for (dx, dy) in self.offsets() {
            for d in self.facings() {
                v.push(((x0 + dx, y0 + dy), d));
            }
        }
        v
    }

    pub fn facings(&self) -> Vec<(i16, i16)> {
        if let Some(d) = self.facing() {
            return vec![d];
        }
        let m = self
            .movement
            .strip_prefix("MOVEMENT_TYPE_")
            .unwrap_or(&self.movement);
        let fixed = m.starts_with("FACE_") || m.starts_with("WANDER_") || m.starts_with("WALK_");
        let words: Vec<&str> = m.split('_').collect();
        let mut v = vec![];
        for (w, d) in [
            ("UP", (0, -1)),
            ("DOWN", (0, 1)),
            ("LEFT", (-1, 0)),
            ("RIGHT", (1, 0)),
        ] {
            if words.contains(&w) {
                v.push(d);
            }
        }
        if !fixed || v.is_empty() || m.contains("AROUND") {
            return vec![(0, -1), (0, 1), (-1, 0), (1, 0)];
        }
        v
    }

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
        let trainers_inc =
            std::fs::read_to_string(root.join("data/scripts/trainers.inc")).unwrap_or_default();
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
            // Most trainers' scripts live in `data/scripts/trainers.inc`, not the map's own
            // `scripts.inc` (`Route3_EventScript_Ben`: `trainerbattle_single TRAINER_YOUNGSTER_BEN`):
            // without it no trainer had a beaten flag and beaten trainers' lines stayed walls.
            let scripts =
                std::fs::read_to_string(root.join(format!("data/maps/{name}/scripts.inc")))
                    .unwrap_or_default()
                    + &trainers_inc;
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
                        range_x: int(&o["movement_range_x"]) as i16,
                        range_y: int(&o["movement_range_y"]) as i16,
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
    /// Plan for the fewest steps only (no grass or turn tie-breakers, no wait price): the
    /// "shortest legal path" the detour check compares a walked leg with.
    pub steps_only: bool,
}

pub type Node = (MapId, i16, i16);

/// What the planner knows of the live world when it plans.
struct View {
    cur_map: MapId,
    live: Vec<LiveObject>,
    /// The current map's object templates in RAM: (local id, x, y).
    templates: Vec<(u8, i16, i16)>,
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
thread_local! {
    /// The grass price of this thread's session (`Params::walk_grass_cost`, default [`GRASS`]).
    static GRASS_COST: std::cell::Cell<u32> = const { std::cell::Cell::new(GRASS) };
}

/// Set the walker's grass price for this thread (the session does at the start of `play`).
pub fn set_grass_cost(c: u32) {
    GRASS_COST.with(|g| g.set(c));
}
const SIGHT: u32 = 300;
/// Crossing a turning trainer's possible line: the expected wait for it to face away (~90 frames,
/// about 10 running steps; review item 17 estimates 1.5 s for Lass Robin).
const WATCH: u32 = 1000;
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
    /// Per node: the local ids of the turning trainers on screen that could see it (wait for
    /// them to face away before stepping there).
    pub watch: Vec<Vec<u8>>,
    /// Per node: inside a spinner's range box (enter it walking, B released).
    pub norun: Vec<bool>,
    /// Per node: the local id of an unbeaten trainer whose line it is on (a fight the plan could
    /// not walk around), if any.
    pub seen: Vec<Option<u8>>,
    /// The live objects (map, local id) the plan was made with.
    pub live: Vec<(MapId, u8)>,
}

/// What the objects of one map do to the planner.
#[derive(Clone, Default)]
struct Blk {
    /// Tiles an object stands on (a live object also blocks the tile it is leaving).
    b: HashSet<(i16, i16)>,
    trees: HashSet<(i16, i16)>,
    /// Tiles an unbeaten trainer sees and keeps seeing: a fixed-facing trainer's line, or a
    /// turning trainer's line when it is not on screen (the planner cannot wait for a facing it
    /// does not see). Avoided at the price of a fight.
    sight: HashMap<(i16, i16), u8>,
    /// Tiles a turning trainer on screen (a wanderer, `LOOK_AROUND`, ...) can see from where it
    /// can be: crossed only after waiting for it to face away (`follow`), so priced as a wait.
    /// Each tile maps to the local ids of the trainers that watch it.
    watch: HashMap<(i16, i16), Vec<u8>>,
    /// Tiles inside a spinner's range box (`LOOK_AROUND`, `FACE_A_AND_B`, `ROTATE_*` trainers on
    /// screen): a player who *runs* there makes it turn to face him at once
    /// (`ObjectEventIsTrainerAndCloseToPlayer` needs `PLAYER_AVATAR_FLAG_DASH` and the player within
    /// `trainerRange` on both axes, `decompiled/src/event_object_movement.c:2772-2798`; used by
    /// `MovementType_LookAround_Step3/4`, `:3044-3066`, and every `FACE_*_AND_*` type), so these
    /// tiles are entered walking. Wanderers do not react (`MovementType_WanderAround_Step3`, `:2721`).
    norun: HashSet<(i16, i16)>,
}

/// The tiles a trainer at `(x, y)` sees in direction `d` with range `r`: up to `r` tiles, cut
/// at the first impassable tile (`CheckPathBetweenTrainerAndPlayer`, `decompiled/src/trainer_see.c:196-232`:
/// any collision on the way but the movement-range bit ends the line).
fn sight_line(m: &MapInfo, x: i16, y: i16, d: (i16, i16), r: i16, out: &mut Vec<(i16, i16)>) {
    for k in 1..=r {
        let (tx, ty) = (x + d.0 * k, y + d.1 * k);
        match m.tile(tx, ty) {
            Some(t) if t.collision == 0 || is_warp_tile(t.behavior) => out.push((tx, ty)),
            _ => break,
        }
    }
}

impl Plan {
    /// Length in tiles: a ledge hop or a spin slide covers several, a warp or a connection one.
    pub fn tiles(&self) -> usize {
        self.nodes.windows(2).map(|w| step_tiles(w[0], w[1])).sum()
    }
}

fn step_tiles(a: Node, b: Node) -> usize {
    if a.0 == b.0 {
        ((a.1 - b.1).abs() + (a.2 - b.2).abs()) as usize
    } else {
        1
    }
}

impl Graph {
    /// `m` is the map as the planner sees it (the live grid for the current map).
    fn blocked(&self, view: &View, mid: MapId, m: &MapInfo, opts: &Opts) -> Blk {
        let mut out = Blk::default();
        let Some(base) = self.maps.get(&mid) else {
            return out;
        };
        let cur = mid == view.cur_map;
        // Live objects of this map (an object of the map the player just left can linger,
        // with its own `mapNum`/`mapGroup`; it still collides but is not this map's template).
        let live_ids: HashSet<u8> = if cur {
            view.live
                .iter()
                .filter(|l| l.map == mid)
                .map(|l| l.local_id)
                .collect()
        } else {
            HashSet::new()
        };
        let beaten =
            |o: &ObjInfo| o.beaten != 0 && view.flags.get(&o.beaten).copied().unwrap_or(false);
        for o in &base.objects {
            // A set flag means the object was never spawned (`TrySpawnObjectEvents` skips a
            // template whose flag is set, `decompiled/src/event_object_movement.c`).
            if o.flag != 0 && view.flags.get(&o.flag).copied().unwrap_or(false) {
                continue;
            }
            if opts.ignore.iter().any(|t| o.script.contains(t)) {
                continue;
            }
            if cur && live_ids.contains(&o.local_id) {
                continue; // the live object below is the truth
            }
            // On the current map the template in RAM says where the object (re)spawns.
            let (ox, oy) = view
                .templates
                .iter()
                .find(|t| cur && t.0 == o.local_id)
                .map_or((o.x, o.y), |t| (t.1, t.2));
            if o.is_cut_tree() {
                out.trees.insert((ox, oy));
                continue;
            }
            if o.stationary() {
                out.b.insert((ox, oy));
            }
            if o.trainer && o.sight > 0 && !beaten(o) {
                // Off screen: every tile it could see from anywhere in its range box.
                let mut line = vec![];
                for ((px, py), d) in o.poses(m, ox, oy) {
                    sight_line(m, px, py, d, o.sight, &mut line);
                }
                out.sight.extend(line.into_iter().map(|t| (t, o.local_id)));
            }
        }
        if cur {
            for l in &view.live {
                let info = if l.map == mid {
                    base.objects.iter().find(|o| o.local_id == l.local_id)
                } else {
                    None
                };
                if info.is_some_and(|o| opts.ignore.iter().any(|t| o.script.contains(t))) {
                    continue;
                }
                if info.is_some_and(ObjInfo::is_cut_tree) {
                    out.trees.insert((l.x, l.y));
                    continue;
                }
                // `DoesObjectCollideWithObjectAt` checks both coordinates
                // (`decompiled/src/event_object_movement.c:4899-4915`).
                out.b.insert((l.x, l.y));
                out.b.insert((l.px, l.py));
                let Some(o) = info else { continue };
                // `CheckForTrainersWantingBattle` runs every frame the player has control
                // (`ProcessPlayerFieldInput`, `decompiled/src/field_control_avatar.c:209`) and looks
                // from the trainer's *current* coordinates along its *current* facing (`TRAINER_TYPE_NORMAL`,
                // `GetTrainerApproachDistance`, `decompiled/src/trainer_see.c:122-143`).
                if l.trainer_type == 0 || l.range == 0 || beaten(o) {
                    continue;
                }
                let r = l.range as i16;
                let facings = o.facings();
                if l.trainer_type != 1 {
                    // `TRAINER_TYPE_SEE_ALL_DIRECTIONS` / `BURIED`: all four, always.
                    let mut line = vec![];
                    for d in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                        sight_line(m, l.x, l.y, d, r, &mut line);
                    }
                    out.sight.extend(line.into_iter().map(|t| (t, l.local_id)));
                    continue;
                }
                let mut now = vec![];
                if let Some(d) = dir_vec(l.facing) {
                    sight_line(m, l.x, l.y, d, r, &mut now);
                }
                if facings.len() <= 1 {
                    out.sight.extend(now.into_iter().map(|t| (t, l.local_id)));
                    continue;
                }
                // A turning trainer: every line it can have from anywhere in its box.
                let (ix, iy) = view
                    .templates
                    .iter()
                    .find(|t| t.0 == o.local_id)
                    .map_or((o.x, o.y), |t| (t.1, t.2));
                let mut line = vec![];
                for ((px, py), d) in o.poses(m, ix, iy) {
                    sight_line(m, px, py, d, r, &mut line);
                }
                for &d in &facings {
                    sight_line(m, l.x, l.y, d, r, &mut line);
                }
                let m_ = o.movement.as_str();
                if !m_.contains("WANDER") && !m_.contains("WALK_") {
                    for dx in -r..=r {
                        for dy in -r..=r {
                            out.norun.insert((l.x + dx, l.y + dy));
                        }
                    }
                }
                for t in line.into_iter().chain(now) {
                    let w = out.watch.entry(t).or_default();
                    if !w.contains(&l.local_id) {
                        w.push(l.local_id);
                    }
                }
            }
        }
        out
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
        let mut cache: HashMap<MapId, std::rc::Rc<Blk>> = HashMap::new();
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
            let bl = cache
                .entry(mid)
                .or_insert_with(|| std::rc::Rc::new(self.blocked(view, mid, m, opts)))
                .clone();
            let (blk, trees, sight, watch) = (&bl.b, &bl.trees, &bl.sight, &bl.watch);
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
                let turn = if ld != 4 && ld as usize != di && !opts.steps_only {
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
                let src_b = m.tile(x, y).map_or(0, |t| t.behavior);
                if dir_blocked(src_b, t.behavior, dx, dy) {
                    continue;
                }
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
                // Off the water only onto elevation-3 land: `CanStopSurfing`
                // (`decompiled/src/field_player_avatar.c:597-606`) needs `MapGridGetElevationAt == 3`;
                // other land is an elevation mismatch, i.e. a bump (Cerulean Cave 1F's elevation-4
                // rims: 18 of the v4 run's bumps).
                if surfing && !water && !opened && !matches!(t.elevation, 0 | 3 | 15) {
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
                if t.land && !opts.steps_only {
                    cost += GRASS_COST.with(|g| g.get());
                }
                if sight.contains_key(&(nx, ny)) {
                    // An unbeaten trainer's sight line costs about a battle (~60 steps of
                    // walking) when asked, so only a shorter detour avoids the fight.
                    cost += if opts.cone_walls { 60 * STEP } else { SIGHT };
                } else if watch.contains_key(&(nx, ny)) && !opts.steps_only {
                    // A turning trainer's possible line: crossed after waiting for it to face
                    // away (`follow`), ~1.5 s (review item 17).
                    cost += WATCH;
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
        let mut watch = vec![];
        let mut seen = vec![];
        let mut norun = vec![];
        for n in &nodes {
            let b = cache.get(&n.0);
            watch.push(
                b.and_then(|b| b.watch.get(&(n.1, n.2)).cloned())
                    .unwrap_or_default(),
            );
            seen.push(b.and_then(|b| b.sight.get(&(n.1, n.2)).copied()));
            norun.push(b.is_some_and(|b| b.norun.contains(&(n.1, n.2))));
        }
        let mut live: Vec<(MapId, u8)> = view.live.iter().map(|l| (l.map, l.local_id)).collect();
        live.sort_unstable();
        Some(Plan {
            nodes,
            moves,
            watch,
            seen,
            norun,
            live,
        })
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

/// The active object events other than the player (`gObjectEvents`).
pub fn live_objects(s: &mut Session) -> Vec<LiveObject> {
    let ram = super::ram::Ram::new(&s.obs);
    let emu = s.hf.rec.emu();
    ram.objects(emu)
}

fn view(s: &mut Session, temp_block: &HashSet<Node>) -> View {
    let cur_map = here(s).map(|n| n.0).unwrap_or((0, 0));
    let live = live_objects(s);
    let templates = super::ram::Ram::new(&s.obs).templates(s.hf.rec.emu());
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
        templates,
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
    /// The set of live objects changed while a step was under way (someone came on screen or
    /// left): plan again from the tile being entered, without letting go of the d-pad (the key
    /// held so far).
    Refresh(u16, Node),
    /// Bumped into something unexpected at this node.
    Bumped(Node),
}

/// What one walk did, for the detour check.
#[derive(Default)]
struct WalkStats {
    /// Tiles moved (a ledge hop counts two, a warp or a connection one).
    moves: usize,
    /// Stops in front of an object that stood in (or stepped into) the next tile.
    blocked: usize,
    /// Waits for a turning trainer to face away.
    waits: usize,
    battles: usize,
    replans: usize,
}

/// Walk to any node satisfying `goal`, re-planning after every interruption.
///
/// Review round 1 (R1, `human-run/PROMPT.md`): a bump is a controller bug, so every one is a
/// `warn` event with `"bump": true` (counted per stage by [`bump_summary`]); a walked leg longer
/// than the shortest legal path at planning time is a `warn` with the reason.
pub fn go_to(
    s: &mut Session,
    what: &str,
    goal: &dyn Fn(&Node) -> bool,
    opts: &Opts,
) -> PResult<()> {
    // Cut trees are walkable only with a party mon that knows Cut (the field move menu lists it only
    // then, `decompiled/src/party_menu.c` `SetPartyMonFieldSelectionActions`): a carrier that can only
    // *learn* it (the Route 5 Meowth before `bike-redeem`) made the walker plan through Cerulean's
    // (26, 32) tree and press A at it forever (2026-11-27 fly1 replay).
    let no_cut;
    let opts = if opts.cut
        && !super::ram::Ram::new(&s.obs)
            .party(s.hf.rec.emu())
            .iter()
            .any(|m| m.knows(super::names::id("MOVE_CUT")).is_some())
    {
        no_cut = Opts {
            cut: false,
            ..opts.clone()
        };
        &no_cut
    } else {
        opts
    };
    // Tiles bumped on two earlier walks of this stage stay blocked (Victory Road 2F (33, 17) was
    // bumped 57 times by one hunt's pacing in the 11-22 canonical run, ~50 frames each).
    let mut temp_block: HashSet<Node> = s
        .bumped
        .iter()
        .filter(|(_, &c)| c >= 2)
        .map(|(n, _)| *n)
        .collect();
    let mut bumps = 0;
    let mut plans = 0;
    let mut refreshes = 0;
    let mut bike_tried: Option<MapId> = None;
    let mut held = 0u16;
    let mut came_from: Option<Node> = None;
    let mut stats = WalkStats::default();
    // (shortest legal moves, the first plan's moves, the first plan's reason) at planning time.
    let mut first: Option<(usize, usize, String)> = None;
    let mut warned_sight: HashSet<(MapId, u8)> = HashSet::new();
    loop {
        if held == 0 {
            settle(s)?;
        }
        if let Some(act) = super::early::field_guard_due(s)? {
            if held != 0 {
                stop(s)?;
                held = 0;
            }
            super::early::field_guard(s, act)?;
            continue;
        }
        let Some(cur) = here(s) else {
            return Err(PlayError::Fatal("no save block while walking".into()));
        };
        if goal(&cur) {
            if held != 0 {
                // Arrived on a refresh: let the step land.
                stop(s)?;
            }
            break;
        }
        // On the bike wherever the map allows it (`allow_cycling`) and the walk is more than a
        // few tiles: the human gets on with the registered SELECT (once per map per walk).
        if held == 0
            && bike_tried != Some(cur.0)
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
        if held == 0 {
            plans += 1;
        } else {
            refreshes += 1;
        }
        if std::env::var_os("FRLG_WALK_DEBUG").is_some() {
            eprintln!(
                "walk: {what}: plan {plans}/{refreshes} from {cur:?} at frame {}",
                s.frame()
            );
        }
        if plans > 60 || refreshes > 2000 {
            return Err(PlayError::Death(format!(
                "soft-lock: {what}: {plans} re-plans without arriving"
            )));
        }
        // A refresh never turns straight back to the tile just left unless nothing else goes
        // (two views that differ by one object flickering on the screen edge made the plans
        // alternate between two tiles: Viridian Gym, 2000 refreshes in place).
        let mut v = view(s, &temp_block);
        if let Some(b) = came_from.take() {
            v.temp_block.insert(b);
            let walls = Opts {
                cone_walls: true,
                ..opts.clone()
            };
            if s.graph().plan(&v, cur, goal, &walls).is_none() {
                v.temp_block.remove(&b);
            }
        }
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
        if first.is_none() {
            if let Some(p) = &live {
                first = Some(detour_check(s, what, &v, cur, goal, &walls, p));
            }
        }
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
            if held != 0 {
                stop(s)?;
                held = 0;
                continue;
            }
            if !temp_block.is_empty() {
                temp_block.clear();
                continue;
            }
            return Err(PlayError::Fatal(format!(
                "{what}: no path from {cur:?} ({})",
                s.graph().maps.get(&cur.0).map_or("?", |m| m.name.as_str())
            )));
        };
        // A path through an unbeaten trainer's line: a fight the walker cannot walk around (the
        // plan's forced trainers, `human-run/tools/forced.py`). Logged once per trainer.
        for (i, sn) in plan.seen.iter().enumerate() {
            let Some(id) = *sn else { continue };
            let n = plan.nodes[i];
            if !warned_sight.insert((n.0, id)) {
                continue;
            }
            let g = s.graph();
            let scr = g
                .maps
                .get(&n.0)
                .and_then(|m| m.objects.iter().find(|o| o.local_id == id))
                .map_or("?".to_string(), |o| o.script.clone());
            s.event(
                "decision",
                format!(
                    "walk: {what}: no way around the sight line of {scr} at ({}, {})",
                    n.1, n.2
                ),
                json!({"sight": scr}),
            );
        }
        if std::env::var("FRLG_WALK_DEBUG").is_ok_and(|v| v == "2") {
            let ns: Vec<String> = plan
                .nodes
                .iter()
                .map(|n| format!("({},{})", n.1, n.2))
                .collect();
            eprintln!("walk: path {}", ns.join(" "));
        }
        let r = follow(s, &plan, held, &mut stats, opts)?;
        held = 0;
        match r {
            Follow::Arrived => {}
            Follow::Replan => stats.replans += 1,
            Follow::Refresh(k, back) => {
                held = k;
                came_from = Some(back);
            }
            Follow::Bumped(n) => {
                bumps += 1;
                let mn = s
                    .graph()
                    .maps
                    .get(&n.0)
                    .map_or("?".to_string(), |m| m.name.clone());
                let seg = s.seg.clone();
                s.event(
                    "warn",
                    format!(
                        "bumped into something at {mn} ({}, {}); re-planning",
                        n.1, n.2
                    ),
                    json!({"bump": true, "stage": seg, "map": mn, "x": n.1, "y": n.2, "walk": what}),
                );
                temp_block.insert(n);
                let c = s.bumped.entry(n).or_insert(0);
                *c += 1;
                if *c >= 2 && goal(&n) && plan.nodes.last() == Some(&n) {
                    // The goal tile itself cannot be entered (twice now): the caller picks
                    // another (hunt pacing pairs) or the branch dies as before.
                    return Err(PlayError::Death(format!(
                        "soft-lock: {what}: goal tile bumped ({}, {})",
                        n.1, n.2
                    )));
                }
                if bumps > 40 {
                    return Err(PlayError::Death(format!("soft-lock: {what}: 40 bumps")));
                }
            }
        }
    }
    // The detour check (`PROMPT.md`, *No detours*): walked moves against the shortest legal path
    // when the walk was planned.
    if let Some((short, planned, why)) = first {
        if stats.moves > short.max(planned) {
            let mut r = vec![];
            if bumps > 0 {
                r.push(format!("{bumps} bumps"));
            }
            if stats.blocked > 0 {
                r.push(format!("{} stops for a person in the way", stats.blocked));
            }
            if stats.battles > 0 {
                r.push(format!("{} battles", stats.battles));
            }
            if stats.replans > 0 {
                r.push(format!("{} re-plans", stats.replans));
            }
            if !why.is_empty() {
                r.push(why);
            }
            if r.is_empty() {
                r.push("unexplained".into());
            }
            s.event(
                "warn",
                format!(
                    "detour: {what}: walked {} tiles, shortest legal {short} when planned ({})",
                    stats.moves,
                    r.join(", ")
                ),
                json!({"detour": stats.moves - short, "walked": stats.moves, "shortest": short}),
            );
        }
    }
    if stats.waits > 0 && std::env::var_os("FRLG_WALK_DEBUG").is_some() {
        eprintln!(
            "walk: {what}: {} waits for a trainer to face away",
            stats.waits
        );
    }
    Ok(())
}

/// Compare the plan `p` with the shortest legal path from `cur` (the same walls, no grass or turn
/// tie-breakers, no wait price); log a `warn` when the plan is longer, with the reason. Returns
/// (shortest moves, planned moves, reason).
fn detour_check(
    s: &mut Session,
    what: &str,
    v: &View,
    cur: Node,
    goal: &dyn Fn(&Node) -> bool,
    walls: &Opts,
    p: &Plan,
) -> (usize, usize, String) {
    let legal = Opts {
        steps_only: true,
        ..walls.clone()
    };
    let g = s.graph();
    let Some(q) = g.plan(v, cur, goal, &legal) else {
        return (p.tiles(), p.tiles(), String::new());
    };
    if p.tiles() <= q.tiles() {
        return (q.tiles(), p.tiles(), String::new());
    }
    let grass = |pl: &Plan| {
        pl.nodes
            .iter()
            .filter(|n| {
                g.maps
                    .get(&n.0)
                    .and_then(|m| m.tile(n.1, n.2))
                    .is_some_and(|t| t.land)
            })
            .count()
    };
    let watched = |pl: &Plan| pl.watch.iter().filter(|w| !w.is_empty()).count();
    let mut why = vec![];
    if grass(&q) > grass(p) {
        why.push(format!("avoids {} grass tiles", grass(&q) - grass(p)));
    }
    if watched(&q) > watched(p) {
        why.push(format!(
            "avoids {} tiles a turning trainer may see",
            watched(&q) - watched(p)
        ));
    }
    if why.is_empty() {
        why.push("fewer turns".into());
    }
    let why = why.join(", ");
    s.event(
        "warn",
        format!(
            "detour: {what}: planned {} tiles, shortest legal {} ({why})",
            p.tiles(),
            q.tiles()
        ),
        json!({"detour": p.tiles() - q.tiles(), "planned": p.tiles(),
               "shortest": q.tiles()}),
    );
    (q.tiles(), p.tiles(), why)
}

/// Bumps in an event log, in total and per stage (the stage a `bump` event names, or for older
/// logs the stage after the last checkpoint before it). Older controllers wrote "bumped
/// something ..." without the `bump` field; those count too.
pub fn bump_summary(events: &[super::events::Event]) -> (u32, Vec<(String, u32)>) {
    let mut per: Vec<(String, u32)> = vec![];
    let mut after = String::from("start");
    let mut total = 0;
    for e in events {
        if let Some(c) = e.nums.get("checkpoint").and_then(|v| v.as_str()) {
            after = format!("after {c}");
            continue;
        }
        if e.kind != "warn" || !e.text.contains("bumped") {
            continue;
        }
        total += 1;
        let st = e
            .nums
            .get("stage")
            .and_then(|v| v.as_str())
            .map_or(after.clone(), str::to_string);
        match per.iter_mut().find(|p| p.0 == st) {
            Some(p) => p.1 += 1,
            None => per.push((st, 1)),
        }
    }
    (total, per)
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

/// Let a step under way land: idle until the player is free (or a battle or script took over).
fn stop(s: &mut Session) -> PResult<()> {
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
    Ok(())
}

/// The live objects (map, local id), sorted: a change means someone came on screen or left.
fn live_set(s: &mut Session) -> Vec<(MapId, u8)> {
    let mut v: Vec<(MapId, u8)> = live_objects(s)
        .iter()
        .map(|l| (l.map, l.local_id))
        .collect();
    v.sort_unstable();
    v
}

/// The object in `(x, y)` of the current map right now: standing there, stepping into it, or
/// still leaving it (`DoesObjectCollideWithObjectAt` checks `currentCoords` and `previousCoords`,
/// `decompiled/src/event_object_movement.c:4899-4915`). `Some(true)` when it is only leaving.
fn occupant(s: &mut Session, x: i16, y: i16) -> Option<bool> {
    let mut leaving = None;
    for l in live_objects(s) {
        if (l.x, l.y) == (x, y) {
            return Some(false);
        }
        if (l.px, l.py) == (x, y) {
            leaving = Some(true);
        }
    }
    leaving
}

/// Does the trainer `l` see any of `tiles` from where it stands, the way it faces now
/// (`GetTrainerApproachDistance`, `decompiled/src/trainer_see.c:122-143`)?
fn sees_now(m: &MapInfo, l: &LiveObject, tiles: &[(i16, i16)]) -> bool {
    let mut line = vec![];
    match (l.trainer_type, dir_vec(l.facing)) {
        (1, Some(d)) => sight_line(m, l.x, l.y, d, l.range as i16, &mut line),
        (1, None) => {}
        _ => {
            for d in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
                sight_line(m, l.x, l.y, d, l.range as i16, &mut line);
            }
        }
    }
    line.iter().any(|t| tiles.contains(t))
}

fn c4_trace(s: &mut Session, tag: &str) {
    if std::env::var_os("FRLG_C4_TRACE").is_none() {
        return;
    }
    let f = s.frame();
    let p = here(s);
    let o: Vec<String> = live_objects(s)
        .iter()
        .filter(|l| l.trainer_type != 0)
        .map(|l| {
            format!(
                "{}:({},{})<-({},{})f{}",
                l.local_id, l.x, l.y, l.px, l.py, l.facing
            )
        })
        .collect();
    eprintln!("c4 {f} {tag} {p:?} {}", o.join(" "));
}

/// Is `to` inside the box of an unbeaten, on-screen trainer that turns to face a *running* player?
/// `LOOK_AROUND` and the `FACE_*_AND_*` types test `ObjectEventIsTrainerAndCloseToPlayer` while they
/// wait out a delay: with the dash flag set (B held while stepping) and the player's destination
/// within `trainerRange` tiles on both axes, the delay is skipped and the trainer turns toward the
/// player along the longer axis (`TryGetTrainerEncounterDirection`,
/// `decompiled/src/event_object_movement.c:2772-2799,2992-3015,3044-3064`). That is a sight line
/// whenever the player is in its row or column. Walking (no B) never triggers it. The plan's
/// `norun` is drawn when the walk is planned, before the trainer is on screen; this is live.
fn in_turn_box(s: &mut Session, to: Node) -> bool {
    if here(s).is_none_or(|n| n.0 != to.0) {
        return false;
    }
    let live = live_objects(s);
    let objs: Vec<ObjInfo> = s
        .graph()
        .maps
        .get(&to.0)
        .map(|m| m.objects.clone())
        .unwrap_or_default();
    for l in live {
        if l.map != to.0 || l.invisible || !matches!(l.trainer_type, 1 | 3) || l.range == 0 {
            continue;
        }
        let Some(o) = objs.iter().find(|o| o.local_id == l.local_id) else {
            continue;
        };
        let m = o.movement.as_str();
        let looks = m.contains("LOOK_AROUND")
            || (m.starts_with("MOVEMENT_TYPE_FACE_") && m.contains("_AND_"));
        if !looks {
            continue;
        }
        if o.beaten != 0 && s.obs.flag(s.hf.rec.emu(), o.beaten).unwrap_or(false) {
            continue;
        }
        let r = l.range as i16;
        if (to.1 - l.x).abs() <= r && (to.2 - l.y).abs() <= r {
            return true;
        }
    }
    false
}

/// One step of a trainer's predicted walk: it stands at `(x, y)` facing `dir` (a step's
/// destination coordinates and facing are set on its first frame) for `t0..t1` frames from now.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SeqPose {
    t0: i64,
    t1: i64,
    x: i16,
    y: i16,
    dir: (i16, i16),
}

/// Frames one NPC step takes (`MOVEMENT_ACTION_WALK_NORMAL_*`, one pixel a frame,
/// `decompiled/src/event_object_movement.c`); measured on the Mt. Moon Lass: her coordinates
/// change every 16 frames.
const NPC_STEP: i64 = 16;

/// Predict a `WALK_SEQUENCE_A_B_C_D` trainer for `horizon` frames from the state in RAM
/// (`MoveNextDirectionInSequence`, `decompiled/src/event_object_movement.c:3909-3935`, and the
/// per-variant `_Step1` that skips to leg 3 when the object is back on its initial row or
/// column, `:3945-4400`). Deterministic: no `Random()`. `next_start` is the frame the next step
/// begins. The player standing in its way is ignored (it would walk in place).
fn predict_sequence(
    o: &ObjInfo,
    m: &MapInfo,
    l: &LiveObject,
    next_start: i64,
    horizon: i64,
) -> Option<Vec<SeqPose>> {
    let dirs = o.sequence_dirs()?;
    let opposite = |a: (i16, i16), b: (i16, i16)| a.0 == -b.0 && a.1 == -b.1;
    // Legs 0 and 1 opposite (`RightLeftDownUp`): the skip is tested at leg 1, else at leg 2.
    let k = if opposite(dirs[0], dirs[1]) { 1 } else { 2 };
    let axis_x = dirs[k].0 != 0;
    let out_of_range = |x: i16, y: i16| {
        (o.range_x != 0 && (x - l.ix).abs() > o.range_x)
            || (o.range_y != 0 && (y - l.iy).abs() > o.range_y)
    };
    let blocked = |x: i16, y: i16| m.tile(x, y).is_none_or(|t| t.collision != 0);
    let (mut x, mut y) = (l.x, l.y);
    let mut idx = (l.seq_idx as usize).min(3);
    let mut out = vec![SeqPose {
        t0: -1000,
        t1: next_start,
        x,
        y,
        dir: dir_vec(l.facing).unwrap_or((0, 0)),
    }];
    let mut t = next_start;
    while t < horizon {
        if idx == k && (if axis_x { l.ix == x } else { l.iy == y }) {
            idx = k + 1;
        }
        if idx == 3 && (l.ix, l.iy) == (x, y) {
            idx = 0;
        }
        let mut d = dirs[idx];
        if out_of_range(x + d.0, y + d.1) {
            idx = (idx + 1).min(3);
            d = dirs[idx];
        }
        let walk = !out_of_range(x + d.0, y + d.1) && !blocked(x + d.0, y + d.1);
        if walk {
            x += d.0;
            y += d.1;
        }
        out.push(SeqPose {
            t0: t,
            t1: t + NPC_STEP,
            x,
            y,
            dir: d,
        });
        t += NPC_STEP;
    }
    Some(out)
}

/// Does a trainer standing at `p` (type `trainer_type`, sight `range`) see tile `t`?
fn pose_sees(m: &MapInfo, trainer_type: u8, range: i16, p: &SeqPose, t: (i16, i16)) -> bool {
    let mut line = vec![];
    if trainer_type == 1 {
        sight_line(m, p.x, p.y, p.dir, range, &mut line);
    } else {
        for d in [(0, -1), (0, 1), (-1, 0), (1, 0)] {
            sight_line(m, p.x, p.y, d, range, &mut line);
        }
    }
    line.contains(&t)
}

/// Is every tile of `tiles` unseen while the player is on it, if the player takes its first
/// step `d` frames from now (every `d` in `d_lo..=d_hi`) at `period` frames a tile? The player's
/// destination is the tile from the frame its step starts, and the sight check runs on frames it
/// can move (`ProcessPlayerFieldInput`, `decompiled/src/field_control_avatar.c:209`; the
/// destination: `PlayerGetDestCoords`, `trainer_see.c:127`), so tile `j` is exposed from its
/// step's start to the next step's start; one frame of slack on each side.
fn crossing_clear(
    m: &MapInfo,
    (trainer_type, range): (u8, i16),
    poses: &[SeqPose],
    tiles: &[(i16, i16)],
    (d_lo, d_hi): (i64, i64),
    period: i64,
) -> bool {
    for d in d_lo..=d_hi {
        for (j, &t) in tiles.iter().enumerate() {
            let a = d + j as i64 * period - 1;
            let b = d + (j as i64 + 1) * period + 1;
            for p in poses {
                if p.t1 > a && p.t0 < b && pose_sees(m, trainer_type, range, p, t) {
                    return false;
                }
            }
        }
    }
    true
}

/// Wait on the current tile until crossing `tiles` is safe from the trainer `id`.
///
/// A walk-sequence patroller (`MOVEMENT_TYPE_WALK_SEQUENCE_*`: Lass Miriam on Mt. Moon 1F) is
/// deterministic, so the human learns its loop by watching: the walker predicts its poses from
/// its RAM state (`predict_sequence`) and the step phase it has seen, and starts only when no
/// predicted sight line covers a tile while the player stands on it, over the whole window
/// of reaction time plus input jitter. (Waiting for one visible turn away, as turners are
/// handled below, is not enough for a walker: it turns back within the crossing.)
///
/// A wanderer or looker keeps a facing for at least 32 frames after a turn
/// (`gMovementDelaysMedium` {32, 64, 96, 128} after each turn, `MovementType_LookAround_Step2`,
/// `decompiled/src/event_object_movement.c:3033-3041`), and its sight is checked on every frame
/// the player has control. So the human waits for a *visible* turn or step away, reacts
/// (`U{15..30}`), and crosses while the new facing lasts. Returns the frames waited.
fn wait_away(s: &mut Session, map: MapId, id: u8, tiles: &[(i16, i16)]) -> PResult<u64> {
    let t0 = s.frame();
    let Some(m) = live_map(s, map).or_else(|| s.graph().maps.get(&map).cloned()) else {
        return Ok(0);
    };
    let info = s
        .graph()
        .maps
        .get(&map)
        .and_then(|g| g.objects.iter().find(|o| o.local_id == id).cloned());
    let find = |s: &mut Session| {
        live_objects(s)
            .into_iter()
            .find(|l| l.map == map && l.local_id == id)
    };
    let seq = info.as_ref().is_some_and(|o| o.sequence_dirs().is_some());
    let period: i64 = if run_bit(s) != 0 { 9 } else { 17 };
    let (rlo, rhi) = (s.hf.model.react_lo as i64, s.hf.model.react_hi as i64);
    let jitter = s.hf.model.jitter_n as i64;
    let mut prev: Option<(i16, i16, u8)> = None;
    // The frame the trainer's coordinates or facing last changed: a walker's step began then.
    let mut changed: Option<u64> = None;
    for _ in 0..1800 {
        if s.obs.in_battle(s.hf.rec.emu()) || !is_free(s) {
            return Ok(s.frame() - t0);
        }
        let Some(l) = find(s) else {
            return Ok(s.frame() - t0);
        };
        let now = (l.x, l.y, l.facing);
        let turned = prev.is_some_and(|p| p != now);
        if turned {
            changed = Some(s.frame());
        }
        prev = Some(now);
        c4_trace(s, "wait");
        if seq {
            if let (Some(o), Some(c)) = (info.as_ref(), changed) {
                let el = (s.frame() - c) as i64;
                let next = NPC_STEP - el % NPC_STEP;
                if let Some(poses) = predict_sequence(o, &m, &l, next, 1200) {
                    if crossing_clear(
                        &m,
                        (l.trainer_type, l.range as i16),
                        &poses,
                        tiles,
                        (rlo, rhi + jitter + 1),
                        period,
                    ) {
                        s.hf.react()?;
                        return Ok(s.frame() - t0);
                    }
                }
            }
            s.hf.step(0)?;
            continue;
        }
        if turned && !sees_now(&m, &l, tiles) {
            s.hf.react()?;
            match find(s) {
                Some(l2) if sees_now(&m, &l2, tiles) => {
                    // Turned back while the human reacted: keep watching.
                    prev = Some((l2.x, l2.y, l2.facing));
                    continue;
                }
                _ => return Ok(s.frame() - t0),
            }
        }
        s.hf.step(0)?;
    }
    s.event(
        "warn",
        format!("waited 1800 frames for trainer {id} to face away; crossing anyway"),
        json!({}),
    );
    Ok(s.frame() - t0)
}

/// Is the rest of `plan` (from node `k`) still good with the live objects as they are now: no
/// node blocked by an object, inside a sight line or a turning trainer's watch it did not have
/// when planned?
fn still_good(s: &mut Session, plan: &Plan, k: usize, opts: &Opts) -> bool {
    let v = view(s, &HashSet::new());
    let g = s.graph();
    let Some(m) = v
        .cur_live
        .clone()
        .or_else(|| g.maps.get(&v.cur_map).cloned())
    else {
        return false;
    };
    let b = g.blocked(&v, v.cur_map, &m, opts);
    for j in k..plan.nodes.len() {
        let n = plan.nodes[j];
        if n.0 != v.cur_map {
            continue;
        }
        let t = (n.1, n.2);
        if b.b.contains(&t)
            || (b.sight.contains_key(&t) && plan.seen[j].is_none())
            || (b.watch.contains_key(&t) && plan.watch[j].is_empty())
        {
            if std::env::var_os("FRLG_WALK_DEBUG").is_some() {
                eprintln!(
                    "walk: plan spoiled at {t:?}: object {} sight {:?} watch {:?}",
                    b.b.contains(&t),
                    b.sight.get(&t),
                    b.watch.get(&t)
                );
            }
            return false;
        }
    }
    true
}

fn follow(
    s: &mut Session,
    plan: &Plan,
    held: u16,
    stats: &mut WalkStats,
    opts: &Opts,
) -> PResult<Follow> {
    let mut last_key = held;
    let mut known = plan.live.clone();
    for (i, mv) in plan.moves.iter().enumerate() {
        let from = plan.nodes[i];
        let to = plan.nodes[i + 1];
        if here(s) != Some(from) {
            return Ok(Follow::Replan);
        }
        // Entering a turning trainer's possible line: stop short of it and wait for the trainer
        // to face away (review item 17, Lass Robin).
        let fresh: Vec<u8> = plan.watch[i + 1]
            .iter()
            .filter(|t| !plan.watch[i].contains(t))
            .copied()
            .collect();
        if !fresh.is_empty() && matches!(mv, Move::Step(_)) && to.0 == from.0 {
            stop(s)?;
            if here(s) != Some(from) || !is_free(s) {
                settle(s)?;
                return Ok(Follow::Replan);
            }
            for id in fresh {
                let tiles: Vec<(i16, i16)> = plan.nodes[i + 1..]
                    .iter()
                    .zip(&plan.watch[i + 1..])
                    .take_while(|(n, w)| n.0 == from.0 && w.contains(&id))
                    .map(|(n, _)| (n.1, n.2))
                    .collect();
                let waited = wait_away(s, from.0, id, &tiles)?;
                stats.waits += 1;
                let g = s.graph();
                let scr = g
                    .maps
                    .get(&from.0)
                    .and_then(|m| m.objects.iter().find(|o| o.local_id == id))
                    .map_or("?".to_string(), |o| o.script.clone());
                s.event(
                    "decision",
                    format!(
                        "waited {waited} frames at ({}, {}) for {scr} to face away",
                        from.1, from.2
                    ),
                    json!({"wait": waited, "trainer": scr}),
                );
                if s.obs.in_battle(s.hf.rec.emu()) {
                    settle(s)?;
                    stats.battles += 1;
                    return Ok(Follow::Replan);
                }
            }
            // The reaction already happened: step at once.
            last_key = match mv {
                Move::Step(d) => *d,
                _ => 0,
            };
        }
        match *mv {
            Move::Cut(dir) | Move::Surf(dir) => {
                stop(s)?;
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
                stats.moves += 1;
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
                let mut total = 0;
                loop {
                    let n = here(s);
                    // A warp to the same map (the Saffron gym's pads, `SaffronCity_Gym/map.json`
                    // warps 3-32) only moves the player: wait for any change of node.
                    if n.is_some_and(|n| n != from) || s.obs.in_battle(s.hf.rec.emu()) {
                        break;
                    }
                    // Only frames on which the player could move and did not count: once the
                    // warp fires, the fade and the map load hold the controls for 60-200 frames
                    // (27 of the v4 run's "bumps" were warps still loading at the old 90-frame
                    // cut-off).
                    if fed > 90 || total > 2000 {
                        return Ok(Follow::Bumped(to));
                    }
                    let emu = s.hf.rec.emu();
                    let can = s.obs.player_can_step(emu) && !s.obs.field_controls_locked(emu);
                    s.hf.step(if can { key } else { 0 })?;
                    total += 1;
                    if can {
                        fed += 1;
                    }
                }
                settle(s)?;
                stats.moves += 1;
                return Ok(Follow::Replan);
            }
            Move::Step(dir) => {
                if dir != last_key {
                    // A change of direction (or a start): the human's thumb is late by U{0..N}.
                    s.hf.late()?;
                }
                last_key = dir;
                let mut fed = 0;
                let mut waited = 0;
                // Across a connection the map switches first and the player stands one tile
                // outside the new map (in its coordinates) until the step lands.
                let (dx, dy) = match dir {
                    keys::UP => (0, -1),
                    keys::DOWN => (0, 1),
                    keys::LEFT => (-1, 0),
                    _ => (1, 0),
                };
                let alias = (to.0 != from.0).then_some((to.0, to.1 - dx, to.2 - dy));
                loop {
                    let emu = s.hf.rec.emu();
                    if s.obs.in_battle(emu) {
                        settle(s)?;
                        stats.battles += 1;
                        return Ok(Follow::Replan);
                    }
                    let n = here(s);
                    if n != Some(from) && (alias.is_none() || n != alias) {
                        if n == Some(to) {
                            break;
                        }
                        // A spin-tile slide (`to` several tiles away) passes through the tiles
                        // between: let it run out, then see whether it ended where planned.
                        if step_tiles(from, to) > 1 && n.is_some_and(|n| n.0 == from.0) {
                            let mut k = 0;
                            while !is_free(s) && k < 1200 && !s.obs.in_battle(s.hf.rec.emu()) {
                                s.hf.step(0)?;
                                k += 1;
                            }
                            if here(s) == Some(to) {
                                // The slide stopped the player: the next press is a new one.
                                last_key = 0;
                                break;
                            }
                        }
                        // A map change through a connection or warp, or a forced move.
                        if std::env::var_os("FRLG_WALK_DEBUG").is_some() {
                            eprintln!("walk: at {n:?}, expected {from:?} -> {to:?}");
                        }
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
                    // The tile ahead must be free when the step starts: someone standing or
                    // stepping there turns the step into a bump. Someone only leaving it is
                    // waited out (a few frames); someone in it is planned around.
                    if to.0 == from.0 {
                        match occupant(s, to.1, to.2) {
                            Some(true) if waited < 24 => {
                                waited += 1;
                                s.hf.step(0)?;
                                continue;
                            }
                            Some(_) => {
                                stats.blocked += 1;
                                stop(s)?;
                                settle(s)?;
                                return Ok(Follow::Replan);
                            }
                            None => {}
                        }
                    }
                    // Into a spinner's box: walk (see `Blk::norun`).
                    let b = if plan.norun[i + 1] || in_turn_box(s, to) {
                        0
                    } else {
                        run_bit(s)
                    };
                    c4_trace(s, "go");
                    s.hf.step(dir | b)?;
                    fed += 1;
                }
                stats.moves += step_tiles(from, to);
                // The step into `to` has started. Someone came on screen or left: plan again
                // from `to` while the step runs (no frame passes while planning).
                // Only a change that spoils the rest of the plan re-plans: alternating between
                // two near-equal paths on every flicker of the object set made the walker
                // circle the Viridian Gym spinners.
                if i + 1 < plan.moves.len() {
                    let now = live_set(s);
                    if now != known {
                        known = now;
                        if !still_good(s, plan, i + 2, opts) {
                            return Ok(Follow::Refresh(dir, from));
                        }
                    }
                }
            }
        }
    }
    // Let the last step land.
    stop(s)?;
    Ok(Follow::Arrived)
}

#[cfg(test)]
mod patrol_tests {
    use super::*;

    /// Lass Miriam (`MtMoon_1F/map.json`: spawn (33, 4), `WALK_SEQUENCE_LEFT_DOWN_RIGHT_UP`, range
    /// 5 x 3, sight 2) on an open 40 x 12 floor: her loop is 16 steps, and she faces down on
    /// (28, 7) for one step only, the only time she sees row 9.
    #[test]
    fn miriam_loop_and_crossing_window() {
        let t = Tile {
            collision: 0,
            behavior: 0,
            land: false,
            elevation: 3,
        };
        let o = ObjInfo {
            x: 33,
            y: 4,
            local_id: 4,
            movement: "MOVEMENT_TYPE_WALK_SEQUENCE_LEFT_DOWN_RIGHT_UP".into(),
            script: String::new(),
            graphics: String::new(),
            flag: 0,
            trainer: true,
            sight: 2,
            beaten: 0,
            range_x: 5,
            range_y: 3,
        };
        let m = MapInfo {
            name: String::new(),
            id: String::new(),
            w: 40,
            h: 12,
            tiles: vec![t; 40 * 12],
            warps: vec![],
            conns: vec![],
            objects: vec![o.clone()],
            bike: false,
        };
        // Just stepped left from (33, 4) to (32, 4): index 0, facing west.
        let l = LiveObject {
            local_id: 4,
            x: 32,
            y: 4,
            graphics: 0,
            px: 33,
            py: 4,
            facing: 3,
            movement: 0,
            trainer_type: 1,
            range: 2,
            map: (0, 0),
            elevation: 3,
            invisible: false,
            ix: 33,
            iy: 4,
            seq_idx: 0,
        };
        let poses = predict_sequence(&o, &m, &l, 16, 16 * 30).unwrap();
        // Steps from (32, 4): 4 left to x=28, 3 down to y=7, 5 right to x=33, 3 up to y=4.
        let at = |i: usize| (poses[i].x, poses[i].y, poses[i].dir);
        assert_eq!(at(4), (28, 4, (-1, 0)));
        assert_eq!(at(7), (28, 7, (0, 1)));
        assert_eq!(at(8), (29, 7, (1, 0)));
        assert_eq!(at(12), (33, 7, (1, 0)));
        assert_eq!(at(15), (33, 4, (0, -1)));
        assert_eq!(at(16), (32, 4, (-1, 0)));
        // She sees (28, 9) only while she walks onto (28, 7), once per 16-step loop.
        let seen: Vec<usize> = (0..poses.len())
            .filter(|&i| pose_sees(&m, 1, 2, &poses[i], (28, 9)))
            .collect();
        assert_eq!(seen, vec![7, 23]);
        // A start 15-40 frames from now is in that step's window (112..128): not clear.
        assert!(!crossing_clear(
            &m,
            (1, 2),
            &poses,
            &[(28, 9)],
            (60, 120),
            9
        ));
        // Right after it, up to the next loop's (368..384): clear.
        assert!(crossing_clear(
            &m,
            (1, 2),
            &poses,
            &[(28, 9)],
            (130, 340),
            9
        ));
    }
}
