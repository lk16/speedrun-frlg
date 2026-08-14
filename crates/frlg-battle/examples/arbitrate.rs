//! Put the engine + solver to work on the committed route: find every
//! battle the model says could beat the committed 2409, and let the
//! emulator arbitrate the gates for real.
//!
//! Two levers, both bounded by the global floor (`global-floor`: no start
//! state plays below 2392, so nothing here chases more than 17 frames):
//!
//! - **Turn delays from the committed anchor** (cost already inside the
//!   leaf's frame count): a denser plan grid than `pure-search`'s -- every
//!   delay 0..=12 per turn, not just even ones -- engine-scanned, then the
//!   best plans replayed on libmgba from the committed battle-start state.
//! - **Pre-battle waits** (cost w frames, 1:1): w idle frames physically
//!   inserted at the head of `08-battle-start`'s committed log, the shifted
//!   segment replayed, and the *measured* battle-start `gRngValue` -- which
//!   only equals `anchor.jump(w)` if nothing else rolls in the window, so
//!   it is measured, not assumed -- engine-scanned like the committed one.
//!   Any wait whose best leaf + w beats the best real total is arbitrated
//!   on the state the wait actually reaches (gates are scene state, so a
//!   RAM-written anchor would not arbitrate them faithfully).
//!
//!     cargo run --release -p frlg-battle --example arbitrate [-- MAX_WAIT [TOP_PLANS]]

use std::path::{Path, PathBuf};

use frlg_battle::engine::{simulate, SimResult};
use frlg_battle::Mon;
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

/// gBattleMons for the committed route's battle (measured; battle-truth).
/// Fixed across stream shifts -- `tests/trace_vs_engine.rs` and
/// `tests/engine_vs_emulator.rs` hold them constant over 64 shifted anchors
/// and fresh emulator runs.
fn mons() -> (Mon, Mon) {
    (
        Mon {
            hp: 20,
            max_hp: 20,
            attack: 11,
            defense: 10,
            speed: 11,
            level: 5,
            atk_stage: 6,
            def_stage: 6,
        },
        Mon {
            hp: 18,
            max_hp: 18,
            attack: 11,
            defense: 9,
            speed: 9,
            level: 5,
            atk_stage: 6,
            def_stage: 6,
        },
    )
}

/// The route search's drive, verbatim in control flow (`run_plan` in
/// frlg-rng's battle-plan-scan / fit-pacing): loop A to action selection,
/// the plan's idle, loop B through the commit, repeat. Returns (won,
/// frames) with frames counted exactly like the search scores a battle.
fn run_plan(
    emu: &mut Emu,
    observer: &Observer,
    start: &SaveState,
    mash: &[u16],
    plan: &[u32],
) -> (bool, u32) {
    emu.load_state(start).expect("load state");
    let mut frames = 0u32;
    for _ in 0..plan.first().copied().unwrap_or(0) {
        emu.step(0);
        frames += 1;
    }
    let mut turns = 0usize;
    let won = loop {
        let mut mash_phase = 0usize;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if observer.battle_outcome(emu) != 0
                || observer.battle_choosing_actions(emu)
                || frames >= FRAME_BUDGET
            {
                break;
            }
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
        if frames >= FRAME_BUDGET {
            break false;
        }
        turns += 1;
        for _ in 0..plan.get(turns).copied().unwrap_or(0) {
            emu.step(0);
            frames += 1;
        }
        let mut mash_phase = 0usize;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if observer.battle_outcome(emu) != 0
                || !observer.battle_choosing_actions(emu)
                || frames >= FRAME_BUDGET
            {
                break;
            }
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
        if frames >= FRAME_BUDGET {
            break false;
        }
    };
    (won, frames)
}

/// Engine scan of one anchor over the dense plan grid: every plan whose
/// best winning leaf is strictly below `bar`, best-first.
fn engine_candidates(anchor: Rng, bar: u32) -> Vec<(u32, Vec<u32>)> {
    let (us, rival) = mons();
    let mut out: Vec<(u32, Vec<u32>)> = Vec::new();
    for d0 in 0..5u32 {
        for d1 in 0..=12u32 {
            for d2 in 0..=12u32 {
                for d3 in 0..=12u32 {
                    let plan = [d0, d1, d2, d3];
                    let mut best: Option<u32> = None;
                    for leaf in simulate(&plan, anchor, us, rival) {
                        if let SimResult::Win { frames } = leaf.result {
                            best = Some(best.map_or(frames, |b| b.min(frames)));
                        }
                    }
                    if let Some(b) = best {
                        if b < bar {
                            out.push((b, plan.to_vec()));
                        }
                    }
                }
            }
        }
    }
    out.sort();
    out
}

fn main() {
    let max_wait: u32 = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("MAX_WAIT"))
        .unwrap_or(16);
    let top_plans: usize = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("TOP_PLANS"))
        .unwrap_or(20);

    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    // Replay 01..08, keeping the state at 07's end (wait replays restart
    // there) and the committed 08 masks (they get replayed shifted).
    let mut state07: Option<SaveState> = None;
    let mut log08: Vec<u16> = Vec::new();
    let mut frame = 0u32;
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        if entry.name == "08-battle-start" {
            state07 = Some(emu.save_state().expect("state at 07 end"));
            log08 = log.frames.clone();
        }
        for &mask in &log.frames {
            emu.step(mask);
            frame += 1;
        }
    }
    let state07 = state07.expect("08-battle-start in the ledger");
    assert!(
        observer.in_battle(&mut emu),
        "replay must end in the battle"
    );
    let battle_start_frame = frame;
    let anchor0 = Rng(emu.read32(rng_addr));
    let start0 = emu.save_state().expect("state at battle start");
    println!(
        "battle starts at frame {battle_start_frame}, gRngValue {:#010x}",
        anchor0.0
    );

    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };

    // The committed battle is the bar to beat, re-measured rather than
    // trusted: run the ledger's plan once.
    let committed_plan: Vec<u32> = vec![4, 3, 3, 3];
    let (won, committed_frames) = run_plan(&mut emu, &observer, &start0, &mash, &committed_plan);
    assert!(won, "the committed plan must still win");
    println!("committed plan {committed_plan:?} replays to {committed_frames} frames");
    let mut best_total = committed_frames; // totals are relative to frame 7249
    let mut best_desc = format!("committed {committed_plan:?}");

    // Lever 1: turn delays from the committed anchor.
    let candidates = engine_candidates(anchor0, best_total);
    println!(
        "\nengine: {} plans with a winning leaf below {best_total} on the committed anchor",
        candidates.len()
    );
    for (predicted, plan) in candidates.iter().take(top_plans) {
        let (won, frames) = run_plan(&mut emu, &observer, &start0, &mash, plan);
        let verdict = if !won {
            "LOSS".to_string()
        } else {
            format!("{frames}")
        };
        println!("  predicted {predicted} plan {plan:?} -> real {verdict}");
        if won && frames < best_total {
            best_total = frames;
            best_desc = format!("wait 0, plan {plan:?}");
        }
    }

    // Lever 2: pre-battle waits.
    println!("\npre-battle waits (w idles at the head of 08-battle-start):");
    for w in 1..=max_wait {
        emu.load_state(&state07).expect("load 07 state");
        for _ in 0..w {
            emu.step(0);
        }
        for &mask in &log08 {
            emu.step(mask);
        }
        if !observer.in_battle(&mut emu) {
            println!("  w={w}: shifted 08 desyncs (battle not reached), skipped");
            continue;
        }
        let anchor_w = Rng(emu.read32(rng_addr));
        let start_w = emu.save_state().expect("state");
        let clean = anchor0.jump(w);
        let note = if anchor_w == clean {
            "= jump(w)".to_string()
        } else {
            format!(
                "jump(w) {:#010x}, extra rolls {}",
                clean.0,
                clean.distance_to(anchor_w)
            )
        };
        // Anything whose leaf + w cannot beat the best real total is not
        // worth an emulator run; the -1 asks for strict improvement.
        let bar = best_total.saturating_sub(w);
        let candidates = engine_candidates(anchor_w, bar);
        println!(
            "  w={w}: anchor {:#010x} ({note}), {} plans could beat {bar}",
            anchor_w.0,
            candidates.len()
        );
        for (predicted, plan) in candidates.iter().take(top_plans.min(8)) {
            let (won, frames) = run_plan(&mut emu, &observer, &start_w, &mash, plan);
            let verdict = if !won {
                "LOSS".to_string()
            } else {
                format!("{} (total {})", frames, frames + w)
            };
            println!("    predicted {predicted} plan {plan:?} -> real {verdict}");
            if won && frames + w < best_total {
                best_total = frames + w;
                best_desc = format!("wait {w}, plan {plan:?}");
            }
        }
    }

    println!(
        "\nbest real battle: {best_total} frames from the battle-start frame \
         ({best_desc}); committed was {committed_frames}"
    );
}
