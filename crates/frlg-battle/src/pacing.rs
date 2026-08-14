//! The battle's frame pacing, measured event by event, per fight.
//!
//! A [`Pacing`] value holds every fitted constant for one fight under one
//! drive. Two are committed:
//!
//! - [`RIVAL1`]: the rival-1 lab battle (our Bulbasaur vs Charmander), under
//!   the `run_plan` drive (A-mash from battle start, the plan's first entry
//!   idled *before* the battle's mash starts). Fitted 2026-08-12 by
//!   `examples/fit-pacing.rs` (~280 emulator battles over start delays,
//!   per-turn delay sweeps and stream shifts -10..=10, zero label failures).
//! - [`SQUIRTLE_LAB`]: the defeat-brock lab battle (our Squirtle vs the
//!   rival's Bulbasaur), under the `win_battle` drive (B-mash intro,
//!   delays spent at each action menu; `crates/frlg-route/src/brock.rs`).
//!   Fitted 2026-08-14 by the same fitter (`FRLG_LEDGER=route/defeat-brock/
//!   ledger.json FRLG_DRIVE=menu`); the evidence notes sit with each
//!   constant.
//!
//! None of it is derived from the decomp: these are measured properties of
//! one fight under one drive (`text_hold` mash of the route's tuning).
//! Change the drive, the starter, or the opponent and the table must be
//! re-fitted.
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
//! picks the real one. Everything else here is single-valued in its fit.

/// Every fitted pacing quantity for one fight under one drive. Tables use
/// 0 for "never observed" (real gaps are all far from 0); the accessor
/// methods turn that into `Option`.
#[derive(Debug, Clone)]
pub struct Pacing {
    /// Battle-segment frame of the pre-turn-1 roll, by `plan[0] % 5` (the
    /// mash period). A drive whose intro does not depend on the plan's
    /// head repeats one value five times.
    pub intro_preturn: [u32; 5],
    /// `turnend`/`preturn` roll (== the `choosing_actions` detection frame)
    /// to the AI block. Independent of the plan's delay: the opponent
    /// controller answers before the player commits.
    pub det_to_ai: u32,
    /// Player crit roll relative to the resolution-mash start (`lbe + 1`).
    pub loop_a_to_pcrit: u32,
    /// Crit roll to damage roll (`adjustnormaldamage` script step).
    pub pcrit_to_pdmg: u32,
    /// HP write to the attacker's trailing secondary-effect roll...
    pub hp_to_sec: u32,
    /// ...which waits out "A critical hit!" when the hit crit.
    pub hp_to_sec_crit: u32,
    /// Player secondary roll to the rival's accuracy roll, by announced
    /// move: the rival's damaging move, and Growl.
    pub psec_to_racc_hit: u32,
    pub psec_to_racc_growl: u32,
    /// Rival accuracy roll to its crit roll (the hit move's animation).
    pub racc_to_rcrit: u32,
    pub rcrit_to_rdmg: u32,
    /// Rival secondary roll to the turn-end roll.
    pub rsec_to_turnend: u32,
    /// The rival's damaging move missing (only possible when its accuracy
    /// is below 100): accuracy roll to the turn-end roll, through the
    /// "attack missed" text. 0 = never observed (a fight whose rival
    /// cannot miss).
    pub racc_miss_to_turnend: u32,
    /// The rival's first Growl: accuracy roll to the stat-stage write, and
    /// the stage write to the turn-end roll (Oak's stat-change interjection
    /// included; a second rival Growl cannot happen -- its score is then
    /// 99 < 100).
    pub racc_to_stagefall_first: u32,
    pub stagefall_first_to_turnend: u32,
    /// GATE: how long the commit mash runs, `8 + 5 * k` with `k` the index
    /// of the first menu press that registered, by whether the turn's delay
    /// is small (<= 3) or large. See [`Pacing::commit_durations`].
    pub commit_small: &'static [u32],
    pub commit_large: &'static [u32],
    /// HP-bar drain (damage roll to the `gBattleMons` HP write) for the
    /// rival's bar, by HP actually lost -- a killing hit drains only what
    /// was left. Crit and non-crit hits drain identically; index 0 unused,
    /// 0 = unobserved delta.
    pub rhp_drain: &'static [u32],
    /// The same drain when it is the player's first landed hit: Oak's
    /// "the enemy's HP bar!" interjection sits in the middle. Only the
    /// no-crit damage range is reachable (crits are suppressed until the
    /// first hit lands).
    pub rhp_drain_first: &'static [u32],
    /// HP-bar drain for the player's bar (a bar pixel covers different HP
    /// than the rival's when max HP differs, so the table is separate).
    pub uhp_drain: &'static [u32],
    /// GATE: the win sequence (final secondary roll to `gBattleOutcome`
    /// being set: faint, EXP, level-up, Oak). Its texts run on the
    /// resolution mash's press grid, so the base depends on
    /// `(psec - loop_a_start) % 5`, and the last press can slip one mash
    /// period. 0 = phase never observed.
    pub outcome_win_base: [u32; 5],
}

impl Pacing {
    pub fn commit_durations(&self, delay: u32) -> &'static [u32] {
        if delay <= 3 {
            self.commit_small
        } else {
            self.commit_large
        }
    }

    fn table(t: &[u32], delta: u16) -> Option<u32> {
        match t.get(delta as usize).copied() {
            Some(0) | None => None,
            some => some,
        }
    }

    pub fn rhp_drain(&self, delta: u16) -> Option<u32> {
        Self::table(self.rhp_drain, delta)
    }

    pub fn rhp_drain_first(&self, delta: u16) -> Option<u32> {
        Self::table(self.rhp_drain_first, delta)
    }

    pub fn uhp_drain(&self, delta: u16) -> Option<u32> {
        Self::table(self.uhp_drain, delta)
    }

    /// Both end-gate candidates for the given press phase, or `None` for a
    /// phase the fit never observed.
    pub fn outcome_win_gaps(&self, phase: u32) -> Option<[u32; 2]> {
        match self.outcome_win_base[phase as usize % 5] {
            0 => None,
            base => Some([base, base + 5]),
        }
    }

    pub fn racc_miss_to_turnend(&self) -> Option<u32> {
        match self.racc_miss_to_turnend {
            0 => None,
            v => Some(v),
        }
    }
}

/// The rival-1 lab battle (Bulbasaur vs Charmander) under the `run_plan`
/// drive -- the fitted values documented in this module's header. Delays
/// 0..=3 have hundreds of observations and never registered their first
/// commit press (it falls within 4 frames of detection), so their gate set
/// is the {13, 18} pair. Larger delays are sparsely observed and their
/// first press sometimes registers and sometimes whiffs, so they carry the
/// full union until more data narrows them.
///
/// Arbitration evidence (2026-08-14, `examples/arbitrate*`): in 40+ real
/// replays on the committed stream at delays 4-11, the 8 never fired once,
/// and within {13, 18} the resolution flipped with the plan's tail (the
/// committed [4,3,3,3] resolves its final gate to 13; [4,3,3,0] to 18).
/// Every measured battle still equalled one of the enumerated leaves --
/// the gate set is sound, but a search must not bank a margin smaller
/// than the gate spread without an emulator run.
pub const RIVAL1: Pacing = Pacing {
    intro_preturn: [1048, 1049, 1050, 1046, 1047],
    det_to_ai: 5,
    loop_a_to_pcrit: 30,
    pcrit_to_pdmg: 3,
    hp_to_sec: 5,
    hp_to_sec_crit: 84,
    psec_to_racc_hit: 12,
    psec_to_racc_growl: 14,
    racc_to_rcrit: 31,
    rcrit_to_rdmg: 3,
    rsec_to_turnend: 10,
    racc_miss_to_turnend: 0, // Scratch is 100 accurate: no miss exists
    racc_to_stagefall_first: 29,
    stagefall_first_to_turnend: 213,
    commit_small: &[13, 18],
    commit_large: &[8, 13, 18],
    // Deltas 1..=10; rival max HP 18.
    rhp_drain: &[0, 77, 80, 82, 85, 88, 90, 93, 96, 98, 101],
    // Only deltas 4 and 5 are reachable (crits suppressed on the first hit).
    rhp_drain_first: &[0, 0, 0, 0, 210, 215],
    // Deltas 1 and 6 were only ever observed on killing blows, where the
    // battle is decided anyway. Player max HP 20.
    uhp_drain: &[0, 0, 79, 82, 84, 87, 0, 91, 94, 96, 99],
    // Phase 1 was never observed.
    outcome_win_base: [424, 0, 427, 426, 425],
};

// --- Backwards-compatible re-exports of the rival-1 constants. The fitter
// and the older examples name them directly; new code should carry a
// `&Pacing` instead. ---

pub const DET_TO_AI: u32 = RIVAL1.det_to_ai;
pub const LOOP_A_TO_PCRIT: u32 = RIVAL1.loop_a_to_pcrit;
pub const PCRIT_TO_PDMG: u32 = RIVAL1.pcrit_to_pdmg;
pub const HP_TO_SEC: u32 = RIVAL1.hp_to_sec;
pub const HP_TO_SEC_CRIT: u32 = RIVAL1.hp_to_sec_crit;
pub const PSEC_TO_RACC_SCRATCH: u32 = RIVAL1.psec_to_racc_hit;
pub const PSEC_TO_RACC_GROWL: u32 = RIVAL1.psec_to_racc_growl;
pub const RACC_TO_RCRIT: u32 = RIVAL1.racc_to_rcrit;
pub const RCRIT_TO_RDMG: u32 = RIVAL1.rcrit_to_rdmg;
pub const RSEC_TO_TURNEND: u32 = RIVAL1.rsec_to_turnend;
pub const RACC_TO_STAGEFALL_FIRST: u32 = RIVAL1.racc_to_stagefall_first;
pub const STAGEFALL_FIRST_TO_TURNEND: u32 = RIVAL1.stagefall_first_to_turnend;
pub const INTRO_PRETURN: [u32; 5] = RIVAL1.intro_preturn;
pub const RHP_DRAIN: [u32; 11] = [0, 77, 80, 82, 85, 88, 90, 93, 96, 98, 101];

pub fn commit_durations(delay: u32) -> &'static [u32] {
    RIVAL1.commit_durations(delay)
}

pub fn rhp_drain_first(delta: u16) -> Option<u32> {
    RIVAL1.rhp_drain_first(delta)
}

pub fn uhp_drain(delta: u16) -> Option<u32> {
    RIVAL1.uhp_drain(delta)
}

pub fn outcome_win_gaps(phase: u32) -> Option<[u32; 2]> {
    RIVAL1.outcome_win_gaps(phase)
}

/// The defeat-brock lab battle (Squirtle vs the rival's Bulbasaur) under
/// the `win_battle` drive: B-mash intro (fixed length, so `intro_preturn`
/// repeats one value), then per-menu delays with an A-mash. Placeholder
/// values are 0 until the fit lands; [`SQUIRTLE_LAB_FITTED`] flips when
/// they are real.
pub const SQUIRTLE_LAB_FITTED: bool = false;
pub const SQUIRTLE_LAB: Pacing = Pacing {
    intro_preturn: [0; 5],
    det_to_ai: 0,
    loop_a_to_pcrit: 0,
    pcrit_to_pdmg: 0,
    hp_to_sec: 0,
    hp_to_sec_crit: 0,
    psec_to_racc_hit: 0,
    psec_to_racc_growl: 0,
    racc_to_rcrit: 0,
    rcrit_to_rdmg: 0,
    rsec_to_turnend: 0,
    racc_miss_to_turnend: 0,
    racc_to_stagefall_first: 0,
    stagefall_first_to_turnend: 0,
    commit_small: &[],
    commit_large: &[],
    rhp_drain: &[],
    rhp_drain_first: &[],
    uhp_drain: &[],
    outcome_win_base: [0; 5],
};
