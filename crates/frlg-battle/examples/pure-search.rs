//! The battle search with no emulator in the loop: enumerate delay plans,
//! simulate every gate leaf of each, and print the candidates an emulator
//! run would have to arbitrate -- plus how much of the space the engine
//! prunes outright (every leaf loses, or no leaf beats the committed 2409).
//!
//! Start delays collapse mod 5 (`pacing::INTRO_PRETURN`), so stage 1 is five
//! plans, not sixty-four. The full grid here -- five start classes times
//! four turn delays 0..8 -- is ~26k battles, which the emulator prices at
//! ~18 hours and this enumeration at well under a minute.
//!
//!     cargo run --release -p frlg-battle --example pure-search [-- RNG_HEX]
//!
//! RNG_HEX is the battle-start `gRngValue` (default: the committed route's,
//! as printed by fit-pacing).

use frlg_battle::engine::{simulate, SimResult};
use frlg_battle::Mon;
use frlg_rng::Rng;

fn main() {
    let start = Rng(std::env::args()
        .nth(1)
        .map(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).expect("RNG_HEX"))
        .unwrap_or(0xed94271d));

    // gBattleMons for the committed route's battle (measured; battle-truth).
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

    let mut plans: Vec<Vec<u32>> = Vec::new();
    for d0 in 0..5u32 {
        for d1 in (0..9).step_by(2) {
            for d2 in (0..9).step_by(2) {
                for d3 in (0..9).step_by(2) {
                    for d4 in (0..9).step_by(2) {
                        plans.push(vec![d0, d1, d2, d3, d4]);
                    }
                }
            }
        }
    }

    let t0 = std::time::Instant::now();
    let mut candidates: Vec<(u32, u32, Vec<u32>)> = Vec::new(); // (best, worst-win, plan)
    let mut all_lose = 0usize;
    let mut unmodelled = 0usize;
    for plan in &plans {
        let leaves = simulate(plan, start, us, rival);
        let mut best: Option<u32> = None;
        let mut worst: Option<u32> = None;
        let mut escaped = false;
        for leaf in &leaves {
            match leaf.result {
                SimResult::Win { frames } => {
                    best = Some(best.map_or(frames, |b: u32| b.min(frames)));
                    worst = Some(worst.map_or(frames, |w: u32| w.max(frames)));
                }
                SimResult::Loss => {}
                SimResult::Unmodelled(_) => escaped = true,
            }
        }
        if escaped {
            unmodelled += 1;
        }
        match best {
            None if !escaped => all_lose += 1,
            Some(b) => candidates.push((b, worst.unwrap(), plan.clone())),
            None => {}
        }
    }
    let elapsed = t0.elapsed();

    candidates.sort();
    println!(
        "{} plans enumerated in {elapsed:.2?}: {} lose on every leaf, \
         {} have a winning leaf, {} left the model's vocabulary",
        plans.len(),
        all_lose,
        candidates.len(),
        unmodelled,
    );
    println!(
        "\nbest candidates (frames if the gates cooperate / if they don't; \
         only an emulator replay decides which):"
    );
    for (best, worst, plan) in candidates.iter().take(12) {
        println!("  {best} / {worst}  {plan:?}");
    }
    let committed = simulate(&[4, 3, 3, 3], start, us, rival);
    let anchor = committed
        .iter()
        .filter_map(|l| match l.result {
            SimResult::Win { frames } => Some(frames),
            _ => None,
        })
        .min();
    println!("\nanchor: committed [4, 3, 3, 3] best winning leaf = {anchor:?} (route: 2409)");
}
