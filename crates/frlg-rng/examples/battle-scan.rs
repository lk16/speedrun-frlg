//! What would the rival battle cost if the stream arrived shifted?
//!
//! Replays the committed route to the end of `08-battle-start`, then for each
//! stream shift k (and optionally a small start delay) writes `jump(k)` of
//! the real state into `gRngValue` and runs the battle as a pure mash. The
//! result is a map from (shift, delay) to battle length, which prices the
//! free levers (NPC roll avoidance upstream shifts the stream without
//! costing a frame, `docs/rival-1/journal/` 2026-08-12) against the committed
//! battle's 2409 frames (delay plan [4, 3, 3, 3]).
//!
//! The RNG write is exploratory, not evidence: a shift that wins fast still
//! has to be *reached* by real inputs before it means anything. This scan
//! only says which shifts are worth reaching.
//!
//!     cargo run --release -p frlg-rng --example battle-scan [-- FROM TO [DELAYS]]
//!
//! DELAYS is a comma list of start delays to try per shift (default "0").

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

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

/// `B_OUTCOME_WON`, `decompiled/include/constants/battle.h:76`.
const WON: u8 = 1;
/// Far above any battle seen on this route; running past it is a loss.
const FRAME_BUDGET: u32 = 20_000;

struct Job {
    shift: i64,
    delay: u32,
}

struct Outcome {
    shift: i64,
    delay: u32,
    won: bool,
    frames: u32,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let from: i64 = args.next().map_or(-24, |a| a.parse().expect("FROM"));
    let to: i64 = args.next().map_or(24, |a| a.parse().expect("TO"));
    let delays: Vec<u32> = args.next().map_or_else(
        || vec![0],
        |list| {
            list.split(',')
                .map(|d| d.parse().expect("DELAYS"))
                .collect()
        },
    );

    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    let mut replayed = 0usize;
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
        replayed += log.frames.len();
    }
    let start = emu.save_state().expect("state at battle start");
    let base = Rng(emu.read32(rng_addr));
    drop(emu);
    println!(
        "battle starts at frame {replayed}, gRngValue {:#010x}; committed battle: 2409 frames",
        base.0
    );

    // The mash the route uses: hold A text_hold frames, release one.
    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };

    let jobs: Vec<Job> = (from..=to)
        .flat_map(|shift| delays.iter().map(move |&delay| Job { shift, delay }))
        .collect();
    let total = jobs.len();
    let jobs = Arc::new(Mutex::new(jobs));
    let (tx, rx) = mpsc::channel::<Outcome>();

    let workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(2).clamp(1, 12))
        .unwrap_or(4);
    for _ in 0..workers {
        let jobs = Arc::clone(&jobs);
        let tx = tx.clone();
        let rom = rom.clone();
        let start: SaveState = start.clone();
        let mash = mash.clone();
        let bios = ledger.bios.clone();
        std::thread::spawn(move || {
            let mut emu = Emu::new(&rom).expect("core");
            let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
            assert_eq!(boot, bios);
            let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
            let observer = Observer::new(syms).expect("observer");
            loop {
                let job = {
                    let mut jobs = jobs.lock().unwrap();
                    match jobs.pop() {
                        Some(job) => job,
                        None => return,
                    }
                };
                emu.load_state(&start).expect("load");
                // A negative shift is a jump of 2^32 + shift: the cast wraps
                // to exactly that, and the LCG's period is 2^32.
                let shifted = base.jump(job.shift as u32);
                for (i, byte) in shifted.0.to_le_bytes().iter().enumerate() {
                    emu.write8(rng_addr + i as u32, *byte);
                }
                for _ in 0..job.delay {
                    emu.step(0);
                }
                let mut frames = job.delay;
                let outcome = loop {
                    emu.step(mash[(frames - job.delay) as usize % mash.len()]);
                    frames += 1;
                    let outcome = observer.battle_outcome(&mut emu);
                    if outcome != 0 || frames >= FRAME_BUDGET {
                        break outcome;
                    }
                };
                tx.send(Outcome {
                    shift: job.shift,
                    delay: job.delay,
                    won: outcome == WON,
                    frames,
                })
                .unwrap();
            }
        });
    }
    drop(tx);

    let mut results: Vec<Outcome> = rx.into_iter().collect();
    assert_eq!(results.len(), total);
    results.sort_by_key(|o| (o.shift, o.delay));

    let mut best: Option<&Outcome> = None;
    for o in &results {
        let verdict = if o.won { "WIN " } else { "loss" };
        println!(
            "  shift {:>4} delay {:>2}: {verdict} {:>5} frames",
            o.shift, o.delay, o.frames
        );
        if o.won && best.is_none_or(|b| o.frames < b.frames) {
            best = Some(o);
        }
    }
    match best {
        Some(o) => println!(
            "best: shift {} delay {} wins in {} frames ({:+} vs committed 2409)",
            o.shift,
            o.delay,
            o.frames,
            o.frames as i64 - 2409
        ),
        None => println!("nothing in the scan wins"),
    }
}
