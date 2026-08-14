//! The constraint solver + engine put to work on the defeat-brock route's
//! rival battle (Squirtle vs Bulbasaur, the committed 2613 at plan
//! [0, 217, 0, 0]): what is the fastest this fight can ever play, which
//! start states play it, and can the committed anchor -- or a cheap
//! pre-battle wait -- reach one for real?
//!
//! Phases, each printed with its evidence:
//!
//! 1. **Global floor** (model only): sample the 2^32 battle-start space
//!    with `engine::simulate_with` over the cheap plan grid (d0 in 0..5 --
//!    the intro's five press-phase streams; menu delays 0: every larger
//!    delay's best commit total ties or loses, `pacing::SQUIRTLE_LAB`'s
//!    gate sets), then hand the fastest classes to
//!    `trace::extract_leaf_with` + `ConstraintSet::count_all` for exact
//!    densities.
//! 2. **On-anchor scan + arbitration** (emulator): dense plan grid from
//!    the committed anchor, every plan whose best leaf beats the committed
//!    battle replayed for real under the `win_battle` drive.
//! 3. **Pre-battle waits** (emulator): w idles at the head of
//!    `08-battle-start`, the *measured* anchor each reaches (overworld
//!    rolls are not 1:1 with frames once scripted events move), the
//!    engine's candidates on that anchor, and the promising ones replayed.
//!
//! Adoption stays with the emulator: a candidate is only real once it
//! replays faster from the actual reachable state (the rival-1 lesson:
//! gates resolve 3-5-frame margins for real, so the model enumerates and
//! orders, never decides).
//!
//!     cargo run --release -p frlg-battle --example squirtle-solver \
//!         [-- LOG2_SAMPLES [MAX_WAIT [TOP_PLANS]]]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use frlg_battle::engine::{simulate_with, SimResult};
use frlg_battle::{pacing, trace, Mon, Move};
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

const WON: u8 = 1;
const FRAME_BUDGET: u32 = 20_000;

/// gBattleMons for the committed route's battle (measured; battle-truth
/// with `FRLG_LEDGER=route/defeat-brock/ledger.json`, and re-checked by
/// `tests/squirtle_committed_battle.rs` on every `cargo test`).
fn mons() -> (Mon, Mon) {
    (
        Mon {
            hp: 20,
            max_hp: 20,
            attack: 10,
            defense: 11,
            speed: 10,
            level: 5,
            atk_stage: 6,
            def_stage: 6,
        },
        Mon {
            hp: 19,
            max_hp: 19,
            attack: 9,
            defense: 9,
            speed: 9,
            level: 5,
            atk_stage: 6,
            def_stage: 6,
        },
    )
}

fn simulate_squirtle(plan: &[u32], anchor: Rng) -> Vec<frlg_battle::engine::Leaf> {
    let (us, rival) = mons();
    simulate_with(plan, anchor, us, rival, Move::Tackle, &pacing::SQUIRTLE_LAB)
}

/// The `win_battle` drive, minimally: `plan[0]` idles, B-mash to the first
/// action menu, then per menu k: idle `plan[k]`, A-mash through the commit
/// and the resolution. Exactly the drive the pacing was fitted under
/// (fit-pacing with FRLG_DRIVE=menu), which `tests/squirtle_committed_battle.rs`
/// shows agrees gate-for-gate with the committed `win_battle` log.
fn menu_run_plan(
    emu: &mut Emu,
    observer: &Observer,
    start: &SaveState,
    intro_mash: &[u16],
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
    loop {
        let phase_mash = if turns == 0 { intro_mash } else { mash };
        let mut mash_phase = 0usize;
        loop {
            emu.step(phase_mash[mash_phase % phase_mash.len()]);
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
            return (outcome == WON, frames);
        }
        if frames >= FRAME_BUDGET {
            return (false, frames);
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
            return (outcome == WON, frames);
        }
        if frames >= FRAME_BUDGET {
            return (false, frames);
        }
    }
}

/// One scanned plan: its best winning leaf, and -- when every leaf of the
/// plan is a win -- its worst, which is the frame count the plan
/// guarantees no matter how the gates resolve (up to model correctness).
#[derive(Clone, Debug)]
struct Candidate {
    best: u32,
    /// `Some(worst)` only when all leaves are wins.
    guaranteed: Option<u32>,
    plan: Vec<u32>,
}

/// Engine scan of one anchor: every plan in the dense grid whose best
/// winning leaf is strictly below `bar`. d1 spans the stage-1 range (the
/// committed 217 sits there); d2/d3 the stage-2 range. Threaded over d1.
fn engine_candidates(anchor: Rng, bar: u32, threads: usize) -> Vec<Candidate> {
    let d1_max: u32 = 256;
    let next = AtomicUsize::new(0);
    let out: Mutex<Vec<Candidate>> = Mutex::new(Vec::new());
    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| {
                let mut local: Vec<Candidate> = Vec::new();
                loop {
                    let d1 = next.fetch_add(1, Ordering::Relaxed) as u32;
                    if d1 > d1_max {
                        break;
                    }
                    for d0 in 0..5u32 {
                        for d2 in 0..=24u32 {
                            for d3 in 0..=24u32 {
                                let plan = [d0, d1, d2, d3];
                                let mut best: Option<u32> = None;
                                let mut worst: Option<u32> = None;
                                let mut all_win = true;
                                for leaf in simulate_squirtle(&plan, anchor) {
                                    match leaf.result {
                                        SimResult::Win { frames } => {
                                            best = Some(best.map_or(frames, |b| b.min(frames)));
                                            worst = Some(worst.map_or(frames, |w| w.max(frames)));
                                        }
                                        _ => all_win = false,
                                    }
                                }
                                if let Some(b) = best {
                                    if b < bar {
                                        local.push(Candidate {
                                            best: b,
                                            guaranteed: worst.filter(|_| all_win),
                                            plan: plan.to_vec(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                out.lock().unwrap().append(&mut local);
            });
        }
    });
    let mut out = out.into_inner().unwrap();
    out.sort_by_key(|c| (c.best, c.plan.clone()));
    out
}

/// The arbitration list for one anchor: the strongest `guaranteed` plans
/// first (their whole gate envelope beats the bar -- the rival-1 lesson is
/// that gates eat 3-5-frame best-leaf margins, and a guaranteed plan does
/// not care), then the best optimistic plans.
fn arbitration_order(candidates: &[Candidate], bar: u32, top: usize) -> Vec<(String, Vec<u32>)> {
    let mut picked: Vec<(String, Vec<u32>)> = Vec::new();
    let mut guaranteed: Vec<&Candidate> = candidates
        .iter()
        .filter(|c| c.guaranteed.is_some_and(|g| g < bar))
        .collect();
    guaranteed.sort_by_key(|c| (c.guaranteed.unwrap(), c.best));
    for c in guaranteed.iter().take(top / 2) {
        picked.push((
            format!("guaranteed {} (best {})", c.guaranteed.unwrap(), c.best),
            c.plan.clone(),
        ));
    }
    for c in candidates.iter() {
        if picked.len() >= top {
            break;
        }
        if picked.iter().any(|(_, p)| *p == c.plan) {
            continue;
        }
        picked.push((format!("best {}", c.best), c.plan.clone()));
    }
    picked
}

/// Murmur3-finalizer bit-mixer: index -> sample state, bijective. Plain
/// `i * odd` correlates with the constraints' affine maps (rival-1's
/// global-floor measured a several-fold density bias); the mixer does not.
fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x85eb_ca6b);
    x ^= x >> 13;
    x = x.wrapping_mul(0xc2b2_ae35);
    x ^ (x >> 16)
}

fn main() {
    let log2: u32 = std::env::args()
        .nth(1)
        .map(|s| s.parse().expect("LOG2_SAMPLES"))
        .unwrap_or(21);
    let max_wait: u32 = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("MAX_WAIT"))
        .unwrap_or(48);
    let top_plans: usize = std::env::args()
        .nth(3)
        .map(|s| s.parse().expect("TOP_PLANS"))
        .unwrap_or(24);
    assert!(pacing::SQUIRTLE_LAB_FITTED);
    let (us, rival) = mons();
    let threads = std::thread::available_parallelism().map_or(8, |n| n.get());

    // ---- Phase 1: the global floor. ----
    let samples: u64 = 1 << log2;
    let grid: Vec<Vec<u32>> = (0..5u32).map(|d0| vec![d0, 0, 0, 0]).collect();
    let chunk = samples.div_ceil(threads as u64);
    let t0 = std::time::Instant::now();
    let mut hist: BTreeMap<u32, (u64, Rng, Vec<u32>, Vec<u32>)> = BTreeMap::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..threads)
            .map(|t| {
                let grid = &grid;
                scope.spawn(move || {
                    let mut local: BTreeMap<u32, (u64, Rng, Vec<u32>, Vec<u32>)> = BTreeMap::new();
                    let lo = t as u64 * chunk;
                    for i in lo..(lo + chunk).min(samples) {
                        let anchor = Rng(mix(i as u32));
                        let mut best: Option<(u32, &Vec<u32>, Vec<u32>)> = None;
                        for plan in grid {
                            for leaf in simulate_squirtle(plan, anchor) {
                                if let SimResult::Win { frames } = leaf.result {
                                    if best.as_ref().is_none_or(|(b, ..)| frames < *b) {
                                        best = Some((frames, plan, leaf.commit_durs));
                                    }
                                }
                            }
                        }
                        if let Some((frames, plan, durs)) = best {
                            local.entry(frames).and_modify(|e| e.0 += 1).or_insert((
                                1,
                                anchor,
                                plan.clone(),
                                durs,
                            ));
                        }
                    }
                    local
                })
            })
            .collect();
        for h in handles {
            for (frames, (n, anchor, plan, durs)) in h.join().expect("scan thread") {
                hist.entry(frames)
                    .and_modify(|e| e.0 += n)
                    .or_insert((n, anchor, plan, durs));
            }
        }
    });
    let total_best: u64 = hist.values().map(|(n, ..)| n).sum();
    println!(
        "phase 1: {samples} anchors in {:.1?}: {total_best} have a winning leaf ({:.1}%), \
         {} distinct best-leaf frame counts",
        t0.elapsed(),
        100.0 * total_best as f64 / samples as f64,
        hist.len(),
    );
    println!("fastest best-leaf classes (sample count, density, example):");
    for (frames, (n, anchor, plan, durs)) in hist.iter().take(12) {
        println!(
            "  {frames}  n={n:<6} d={:.2e}  anchor {:#010x} plan {plan:?} gates {durs:?}",
            *n as f64 / samples as f64,
            anchor.0,
        );
    }
    println!("\nexact counts for the fastest classes (constraint solver, full 2^32):");
    for (frames, (_, anchor, plan, durs)) in hist.iter().take(4) {
        match trace::extract_leaf_with(
            plan,
            durs,
            *anchor,
            us,
            rival,
            Move::Tackle,
            &pacing::SQUIRTLE_LAB,
        ) {
            Ok(tr) => {
                let set = frlg_rng::constraint::ConstraintSet::new(&tr.constraints);
                let count = set.count_all(threads);
                println!(
                    "  {frames} (leaf candidates {:?}): {} constraints over {} calls, \
                     {count} states ({:.3e} exact), rival moves {:?}",
                    tr.frame_candidates,
                    tr.constraints.len(),
                    tr.total_calls,
                    count as f64 / 2f64.powi(32),
                    tr.rival_moves,
                );
            }
            Err(e) => println!("  {frames}: extraction refused: {e}"),
        }
    }

    // ---- Emulator setup for phases 2 and 3. ----
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/defeat-brock/ledger.json"))
        .expect("committed ledger");
    assert_eq!(ledger.starter, "squirtle");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    let mut state07: Option<SaveState> = None;
    let mut log08: Vec<u16> = Vec::new();
    let mut committed_frames_09 = 0u32;
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            committed_frames_09 = entry.frames as u32;
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
        }
    }
    let state07 = state07.expect("08-battle-start in the ledger");
    assert!(
        observer.in_battle(&mut emu),
        "replay must end in the battle"
    );
    let anchor0 = Rng(emu.read32(rng_addr));
    let start0 = emu.save_state().expect("state at battle start");
    println!(
        "\nbattle-start anchor {:#010x}; committed 09-battle-win is {committed_frames_09} frames",
        anchor0.0
    );

    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };
    let intro_mash: Vec<u16> = {
        let mut m = vec![keys::B; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };

    // Re-measure the committed plan rather than trusting the ledger.
    let committed_plan: Vec<u32> = vec![0, 217, 0, 0];
    let (won, committed_replay) = menu_run_plan(
        &mut emu,
        &observer,
        &start0,
        &intro_mash,
        &mash,
        &committed_plan,
    );
    assert!(won, "the committed plan must still win");
    assert_eq!(committed_replay, committed_frames_09, "drive parity");
    let mut best_total = committed_replay;
    let mut best_desc = format!("committed {committed_plan:?}");

    // ---- Phase 2: on-anchor dense scan, then arbitration. ----
    let t2 = std::time::Instant::now();
    let candidates = engine_candidates(anchor0, best_total, threads);
    let guaranteed_n = candidates
        .iter()
        .filter(|c| c.guaranteed.is_some_and(|g| g < best_total))
        .count();
    println!(
        "\nphase 2: {} plans with a winning leaf below {best_total} on the committed anchor, \
         {guaranteed_n} with their whole gate envelope below it (scan {:.1?}); arbitrating {}:",
        candidates.len(),
        t2.elapsed(),
        top_plans.min(candidates.len()),
    );
    // Arbitrate in parallel: candidates are independent deterministic
    // replays from start0.
    let picked = arbitration_order(&candidates, best_total, top_plans);
    let results: Mutex<Vec<(usize, bool, u32)>> = Mutex::new(Vec::new());
    let next = AtomicUsize::new(0);
    std::thread::scope(|s| {
        for _ in 0..threads.min(picked.len()) {
            s.spawn(|| {
                let Ok(mut emu) = Emu::new(&rom) else { return };
                let Ok(b) = frlg_emu::boot_with_default_bios(&mut emu) else {
                    return;
                };
                if b != ledger.bios {
                    return;
                }
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some((_, plan)) = picked.get(i) else {
                        break;
                    };
                    let (won, frames) =
                        menu_run_plan(&mut emu, &observer, &start0, &intro_mash, &mash, plan);
                    results.lock().unwrap().push((i, won, frames));
                }
            });
        }
    });
    let mut measured = results.into_inner().unwrap();
    measured.sort();
    for (i, won, frames) in &measured {
        let (predicted, plan) = &picked[*i];
        let verdict = if !won {
            "LOSS".to_string()
        } else {
            format!("{frames}")
        };
        println!("  predicted {predicted} plan {plan:?} -> real {verdict}");
        if *won && *frames < best_total {
            best_total = *frames;
            best_desc = format!("wait 0, plan {plan:?}");
        }
    }

    // ---- Phase 3: pre-battle waits. ----
    println!("\nphase 3: pre-battle waits (w idles at the head of 08-battle-start):");
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
            format!("jump(w)+{}", clean.distance_to(anchor_w))
        };
        let bar = best_total.saturating_sub(w);
        let candidates = engine_candidates(anchor_w, bar, threads);
        let guaranteed_n = candidates
            .iter()
            .filter(|c| c.guaranteed.is_some_and(|g| g < bar))
            .count();
        println!(
            "  w={w}: anchor {:#010x} ({note}), {} plans could beat {bar} ({guaranteed_n} \
             gate-guaranteed)",
            anchor_w.0,
            candidates.len()
        );
        for (predicted, plan) in arbitration_order(&candidates, bar, top_plans.min(8)) {
            let (won, frames) =
                menu_run_plan(&mut emu, &observer, &start_w, &intro_mash, &mash, &plan);
            let verdict = if !won {
                "LOSS".to_string()
            } else {
                format!("{frames} (total {})", frames + w)
            };
            println!("    predicted {predicted} plan {plan:?} -> real {verdict}");
            if won && frames + w < best_total {
                best_total = frames + w;
                best_desc = format!("wait {w}, plan {plan:?}");
            }
        }
    }

    println!(
        "\nbest real battle: {best_total} frames from the battle-start frame ({best_desc}); \
         committed was {committed_replay}"
    );
}
