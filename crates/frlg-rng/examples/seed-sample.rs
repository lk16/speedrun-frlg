//! The naming-exit seed dial, sampled for real (`docs/route.md`, "What is
//! not optimised"): insert N idle frames at the start of `03-names` -- which
//! delays the naming screen's exit press by N and therefore moves the
//! timer-1 seed by 18753·N (mod 2^16) -- replay the committed logs to the
//! battle on the *real* stream (no RAM writes anywhere), and run the route's
//! two-stage battle search there.
//!
//! Variant N wins iff its battle beats the committed 2409 by more than N,
//! since the battle now starts N frames later. N = 0 is the sanity anchor:
//! it must reproduce seed 0xdf93 and the committed 2409/[4, 3, 3, 3].
//!
//!     cargo run --release -p frlg-rng --example seed-sample -- N

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
    mash: Vec<u16>,
}

impl BattleLab {
    /// One battle under `plan` from the real battle-start state. Same drive
    /// shape as the route's search: the mash restarts at every stage.
    fn run_plan(&mut self, plan: &[u32]) -> (bool, u32, usize) {
        self.emu.load_state(&self.start).expect("load");
        let mut frames = 0u32;
        for _ in 0..plan.first().copied().unwrap_or(0) {
            self.emu.step(0);
            frames += 1;
        }
        let mut turns = 0usize;
        let won = loop {
            let mut over = false;
            let mut mash_phase = 0usize;
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
            for _ in 0..plan.get(turns).copied().unwrap_or(0) {
                self.emu.step(0);
                frames += 1;
            }
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

    fn search(&mut self) -> Option<(Vec<u32>, u32, u32)> {
        let mut best: Option<(Vec<u32>, u32, usize)> = None;
        let mut wins = 0;
        for delay in START_DELAYS {
            let (won, frames, turns) = self.run_plan(&[delay]);
            wins += won as u32;
            if won && best.as_ref().is_none_or(|&(_, b, _)| frames < b) {
                best = Some((vec![delay], frames, turns));
            }
        }
        let (mut plan, mut best_frames, mut best_turns) = best?;
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
                    let (won, frames, turns) = self.run_plan(&candidate);
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
        Some((plan, best_frames, wins))
    }
}

fn main() {
    let n: u32 = std::env::args()
        .nth(1)
        .expect("pass N, the idle frames inserted before the naming exit")
        .parse()
        .expect("N");

    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    // Replay 01..08 with N idles inserted at the start of 03-names, tracking
    // reseeds on the real stream.
    let mut model = Rng(0);
    let mut reseeds: Vec<(u32, u32)> = Vec::new();
    let mut frame = 0u32;
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        let inserted = if entry.name == "03-names" { n } else { 0 };
        for &mask in std::iter::repeat_n(&0u16, inserted as usize).chain(&log.frames) {
            emu.step(mask);
            frame += 1;
            let observed = Rng(observer.rng(&mut emu));
            if model.distance_to(observed) > 5_000 {
                reseeds.push((frame, observed.0));
            }
            model = observed;
        }
    }
    assert_eq!(
        reseeds.len(),
        2,
        "expected both SeedRng events: {reseeds:x?}"
    );
    let seed_frame = reseeds[1].0;
    let seed_state = reseeds[1].1;
    if !observer.in_battle(&mut emu) {
        println!(
            "N {n}: seed {seed_state:#06x} at frame {seed_frame} -- downstream logs \
             desynced before the battle, variant unusable"
        );
        return;
    }

    let start = emu.save_state().expect("state");
    let mash = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };
    let battle_start = frame;
    let mut lab = BattleLab {
        emu,
        observer,
        start,
        mash,
    };
    match lab.search() {
        Some((plan, battle, wins)) => {
            let total = battle_start + battle;
            println!(
                "N {n}: seed {seed_state:#06x}, battle starts at {battle_start}, \
                 {wins}/64 start delays win, best battle {battle} (plan {plan:?}), \
                 total {total} ({:+} vs 9658)",
                total as i64 - 9658
            );
        }
        None => println!(
            "N {n}: seed {seed_state:#06x}, battle starts at {battle_start}, \
             no start delay wins at all"
        ),
    }
}
