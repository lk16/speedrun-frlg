//! The overworld as data: collision, encounter tiles, behaviors, objects,
//! warps -- decoded from the decomp checkout's own files rather than poked
//! out of the emulator tile by tile.
//!
//! Sources, all under `$FRLG_DECOMP` (falling back to `$FRLG_DECOMP_RO`):
//!
//! - `data/maps/map_groups.json` -- the (group, num) -> map name table the
//!   save block's `location` indexes (`include/global.h:392`).
//! - `data/maps/<Name>/map.json` -- layout id, warps, object events.
//! - `data/layouts/layouts.json` -- width/height and `map.bin` path per
//!   layout.
//! - `data/layouts/<L>/map.bin` -- one little-endian u16 per tile:
//!   metatile id bits 0-9, collision 10-11, elevation 12-15
//!   (`include/global.fieldmap.h:7-11`).
//! - `data/tilesets/{primary,secondary}/<t>/metatile_attributes.bin` -- one
//!   u32 per metatile: behavior bits 0-8, encounter type bits 24-26
//!   (`src/fieldmap.c:61-83`). Metatile ids below
//!   `NUM_METATILES_IN_PRIMARY` (640, `include/fieldmap.h:8`) index the
//!   primary tileset's attributes, the rest the secondary's.
//!
//! The tileset-name -> attributes-file mapping is parsed from
//! `src/data/tilesets/headers.h` (`.metatileAttributes = gMetatileAttributes_X`)
//! and `src/data/tilesets/metatiles.h` (`INCBIN_U32("data/tilesets/...")`),
//! so it tracks the decomp rather than a transcription.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// `TILE_ENCOUNTER_LAND` (`include/global.fieldmap.h:41`).
pub const ENCOUNTER_LAND: u8 = 1;

/// Ledge-hop behaviors (`include/constants/metatile_behaviors.h:47-50`).
/// Walking into one from its jump side hops the player two tiles; from any
/// other side it is a wall.
pub const MB_JUMP_EAST: u16 = 0x38;
pub const MB_JUMP_WEST: u16 = 0x39;
pub const MB_JUMP_NORTH: u16 = 0x3A;
pub const MB_JUMP_SOUTH: u16 = 0x3B;

/// Directional walls (`include/constants/metatile_behaviors.h:39-46`):
/// passable except from the named side(s).
pub const MB_IMPASSABLE_EAST: u16 = 0x30;
pub const MB_IMPASSABLE_SOUTHWEST: u16 = 0x37;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tile {
    /// `MAPGRID_COLLISION_MASK >> 10`: 0 is walkable.
    pub collision: u8,
    /// Metatile attribute bits 0-8.
    pub behavior: u16,
    /// Metatile attribute bits 24-26: 1 = land encounters roll here.
    pub land: bool,
}

/// One `object_events` entry of a map.json, reduced to what routing needs.
#[derive(Debug, Clone)]
pub struct MapObject {
    pub x: i16,
    pub y: i16,
    /// `MOVEMENT_TYPE_*` verbatim.
    pub movement_type: String,
    pub range_x: i16,
    pub range_y: i16,
    /// `TRAINER_TYPE_*` verbatim; `TRAINER_TYPE_NORMAL` means a sight line.
    pub trainer_type: String,
    /// For trainers: sight range in tiles.
    pub sight: i16,
}

impl MapObject {
    /// The facing directions this object can end up in, as (dx, dy) unit
    /// steps -- derived from its movement type. Wanderers and rotators get
    /// all four.
    pub fn facings(&self) -> Vec<(i16, i16)> {
        match self.movement_type.as_str() {
            "MOVEMENT_TYPE_FACE_UP" => vec![(0, -1)],
            "MOVEMENT_TYPE_FACE_DOWN" => vec![(0, 1)],
            "MOVEMENT_TYPE_FACE_LEFT" => vec![(-1, 0)],
            "MOVEMENT_TYPE_FACE_RIGHT" => vec![(1, 0)],
            _ => vec![(0, -1), (0, 1), (-1, 0), (1, 0)],
        }
    }

    /// Whether this object stays on its home tile (no wander range).
    pub fn stationary(&self) -> bool {
        self.movement_type.starts_with("MOVEMENT_TYPE_FACE_")
            || self.movement_type.starts_with("MOVEMENT_TYPE_ROTATE_")
            || (self.range_x <= 1 && self.range_y <= 1)
    }
}

#[derive(Debug, Clone)]
pub struct Warp {
    pub x: i16,
    pub y: i16,
    pub dest_map: String,
}

pub struct MapData {
    pub name: String,
    pub width: usize,
    pub height: usize,
    tiles: Vec<Tile>,
    pub objects: Vec<MapObject>,
    pub warps: Vec<Warp>,
    /// `coord_events` tiles -- stepping on one may fire a script.
    pub coord_events: Vec<(i16, i16)>,
}

impl MapData {
    pub fn tile(&self, x: i16, y: i16) -> Option<Tile> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        Some(self.tiles[x as usize + y as usize * self.width])
    }

    /// ASCII rendering: `#` blocked, `,` grass (land encounter), `.` open,
    /// `J` ledge, `W` warp, `T` object. The forest decode that used to be a
    /// one-off (`docs/defeat-brock/research/forest-map.txt`) is this.
    pub fn ascii(&self) -> String {
        let mut out = String::new();
        for y in 0..self.height as i16 {
            for x in 0..self.width as i16 {
                let t = self.tile(x, y).unwrap();
                // Ledge tiles carry collision 1 in the grid -- the jump
                // logic overrides it in the jump direction only -- so the
                // behavior is checked first or every ledge prints as wall.
                let mut c = if (MB_JUMP_EAST..=MB_JUMP_SOUTH).contains(&t.behavior) {
                    'J'
                } else if t.collision != 0 {
                    '#'
                } else if t.land {
                    ','
                } else {
                    '.'
                };
                if self.warps.iter().any(|w| (w.x, w.y) == (x, y)) {
                    c = 'W';
                }
                if self.objects.iter().any(|o| (o.x, o.y) == (x, y)) {
                    c = 'T';
                }
                out.push(c);
            }
            out.push('\n');
        }
        out
    }
}

/// The decomp checkout's map database, loaded lazily per map.
pub struct World {
    root: PathBuf,
    /// group -> list of map names, from map_groups.json's `group_order`.
    groups: Vec<Vec<String>>,
    /// layout id -> (width, height, map.bin path, primary tileset, secondary).
    layouts: HashMap<String, (usize, usize, String, String, String)>,
    /// tileset symbol (`gTileset_X`) -> metatile_attributes.bin path.
    attr_paths: HashMap<String, String>,
    /// attributes cache, keyed by file path.
    attrs: HashMap<String, Vec<u32>>,
    maps: HashMap<(u8, u8), MapData>,
}

fn decomp_root() -> Result<PathBuf, String> {
    for var in ["FRLG_DECOMP", "FRLG_DECOMP_RO"] {
        if let Some(v) = std::env::var_os(var) {
            let p = PathBuf::from(v);
            if p.join("data/maps/map_groups.json").exists() {
                return Ok(p);
            }
        }
    }
    Err("neither $FRLG_DECOMP nor $FRLG_DECOMP_RO points at a decomp checkout".into())
}

fn read_json(path: &Path) -> Result<serde_json::Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

impl World {
    pub fn load() -> Result<Self, String> {
        let root = decomp_root()?;
        let groups_json = read_json(&root.join("data/maps/map_groups.json"))?;
        let order = groups_json["group_order"]
            .as_array()
            .ok_or("map_groups.json has no group_order")?;
        let groups = order
            .iter()
            .map(|g| {
                groups_json[g.as_str().unwrap_or_default()]
                    .as_array()
                    .map(|maps| {
                        maps.iter()
                            .filter_map(|m| m.as_str().map(str::to_owned))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect();

        let layouts_json = read_json(&root.join("data/layouts/layouts.json"))?;
        let mut layouts = HashMap::new();
        for l in layouts_json["layouts"].as_array().into_iter().flatten() {
            // A handful of entries are null padding in some checkouts.
            let (Some(id), Some(w), Some(h), Some(bin), Some(prim), Some(sec)) = (
                l["id"].as_str(),
                l["width"].as_u64(),
                l["height"].as_u64(),
                l["blockdata_filepath"].as_str(),
                l["primary_tileset"].as_str(),
                l["secondary_tileset"].as_str(),
            ) else {
                continue;
            };
            layouts.insert(
                id.to_owned(),
                (
                    w as usize,
                    h as usize,
                    bin.to_owned(),
                    prim.to_owned(),
                    sec.to_owned(),
                ),
            );
        }

        // `gTileset_X` -> `gMetatileAttributes_Y` (headers.h), then
        // `gMetatileAttributes_Y` -> bin path (metatiles.h).
        let headers = fs::read_to_string(root.join("src/data/tilesets/headers.h"))
            .map_err(|e| format!("tilesets/headers.h: {e}"))?;
        let metatiles = fs::read_to_string(root.join("src/data/tilesets/metatiles.h"))
            .map_err(|e| format!("tilesets/metatiles.h: {e}"))?;
        let mut attr_syms = HashMap::new();
        let mut current = String::new();
        for line in headers.lines() {
            if let Some(rest) = line.strip_prefix("const struct Tileset ") {
                current = rest.trim_end_matches([' ', '=']).to_owned();
            } else if let Some(idx) = line.find(".metatileAttributes = ") {
                let sym = line[idx + ".metatileAttributes = ".len()..]
                    .trim_end_matches([',', ' '])
                    .to_owned();
                attr_syms.insert(current.clone(), sym);
            }
        }
        let mut sym_paths = HashMap::new();
        for line in metatiles.lines() {
            if let Some(idx) = line.find("gMetatileAttributes_") {
                let sym = line[idx..].split('[').next().unwrap_or_default().to_owned();
                if let Some(q) = line.find("INCBIN_U32(\"") {
                    let rest = &line[q + "INCBIN_U32(\"".len()..];
                    if let Some(end) = rest.find('"') {
                        sym_paths.insert(sym, rest[..end].to_owned());
                    }
                }
            }
        }
        let attr_paths = attr_syms
            .into_iter()
            .filter_map(|(tileset, sym)| sym_paths.get(&sym).map(|p| (tileset, p.clone())))
            .collect();

        Ok(World {
            root,
            groups,
            layouts,
            attr_paths,
            attrs: HashMap::new(),
            maps: HashMap::new(),
        })
    }

    pub fn map_name(&self, map: (u8, u8)) -> Option<&str> {
        self.groups
            .get(map.0 as usize)?
            .get(map.1 as usize)
            .map(String::as_str)
    }

    fn attributes(&mut self, path: &str) -> Result<&[u32], String> {
        if !self.attrs.contains_key(path) {
            let bytes = fs::read(self.root.join(path)).map_err(|e| format!("{path}: {e}"))?;
            let words = bytes
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            self.attrs.insert(path.to_owned(), words);
        }
        Ok(&self.attrs[path])
    }

    pub fn map(&mut self, map: (u8, u8)) -> Result<&MapData, String> {
        if !self.maps.contains_key(&map) {
            let data = self.load_map(map)?;
            self.maps.insert(map, data);
        }
        Ok(&self.maps[&map])
    }

    fn load_map(&mut self, map: (u8, u8)) -> Result<MapData, String> {
        let name = self
            .map_name(map)
            .ok_or_else(|| format!("no map at group {} num {}", map.0, map.1))?
            .to_owned();
        let json = read_json(&self.root.join(format!("data/maps/{name}/map.json")))?;
        let layout_id = json["layout"]
            .as_str()
            .ok_or_else(|| format!("{name}: no layout"))?;
        let (width, height, bin_path, prim, sec) = self
            .layouts
            .get(layout_id)
            .ok_or_else(|| format!("{name}: unknown layout {layout_id}"))?
            .clone();

        let prim_path = self
            .attr_paths
            .get(&prim)
            .ok_or_else(|| format!("{name}: no attributes for {prim}"))?
            .clone();
        let sec_path = self
            .attr_paths
            .get(&sec)
            .ok_or_else(|| format!("{name}: no attributes for {sec}"))?
            .clone();
        /// `NUM_METATILES_IN_PRIMARY` (`include/fieldmap.h:8`).
        const PRIMARY_COUNT: usize = 640;
        let prim_attrs = self.attributes(&prim_path)?.to_vec();
        let sec_attrs = self.attributes(&sec_path)?.to_vec();

        let bin = fs::read(self.root.join(&bin_path)).map_err(|e| format!("{bin_path}: {e}"))?;
        if bin.len() != width * height * 2 {
            return Err(format!(
                "{bin_path}: {} bytes for {width}x{height}",
                bin.len()
            ));
        }
        let tiles = bin
            .chunks_exact(2)
            .map(|c| {
                let grid = u16::from_le_bytes([c[0], c[1]]);
                let metatile = (grid & 0x03FF) as usize;
                let attr = if metatile < PRIMARY_COUNT {
                    prim_attrs.get(metatile).copied().unwrap_or(0)
                } else {
                    sec_attrs
                        .get(metatile - PRIMARY_COUNT)
                        .copied()
                        .unwrap_or(0)
                };
                Tile {
                    collision: ((grid >> 10) & 3) as u8,
                    behavior: (attr & 0x1FF) as u16,
                    land: ((attr >> 24) & 7) as u8 == ENCOUNTER_LAND,
                }
            })
            .collect();

        let str_of = |v: &serde_json::Value, key: &str| -> String {
            v[key].as_str().unwrap_or_default().to_owned()
        };
        let int_of = |v: &serde_json::Value, key: &str| -> i16 {
            v[key]
                .as_i64()
                .or_else(|| v[key].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(0) as i16
        };
        let objects = json["object_events"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|o| MapObject {
                x: int_of(o, "x"),
                y: int_of(o, "y"),
                movement_type: str_of(o, "movement_type"),
                range_x: int_of(o, "movement_range_x"),
                range_y: int_of(o, "movement_range_y"),
                trainer_type: str_of(o, "trainer_type"),
                sight: int_of(o, "trainer_sight_or_berry_tree_id"),
            })
            .collect();
        let warps = json["warp_events"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|w| Warp {
                x: int_of(w, "x"),
                y: int_of(w, "y"),
                dest_map: str_of(w, "dest_map"),
            })
            .collect();

        let coord_events = json["coord_events"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|c| (int_of(c, "x"), int_of(c, "y")))
            .collect();

        Ok(MapData {
            name,
            width,
            height,
            tiles,
            objects,
            warps,
            coord_events,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The forest grid against the committed hand-decoded reference
    /// (`docs/defeat-brock/research/forest-map.txt`): same walls, same grass,
    /// on the rows the reference shows.
    #[test]
    fn forest_matches_the_committed_decode() {
        let mut world = match World::load() {
            Ok(w) => w,
            // No decomp mounted (plain `cargo test` on a dev box): nothing
            // to check against.
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };
        let forest = world.map((1, 0)).expect("Viridian Forest decodes");
        assert_eq!(forest.name, "ViridianForest");
        assert_eq!((forest.width, forest.height), (54, 69));

        let reference = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/defeat-brock/research/forest-map.txt"
        ));
        for line in reference.lines().skip(1) {
            let Some((y, row)) = line.split_at_checked(3).map(|(n, r)| (n.trim(), r)) else {
                continue;
            };
            let Ok(y) = y.parse::<i16>() else { continue };
            for (x, ch) in row.chars().enumerate() {
                let t = forest.tile(x as i16, y).unwrap();
                match ch {
                    '#' => assert_ne!(t.collision, 0, "({x},{y}) walkable but reference walls it"),
                    ',' => assert!(
                        t.collision == 0 && t.land,
                        "({x},{y}) expected grass, got {t:?}"
                    ),
                    '.' => assert!(
                        t.collision == 0 && !t.land,
                        "({x},{y}) expected open, got {t:?}"
                    ),
                    _ => {}
                }
            }
        }

        // Sammy's committed sight row: a trainer object stands in column 1's
        // corridor (`research/story-gates.md`).
        assert!(forest
            .objects
            .iter()
            .any(|o| o.trainer_type == "TRAINER_TYPE_NORMAL"));
    }

    #[test]
    fn route1_decodes_and_has_grass() {
        let mut world = match World::load() {
            Ok(w) => w,
            Err(e) => {
                eprintln!("skipping: {e}");
                return;
            }
        };
        let r1 = world.map((3, 19)).expect("Route 1 decodes");
        assert_eq!(r1.name, "Route1");
        let grass = (0..r1.height as i16)
            .flat_map(|y| (0..r1.width as i16).map(move |x| (x, y)))
            .filter(|&(x, y)| r1.tile(x, y).is_some_and(|t| t.land && t.collision == 0))
            .count();
        // research/story-gates.md: >= 20 forced grass tiles on the way north.
        assert!(grass >= 20, "Route 1 shows {grass} grass tiles");
    }
}
