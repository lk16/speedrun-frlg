//! The battle search, re-run under a stream shift: for each shift k, write
//! `jump(k)` of the real battle-start `gRngValue` and run the same two-stage
//! search the route uses (start delay, then greedy per-turn delays to a
//! fixpoint) -- so shifted streams are compared on what they can *actually*
//! do, not on what a pure mash does with them.
//!
//! Sanity anchor: shift 0 must land on the committed battle's 2409 frames
//! (plan [4, 3, 3, 3]), because it is the same search on the same stream.
//!
//! Like battle-scan, the RNG write is exploratory -- a winning shift still
//! has to be reached by real inputs to count.
//!
//!     cargo run --release -p frlg-rng --example battle-plan-scan -- SHIFT [SHIFT..]

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

/// `B_OUTCOME_WON`, `decompiled/include/constants/battle.h:76`.
const WON: u8 = 1;
const FRAME_BUDGET: u32 = 20_000;
const START_DELAYS: std::ops::Range<u32> = 0..64;
const TURN_DELAYS: std::ops::Range<u32> = 1..16;
const MAX_PASSES: usize = 8;

struct BattleLab {
    emu: Emu,
    observer: Observer,
    start: SaveState,
    rng_addr: u32,
    mash: Vec<u16>,
}

impl BattleLab {
    /// One battle under `plan` (plan[0] start delay, plan[k] idle frames at
    /// the k-th turn's action selection), on the stream shifted to `state`.
    /// Returns (won, frames, turns).
    fn run_plan(&mut self, state: Rng, plan: &[u32]) -> (bool, u32, usize) {
        self.emu.load_state(&self.start).expect("load");
        for (i, byte) in state.0.to_le_bytes().iter().enumerate() {
            self.emu.write8(self.rng_addr + i as u32, *byte);
        }
        let mut frames = 0u32;
        let idle = |emu: &mut Emu, n: u32, frames: &mut u32| {
            for _ in 0..n {
                emu.step(0);
                *frames += 1;
            }
        };
        idle(
            &mut self.emu,
            plan.first().copied().unwrap_or(0),
            &mut frames,
        );
        let mut turns = 0usize;
        let won = loop {
            // To this turn's action selection, or the end. The mash pattern
            // restarts at every stage, exactly like the route's
            // advance_while (record.rs) -- the drive shape is part of the
            // stream, and battles are only comparable within one shape.
            let mut mash_phase = 0usize;
            let mut over = false;
            loop {
                self.emu.step(self.mash[mash_phase % self.mash.len()]);
                mash_phase += 1;
                frames += 1;
                if self.observer.battle_outcome(&mut self.emu) != 0
                    || self.observer.battle_choosing_actions(&mut self.emu)
                {
                    break;
                }
                if frames >= FRAME_BUDGET {
                    over = true;
                    break;
                }
            }
            if over {
                break false;
            }
            let outcome = self.observer.battle_outcome(&mut self.emu);
            if outcome != 0 {
                break outcome == WON;
            }
            turns += 1;
            idle(
                &mut self.emu,
                plan.get(turns).copied().unwrap_or(0),
                &mut frames,
            );
            // Commit this turn's actions: mash until the state exits.
            mash_phase = 0;
            loop {
                self.emu.step(self.mash[mash_phase % self.mash.len()]);
                mash_phase += 1;
                frames += 1;
                if self.observer.battle_outcome(&mut self.emu) != 0
                    || !self.observer.battle_choosing_actions(&mut self.emu)
                {
                    break;
                }
                if frames >= FRAME_BUDGET {
                    over = true;
                    break;
                }
            }
            if over {
                break false;
            }
            let outcome = self.observer.battle_outcome(&mut self.emu);
            if outcome != 0 {
                break outcome == WON;
            }
        };
        (won, frames, turns)
    }

    /// The route's two-stage search on the shifted stream; returns the best
    /// (plan, frames), or None if no start delay wins.
    fn search(&mut self, state: Rng) -> Option<(Vec<u32>, u32)> {
        let mut best: Option<(Vec<u32>, u32, usize)> = None;
        let mut wins = 0;
        for delay in START_DELAYS {
            let (won, frames, turns) = self.run_plan(state, &[delay]);
            wins += won as u32;
            if won && best.as_ref().is_none_or(|&(_, b, _)| frames < b) {
                best = Some((vec![delay], frames, turns));
            }
        }
        let (mut plan, mut best_frames, mut best_turns) = best?;
        eprintln!(
            "    stage 1: {wins}/{} delays win, delay {} at {best_frames} frames",
            START_DELAYS.end, plan[0]
        );
        for _pass in 1..=MAX_PASSES {
            let mut adopted = false;
            let pass_turns = best_turns;
            for turn in 1..=pass_turns {
                for delay in TURN_DELAYS {
                    let mut candidate = plan.clone();
                    if candidate.len() < turn + 1 {
                        candidate.resize(turn + 1, 0);
                    }
                    if candidate[turn] == delay {
                        continue;
                    }
                    candidate[turn] = delay;
                    let (won, frames, turns) = self.run_plan(state, &candidate);
                    if won && frames < best_frames {
                        plan = candidate;
                        best_frames = frames;
                        best_turns = turns;
                        adopted = true;
                    }
                }
            }
            if !adopted {
                break;
            }
        }
        Some((plan, best_frames))
    }
}

fn main() {
    let shifts: Vec<i64> = std::env::args()
        .skip(1)
        .map(|a| a.parse().expect("SHIFT"))
        .collect();
    assert!(!shifts.is_empty(), "pass at least one shift");

    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
    }
    let start = emu.save_state().expect("state");
    let base = Rng(emu.read32(rng_addr));
    println!(
        "battle-start gRngValue {:#010x}; committed battle: 2409 frames",
        base.0
    );

    let mash = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };
    let mut lab = BattleLab {
        emu,
        observer,
        start,
        rng_addr,
        mash,
    };

    for shift in shifts {
        println!("shift {shift}:");
        match lab.search(base.jump(shift as u32)) {
            Some((plan, frames)) => println!(
                "  best {frames} frames ({:+} vs 2409), plan {plan:?}",
                frames as i64 - 2409
            ),
            None => println!("  no start delay wins"),
        }
    }
}
