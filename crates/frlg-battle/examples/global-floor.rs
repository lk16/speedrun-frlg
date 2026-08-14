//! The fastest battle this fight can *ever* play, and how common it is:
//! sample the 2^32 battle-start states with `engine::simulate`, rank the
//! distinct best-leaf frame counts, then hand the fastest classes to the
//! constraint solver for their exact satisfying-state counts.
//!
//! Plans: d0 in 0..5 (frame-free -- `pacing::INTRO_PRETURN`: start delays
//! collapse mod 5 with identical run frames, so the five residues are five
//! distinct battles at equal cost) crossed with turn delays in {0, 4}. A
//! turn delay costs its frames 1:1 and shifts the stream -- which anchor
//! choice covers for free -- but delay 4 also unlocks the 8-frame commit
//! gate (`pacing::commit_durations`: {8, 13, 18} for delay >= 4 vs {13, 18}
//! below), a net -1 frame per turn *if* the gate cooperates. Larger delays
//! buy nothing a cheaper delay plus a different anchor does not.
//!
//! The best-leaf number is optimistic per plan (gates must cooperate;
//! `engine` enumerates them and only the emulator arbitrates), so the floor
//! printed here is "no manipulation can do better than this", not "this is
//! achievable" -- the achievable check is the wait scan + emulator, which
//! is the next tool over (`wait-scan`).
//!
//!     cargo run --release -p frlg-battle --example global-floor [-- LOG2_SAMPLES]

use std::collections::BTreeMap;

use frlg_battle::engine::{simulate, SimResult};
use frlg_battle::{trace, Mon};
use frlg_rng::Rng;

/// gBattleMons for the committed route's battle (measured; battle-truth).
fn mons() -> (Mon, Mon) {
    let us = Mon {
        hp: 20,
        max_hp: 20,
        attack: 11,
        defense: 10,
        speed: 11,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    };
    let rival = Mon {
        hp: 18,
        max_hp: 18,
        attack: 11,
        defense: 9,
        speed: 9,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    };
    (us, rival)
}

/// The floor's plan grid: five start residues crossed with {0, 4} at each
/// of three turns (a fourth turn cannot be part of a floor battle: three
/// turns is the minimum kill and every extra turn costs a full turn cycle).
fn plans() -> Vec<Vec<u32>> {
    let mut out = Vec::new();
    for d0 in 0..5u32 {
        for d1 in [0u32, 4] {
            for d2 in [0u32, 4] {
                for d3 in [0u32, 4] {
                    out.push(vec![d0, d1, d2, d3]);
                }
            }
        }
    }
    out
}

/// Best winning leaf over the plan grid, with the plan and gates that
/// achieved it.
fn best_battle(
    plans: &[Vec<u32>],
    anchor: Rng,
    us: Mon,
    rival: Mon,
) -> Option<(u32, Vec<u32>, Vec<u32>)> {
    let mut best: Option<(u32, Vec<u32>, Vec<u32>)> = None;
    for plan in plans {
        for leaf in simulate(plan, anchor, us, rival) {
            if let SimResult::Win { frames } = leaf.result {
                if best.as_ref().is_none_or(|(b, ..)| frames < *b) {
                    best = Some((frames, plan.clone(), leaf.commit_durs));
                }
            }
        }
    }
    best
}

/// A bijective bit-mixer (murmur3 finalizer): index -> sample state. The
/// first cut of this scan used plain `i * ODD`, and that sequence correlates
/// with the constraints' own affine maps badly enough to bias class
/// densities several-fold (measured against `count_all`, which this
/// example's exact section now double-checks).
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
        .unwrap_or(22);
    let samples: u64 = 1 << log2;
    let (us, rival) = mons();
    let grid = plans();

    let threads = std::thread::available_parallelism().map_or(8, |n| n.get());
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
                        if let Some((frames, plan, durs)) = best_battle(grid, anchor, us, rival) {
                            local
                                .entry(frames)
                                .and_modify(|e| e.0 += 1)
                                .or_insert((1, anchor, plan, durs));
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
    let elapsed = t0.elapsed();

    let total_best: u64 = hist.values().map(|(n, ..)| n).sum();
    println!(
        "{samples} anchors sampled in {elapsed:.1?}: {total_best} have a winning leaf \
         ({:.1}%); {} distinct best-leaf frame counts",
        100.0 * total_best as f64 / samples as f64,
        hist.len(),
    );
    println!("\nfastest best-leaf classes (sample count, density, example):");
    for (frames, (n, anchor, plan, durs)) in hist.iter().take(12) {
        println!(
            "  {frames}  n={n:<6} d={:.2e}  anchor {:#010x} plan {plan:?} gates {durs:?}",
            *n as f64 / samples as f64,
            anchor.0,
        );
    }
    let committed: Vec<(u32, u64)> = hist
        .iter()
        .filter(|(f, _)| (2405..=2415).contains(*f))
        .map(|(f, (n, ..))| (*f, *n))
        .collect();
    println!("\naround the committed 2409: {committed:?}");

    // Exact solver answer for the fastest classes: extract the example
    // leaf's constraints and count every satisfying state in 2^32.
    println!("\nexact counts for the fastest classes (constraint solver, full 2^32):");
    for (frames, (_, anchor, plan, durs)) in hist.iter().take(4) {
        match trace::extract_leaf(plan, durs, *anchor, us, rival) {
            Ok(tr) => {
                let set = frlg_rng::constraint::ConstraintSet::new(&tr.constraints);
                let count = set.count_all(threads);
                println!(
                    "  {frames} (leaf candidates {:?}): {} constraints over {} calls, \
                     {count} states satisfy ({:.3e} exact), rival moves {:?}",
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
}
