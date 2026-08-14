//! The battle engine: play a lab rival battle forward from a `gRngValue`
//! and a delay plan without the emulator, using the v1 roll semantics for
//! what the rolls decide (`crate::` root) and a fitted [`Pacing`] table for
//! where in the stream they land.
//!
//! Where the pacing model has a gate (a commit press or the end sequence
//! whose exact frame it cannot decide), the engine enumerates the measured
//! candidates rather than guessing: `simulate` returns one [`Leaf`] per
//! combination, and exactly one leaf is the battle the emulator would play.
//! That is enough for a search -- a plan whose every leaf loses can be
//! discarded without emulation, and one whose best leaf wins fast is a
//! candidate to verify (`tests/engine_vs_emulator.rs` is the evidence for
//! both directions).
//!
//! A leaf simulates in the order of a microsecond; the emulator pays about a
//! millisecond per *frame*.

use frlg_rng::Rng;

use crate::pacing::{self, Pacing};
use crate::{apply_variance, base_damage, rival_choose_move_with, Mon, Move};

/// How one enumerated battle ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimResult {
    /// `gBattleOutcome == B_OUTCOME_WON`; `frames` counts like the search's
    /// `run_plan` (steps until the outcome byte goes nonzero).
    Win { frames: u32 },
    /// We fainted (or the model's 16-turn cap tripped). The exact loss frame
    /// is not modelled; a search discards losses.
    Loss,
    /// The battle left the fitted vocabulary (an unmeasured HP-bar delta or
    /// press phase). The caller must fall back to the emulator.
    Unmodelled(&'static str),
}

/// One resolved combination of gate choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leaf {
    /// The commit duration chosen at each turn's gate, in turn order --
    /// matching `Marks::loop_b_end - Marks::loop_b_start` from the fitter,
    /// which is how a leaf is matched against an emulator replay.
    pub commit_durs: Vec<u32>,
    pub result: SimResult,
}

/// The stream, placed on the battle's frame axis: every stepped frame rolls
/// its VBlank pair first, the game's own rolls trail within their frame. A
/// roll at frame `f` therefore sits after `2 * (f + 1)` VBlank rolls plus
/// every logic roll before it.
#[derive(Clone, Copy)]
struct Stream {
    rng: Rng,
    /// Number of frames whose VBlank pair has been consumed.
    consumed: u32,
}

impl Stream {
    fn roll_at(&mut self, frame: u32) -> u16 {
        debug_assert!(frame + 1 >= self.consumed);
        self.rng = self.rng.jump(2 * (frame + 1 - self.consumed));
        self.consumed = frame + 1;
        self.rng.random()
    }
}

#[derive(Clone, Copy)]
struct BattleState<'a> {
    us: Mon,
    rival: Mon,
    rival_hit: Move,
    pacing: &'a Pacing,
    stream: Stream,
    /// FIRST_BATTLE_MSG_FLAG_INFLICT_DMG: crits live once our first hit's
    /// bar has drained.
    crit_enabled: bool,
    /// The frame of the pending turn-end (or pre-turn) roll: the detection
    /// frame the next turn is built on.
    det: u32,
    turn: u32,
}

/// [`simulate_with`] for the rival-1 fight: Charmander answers with Scratch,
/// pacing per [`pacing::RIVAL1`] (the `run_plan` drive).
pub fn simulate(plan: &[u32], start: Rng, us: Mon, rival: Mon) -> Vec<Leaf> {
    simulate_with(plan, start, us, rival, Move::Scratch, &pacing::RIVAL1)
}

/// Play every gate combination of `plan` (same semantics as the route
/// search: `plan[0]` idle frames before the battle's mash starts, `plan[k]`
/// idle frames at turn k's action selection) from the battle-start
/// `gRngValue`. `us` and `rival` are the mons as `gBattleMons` will hold
/// them; the player must be the faster side (both fitted fights' is -- the
/// model has no speed-tie rolls). `rival_hit` is the rival's damaging move
/// (its slot 0); the player always attacks with Tackle-or-Scratch slot 0,
/// whose accuracy is never rolled in these battles (the ACC_CURR_MOVE
/// quirk, crate root).
pub fn simulate_with(
    plan: &[u32],
    start: Rng,
    us: Mon,
    rival: Mon,
    rival_hit: Move,
    pacing: &Pacing,
) -> Vec<Leaf> {
    assert!(
        us.speed > rival.speed,
        "the pacing model is fitted for a player that acts first"
    );
    let start_delay = plan.first().copied().unwrap_or(0);
    let preturn = pacing.intro_preturn[start_delay as usize % 5];
    let mut state = BattleState {
        us,
        rival,
        rival_hit,
        pacing,
        stream: Stream {
            rng: start,
            consumed: 0,
        },
        crit_enabled: false,
        det: preturn,
        turn: 0,
    };
    let _pre_turn_roll = state.stream.roll_at(preturn);

    let mut leaves = Vec::new();
    walk(state, plan, Vec::new(), &mut leaves);
    leaves
}

fn leaf(leaves: &mut Vec<Leaf>, durs: Vec<u32>, result: SimResult) {
    leaves.push(Leaf {
        commit_durs: durs,
        result,
    });
}

fn walk(mut state: BattleState<'_>, plan: &[u32], durs: Vec<u32>, leaves: &mut Vec<Leaf>) {
    state.turn += 1;
    if state.turn > 16 {
        return leaf(leaves, durs, SimResult::Loss);
    }
    let delay = plan.get(state.turn as usize).copied().unwrap_or(0);

    // The AI block: every roll on one frame, before the player commits.
    let ai_frame = state.det + state.pacing.det_to_ai;
    let (us, rival, rival_hit) = (state.us, state.rival, state.rival_hit);
    let rival_move = {
        let stream = &mut state.stream;
        rival_choose_move_with(rival_hit, &us, &rival, &mut || stream.roll_at(ai_frame))
    };

    let lb = state.det + delay + 1;
    for &dur in state.pacing.commit_durs(delay) {
        let mut s = state;
        let mut durs = durs.clone();
        durs.push(dur);
        let loop_a = lb + dur + 1;
        let result = play_turn(&mut s, rival_move, loop_a, leaves, &durs);
        match result {
            TurnEnd::Decided => {}
            TurnEnd::NextTurn => walk(s, plan, durs, leaves),
        }
    }
}

enum TurnEnd {
    /// Leaves were already emitted (win, loss, or unmodelled).
    Decided,
    /// The battle continues; `state` holds the pending detection frame.
    NextTurn,
}

fn play_turn(
    s: &mut BattleState<'_>,
    rival_move: Move,
    loop_a: u32,
    leaves: &mut Vec<Leaf>,
    durs: &[u32],
) -> TurnEnd {
    let pacing = s.pacing;
    let emit = |leaves: &mut Vec<Leaf>, result| {
        leaf(leaves, durs.to_vec(), result);
        TurnEnd::Decided
    };

    // Player attack: crit, damage, bar drain, secondary. No accuracy roll
    // on this route (the ACC_CURR_MOVE quirk, crate root).
    let pcrit_f = loop_a + pacing.loop_a_to_pcrit;
    let crit = s.stream.roll_at(pcrit_f).is_multiple_of(16) && s.crit_enabled;
    let pdmg_f = pcrit_f + pacing.pcrit_to_pdmg;
    let damage = apply_variance(
        base_damage(&s.us, &s.rival, Move::Tackle, crit),
        s.stream.roll_at(pdmg_f),
    );
    let delta = s.rival.hp.min(damage as u16);
    let drain = if s.crit_enabled {
        pacing.rhp_drain(delta)
    } else {
        // Our first hit has not landed yet: Oak's interjection variant.
        pacing.rhp_drain_first(delta)
    };
    let Some(drain) = drain else {
        return emit(leaves, SimResult::Unmodelled("rival HP-bar delta"));
    };
    let rhp_f = pdmg_f + drain;
    s.rival.hp -= delta;
    s.crit_enabled = true;
    let psec_f = rhp_f
        + if crit {
            pacing.hp_to_sec_crit
        } else {
            pacing.hp_to_sec
        };
    let _ = s.stream.roll_at(psec_f);

    if s.rival.hp == 0 {
        let Some(gaps) = pacing.outcome_win_gaps((psec_f - loop_a) % 5) else {
            return emit(leaves, SimResult::Unmodelled("end-sequence press phase"));
        };
        for gap in gaps {
            // run_plan counts steps, so the outcome frame index + 1.
            leaf(
                leaves,
                durs.to_vec(),
                SimResult::Win {
                    frames: psec_f + gap + 1,
                },
            );
        }
        return TurnEnd::Decided;
    }

    // The rival's answer.
    match rival_move {
        Move::Growl => {
            let racc_f = psec_f + pacing.psec_to_racc_growl;
            let _ = s.stream.roll_at(racc_f); // Growl is 100 accurate
            assert!(s.us.atk_stage > 0, "a second rival Growl cannot score 100");
            s.us.atk_stage -= 1;
            let stagefall_f = racc_f + pacing.racc_to_stagefall_first;
            s.det = stagefall_f + pacing.stagefall_first_to_turnend;
        }
        mv => {
            let racc_f = psec_f + pacing.psec_to_racc_hit;
            let acc_roll = s.stream.roll_at(racc_f);
            if (acc_roll as u32 % 100 + 1) > mv.accuracy() {
                // The rival's move missed (only possible below 100
                // accuracy, e.g. Bulbasaur's Tackle at 95).
                let Some(gap) = pacing.racc_miss_to_turnend() else {
                    return emit(leaves, SimResult::Unmodelled("rival miss"));
                };
                s.det = racc_f + gap;
            } else {
                let rcrit_f = racc_f + pacing.racc_to_rcrit;
                let crit = s.stream.roll_at(rcrit_f).is_multiple_of(16) && s.crit_enabled;
                let rdmg_f = rcrit_f + pacing.rcrit_to_rdmg;
                let damage = apply_variance(
                    base_damage(&s.rival, &s.us, mv, crit),
                    s.stream.roll_at(rdmg_f),
                );
                let delta = s.us.hp.min(damage as u16);
                s.us.hp -= delta;
                if s.us.hp == 0 {
                    // The fatal hit's trailing secondary roll happens, but
                    // nothing after it can matter to a search.
                    return emit(leaves, SimResult::Loss);
                }
                let Some(drain) = pacing.uhp_drain(delta) else {
                    return emit(leaves, SimResult::Unmodelled("player HP-bar delta"));
                };
                let uhp_f = rdmg_f + drain;
                let rsec_f = uhp_f
                    + if crit {
                        pacing.hp_to_sec_crit
                    } else {
                        pacing.hp_to_sec
                    };
                let _ = s.stream.roll_at(rsec_f);
                s.det = rsec_f + pacing.rsec_to_turnend;
            }
        }
    }
    let _turn_end_roll = s.stream.roll_at(s.det);
    TurnEnd::NextTurn
}
