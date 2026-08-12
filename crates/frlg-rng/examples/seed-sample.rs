//! The naming-exit seed dial, sampled for real (`docs/rival-1/route.md`, "What is
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

/// A snapshot at one turn's action-selection arrival: the core state and how
/// many battle frames it took to get there. Trials for that turn replay only
/// the suffix -- in-memory savestates are a ~0.5 MB memcpy, so the prefix is
/// paid once per adopted plan instead of once per candidate.
struct Checkpoint {
    state: SaveState,
    frames: u32,
}

/// What one trial came back with. `Aborted` means it reached `abort_at`
/// frames while still unresolved -- it cannot beat the current best, and
/// whether it would eventually have won is deliberately unknown.
enum Trial {
    Done {
        won: bool,
        frames: u32,
        turns: usize,
    },
    Aborted,
}

impl BattleLab {
    /// The drive from a turn menu onward: idle `plan[turn]`, commit the
    /// turn, then alternate to-menu / commit stages for the later turns.
    /// Same shape as the route's search: the mash restarts at every stage.
    /// Aborts as unresolvable once `frames` reaches `abort_at`.
    fn drive_from_menu(
        &mut self,
        plan: &[u32],
        mut turns: usize,
        mut frames: u32,
        abort_at: u32,
    ) -> Trial {
        loop {
            // Arrived at turn `turns + 1`'s menu: idle its delay, commit it.
            turns += 1;
            for _ in 0..plan.get(turns).copied().unwrap_or(0) {
                self.emu.step(0);
                frames += 1;
            }
            for stage in [false, true] {
                // false: mash until the selection state exits (the turn is
                // committed); true: mash to the next menu or the end.
                let mut mash_phase = 0usize;
                loop {
                    self.emu.step(self.mash[mash_phase % self.mash.len()]);
                    mash_phase += 1;
                    frames += 1;
                    let outcome = self.observer.battle_outcome(&mut self.emu);
                    if outcome != 0 {
                        return Trial::Done {
                            won: outcome == WON,
                            frames,
                            turns,
                        };
                    }
                    if self.observer.battle_choosing_actions(&mut self.emu) == stage {
                        break;
                    }
                    if frames >= abort_at.min(FRAME_BUDGET) {
                        return Trial::Aborted;
                    }
                }
            }
        }
    }

    /// One whole battle from the battle-start state under `plan`.
    fn run_full(&mut self, plan: &[u32], abort_at: u32) -> Trial {
        self.emu.load_state(&self.start).expect("load");
        let mut frames = 0u32;
        for _ in 0..plan.first().copied().unwrap_or(0) {
            self.emu.step(0);
            frames += 1;
        }
        // To the first turn's menu (or a pre-menu outcome).
        let mut mash_phase = 0usize;
        loop {
            self.emu.step(self.mash[mash_phase % self.mash.len()]);
            mash_phase += 1;
            frames += 1;
            let outcome = self.observer.battle_outcome(&mut self.emu);
            if outcome != 0 {
                return Trial::Done {
                    won: outcome == WON,
                    frames,
                    turns: 0,
                };
            }
            if self.observer.battle_choosing_actions(&mut self.emu) {
                break;
            }
            if frames >= abort_at.min(FRAME_BUDGET) {
                return Trial::Aborted;
            }
        }
        self.drive_from_menu(plan, 0, frames, abort_at)
    }

    /// Replays `plan`'s battle once, capturing a checkpoint at every turn
    /// menu arrival. The winning plan replays exactly as before -- the
    /// captures are on the arrival frames, before any delay is applied.
    fn checkpoints(&mut self, plan: &[u32]) -> Vec<Checkpoint> {
        self.emu.load_state(&self.start).expect("load");
        let mut frames = 0u32;
        for _ in 0..plan.first().copied().unwrap_or(0) {
            self.emu.step(0);
            frames += 1;
        }
        let mut cps = Vec::new();
        let mut turns = 0usize;
        'battle: loop {
            let mut mash_phase = 0usize;
            loop {
                self.emu.step(self.mash[mash_phase % self.mash.len()]);
                mash_phase += 1;
                frames += 1;
                if self.observer.battle_outcome(&mut self.emu) != 0 || frames >= FRAME_BUDGET {
                    break 'battle;
                }
                if self.observer.battle_choosing_actions(&mut self.emu) {
                    break;
                }
            }
            cps.push(Checkpoint {
                state: self.emu.save_state().expect("state"),
                frames,
            });
            turns += 1;
            for _ in 0..plan.get(turns).copied().unwrap_or(0) {
                self.emu.step(0);
                frames += 1;
            }
            let mut mash_phase = 0usize;
            loop {
                self.emu.step(self.mash[mash_phase % self.mash.len()]);
                mash_phase += 1;
                frames += 1;
                if self.observer.battle_outcome(&mut self.emu) != 0 || frames >= FRAME_BUDGET {
                    break 'battle;
                }
                if !self.observer.battle_choosing_actions(&mut self.emu) {
                    break;
                }
            }
        }
        cps
    }

    /// A stage-2 trial: from the checkpoint at `turn`'s menu, run the rest
    /// of the battle under `plan`.
    fn run_from(&mut self, cp: &Checkpoint, plan: &[u32], turn: usize, abort_at: u32) -> Trial {
        self.emu.load_state(&cp.state).expect("load");
        self.drive_from_menu(plan, turn - 1, cp.frames, abort_at)
    }

    fn search(&mut self) -> Option<(Vec<u32>, u32, u32)> {
        let mut best: Option<(Vec<u32>, u32, usize)> = None;
        let mut wins = 0;
        for delay in START_DELAYS {
            // Anything at or past the current best cannot be adopted.
            let bar = best.as_ref().map_or(FRAME_BUDGET, |&(_, b, _)| b);
            if let Trial::Done { won, frames, turns } = self.run_full(&[delay], bar) {
                wins += won as u32;
                if won && frames < bar {
                    best = Some((vec![delay], frames, turns));
                }
            }
        }
        let (mut plan, mut best_frames, mut best_turns) = best?;
        let mut cps = self.checkpoints(&plan);
        for _pass in 1..=MAX_PASSES {
            let mut adopted = false;
            let pass_turns = best_turns;
            for turn in 1..=pass_turns {
                if turn > cps.len() {
                    continue;
                }
                for delay in TURN_DELAYS {
                    let mut candidate = plan.clone();
                    if candidate.len() < turn + 1 {
                        candidate.resize(turn + 1, 0);
                    }
                    if candidate[turn] == delay {
                        continue;
                    }
                    candidate[turn] = delay;
                    let cp = &cps[turn - 1];
                    if let Trial::Done { won, frames, turns } =
                        self.run_from(cp, &candidate, turn, best_frames)
                    {
                        if won && frames < best_frames {
                            plan = candidate;
                            best_frames = frames;
                            best_turns = turns;
                            adopted = true;
                            cps = self.checkpoints(&plan);
                        }
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
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
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
                 {wins}/64 start delays won (early-pruned trials not counted), best battle {battle} (plan {plan:?}), \
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
