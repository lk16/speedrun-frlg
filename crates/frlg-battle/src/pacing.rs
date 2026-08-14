//! The battle's frame pacing, measured event by event.
//!
//! Every constant here was fitted by `examples/fit-pacing.rs` (2026-08-12,
//! ~280 emulator battles over start delays, per-turn delay sweeps and stream
//! shifts -10..=10, zero label failures), and none of it is derived from the
//! decomp: these are measured properties of this route's rival battle under
//! this route's drive (the `text_hold = 4` A-mash of `run_plan`, restarted at
//! every stage, `route/rival-1/ledger.json` tuning). Change the drive, the starter,
//! or the opponent and the table must be re-fitted.
//!
//! The frame anatomy of one turn, verified against the marks the fitter
//! records (`Marks` in fit-pacing):
//!
//! ```text
//! det                       turnend roll lands; choosing_actions flips true
//! det + 5                   the whole AI block rolls (opponent controller)
//! lb   = det + delay + 1    the commit mash starts (after the plan's idles)
//! lbe  = lb + dur           choosing_actions flips false; dur is a GATE
//! la   = lbe + 1            the resolution mash starts (loop A)
//! pcrit = la + 30           player crit roll; +3 damage roll; then the
//!                           HP-bar/text chain below to the next det
//! ```
//!
//! Two transitions are *gates*: input-gated moments whose exact frame the
//! model cannot always decide (the residue is scene state -- tested against
//! detection-frame parity, mash phase and turn index, none classify it
//! fully). A gate enumerates its measured candidates instead of guessing;
//! `engine::simulate` returns one leaf per combination, and the emulator
//! picks the real one. Everything else in this file is single-valued in the
//! fit.

/// `turnend`/`preturn` roll (== the `choosing_actions` detection frame) to
/// the AI block. Independent of the plan's delay: the opponent controller
/// answers before the player commits.
pub const DET_TO_AI: u32 = 5;
/// Player crit roll relative to the resolution-mash start (`lbe + 1`).
pub const LOOP_A_TO_PCRIT: u32 = 30;
/// Crit roll to damage roll (`adjustnormaldamage` script step).
pub const PCRIT_TO_PDMG: u32 = 3;
/// HP write to the attacker's trailing secondary-effect roll...
pub const HP_TO_SEC: u32 = 5;
/// ...which waits out "A critical hit!" when the hit crit.
pub const HP_TO_SEC_CRIT: u32 = 84;
/// Player secondary roll to the rival's accuracy roll, by announced move.
pub const PSEC_TO_RACC_SCRATCH: u32 = 12;
pub const PSEC_TO_RACC_GROWL: u32 = 14;
/// Rival accuracy roll to its crit roll (Scratch's animation).
pub const RACC_TO_RCRIT: u32 = 31;
pub const RCRIT_TO_RDMG: u32 = 3;
/// Rival secondary roll to the turn-end roll.
pub const RSEC_TO_TURNEND: u32 = 10;
/// The rival's first Growl: accuracy roll to the stat-stage write, and the
/// stage write to the turn-end roll (Oak's stat-change interjection included;
/// a second rival Growl cannot happen -- its score is then 99 < 100).
pub const RACC_TO_STAGEFALL_FIRST: u32 = 29;
pub const STAGEFALL_FIRST_TO_TURNEND: u32 = 213;

/// Battle-segment frame of the pre-turn-1 roll, by `start_delay % 5`: the
/// whole intro is press-gated on the restarted mash, so delays collapse mod
/// the mash period. Measured for every start delay 0..64 (identical run
/// frames AND identical outcomes across each residue class).
pub const INTRO_PRETURN: [u32; 5] = [1048, 1049, 1050, 1046, 1047];

/// GATE: how long the commit mash runs, `8 + 5 * k` with `k` the index of
/// the first menu press that registered. Delays 0..=3 have hundreds of
/// observations and never registered their first press (it falls within 4
/// frames of detection), so their set is the {13, 18} pair. Larger delays
/// are sparsely observed and their first press sometimes registers and
/// sometimes whiffs (13 was seen at delay 9 on a held-out run after only 8s
/// in the fit), so they carry the full union until more data narrows them.
///
/// Arbitration evidence (2026-08-14, `examples/arbitrate*`): in 40+ real
/// replays on the committed stream at delays 4-11, the 8 never fired once,
/// and within {13, 18} the resolution flipped with the plan's tail (the
/// committed [4,3,3,3] resolves its final gate to 13; [4,3,3,0] to 18).
/// Every measured battle still equalled one of the enumerated leaves --
/// the gate set is sound, but a search must not bank a margin smaller
/// than the gate spread without an emulator run.
pub fn commit_durations(delay: u32) -> &'static [u32] {
    match delay {
        0..=3 => &[13, 18],
        _ => &[8, 13, 18],
    }
}

/// HP-bar drain (damage roll to the `gBattleMons` HP write) for the rival's
/// bar, by HP actually lost -- a killing hit drains only what was left.
/// Crit and non-crit hits drain identically; index 0 is unused.
pub const RHP_DRAIN: [u32; 11] = [0, 77, 80, 82, 85, 88, 90, 93, 96, 98, 101];
/// The same drain when it is the player's first landed hit: Oak's
/// "the enemy's HP bar!" interjection sits in the middle. Only deltas 4 and
/// 5 are reachable (crits are still suppressed on the first hit).
pub fn rhp_drain_first(delta: u16) -> Option<u32> {
    match delta {
        4 => Some(210),
        5 => Some(215),
        _ => None,
    }
}
/// HP-bar drain for the player's bar (max 20 vs the rival's 18: a bar pixel
/// covers more HP, so the table differs). Deltas 1 and 6 were only ever
/// observed on killing blows, where the battle is decided anyway.
pub fn uhp_drain(delta: u16) -> Option<u32> {
    match delta {
        2 => Some(79),
        3 => Some(82),
        4 => Some(84),
        5 => Some(87),
        7 => Some(91),
        8 => Some(94),
        9 => Some(96),
        10 => Some(99),
        _ => None,
    }
}

/// GATE: the win sequence (final secondary roll to `gBattleOutcome` being
/// set: faint, EXP, level-up, Oak). Its texts run on the resolution mash's
/// press grid, so the base depends on `(psec - loop_a_start) % 5`, and the
/// last press can slip one mash period. Phase 1 was never observed.
pub fn outcome_win_gaps(phase: u32) -> Option<[u32; 2]> {
    let base = match phase {
        0 => 424,
        2 => 427,
        3 => 426,
        4 => 425,
        _ => return None,
    };
    Some([base, base + 5])
}
