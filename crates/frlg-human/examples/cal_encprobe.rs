//! Encounter-rate probe (`human-run/calibration.md`, 2026-11-22 calibration section): from a savestate
//! standing on tall grass, pace between the player's tile and a neighbouring grass tile (B held, one
//! tile per press) and count the steps to the first wild battle. Repeats with `k` idle frames first so
//! each trial sees a different RNG state. Prints one line per trial and the mean.
//!
//!     cargo run --release --example cal_encprobe -- <rom> <sym> <state> <dir: L|R|U|D> <trials>
use std::path::Path;

use frlg_emu::{keys, Emu, SymbolTable};
use frlg_route::Observer;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let wild = syms.get("sWildEncounterData").expect("sym").addr;
    let obs = Observer::new(syms).expect("obs");
    let (k1, k2) = match a[3].as_str() {
        "L" => (keys::LEFT, keys::RIGHT),
        "R" => (keys::RIGHT, keys::LEFT),
        "U" => (keys::UP, keys::DOWN),
        _ => (keys::DOWN, keys::UP),
    };
    let trials: usize = a[4].parse().unwrap();
    let mut emu = Emu::new(Path::new(&a[0])).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    let mut sum = 0u64;
    let mut n = 0u64;
    for t in 0..trials {
        emu.load_state_file(Path::new(&a[2])).expect("state");
        emu.idle(1 + 3 * t as u32);
        if t == 0 {
            println!(
                "at start: rng {:#x} behavior {} buff {} steps_since {}",
                emu.read32(wild),
                emu.read16(wild + 4),
                emu.read16(wild + 6),
                emu.read8(wild + 8)
            );
        }
        let mut steps = 0u32;
        let mut dir = k1;
        let mut got = false;
        let mut behav = Vec::new();
        'outer: while steps < 300 {
            let p0 = obs.pos(&mut emu);
            let mut f = 0;
            loop {
                emu.step(dir | keys::B);
                f += 1;
                if obs.in_battle(&mut emu) {
                    got = true;
                    break 'outer;
                }
                if obs.pos(&mut emu) != p0 {
                    break;
                }
                if f > 60 {
                    println!("trial {t}: stuck");
                    break 'outer;
                }
            }
            // let the step finish
            for _ in 0..40 {
                if obs.in_battle(&mut emu) {
                    got = true;
                    steps += 1;
                    break 'outer;
                }
                if obs.player_can_step(&mut emu) {
                    break;
                }
                emu.step(0);
            }
            steps += 1;
            if behav.len() < 12 {
                behav.push((
                    emu.read16(wild + 4),
                    emu.read16(wild + 6),
                    emu.read8(wild + 8),
                ));
            }
            dir = if dir == k1 { k2 } else { k1 };
            // a few frames for a battle transition to begin
            for _ in 0..3 {
                emu.step(0);
                if obs.in_battle(&mut emu) || !obs.callback2_is(&mut emu, "CB2_Overworld") {
                    got = true;
                    break 'outer;
                }
            }
        }
        println!("trial {t}: steps {steps} battle {got} behaviors {behav:?}");
        if got {
            sum += steps as u64;
            n += 1;
        }
    }
    println!(
        "mean steps to battle {:.2} over {n}",
        sum as f64 / n.max(1) as f64
    );
}
