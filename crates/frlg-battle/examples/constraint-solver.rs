//! The committed rival battle, inverted: extract the roll constraints that
//! make it play out exactly as committed, solve for start states, and
//! benchmark the solver shapes against each other and against
//! `engine::simulate` used as a predicate.
//!
//! The extraction walks ONE leaf of the pacing model -- the committed one
//! (plan [4, 3, 3, 3], commit gates 13/13/13, `tests/committed_battle.rs`)
//! -- with a stream that records each roll's absolute call offset, and pins
//! every *decisive* roll to the residue class that reproduces its committed
//! outcome: AI viability branches (% 256 vs 50), the move tie-break parity
//! (when the scores actually tied), crit rolls (% 16, once enabled), and
//! damage variance rolls (the % 16 class giving the same damage -- damage
//! feeds the HP-bar drain, so pacing itself depends on it). Rolls whose
//! value cannot matter (suppressed crits, 100-accuracy checks, secondary
//! rolls, turn-end rolls, unused AI slots) get no constraint. Everything is
//! validated two ways: the committed anchor must satisfy the set, and every
//! wait-scan hit must make `engine::simulate` reproduce the committed
//! 2409-frame [13,13,13] win from that shifted anchor.
//!
//!     cargo run --release -p frlg-battle --example constraint-solver

use std::hint::black_box;
use std::time::Instant;

use frlg_battle::{apply_variance, base_damage, engine, pacing, Mon, Move};
use frlg_rng::constraint::{Constraint, ConstraintSet, Pred};
use frlg_rng::Rng;

/// The committed battle's anchor `gRngValue` (fit-pacing, and the
/// `engine_reproduces_the_committed_battle` unit test).
const ANCHOR: Rng = Rng(0xed94271d);
const PLAN: [u32; 4] = [4, 3, 3, 3];
/// The committed commit-gate duration, every turn.
const GATE: u32 = 13;
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

/// `engine::Stream` with the absolute call offset exposed: `calls` counts
/// every `Random()` since the anchor, so the roll this returns is the
/// `calls`-th call -- exactly `ConstraintSet`'s offset convention.
struct RecStream {
    rng: Rng,
    consumed: u32,
    calls: u32,
}

impl RecStream {
    fn roll_at(&mut self, frame: u32) -> (u32, u16) {
        assert!(frame + 1 >= self.consumed);
        let vblank = 2 * (frame + 1 - self.consumed);
        self.rng = self.rng.jump(vblank);
        self.consumed = frame + 1;
        self.calls += vblank + 1;
        (self.calls, self.rng.random())
    }
}

/// Pin `roll` (consumed at `offset`) to the residues mod `m` that `key`
/// maps to the same outcome as the committed residue. No constraint when
/// every residue agrees (the roll is not decisive); panics if the agreeing
/// class is not contiguous (never happens for these monotone formulas).
fn pin<K: PartialEq>(
    out: &mut Vec<Constraint>,
    offset: u32,
    m: u16,
    roll: u16,
    key: impl Fn(u16) -> K,
) {
    let committed = key(roll % m);
    let passing: Vec<u16> = (0..m).filter(|&r| key(r) == committed).collect();
    if passing.len() == m as usize {
        return;
    }
    let (lo, hi) = (passing[0], *passing.last().unwrap());
    assert_eq!(
        passing.len() as u16,
        hi - lo + 1,
        "non-contiguous residue class at offset {offset}"
    );
    out.push(Constraint {
        offset,
        pred: Pred::ModRange { m, lo, hi },
    });
}

/// What the extraction walked out of the committed leaf.
struct Trace {
    constraints: Vec<Constraint>,
    /// Both end-gate frame candidates; the committed 2409 is one of them.
    frame_candidates: [u32; 2],
    /// Total `Random()` calls from the anchor to the last modelled roll --
    /// what a route search would advance the stream by to skip the battle.
    total_calls: u32,
    rival_moves: Vec<Move>,
}

/// Walk the committed leaf (fixed plan, fixed gates), recording constraints.
/// This mirrors `engine::simulate`/`play_turn` restricted to one gate
/// combination; any drift from the engine is caught by the caller comparing
/// the outcome against `engine::simulate`'s committed leaf.
fn extract(anchor: Rng, us: &mut Mon, rival: &mut Mon) -> Trace {
    let mut stream = RecStream {
        rng: anchor,
        consumed: 0,
        calls: 0,
    };
    let mut constraints = Vec::new();
    let mut rival_moves = Vec::new();
    let mut crit_enabled = false;

    let start_delay = PLAN[0];
    let mut det = pacing::INTRO_PRETURN[start_delay as usize % 5];
    let _pre_turn = stream.roll_at(det);

    let mut turn = 0u32;
    loop {
        turn += 1;
        assert!(turn <= 16, "the committed battle has 3 turns");
        let delay = PLAN.get(turn as usize).copied().unwrap_or(0);

        // The AI block, all on one frame (engine::walk).
        let ai_frame = det + pacing::DET_TO_AI;
        let rival_move = {
            // rival_choose_move's exact consumption (frlg-battle root, with
            // its citations), with a pin at each branch a roll decides.
            let mut simulated = [0u16; 4];
            for (slot, sim) in simulated.iter_mut().enumerate() {
                let (offset, roll) = stream.roll_at(ai_frame);
                *sim = 100 - (roll % 16);
                if slot == 0 {
                    // The only simulatedRNG slot whose value reaches a
                    // branch: AI_TryToFaint scales Scratch's damage by it.
                    let (hp, base) = (us.hp, base_damage(rival, us, Move::Scratch, false));
                    pin(&mut constraints, offset, 16, roll, |r| {
                        hp as i32 <= (base * (100 - r as i32) / 100).max(1)
                    });
                }
            }
            let mut scratch_score = 100i32;
            let mut growl_score = 100i32;
            if us.atk_stage != 6 {
                growl_score -= 1;
                if 100 * rival.hp as u32 / rival.max_hp as u32 <= 90 {
                    growl_score -= 1;
                }
                if us.atk_stage <= 3 {
                    let (offset, roll) = stream.roll_at(ai_frame);
                    pin(&mut constraints, offset, 256, roll, |r| r >= 50);
                    if roll % 256 >= 50 {
                        growl_score -= 2;
                    }
                }
            }
            if 100 * us.hp as u32 / us.max_hp as u32 <= 70 {
                growl_score -= 2;
            }
            let (offset_b, roll_b) = stream.roll_at(ai_frame);
            pin(&mut constraints, offset_b, 256, roll_b, |r| r >= 50);
            if roll_b % 256 >= 50 {
                growl_score -= 2;
            }
            let sim_damage =
                (base_damage(rival, us, Move::Scratch, false) * simulated[0] as i32 / 100).max(1);
            if us.hp as i32 <= sim_damage {
                scratch_score += 4;
            }
            let (offset_tie, tie) = stream.roll_at(ai_frame);
            if scratch_score == growl_score {
                pin(&mut constraints, offset_tie, 2, tie, |r| r == 0);
                if tie.is_multiple_of(2) {
                    Move::Scratch
                } else {
                    Move::Growl
                }
            } else if scratch_score > growl_score {
                Move::Scratch
            } else {
                Move::Growl
            }
        };
        rival_moves.push(rival_move);

        // The committed gate, then the turn (engine::play_turn, one leaf).
        let lb = det + delay + 1;
        let loop_a = lb + GATE + 1;

        // Player Tackle: crit, damage, drain, trailing secondary.
        let pcrit_f = loop_a + pacing::LOOP_A_TO_PCRIT;
        let (offset, roll) = stream.roll_at(pcrit_f);
        let crit = roll.is_multiple_of(16) && crit_enabled;
        if crit_enabled {
            pin(&mut constraints, offset, 16, roll, |r| r == 0);
        }
        let pdmg_f = pcrit_f + pacing::PCRIT_TO_PDMG;
        let base = base_damage(us, rival, Move::Tackle, crit);
        let (offset, roll) = stream.roll_at(pdmg_f);
        pin(&mut constraints, offset, 16, roll, |r| {
            apply_variance(base, r)
        });
        let damage = apply_variance(base, roll);
        let delta = rival.hp.min(damage as u16);
        let drain = if crit_enabled {
            pacing::RHP_DRAIN[delta as usize]
        } else {
            pacing::rhp_drain_first(delta).expect("first-hit drain delta")
        };
        let rhp_f = pdmg_f + drain;
        rival.hp -= delta;
        crit_enabled = true;
        let psec_f = rhp_f
            + if crit {
                pacing::HP_TO_SEC_CRIT
            } else {
                pacing::HP_TO_SEC
            };
        let _ = stream.roll_at(psec_f); // secondary: burned, never read

        if rival.hp == 0 {
            let gaps = pacing::outcome_win_gaps((psec_f - loop_a) % 5).expect("observed phase");
            return Trace {
                constraints,
                frame_candidates: [psec_f + gaps[0] + 1, psec_f + gaps[1] + 1],
                total_calls: stream.calls,
                rival_moves,
            };
        }

        // The rival's answer.
        match rival_move {
            Move::Growl => {
                let racc_f = psec_f + pacing::PSEC_TO_RACC_GROWL;
                let _ = stream.roll_at(racc_f); // 100 accuracy: never decisive
                assert!(us.atk_stage > 0);
                us.atk_stage -= 1;
                let stagefall_f = racc_f + pacing::RACC_TO_STAGEFALL_FIRST;
                det = stagefall_f + pacing::STAGEFALL_FIRST_TO_TURNEND;
            }
            mv => {
                let racc_f = psec_f + pacing::PSEC_TO_RACC_SCRATCH;
                let _ = stream.roll_at(racc_f); // 100 accuracy
                let rcrit_f = racc_f + pacing::RACC_TO_RCRIT;
                let (offset, roll) = stream.roll_at(rcrit_f);
                let crit = roll.is_multiple_of(16) && crit_enabled;
                pin(&mut constraints, offset, 16, roll, |r| r == 0);
                let rdmg_f = rcrit_f + pacing::RCRIT_TO_RDMG;
                let base = base_damage(rival, us, mv, crit);
                let (offset, roll) = stream.roll_at(rdmg_f);
                pin(&mut constraints, offset, 16, roll, |r| {
                    apply_variance(base, r)
                });
                let damage = apply_variance(base, roll);
                let delta = us.hp.min(damage as u16);
                us.hp -= delta;
                assert!(us.hp > 0, "the committed battle is a win");
                let drain = pacing::uhp_drain(delta).expect("player drain delta");
                let uhp_f = rdmg_f + drain;
                let rsec_f = uhp_f
                    + if crit {
                        pacing::HP_TO_SEC_CRIT
                    } else {
                        pacing::HP_TO_SEC
                    };
                let _ = stream.roll_at(rsec_f);
                det = rsec_f + pacing::RSEC_TO_TURNEND;
            }
        }
        let _turn_end = stream.roll_at(det);
    }
}

/// Does `engine::simulate` from this anchor reproduce the committed leaf?
fn engine_reproduces(anchor: Rng) -> bool {
    engine::simulate(&PLAN, anchor, bulbasaur(), charmander())
        .iter()
        .any(|l| {
            l.commit_durs == [GATE; 3]
                && l.result
                    == engine::SimResult::Win {
                        frames: COMMITTED_FRAMES,
                    }
        })
}

fn main() {
    // ---- Extraction and validation --------------------------------------
    let (mut us, mut rival) = (bulbasaur(), charmander());
    let trace = extract(ANCHOR, &mut us, &mut rival);
    assert!(
        trace.frame_candidates.contains(&COMMITTED_FRAMES),
        "extractor's frames {:?} must include the committed {COMMITTED_FRAMES}",
        trace.frame_candidates
    );
    let set = ConstraintSet::new(&trace.constraints);
    assert!(set.satisfied(ANCHOR), "the committed anchor must satisfy");
    assert!(engine_reproduces(ANCHOR), "engine sanity");
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

    // Every wait-scan hit must reproduce the committed battle in the engine.
    const WINDOW: u32 = 1 << 14;
    let hits = set.wait_hits(ANCHOR, 1, WINDOW);
    let engine_hits: Vec<u32> = (0..WINDOW)
        .filter(|&w| engine_reproduces(ANCHOR.jump(w)))
        .collect();
    assert!(hits.contains(&0));
    for &w in &hits {
        assert!(
            engine_hits.contains(&w),
            "constraint hit at wait {w} but the engine disagrees"
        );
    }
    println!(
        "wait scan over {WINDOW} frames (stride 1): {} constraint hits, {} engine \
         reproductions, hits ⊆ engine {}",
        hits.len(),
        engine_hits.len(),
        hits.iter().all(|w| engine_hits.contains(w)),
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
