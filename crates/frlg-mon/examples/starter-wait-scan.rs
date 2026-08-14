//! Starter manipulation as a wait scan: how many frames to idle at the ball
//! so `givemon`'s 4 rolls produce the genome you want -- and why this
//! scenario wants a *state predicate*, not per-roll constraints.
//!
//! The gift path is exactly 4 `Random()` calls (`frlg_mon::create::gift_mon`,
//! citations there): PID low half, PID high half, IV word 1, IV word 2. The
//! quantities a route cares about do not factor into single rolls --
//! nature is `PID % 25` (a function of *both* PID rolls,
//! `decompiled/src/pokemon.c:5020-5023`) and each IV is a 5-bit *bitfield*
//! of an IV word (`:1836-1852`), which `roll % m` residue ranges cannot
//! express (an IV threshold is a range of `(roll >> 5) % 32`, not of
//! `roll % m`). So the creation scenario keeps a dedicated predicate: run
//! the 4-roll model from `anchor.jump(w)` and test the genome. At 4 rolls
//! per candidate it sits in the same nanosecond class as the battle
//! constraint sets; generic per-roll machinery would buy nothing here even
//! where it could express the wish.
//!
//!     cargo run --release -p frlg-mon --example starter-wait-scan

use std::hint::black_box;
use std::time::Instant;

use frlg_mon::create::{gift_mon, Genome};
use frlg_rng::Rng;

/// Adamant: +Atk -SpAtk (`NATURE_STAT_TABLE` row 3, `frlg_mon::stats`).
const ADAMANT: u8 = 3;

/// The wish: Adamant, Atk IV >= 25, Speed IV >= 25 -- the shape of a real
/// starter manipulation (density 1/25 * (7/32)^2 ~= 1.9e-3).
fn wanted(g: &Genome) -> bool {
    g.nature() == ADAMANT && g.ivs.atk >= 25 && g.ivs.spe >= 25
}

fn main() {
    // Any anchor works for the demo; a route would pass the modeled
    // `gRngValue` at the moment the ball can first be taken.
    let anchor = Rng(0xed94_271d);
    // The overworld advances the stream once per frame
    // (`decompiled/src/main.c:412`); a menu wait is a jump(1) per frame.
    let check = |w: u32| {
        let mut rng = anchor.jump(w);
        wanted(&gift_mon(&mut rng))
    };

    let first = (0..1 << 20).find(|&w| check(w));
    println!(
        "first satisfying wait: {first:?} frames (Adamant, Atk>=25, Spe>=25, from {:#010x})",
        anchor.0
    );

    const SCAN: u32 = 1 << 22;
    let start = Instant::now();
    let mut hits = 0u32;
    for w in 0..SCAN {
        hits += check(w) as u32;
    }
    black_box(hits);
    let per = start.elapsed().as_nanos() as f64 / SCAN as f64;
    println!(
        "wait-scan {SCAN} frames | dedicated genome predicate | {per:6.2} ns/wait | {hits} hits \
         (density {:.3e}, wished ~1.914e-3)",
        hits as f64 / SCAN as f64
    );

    // The same scan with the incremental-anchor trick the battle solver
    // uses (state advances by one affine step per wait instead of jump(w)):
    // measured ~5x faster -- the O(log w) jump dominates a 4-roll predicate.
    let start = Instant::now();
    let mut hits2 = 0u32;
    let mut s = anchor;
    for _ in 0..SCAN {
        let mut rng = s;
        hits2 += wanted(&gift_mon(&mut rng)) as u32;
        s = s.next();
    }
    black_box(hits2);
    let per2 = start.elapsed().as_nanos() as f64 / SCAN as f64;
    println!(
        "wait-scan {SCAN} frames | + incremental anchor       | {per2:6.2} ns/wait | {hits2} hits"
    );
    assert_eq!(hits, hits2);
}
