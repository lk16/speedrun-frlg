//! One decided battle leaf, inverted into constraints on its start state.
//!
//! [`extract_leaf`] walks a single gate combination of the pacing model --
//! the same arithmetic as [`crate::engine::simulate`], restricted to one
//! leaf -- with a stream that records each roll's absolute call offset, and
//! pins every *decisive* roll to the residue class that reproduces what the
//! roll did on this stream: AI viability branches (`% 256` vs 50,
//! `decompiled/data/battle_ai_scripts.s:1137,1149`), the move tie-break
//! parity when the scores actually tied
//! (`src/battle_ai_script_commands.c:408`), crit rolls once crits are live
//! (`src/battle_script_commands.c:1199`), and damage variance rolls pinned
//! to the class giving the same damage (`:1557-1568` -- damage feeds the
//! HP-bar drain, so the *pacing itself* depends on it). Rolls whose value
//! cannot matter -- suppressed crits, 100-accuracy checks, secondary rolls,
//! turn-end rolls, unused AI slots -- get no constraint.
//!
//! The result is an exact-trace [`frlg_rng::constraint::ConstraintSet`]:
//! every start state satisfying it plays this battle *identically* (same
//! moves, same damage, same frames), so a route search can skip the battle
//! from any such state -- advance the stream [`Trace::total_calls`], charge
//! the frames, apply the deltas. Exact-trace is sound but strict: a state
//! that fails the set may still reach an equivalent outcome another way
//! (measured on the committed battle: 9 constraint hits vs 22 engine
//! reproductions in a 16k-wait window), so the set under-approximates and
//! `engine::simulate` remains the arbiter of "wins somehow".
//!
//! Correctness is tested, not argued: `tests/trace_vs_engine.rs` requires
//! every winning leaf the engine enumerates to extract into a set its own
//! anchor satisfies, with the leaf's exact frame count among the
//! candidates, and requires shifted anchors that satisfy the committed set
//! to make the engine reproduce the committed battle.

use frlg_rng::constraint::{Constraint, Pred};
use frlg_rng::Rng;

use crate::pacing::{self, Pacing};
use crate::{apply_variance, base_damage, Mon, Move};

/// What one extracted leaf pins down.
#[derive(Debug, Clone)]
pub struct Trace {
    /// The decisive rolls, as constraints on the battle-start `gRngValue`.
    pub constraints: Vec<Constraint>,
    /// Both end-gate frame candidates (the win sequence's last press can
    /// slip one mash period, `pacing::outcome_win_gaps`); the leaf the
    /// gates came from ends on one of them.
    pub frame_candidates: [u32; 2],
    /// Total `Random()` calls from the anchor through the last modelled
    /// roll -- what a route search advances the stream by to skip the
    /// battle.
    pub total_calls: u32,
    /// The rival's chosen move each turn, in turn order.
    pub rival_moves: Vec<Move>,
}

/// `engine::Stream` with the absolute call offset exposed: `calls` counts
/// every `Random()` since the anchor, so the roll returned is the
/// `calls`-th call -- exactly `ConstraintSet`'s offset convention.
struct RecStream {
    rng: Rng,
    consumed: u32,
    calls: u32,
}

impl RecStream {
    fn roll_at(&mut self, frame: u32) -> (u32, u16) {
        debug_assert!(frame + 1 >= self.consumed);
        let vblank = 2 * (frame + 1 - self.consumed);
        self.rng = self.rng.jump(vblank);
        self.consumed = frame + 1;
        self.calls += vblank + 1;
        (self.calls, self.rng.random())
    }
}

/// Pin `roll` (consumed at `offset`) to the residues mod `m` that `key`
/// maps to the same outcome as this stream's residue. No constraint when
/// every residue agrees (the roll is not decisive); panics if the agreeing
/// class is not contiguous, which none of these monotone formulas produce.
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

/// Walk one leaf -- `plan` as in [`crate::engine::simulate`], `gates[k]`
/// the commit duration taken at turn k+1's gate -- and extract its
/// constraints. Errors mirror the engine's undecidable cases
/// ([`crate::engine::SimResult::Unmodelled`] and losses), plus a gate
/// sequence shorter than the battle it describes. The rival-1 fight;
/// [`extract_leaf_with`] takes the fight's parameters.
pub fn extract_leaf(
    plan: &[u32],
    gates: &[u32],
    anchor: Rng,
    us: Mon,
    rival: Mon,
) -> Result<Trace, &'static str> {
    extract_leaf_with(
        plan,
        gates,
        anchor,
        us,
        rival,
        Move::Scratch,
        &pacing::RIVAL1,
    )
}

/// [`extract_leaf`] parameterized by the rival's damaging move and the
/// fight's [`Pacing`] -- mirroring [`crate::engine::simulate_with`]. New
/// decisive roll relative to rival-1: when the rival's hit move has
/// accuracy below 100 (Bulbasaur's Tackle at 95), its accuracy roll
/// decides hit vs miss and gets pinned mod 100.
pub fn extract_leaf_with(
    plan: &[u32],
    gates: &[u32],
    anchor: Rng,
    mut us: Mon,
    mut rival: Mon,
    rival_hit: Move,
    pacing: &Pacing,
) -> Result<Trace, &'static str> {
    assert!(
        us.speed > rival.speed,
        "the pacing model is fitted for a player that acts first"
    );
    let mut stream = RecStream {
        rng: anchor,
        consumed: 0,
        calls: 0,
    };
    let mut constraints = Vec::new();
    let mut rival_moves = Vec::new();
    let mut crit_enabled = false;

    let start_delay = plan.first().copied().unwrap_or(0);
    let mut det = pacing.intro_preturn[start_delay as usize % 5];
    let _pre_turn = stream.roll_at(det);

    let mut turn = 0u32;
    loop {
        turn += 1;
        if turn > 16 {
            return Err("turn cap");
        }
        let delay = plan.get(turn as usize).copied().unwrap_or(0);

        // The AI block, all on one frame (engine::walk):
        // rival_choose_move's exact consumption (crate root, citations
        // there), with a pin at each branch a roll decides.
        let ai_frame = det + pacing.det_to_ai;
        let rival_move = {
            let mut simulated = [0u16; 4];
            for (slot, sim) in simulated.iter_mut().enumerate() {
                let (offset, roll) = stream.roll_at(ai_frame);
                *sim = 100 - (roll % 16);
                if slot == 0 {
                    // The only simulatedRNG slot whose value reaches a
                    // branch: AI_TryToFaint scales Scratch's damage by it.
                    let (hp, base) = (us.hp, base_damage(&rival, &us, rival_hit, false));
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
                (base_damage(&rival, &us, rival_hit, false) * simulated[0] as i32 / 100).max(1);
            if us.hp as i32 <= sim_damage {
                scratch_score += 4;
            }
            let (offset_tie, tie) = stream.roll_at(ai_frame);
            if scratch_score == growl_score {
                pin(&mut constraints, offset_tie, 2, tie, |r| r == 0);
                if tie.is_multiple_of(2) {
                    rival_hit
                } else {
                    Move::Growl
                }
            } else if scratch_score > growl_score {
                rival_hit
            } else {
                Move::Growl
            }
        };
        rival_moves.push(rival_move);

        // This turn's gate, then the turn (engine::play_turn, one leaf).
        let Some(&gate) = gates.get(turn as usize - 1) else {
            return Err("gate sequence shorter than the battle");
        };
        let lb = det + delay + 1;
        let loop_a = lb + gate + 1;

        // Player Tackle: crit, damage, drain, trailing secondary.
        let pcrit_f = loop_a + pacing.loop_a_to_pcrit;
        let (offset, roll) = stream.roll_at(pcrit_f);
        let crit = roll.is_multiple_of(16) && crit_enabled;
        if crit_enabled {
            pin(&mut constraints, offset, 16, roll, |r| r == 0);
        }
        let pdmg_f = pcrit_f + pacing.pcrit_to_pdmg;
        let base = base_damage(&us, &rival, Move::Tackle, crit);
        let (offset, roll) = stream.roll_at(pdmg_f);
        pin(&mut constraints, offset, 16, roll, |r| {
            apply_variance(base, r)
        });
        let damage = apply_variance(base, roll);
        let delta = rival.hp.min(damage as u16);
        let drain = if crit_enabled {
            pacing.rhp_drain(delta)
        } else {
            pacing.rhp_drain_first(delta)
        };
        let Some(drain) = drain else {
            return Err("rival HP-bar delta");
        };
        let rhp_f = pdmg_f + drain;
        rival.hp -= delta;
        crit_enabled = true;
        let psec_f = rhp_f
            + if crit {
                pacing.hp_to_sec_crit
            } else {
                pacing.hp_to_sec
            };
        let _ = stream.roll_at(psec_f); // secondary: burned, never read

        if rival.hp == 0 {
            let Some(gaps) = pacing.outcome_win_gaps((psec_f - loop_a) % 5) else {
                return Err("end-sequence press phase");
            };
            return Ok(Trace {
                constraints,
                frame_candidates: [psec_f + gaps[0] + 1, psec_f + gaps[1] + 1],
                total_calls: stream.calls,
                rival_moves,
            });
        }

        // The rival's answer.
        match rival_move {
            Move::Growl => {
                let racc_f = psec_f + pacing.psec_to_racc_growl;
                let _ = stream.roll_at(racc_f); // 100 accuracy: never decisive
                assert!(us.atk_stage > 0, "a second rival Growl cannot score 100");
                us.atk_stage -= 1;
                let stagefall_f = racc_f + pacing.racc_to_stagefall_first;
                det = stagefall_f + pacing.stagefall_first_to_turnend;
            }
            mv => {
                let racc_f = psec_f + pacing.psec_to_racc_hit;
                let (offset, acc_roll) = stream.roll_at(racc_f);
                let acc = mv.accuracy();
                if acc < 100 {
                    // Decisive: hit iff roll % 100 + 1 <= acc.
                    pin(&mut constraints, offset, 100, acc_roll, |r| {
                        (r as u32 + 1) <= acc
                    });
                }
                if (acc_roll as u32 % 100 + 1) > acc {
                    let Some(gap) = pacing.racc_miss_to_turnend() else {
                        return Err("rival miss");
                    };
                    det = racc_f + gap;
                    let _turn_end = stream.roll_at(det);
                    continue;
                }
                let rcrit_f = racc_f + pacing.racc_to_rcrit;
                let (offset, roll) = stream.roll_at(rcrit_f);
                let crit = roll.is_multiple_of(16) && crit_enabled;
                pin(&mut constraints, offset, 16, roll, |r| r == 0);
                let rdmg_f = rcrit_f + pacing.rcrit_to_rdmg;
                let base = base_damage(&rival, &us, mv, crit);
                let (offset, roll) = stream.roll_at(rdmg_f);
                pin(&mut constraints, offset, 16, roll, |r| {
                    apply_variance(base, r)
                });
                let damage = apply_variance(base, roll);
                let delta = us.hp.min(damage as u16);
                us.hp -= delta;
                if us.hp == 0 {
                    return Err("loss");
                }
                let Some(drain) = pacing.uhp_drain(delta) else {
                    return Err("player HP-bar delta");
                };
                let uhp_f = rdmg_f + drain;
                let rsec_f = uhp_f
                    + if crit {
                        pacing.hp_to_sec_crit
                    } else {
                        pacing.hp_to_sec
                    };
                let _ = stream.roll_at(rsec_f);
                det = rsec_f + pacing.rsec_to_turnend;
            }
        }
        let _turn_end = stream.roll_at(det);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_skips_non_decisive_rolls() {
        let mut out = Vec::new();
        pin(&mut out, 7, 16, 5, |_| 0u8);
        assert!(out.is_empty(), "a constant key pins nothing");
    }

    #[test]
    fn pin_crit_classes() {
        // Crit hit: residue 0 alone. Crit miss: 1..=15, one range.
        let mut out = Vec::new();
        pin(&mut out, 1, 16, 32, |r| r == 0);
        assert_eq!(
            out[0].pred,
            Pred::ModRange {
                m: 16,
                lo: 0,
                hi: 0
            }
        );
        let mut out = Vec::new();
        pin(&mut out, 1, 16, 33, |r| r == 0);
        assert_eq!(
            out[0].pred,
            Pred::ModRange {
                m: 16,
                lo: 1,
                hi: 15
            }
        );
    }

    #[test]
    fn pin_damage_classes_are_contiguous_and_correct() {
        // base 5: residue 0 gives 5, residues 1..=15 give 4
        // (apply_variance truncates 85..99% of 5 to 4).
        let mut out = Vec::new();
        pin(&mut out, 1, 16, 16, |r| apply_variance(5, r));
        assert_eq!(
            out[0].pred,
            Pred::ModRange {
                m: 16,
                lo: 0,
                hi: 0
            }
        );
        let mut out = Vec::new();
        pin(&mut out, 1, 16, 3, |r| apply_variance(5, r));
        assert_eq!(
            out[0].pred,
            Pred::ModRange {
                m: 16,
                lo: 1,
                hi: 15
            }
        );
    }
}
