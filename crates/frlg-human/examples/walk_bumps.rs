//! Review round 1 (R1): look at every bump event of a played run where it happened.
//!
//! Replays `<run>/run.ilog` from power-on and, for every `warn` event whose text says "bumped",
//! prints the scene 50 frames before the event (the walker gives up after 48 frames of a held
//! direction without a move, `play/walk.rs` `STEP_BUDGET`): the player's tile, facing and
//! elevation, the four neighbour tiles (live collision from `VMap`, layout behavior, elevation),
//! every live object event (current and previous coordinates, facing, movement type, map), and
//! where the player stood 200 frames before and 400 frames after. Optionally saves a savestate
//! 300 frames before each bump into `<out-dir>` for the walker regression set.
//!
//!     cargo run --release --example walk_bumps -- <rom> <sym> <run-dir> [<out-dir>]
use std::path::Path;

use frlg_emu::{Emu, InputLog, SymbolTable};
use frlg_human::play::ram::Ram;
use frlg_route::world::World;
use frlg_route::Observer;

fn vmap_tile(emu: &mut Emu, vmap: u32, x: i16, y: i16) -> Option<(u8, u8, u16)> {
    let xsize = emu.read32(vmap) as i32;
    let ysize = emu.read32(vmap + 4) as i32;
    let ptr = emu.read32(vmap + 8);
    let (vx, vy) = (x as i32 + 7, y as i32 + 7);
    if vx < 0 || vy < 0 || vx >= xsize || vy >= ysize {
        return None;
    }
    let v = emu.read16(ptr + ((vy * xsize + vx) * 2) as u32);
    Some((((v >> 10) & 3) as u8, (v >> 12) as u8, v & 0x3FF))
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 3 {
        eprintln!("usage: walk_bumps <rom> <sym> <run-dir> [<out-dir>]");
        std::process::exit(2);
    }
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let obs = Observer::new(syms).expect("observer");
    let vmap = obs.symbols().get("VMap").expect("VMap").addr;
    let dir = Path::new(&a[2]);
    let out = a.get(3).map(Path::new);
    if let Some(o) = out {
        std::fs::create_dir_all(o).expect("out dir");
    }
    let log = InputLog::decode(&std::fs::read(dir.join("run.ilog")).expect("ilog")).expect("ilog");
    let mut bumps: Vec<(u64, String)> = vec![];
    for l in std::fs::read_to_string(dir.join("events.jsonl"))
        .expect("events")
        .lines()
    {
        let v: serde_json::Value = serde_json::from_str(l).unwrap();
        let t = v["text"].as_str().unwrap_or_default();
        if v["kind"] == "warn" && t.contains("bumped") {
            bumps.push((v["frame"].as_u64().unwrap(), t.to_string()));
        }
    }
    let mut world = World::load().expect("world");
    let mut emu = Emu::new(Path::new(&a[0])).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    // Frames at which to look: (frame, bump index, what).
    let mut looks: Vec<(u64, usize, &str)> = vec![];
    for (i, (f, _)) in bumps.iter().enumerate() {
        // The latest frame between 1260 and 60 frames before the bump where the player stands
        // free on the field (a state cut mid-battle or mid-warp cannot start a walk).
        for k in (0..30).rev() {
            looks.push((f.saturating_sub(60 + 40 * k), i, "state"));
        }
        looks.push((f.saturating_sub(200), i, "before"));
        looks.push((f.saturating_sub(50), i, "scene"));
        looks.push((f + 400, i, "after"));
    }
    looks.sort();
    let mut at = 0u64;
    let mut before = vec![String::new(); bumps.len()];
    let mut state_at = vec![String::from("none"); bumps.len()];
    let mut scene = vec![String::new(); bumps.len()];
    let mut after = vec![String::new(); bumps.len()];
    for (f, i, what) in looks {
        while at < f && (at as usize) < log.frames.len() {
            emu.step(log.frames[at as usize]);
            at += 1;
        }
        let map = obs.map(&mut emu).unwrap_or((0, 0));
        let pos = obs.pos(&mut emu).unwrap_or((0, 0));
        let name = world.map_name(map).unwrap_or("?").to_string();
        let ram = Ram::new(&obs);
        match what {
            "state" => {
                let free = !obs.in_battle(&mut emu)
                    && obs.callback2_is(&mut emu, "CB2_Overworld")
                    && !obs.field_controls_locked(&mut emu)
                    && obs.player_can_step(&mut emu);
                if let (Some(o), true) = (out, free) {
                    let p = o.join(format!("bump{i:02}.state"));
                    emu.save_state_file(&p).expect("save");
                    state_at[i] = format!("{f} {name} {pos:?}");
                }
            }
            "before" => before[i] = format!("{name} {pos:?}"),
            "after" => after[i] = format!("{name} {pos:?}"),
            _ => {
                let mut s = String::new();
                let face = ram.player_facing(&mut emu);
                let el = ram.player_elevation(&mut emu);
                let av = obs.player_avatar_flags(&mut emu);
                s += &format!(
                    "  player {name} {map:?} {pos:?} facing {face} elev {el} avatar {av:#x} in_battle {} locked {}\n",
                    obs.in_battle(&mut emu),
                    obs.field_controls_locked(&mut emu)
                );
                let tiles: Vec<Option<frlg_route::world::Tile>> =
                    [(0, -1), (0, 1), (-1, 0), (1, 0)]
                        .iter()
                        .map(|(dx, dy)| {
                            world
                                .map(map)
                                .ok()
                                .and_then(|m| m.tile(pos.0 + dx, pos.1 + dy))
                        })
                        .collect();
                for (k, (dn, dx, dy)) in [("N", 0, -1), ("S", 0, 1), ("W", -1, 0), ("E", 1, 0)]
                    .into_iter()
                    .enumerate()
                {
                    let (x, y) = (pos.0 + dx, pos.1 + dy);
                    let live = vmap_tile(&mut emu, vmap, x, y);
                    let lay = tiles[k];
                    s += &format!(
                        "    {dn} ({x},{y}) live(col,elev,mt)={live:?} layout(col {:?} beh {:#x} elev {:?})\n",
                        lay.map(|t| t.collision),
                        lay.map_or(0, |t| t.behavior),
                        lay.map(|t| t.elevation)
                    );
                }
                for o in ram.objects(&mut emu) {
                    let d = (o.x - pos.0).abs() + (o.y - pos.1).abs();
                    if d <= 3 || (o.px - pos.0).abs() + (o.py - pos.1).abs() <= 2 {
                        s += &format!(
                            "    obj id {} gfx {} map {:?} at ({},{}) prev ({},{}) face {} mv {} tr {} range {} elev {} invis {}\n",
                            o.local_id, o.graphics, o.map, o.x, o.y, o.px, o.py, o.facing,
                            o.movement, o.trainer_type, o.range, o.elevation, o.invisible
                        );
                    }
                }
                scene[i] = s;
            }
        }
    }
    for (i, (f, t)) in bumps.iter().enumerate() {
        println!("#{i:02} frame {f}: {t}");
        println!("  -200: {}   +400: {}", before[i], after[i]);
        println!("  state: {}", state_at[i]);
        print!("{}", scene[i]);
    }
}
