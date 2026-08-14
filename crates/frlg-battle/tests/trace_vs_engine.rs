//! The extraction (`frlg_battle::trace`) held against the engine, both
//! directions, no emulator involved:
//!
//! - every leaf `engine::simulate` enumerates must extract consistently --
//!   a Win leaf's gates give a trace whose constraint set its own anchor
//!   satisfies and whose frame candidates contain the leaf's exact frames;
//!   a Loss or Unmodelled leaf must refuse to extract;
//! - shifted anchors that satisfy the *committed* trace's set must make the
//!   engine reproduce the committed battle from there, frame for frame.
//!
//! The emulator ground truth behind all of this is
//! `tests/engine_vs_emulator.rs`; these tests pin the model-internal
//! algebra (offset bookkeeping, residue pinning) to the model the emulator
//! already vouches for.

use frlg_battle::engine::{simulate, SimResult};
use frlg_battle::trace::extract_leaf;
use frlg_battle::{Mon, Move};
use frlg_rng::constraint::{ConstraintSet, Pred};
use frlg_rng::Rng;

/// The committed battle's anchor and shape (`tests/committed_battle.rs`,
/// `examples/fit-pacing.rs`).
const ANCHOR: Rng = Rng(0xed94271d);
const PLAN: [u32; 4] = [4, 3, 3, 3];
const COMMITTED_GATES: [u32; 3] = [13, 13, 13];
const COMMITTED_FRAMES: u32 = 2409;

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

#[test]
fn committed_leaf_extracts_to_its_known_shape() {
    let trace = extract_leaf(&PLAN, &COMMITTED_GATES, ANCHOR, bulbasaur(), charmander())
        .expect("the committed leaf is a modelled win");
    assert!(trace.frame_candidates.contains(&COMMITTED_FRAMES));
    assert_eq!(trace.total_calls, 4004);
    assert_eq!(trace.rival_moves, vec![Move::Scratch; 3]);
    assert_eq!(trace.constraints.len(), 12);
    let set = ConstraintSet::new(&trace.constraints);
    assert!(set.satisfied(ANCHOR));
    assert!(
        !set.satisfied(ANCHOR.next()),
        "one stream step scrambles every offset"
    );
}

/// Every leaf the engine produces, across plans and shifted anchors, must
/// agree with extraction on the same gates: wins extract and self-satisfy,
/// losses and unmodelled paths refuse.
#[test]
fn every_engine_leaf_extracts_consistently() {
    let plans: [&[u32]; 4] = [&PLAN, &[0], &[7, 1, 2], &[12, 5, 5, 5, 5]];
    let mut wins = 0u32;
    let mut refusals = 0u32;
    let mut growl_turns = 0u32;
    for shift in 0..64u32 {
        let anchor = ANCHOR.jump(37 * shift);
        for plan in plans {
            for leaf in simulate(plan, anchor, bulbasaur(), charmander()) {
                let extracted =
                    extract_leaf(plan, &leaf.commit_durs, anchor, bulbasaur(), charmander());
                match leaf.result {
                    SimResult::Win { frames } => {
                        let trace = extracted.unwrap_or_else(|e| {
                            panic!("win leaf {:?} refused to extract: {e}", leaf.commit_durs)
                        });
                        assert!(
                            trace.frame_candidates.contains(&frames),
                            "leaf frames {frames} not in {:?}",
                            trace.frame_candidates
                        );
                        assert_eq!(trace.rival_moves.len(), leaf.commit_durs.len());
                        assert!(
                            ConstraintSet::new(&trace.constraints).satisfied(anchor),
                            "a trace must accept the stream it was read from"
                        );
                        wins += 1;
                        growl_turns += trace
                            .rival_moves
                            .iter()
                            .filter(|&&m| m == Move::Growl)
                            .count() as u32;
                    }
                    SimResult::Loss | SimResult::Unmodelled(_) => {
                        assert!(
                            extracted.is_err(),
                            "leaf {:?} ({:?}) must not extract",
                            leaf.commit_durs,
                            leaf.result
                        );
                        refusals += 1;
                    }
                }
            }
        }
    }
    // The sweep must actually have exercised the interesting paths.
    assert!(wins > 100, "only {wins} win leaves seen");
    assert!(refusals > 0, "no refusal path exercised");
    assert!(growl_turns > 0, "no rival Growl turn exercised");
}

/// A win leaf's trace pins a crit somewhere in the sweep: the crit-hit
/// residue class {0 mod 16} must occur, or the extraction never took the
/// crit-enabled pin path with a landed crit.
#[test]
fn the_sweep_reaches_a_pinned_crit() {
    let crit_pin = Pred::ModRange {
        m: 16,
        lo: 0,
        hi: 0,
    };
    let found = (0..64u32).any(|shift| {
        let anchor = ANCHOR.jump(37 * shift);
        extract_leaf(&PLAN, &COMMITTED_GATES, anchor, bulbasaur(), charmander())
            .is_ok_and(|t| t.constraints.iter().any(|c| c.pred == crit_pin))
    });
    assert!(found, "no crit-class constraint in 64 shifted extractions");
}

/// Constraint hits are sufficient: every shifted anchor the committed set
/// accepts must make the engine replay the committed battle exactly.
#[test]
fn committed_set_hits_reproduce_the_committed_battle() {
    let trace = extract_leaf(&PLAN, &COMMITTED_GATES, ANCHOR, bulbasaur(), charmander())
        .expect("committed leaf");
    let set = ConstraintSet::new(&trace.constraints);
    let hits = set.wait_hits(ANCHOR, 1, 4096);
    assert!(hits.contains(&0), "the anchor itself is wait 0");
    assert!(hits.len() > 1, "the window should contain another hit");
    for &w in &hits {
        let reproduced = simulate(&PLAN, ANCHOR.jump(w), bulbasaur(), charmander())
            .iter()
            .any(|l| {
                l.commit_durs == COMMITTED_GATES
                    && l.result
                        == SimResult::Win {
                            frames: COMMITTED_FRAMES,
                        }
            });
        assert!(
            reproduced,
            "constraint hit at wait {w} but the engine disagrees"
        );
    }
}
