//! The RNG-consumer experiments, run in the emulator rather than argued:
//!
//! 1. Does pressing A consume RNG in the field?
//! 2. Who consumes RNG while the player idles -- and does the map (its NPCs)
//!    change that?
//! 3. Do two same-length walking paths to the same tile leave the stream in
//!    the same place?
//!
//! Every run counts "extra steps": total stream movement minus one per frame
//! (the VBlank `Random()`, `decompiled/src/main.c:412`), so frame counts
//! cancel out and what remains is exactly the other consumers.
//!
//!     cargo run --release -p frlg-rng --example field-experiments

use std::path::{Path, PathBuf};

use frlg_emu::{keys, Emu, InputLog, SaveState};
use frlg_rng::Rng;
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

struct Lab {
    emu: Emu,
    observer: Observer,
}

impl Lab {
    /// One frame; returns how many stream steps beyond the VBlank call it
    /// consumed (so 0 is a quiet frame).
    fn step_counted(&mut self, keys: u16, model: &mut Rng) -> u32 {
        self.emu.step(keys);
        let observed = Rng(self.observer.rng(&mut self.emu));
        let steps = model.distance_to(observed);
        *model = observed;
        assert!(steps >= 1, "a field frame always runs the VBlank Random()");
        steps - 1
    }

    fn rng(&mut self) -> Rng {
        Rng(self.observer.rng(&mut self.emu))
    }

    fn pos(&mut self) -> (i16, i16) {
        self.observer.pos(&mut self.emu).expect("on the field")
    }

    /// Runs `frames` frames of `mask(frame_index)`, returning total extra
    /// steps and the count of frames with any.
    fn run(&mut self, frames: u32, mask: impl Fn(u32) -> u16) -> (u32, u32) {
        let mut model = self.rng();
        let mut extra = 0;
        let mut busy = 0;
        for index in 0..frames {
            let e = self.step_counted(mask(index), &mut model);
            extra += e;
            busy += (e > 0) as u32;
        }
        (extra, busy)
    }

    /// Walks one tile: holds `dir` until the tile coordinate changes.
    /// Returns (frames, extra rng steps); None if nothing gave within 64
    /// frames (a wall).
    fn walk_tile(&mut self, dir: u16) -> Option<(u32, u32)> {
        let start = self.pos();
        let mut model = self.rng();
        let mut frames = 0;
        let mut extra = 0;
        while self.pos() == start {
            extra += self.step_counted(dir, &mut model);
            frames += 1;
            if frames >= 64 {
                return None;
            }
        }
        Some((frames, extra))
    }
}

/// The live object events: `gObjectEvents`, 16 slots of 0x24 bytes
/// (`decompiled/include/global.fieldmap.h:212`, `OBJECT_EVENTS_COUNT` at
/// `include/constants/global.h:41`). Returns (localId, movementType, x, y,
/// frozen) for every active slot except the player (slot with isPlayer set).
fn live_object_events(emu: &mut Emu, base: u32) -> Vec<(u8, u8, u8, i16, i16, bool)> {
    const SIZE: u32 = 0x24;
    let mut out = Vec::new();
    for slot in 0..16u32 {
        let addr = base + slot * SIZE;
        let flags = emu.read32(addr);
        let active = flags & 1 != 0;
        let frozen = flags & (1 << 8) != 0;
        let is_player = flags & (1 << 16) != 0;
        if !active || is_player {
            continue;
        }
        let local_id = emu.read8(addr + 0x08);
        let graphics_id = emu.read8(addr + 0x05);
        let movement_type = emu.read8(addr + 0x06);
        let x = emu.read16(addr + 0x10) as i16;
        let y = emu.read16(addr + 0x12) as i16;
        out.push((local_id, graphics_id, movement_type, x, y, frozen));
    }
    out
}

fn main() {
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let observer = Observer::new(syms).expect("observer");
    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    // Replay to two checkpoints: end of 04-options (the bedroom, a map with
    // no other object events) and end of 05-house (outside in Pallet Town,
    // NPCs around).
    let mut bedroom: Option<SaveState> = None;
    let mut outside: Option<SaveState> = None;
    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
        match entry.name.as_str() {
            "04-options" => bedroom = Some(emu.save_state().expect("state")),
            "05-house" => {
                outside = Some(emu.save_state().expect("state"));
            }
            _ => {}
        }
        if outside.is_some() {
            break;
        }
    }
    let bedroom = bedroom.expect("route has 04-options");
    let outside = outside.expect("route has 05-house");
    let mut lab = Lab { emu, observer };

    const N: u32 = 600;

    println!("== who consumes while idling {N} frames ==");
    for (name, state) in [("bedroom", &bedroom), ("pallet town", &outside)] {
        lab.emu.load_state(state).expect("load");
        let here = (lab.observer.map(&mut lab.emu), lab.pos());
        let (extra, busy) = lab.run(N, |_| 0);
        println!("  {name:<12} {here:?}: {extra} extra steps on {busy} frames");
    }

    println!("== does pressing A consume RNG (same spot, same {N} frames) ==");
    for (name, mask) in [
        ("idle", 0u16),
        ("hold A", keys::A),
        ("mash A (1 in 4)", 0xFFFF), // sentinel, handled below
    ] {
        lab.emu.load_state(&outside).expect("load");
        let (extra, busy) = if mask == 0xFFFF {
            lab.run(N, |i| if i % 4 == 0 { keys::A } else { 0 })
        } else {
            lab.run(N, move |_| mask)
        };
        println!("  {name:<16}: {extra} extra steps on {busy} frames");
    }

    println!("== same-length paths, same start, same end ==");
    // From outside the house, both paths take the same four tiles' worth of
    // directions {DOWN, DOWN, LEFT, LEFT}, differently ordered, both starting
    // with DOWN so neither pays an initial turn.
    let paths: [(&str, [u16; 4]); 2] = [
        (
            "DOWN DOWN LEFT LEFT",
            [keys::DOWN, keys::DOWN, keys::LEFT, keys::LEFT],
        ),
        (
            "DOWN LEFT DOWN LEFT",
            [keys::DOWN, keys::LEFT, keys::DOWN, keys::LEFT],
        ),
    ];
    for (name, path) in paths {
        lab.emu.load_state(&outside).expect("load");
        let start = lab.pos();
        let mut frames = 0;
        let mut extra = 0;
        for dir in path {
            let (f, e) = lab.walk_tile(dir).expect("path tile blocked");
            frames += f;
            extra += e;
        }
        // Idle out to a common horizon so the final states are comparable
        // even if the step frames differed.
        let (idle_extra, _) = lab.run(200 - frames.min(200), |_| 0);
        println!(
            "  {name}: {start:?} -> {:?}, {frames} frames, {extra} extra during walk, \
             rng at frame 200: {:#010x} ({} extra while idling out)",
            lab.pos(),
            lab.rng().0,
            idle_extra,
        );
    }

    // Object events despawn outside a window around the player
    // (`decompiled/src/event_object_movement.c:1798-1801`: live iff
    // px-9 <= tx <= px+10 and py-7 <= ty <= py+9 in template coords), so
    // where the player stands decides which wanderers are rolling at all.
    // Pallet Town's rollers are the sign lady (3,10) and the fat man (13,17)
    // (`data/maps/PalletTown/map.json`), both MOVEMENT_TYPE_WANDER_AROUND.
    println!("== standing position vs who is rolling ({N} frames each) ==");
    for (name, walk) in [
        ("as landed", vec![]),
        ("4 east", vec![keys::RIGHT; 4]),
        ("4 east, 4 north", {
            let mut w = vec![keys::RIGHT; 4];
            w.extend([keys::UP; 4]);
            w
        }),
        ("3 south (toward the lady)", vec![keys::DOWN; 3]),
        ("3 south, 3 west", {
            let mut w = vec![keys::DOWN; 3];
            w.extend([keys::LEFT; 3]);
            w
        }),
    ] {
        lab.emu.load_state(&outside).expect("load");
        let mut blocked = false;
        for dir in walk {
            if lab.walk_tile(dir).is_none() {
                blocked = true;
                break;
            }
        }
        let map = lab.observer.map(&mut lab.emu);
        let pos = lab.pos();
        let obj_base = lab
            .observer
            .symbols()
            .get("gObjectEvents")
            .expect("gObjectEvents")
            .addr;
        let live = live_object_events(&mut lab.emu, obj_base);
        let (extra, busy) = lab.run(N, |_| 0);
        let note = if blocked { " [walk blocked early]" } else { "" };
        println!("  {name:<28} at {map:?} {pos:?}: {extra} extra steps on {busy} frames{note}");
        println!("      live NPCs (localId, gfx, movementType, x, y, frozen): {live:?}");
    }
}
