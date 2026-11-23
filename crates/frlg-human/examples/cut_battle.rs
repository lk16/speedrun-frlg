//! Cut a pre-battle savestate out of a played run (model honesty 4, `battle-test` inputs).
//!
//! Replays `<run.ilog>` from a checkpoint savestate saved at frame `start` and writes the state
//! `lead` frames before the `n`-th frame (1-based, counting from `start`) on which `gMain.inBattle`
//! turns on, so `frlg-human battle-test` can mash into that battle with any human seed.
//!
//!     cargo run --release --example cut_battle -- <rom> <sym> <run.ilog> <state> <start> <n> <lead> <out.state>
//!
//! Prints the frame of the battle start and of the saved state.
use std::path::Path;

use frlg_emu::{Emu, InputLog, SymbolTable};
use frlg_route::Observer;

fn boot(rom: &str, state: &str) -> Emu {
    // Booted like `Recorder::from_reset` (BIOS, then reset), as `cal_trace` does.
    let mut emu = Emu::new(Path::new(rom)).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    emu.load_state_file(Path::new(state)).expect("state");
    emu
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() != 8 {
        eprintln!(
            "usage: cut_battle <rom> <sym> <run.ilog> <state> <start> <n> <lead> <out.state>"
        );
        std::process::exit(2);
    }
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let obs = Observer::new(syms).expect("observer");
    let log = InputLog::decode(&std::fs::read(&a[2]).expect("ilog")).expect("ilog decode");
    let start: usize = a[4].parse().unwrap();
    let n: usize = a[5].parse().unwrap();
    let lead: usize = a[6].parse().unwrap();
    let mut emu = boot(&a[0], &a[3]);
    let mut seen = 0;
    let mut was = obs.in_battle(&mut emu);
    let mut at = None;
    for f in start..log.frames.len() {
        emu.step(log.frames[f]);
        let now = obs.in_battle(&mut emu);
        if now && !was {
            seen += 1;
            if seen == n {
                at = Some(f + 1);
                break;
            }
        }
        was = now;
    }
    let at = at.expect("battle not found in the log");
    let cut = at.saturating_sub(lead).max(start);
    let mut emu = boot(&a[0], &a[3]);
    for f in start..cut {
        emu.step(log.frames[f]);
    }
    emu.save_state_file(Path::new(&a[7])).expect("save");
    println!("battle_start {at} saved {cut} -> {}", a[7]);
}
