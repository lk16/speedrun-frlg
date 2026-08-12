//! When does the tutorial's INFLICT_DMG flag actually set? Replay the
//! committed battle watching every byte of `*gBattleStruct`
//! (`decompiled/include/battle.h:300+`), and report bytes that sit at 0 for
//! the battle's first thousand frames and then flip -- the
//! `simulatedInputState[2]` flag byte (`battle.h:425`) is among them, and
//! its flip frame is the fact the accuracy/crit gates hinge on.
//!
//!     cargo run --release -p frlg-rng --example flag-probe

use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

const WATCH: u32 = 0x300;

fn main() {
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let struct_ptr_addr = syms.get("gBattleStruct").expect("gBattleStruct").addr;
    let mut emu = Emu::new(&rom).expect("core");
    frlg_emu::boot_with_default_bios(&mut emu).expect("boot");

    let mut battle_log: Option<InputLog> = None;
    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        if entry.name == "09-battle-win" {
            battle_log = Some(log);
            break;
        }
        for &mask in &log.frames {
            emu.step(mask);
        }
    }
    let battle_log = battle_log.expect("route has 09-battle-win");

    let base = emu.read32(struct_ptr_addr);
    println!("gBattleStruct -> {base:#010x}");
    let mut prev = emu.read_bytes(base, WATCH);
    // (offset, frame, old, new) for bytes that were 0 from battle start
    // until at least frame 800 and then changed.
    let mut zero_until: Vec<u32> = vec![u32::MAX; WATCH as usize];
    let mut flips: Vec<(u32, usize, u8)> = Vec::new();
    for (frame, &mask) in battle_log.frames.iter().enumerate() {
        emu.step(mask);
        let now = emu.read_bytes(base, WATCH);
        for (offset, (&old, &new)) in prev.iter().zip(&now).enumerate() {
            if old != new && zero_until[offset] == u32::MAX && old == 0 {
                zero_until[offset] = frame as u32;
                if frame > 800 {
                    flips.push((offset as u32, frame, new));
                }
            }
        }
        prev = now;
    }
    println!("bytes first leaving 0 after battle frame 800 (offset, frame, new value):");
    for (offset, frame, new) in flips {
        println!("  +{offset:#05x} at frame {frame}: -> {new:#04x}");
    }
}
