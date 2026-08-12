//! Attribution probe: idle in Pallet Town and, for every frame that consumes
//! more than the VBlank call, print what moved -- each live object event's
//! coords/facing and the frame index. The cadence and the correlation name
//! the consumer instead of a theory doing it.
//!
//!     cargo run --release -p frlg-rng --example who-rolls

use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog};
use frlg_rng::Rng;
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

const OBJ_SIZE: u32 = 0x24;

fn snapshot(emu: &mut Emu, base: u32) -> Vec<(u8, u8, i16, i16, u8)> {
    let mut out = Vec::new();
    for slot in 0..16u32 {
        let addr = base + slot * OBJ_SIZE;
        let flags = emu.read32(addr);
        if flags & 1 == 0 {
            continue;
        }
        let local_id = emu.read8(addr + 0x08);
        let movement_type = emu.read8(addr + 0x06);
        let x = emu.read16(addr + 0x10) as i16;
        let y = emu.read16(addr + 0x12) as i16;
        let facing = emu.read8(addr + 0x18) & 0x0F;
        out.push((local_id, movement_type, x, y, facing));
    }
    out
}

fn main() {
    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let obj_base = syms.get("gObjectEvents").expect("gObjectEvents").addr;
    let observer = Observer::new(syms).expect("observer");
    let mut emu = Emu::new(&rom).expect("core");
    frlg_emu::boot_with_default_bios(&mut emu).expect("boot");

    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
        if entry.name == "05-house" {
            break;
        }
    }

    let mut model = Rng(observer.rng(&mut emu));
    let mut before = snapshot(&mut emu, obj_base);
    for frame in 0..1200u32 {
        emu.step(0);
        let observed = Rng(observer.rng(&mut emu));
        let steps = model.distance_to(observed);
        model = observed;
        let after = snapshot(&mut emu, obj_base);
        if steps > 1 {
            let moved: Vec<String> = after
                .iter()
                .filter(|now| !before.contains(now))
                .map(|t| format!("{t:?}"))
                .collect();
            println!(
                "frame {frame:>4}: +{} steps; changed objects: {}",
                steps - 1,
                if moved.is_empty() {
                    "none".to_string()
                } else {
                    moved.join(" ")
                }
            );
        }
        before = after;
    }
}
