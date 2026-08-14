//! The committed rival battle, inverted and benchmarked: extract its roll
//! constraints (`frlg_battle::trace`, correctness tested in
//! `tests/trace_vs_engine.rs`), solve for start states, and race the solver
//! shapes against each other and against `engine::simulate` used as a
//! predicate.
//!
//!     cargo run --release -p frlg-battle --example constraint-solver

use std::hint::black_box;
use std::time::Instant;

use frlg_battle::trace::extract_leaf;
use frlg_battle::{engine, Mon};
use frlg_rng::constraint::{ConstraintSet, Pred};
use frlg_rng::Rng;

/// The committed battle's anchor `gRngValue` (fit-pacing, and the
/// `engine_reproduces_the_committed_battle` unit test).
const ANCHOR: Rng = Rng(0xed94271d);
const PLAN: [u32; 4] = [4, 3, 3, 3];
/// The committed commit-gate durations.
const GATES: [u32; 3] = [13, 13, 13];
const COMMITTED_FRAMES: u32 = 2409;

/// Mons as `gBattleMons` holds them at battle start (measured on the
/// committed battle; same values as frlg-battle's unit tests).
fn bulbasaur() -> Mon {
    Mon {
        hp: 20,
        max_hp: 20,
        attack: 11,
        defense: 10,
        speed: 11,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    }
}

fn charmander() -> Mon {
    Mon {
        hp: 18,
        max_hp: 18,
        attack: 11,
        defense: 9,
        speed: 9,
        level: 5,
        atk_stage: 6,
        def_stage: 6,
    }
}

/// Does `engine::simulate` from this anchor reproduce the committed leaf?
fn engine_reproduces(anchor: Rng) -> bool {
    engine::simulate(&PLAN, anchor, bulbasaur(), charmander())
        .iter()
        .any(|l| {
            l.commit_durs == GATES
                && l.result
                    == engine::SimResult::Win {
                        frames: COMMITTED_FRAMES,
                    }
        })
}

fn main() {
    // ---- Extraction (validated in tests/trace_vs_engine.rs) -------------
    let trace = extract_leaf(&PLAN, &GATES, ANCHOR, bulbasaur(), charmander())
        .expect("the committed leaf is a modelled win");
    let set = ConstraintSet::new(&trace.constraints);
    assert!(set.satisfied(ANCHOR), "the committed anchor must satisfy");
    println!(
        "extracted {} constraints from the committed battle ({} rolls consumed, \
         rival played {:?}, frame candidates {:?})",
        trace.constraints.len(),
        trace.total_calls,
        trace.rival_moves,
        trace.frame_candidates,
    );
    println!(
        "modeled density {:.3e} (~{:.0} of 2^32 states)",
        set.density(),
        set.density() * 2f64.powi(32)
    );

    // How strict is exact-trace pinning? Count the window's engine
    // reproductions next to the constraint hits.
    const WINDOW: u32 = 1 << 14;
    let hits = set.wait_hits(ANCHOR, 1, WINDOW);
    let engine_hits = (0..WINDOW)
        .filter(|&w| engine_reproduces(ANCHOR.jump(w)))
        .count();
    println!(
        "wait scan over {WINDOW} frames (stride 1): {} constraint hits, {engine_hits} \
         engine reproductions (exact-trace is sufficient, not necessary)",
        hits.len(),
    );

    // ---- Alternative checker shapes, built from the same compilation ----
    let compiled = set.compiled().to_vec();

    // Dyn: one boxed closure per constraint -- the fully generic shape.
    type DynCheck = (u32, u32, Box<dyn Fn(u16) -> bool + Sync>);
    let dyn_checks: Vec<DynCheck> = compiled
        .iter()
        .map(|k| {
            let pred = k.pred;
            (
                k.a,
                k.c,
                Box::new(move |roll: u16| pred.passes(roll)) as Box<dyn Fn(u16) -> bool + Sync>,
            )
        })
        .collect();
    let dyn_satisfied = |s: u32| {
        dyn_checks
            .iter()
            .all(|(a, c, f)| f((a.wrapping_mul(s).wrapping_add(*c) >> 16) as u16))
    };

    // Dedicated: the battle's constraints flattened to bare arrays, no enum
    // dispatch -- what a per-scenario hand-written checker compiles down to.
    struct Soa {
        a: Vec<u32>,
        c: Vec<u32>,
        m: Vec<u16>,
        lo: Vec<u16>,
        hi: Vec<u16>,
    }
    let soa = {
        let mut soa = Soa {
            a: vec![],
            c: vec![],
            m: vec![],
            lo: vec![],
            hi: vec![],
        };
        for k in &compiled {
            let Pred::ModRange { m, lo, hi } = k.pred else {
                panic!("battle extraction only emits ModRange");
            };
            soa.a.push(k.a);
            soa.c.push(k.c);
            soa.m.push(m);
            soa.lo.push(lo);
            soa.hi.push(hi);
        }
        soa
    };
    let soa_satisfied = |s: u32| {
        (0..soa.a.len()).all(|i| {
            let r = ((soa.a[i].wrapping_mul(s).wrapping_add(soa.c[i]) >> 16) as u16) % soa.m[i];
            soa.lo[i] <= r && r <= soa.hi[i]
        })
    };

    // The three checkers agree everywhere they are sampled.
    for w in 0..WINDOW {
        let s = ANCHOR.jump(w).0;
        assert_eq!(set.satisfied(Rng(s)), dyn_satisfied(s));
        assert_eq!(set.satisfied(Rng(s)), soa_satisfied(s));
    }

    // ---- Benchmarks -----------------------------------------------------
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("\nbenchmarks ({threads} threads where parallel):");

    // Wait scans: single-thread, the shape a route search runs at a battle
    // arrival ("how long do I idle here").
    const SCAN: u32 = 1 << 22;
    let bench_scan = |name: &str, f: &dyn Fn() -> u32| {
        let start = Instant::now();
        let hits = f();
        let per = start.elapsed().as_nanos() as f64 / SCAN as f64;
        println!("  wait-scan {SCAN} frames | {name:<10} | {per:6.2} ns/wait | {hits} hits");
        hits
    };
    let (sa, sc) = Rng::jump_coeffs(1);
    let h_enum = bench_scan("enum", &|| {
        let mut hits = 0u32;
        let mut s = ANCHOR.0;
        for _ in 0..SCAN {
            hits += set.satisfied(Rng(s)) as u32;
            s = sa.wrapping_mul(s).wrapping_add(sc);
        }
        black_box(hits)
    });
    let h_dyn = bench_scan("dyn", &|| {
        let mut hits = 0u32;
        let mut s = ANCHOR.0;
        for _ in 0..SCAN {
            hits += dyn_satisfied(s) as u32;
            s = sa.wrapping_mul(s).wrapping_add(sc);
        }
        black_box(hits)
    });
    let h_soa = bench_scan("dedicated", &|| {
        let mut hits = 0u32;
        let mut s = ANCHOR.0;
        for _ in 0..SCAN {
            hits += soa_satisfied(s) as u32;
            s = sa.wrapping_mul(s).wrapping_add(sc);
        }
        black_box(hits)
    });
    assert!(h_enum == h_dyn && h_enum == h_soa);

    // The engine as the predicate: the fully general fallback that needs no
    // extraction at all -- it enumerates gate leaves and re-decides pacing,
    // so it prices the whole plan, not one leaf.
    {
        const ENGINE_SCAN: u32 = 1 << 14;
        let start = Instant::now();
        let mut wins = 0u32;
        for w in 0..ENGINE_SCAN {
            let leaves = engine::simulate(&PLAN, ANCHOR.jump(w), bulbasaur(), charmander());
            wins += leaves
                .iter()
                .any(|l| matches!(l.result, engine::SimResult::Win { .. }))
                as u32;
        }
        let per = start.elapsed().as_nanos() as f64 / ENGINE_SCAN as f64;
        println!(
            "  wait-scan {ENGINE_SCAN} frames | {:<10} | {per:6.0} ns/wait | {wins} win-any \
             (all leaves, all gates)",
            "engine"
        );
    }

    // Exhaustive 2^32: which start states satisfy the committed trace.
    let start = Instant::now();
    let n = set.count_all(threads);
    let t_enum = start.elapsed();
    println!(
        "  exhaustive 2^32     | {:<10} | {t_enum:8.2?} | {n} states (density {:.3e}, modeled {:.3e})",
        "enum",
        n as f64 / 2f64.powi(32),
        set.density(),
    );
    let par_count = |check: &(dyn Fn(u32) -> bool + Sync)| -> u64 {
        let chunk = (1u64 << 32).div_ceil(threads as u64);
        let mut total = 0u64;
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..threads as u64)
                .map(|t| {
                    let lo = t * chunk;
                    let count = chunk.min((1u64 << 32) - lo);
                    scope.spawn(move || {
                        let mut n = 0u64;
                        let mut s = lo as u32;
                        for _ in 0..count {
                            n += check(s) as u64;
                            s = s.wrapping_add(1);
                        }
                        n
                    })
                })
                .collect();
            for h in handles {
                total += h.join().expect("bench thread");
            }
        });
        total
    };
    let start = Instant::now();
    let n_soa = par_count(&soa_satisfied);
    let t_soa = start.elapsed();
    println!(
        "  exhaustive 2^32     | {:<10} | {t_soa:8.2?} | {n_soa} states (no incremental trick)",
        "dedicated"
    );
    let start = Instant::now();
    let n_dyn = par_count(&dyn_satisfied);
    let t_dyn = start.elapsed();
    println!(
        "  exhaustive 2^32     | {:<10} | {t_dyn:8.2?} | {n_dyn} states",
        "dyn"
    );
    assert!(n == n_soa && n == n_dyn);

    println!(
        "\nskip-the-battle numbers a route search would use: advance the stream by \
         {} calls, charge {} frames, apply the trace's damage/PP deltas.",
        trace.total_calls, COMMITTED_FRAMES
    );
}
