//! A pure-Rust model of the rival-1 battle's RNG consumption and damage
//! arithmetic, so a search can ask "what does this stream do to the fight"
//! without paying ~1 ms per emulator frame.
//!
//! Two layers:
//!
//! - **Roll semantics** (this file): which `Random()` calls a move or the AI
//!   consumes, in what order, and what the damage arithmetic does with them.
//!   Every formula carries its decomp citation.
//! - **Pacing** ([`pacing`], [`engine`]): where in the stream those rolls
//!   land -- between any two logic events the stream also advances 2 per
//!   rendered frame (`decompiled/src/main.c:412` + `src/battle_main.c:1650`),
//!   so predicting a battle means predicting its frames. The tables are
//!   measured, not derived (`examples/fit-pacing.rs`), and the two spots the
//!   measurement cannot pin down are enumerated as gates rather than
//!   guessed; [`engine::simulate`] returns one leaf per gate combination.
//!
//! `tests/` validates both layers against libmgba replays: the committed
//! battle roll for roll, and engine predictions leaf-for-leaf against fresh
//! emulator runs on plans and stream shifts the fit never saw.

pub mod engine;
pub mod pacing;

/// One battler's fighting numbers, as read from `gBattleMons`
/// (`decompiled/include/pokemon.h:170`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mon {
    pub hp: u16,
    pub max_hp: u16,
    pub attack: u16,
    pub defense: u16,
    pub speed: u16,
    pub level: u8,
    /// Attack stat stage, 6 = neutral (`statStages`, `pokemon.h:187`).
    pub atk_stage: u8,
    /// Defense stat stage, 6 = neutral.
    pub def_stage: u8,
}

/// The three moves this battle can contain, with their data from
/// `decompiled/src/data/battle_moves.h` (Scratch :133, Tackle :432,
/// Growl :588).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    /// power 40, accuracy 100, EFFECT_HIT.
    Scratch,
    /// power 35, accuracy 95, EFFECT_HIT.
    Tackle,
    /// power 0, accuracy 100, EFFECT_ATTACK_DOWN.
    Growl,
}

impl Move {
    pub fn power(self) -> i32 {
        match self {
            Move::Scratch => 40,
            Move::Tackle => 35,
            Move::Growl => 0,
        }
    }

    pub fn accuracy(self) -> u32 {
        match self {
            Move::Scratch => 100,
            Move::Tackle => 95,
            Move::Growl => 100,
        }
    }
}

/// `gStatStageRatios`, `decompiled/src/pokemon.c:1442-1457`; index is the
/// raw 0..=12 stage, 6 the neutral `DEFAULT_STAT_STAGE`.
pub const STAT_STAGE_RATIOS: [(i32, i32); 13] = [
    (10, 40),
    (10, 35),
    (10, 30),
    (10, 25),
    (10, 20),
    (10, 15),
    (10, 10),
    (15, 10),
    (20, 10),
    (25, 10),
    (30, 10),
    (35, 10),
    (40, 10),
];

/// `APPLY_STAT_MOD` (`decompiled/src/pokemon.c:2374-2378`): truncating
/// integer scaling of a stat by its stage.
fn apply_stat_mod(stat: u16, stage: u8) -> i32 {
    let (num, den) = STAT_STAGE_RATIOS[stage as usize];
    (stat as i32) * num / den
}

/// The physical branch of `CalculateBaseDamage`
/// (`decompiled/src/pokemon.c:2385`, physical path `:2509-2558`, `+2` at
/// `:2648`), then the crit multiplier from `Cmd_damagecalc`
/// (`src/battle_script_commands.c:1215`). No STAB and 1x type effectiveness
/// on this battle's moves (Normal vs Grass/Poison and vs Fire), no burn, no
/// Reflect, no badges -- those branches are deliberately absent rather than
/// modelled wrong.
///
/// On a crit, the attacker's stat *drops* are ignored and the defender's
/// stat *boosts* are ignored (`pokemon.c:2511-2517`, `:2525-2531`).
pub fn base_damage(attacker: &Mon, defender: &Mon, mv: Move, crit: bool) -> i32 {
    let atk = if crit && attacker.atk_stage <= 6 {
        attacker.attack as i32
    } else {
        apply_stat_mod(attacker.attack, attacker.atk_stage)
    };
    let def = if crit && defender.def_stage >= 6 {
        defender.defense as i32
    } else {
        apply_stat_mod(defender.defense, defender.def_stage)
    };
    let mut damage = atk * mv.power();
    damage *= 2 * (attacker.level as i32) / 5 + 2;
    damage /= def;
    damage /= 50;
    if damage == 0 {
        damage = 1; // pokemon.c:2557-2558
    }
    (damage + 2) * if crit { 2 } else { 1 }
}

/// `ApplyRandomDmgMultiplier` (`decompiled/src/battle_script_commands.c:
/// 1557-1568`): 85-100%, truncating, minimum 1.
pub fn apply_variance(damage: i32, roll: u16) -> i32 {
    let percent = 100 - (roll % 16) as i32;
    let out = damage * percent / 100;
    if damage != 0 && out == 0 {
        1
    } else {
        out
    }
}

/// What one executed move did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Missed,
    Hit {
        damage: i32,
        crit: bool,
    },
    /// Growl connected: target's attack stage falls by one (floor 0).
    AttackLowered,
}

/// The tutorial's crit/accuracy gates, tracked outside the mons because the
/// game keeps them in `gBattleStruct->simulatedInputState[2]`
/// (`decompiled/src/battle_controller_oak_old_man.c:2228-2235`, offset 0x94
/// in `struct BattleStruct` -- computed with the decomp's own headers and
/// confirmed by watching the byte flip in RAM).
#[derive(Debug, Clone, Copy, Default)]
pub struct FirstBattleFlags {
    /// `FIRST_BATTLE_MSG_FLAG_INFLICT_DMG` (`include/battle_controllers.h:287`):
    /// set when the rival's HP bar first finishes draining
    /// (`src/battle_controller_opponent.c:304-306`) -- measured on the
    /// committed battle at battle frame 1185, *before* `gBattleMons.hp`
    /// updates, because Oak's interjection text sits between the bar and the
    /// HP write. Until then crits are suppressed for BOTH sides
    /// (`battle_script_commands.c:1200`), though the crit roll is consumed
    /// regardless (`:1199` sits before `:1200` in the `&&` chain).
    pub inflict_dmg: bool,
    /// `FIRST_BATTLE_MSG_FLAG_STAT_CHG` (`:289`): set by the *player's*
    /// Growl only (`src/battle_controller_oak_old_man.c:1769-1771`).
    ///
    /// **This flag, not INFLICT_DMG, is what gates the player's accuracy
    /// roll -- for every move.** `Cmd_accuracycheck` evaluates its
    /// FIRST_BATTLE skip on the *raw script argument* before the
    /// `ACC_CURR_MOVE -> gCurrentMove` substitution
    /// (`battle_script_commands.c:1005-1018` vs `:1035-1036`), and
    /// `ACC_CURR_MOVE` is 0 (`include/constants/battle_script_commands.h:67`)
    /// = `MOVE_NONE`, whose power is 0 (`src/data/battle_moves.h:3-8`). So
    /// the `power != 0` disjunct is dead and the `power == 0` disjunct
    /// applies to everything the player does: **the player never rolls
    /// accuracy in this battle until their own Growl has landed once** --
    /// verified against the emulator (turn 2's Tackle crits, proving
    /// INFLICT_DMG set, while consuming no accuracy roll).
    pub stat_chg: bool,
}

/// One move's resolution: consumes from `stream` exactly the `Random()`
/// calls `BattleScript_EffectHit` / `EffectStatDown` consume, in script
/// order (`data/battle_scripts_1.s:239-272`, `:518-556`), and returns what
/// happened. `attacker_is_player` selects the tutorial's accuracy skip,
/// which only ever applies to the player's side
/// (`battle_script_commands.c:1010,1014`).
///
/// Roll order per the cited script walk: accuracy (`:1093`, sometimes
/// skipped), crit (`:1199`, always for damaging moves), damage variance
/// (`:1560`), and the secondary-effect roll (`:2789`) which is burned even
/// at 0% chance -- but only *after* the HP update, so callers comparing HP
/// deltas see it trail the hit.
pub fn execute_move(
    attacker: &Mon,
    defender: &mut Mon,
    mv: Move,
    attacker_is_player: bool,
    flags: &mut FirstBattleFlags,
    stream: &mut impl FnMut() -> u16,
) -> Outcome {
    let damaging = mv.power() != 0;
    // The ACC_CURR_MOVE quirk (see FirstBattleFlags::stat_chg): the skip
    // tests MOVE_NONE's power, which is 0, so only the stat_chg disjunct is
    // live and it covers every player move.
    let skip_accuracy = attacker_is_player && !flags.stat_chg;
    if !skip_accuracy {
        // battle_script_commands.c:1093; stage-0 accuracy ratio is 1/1
        // (:578, :1066-1067).
        let roll = stream() as u32;
        if (roll % 100 + 1) > mv.accuracy() {
            return Outcome::Missed;
        }
    }
    if !damaging {
        // Growl: statbuffchange consumes nothing (:6818).
        if defender.atk_stage > 0 {
            defender.atk_stage -= 1;
        }
        return Outcome::AttackLowered;
    }
    // critcalc: roll consumed even while FIRST_BATTLE suppresses the result
    // (:1199 before :1200); 1-in-16 at critChance 0 (:588).
    let crit_roll = stream();
    let crit = crit_roll.is_multiple_of(16) && flags.inflict_dmg;
    let base = base_damage(attacker, defender, mv, crit);
    // adjustnormaldamage -> ApplyRandomDmgMultiplier (:1560).
    let damage = apply_variance(base, stream());
    defender.hp = defender.hp.saturating_sub(damage as u16);
    // The player's first landed damaging hit flips the tutorial flag when
    // the rival's HP bar finishes draining
    // (src/battle_controller_opponent.c:304-306) -- before this move's own
    // trailing secondary roll resolves on a later frame.
    if attacker_is_player {
        flags.inflict_dmg = true;
    }
    // seteffectwithchance: burned unconditionally for EFFECT_HIT moves
    // (:2789, Random() is the left operand).
    let _ = stream();
    Outcome::Hit { damage, crit }
}

/// The rival AI picking its move for one turn, consuming exactly what
/// `BattleAI_SetupAIData` + the three AI scripts + the tie-break consume
/// (the cited walk in `docs/rival-1/journal/`): 4 simulatedRNG rolls
/// (`src/battle_ai_script_commands.c:310`), the `AI_CV_AttackDown` rolls
/// for the Growl slot (`data/battle_ai_scripts.s:1129-1150`), and the
/// unconditional tie-break (`battle_ai_script_commands.c:408`).
///
/// `target` is the player's mon (the AI scores Growl against it). Returns
/// the chosen move.
pub fn rival_choose_move(target: &Mon, rival: &Mon, stream: &mut impl FnMut() -> u16) -> Move {
    // BattleAI_SetupAIData: 4 rolls, all 4 slots, even empty ones
    // (battle_ai_script_commands.c:299-311; empty slots score 0 via
    // CheckMoveLimitations, so they never join a tie).
    let mut simulated = [0u16; 4];
    for slot in &mut simulated {
        *slot = 100 - (stream() % 16);
    }

    // Scores start at 100. AI_CheckBadMove: no changes for Scratch/Growl
    // (EFFECT_HIT absent from its dispatch; AttackDown path has no score op
    // and no reachable roll -- both mons' second ability is NONE so
    // get_ability never rolls, battle_ai_script_commands.c:1169-1173).
    let mut scratch_score = 100i32;
    let mut growl_score = 100i32;

    // AI_CheckViability, AI_CV_AttackDown (battle_ai_scripts.s:1129-1150):
    if target.atk_stage == 6 {
        // if_stat_level_equal jumps straight to AttackDown3: roll A skipped.
    } else {
        growl_score -= 1; // :1131
        let user_hp_pct = 100 * rival.hp as u32 / rival.max_hp as u32;
        if user_hp_pct <= 90 {
            growl_score -= 1; // :1133
        }
        if target.atk_stage <= 3 {
            // roll A, :1137 -- only reachable at stage <= 3.
            if stream() % 256 >= 50 {
                growl_score -= 2; // :1138
            }
        }
    }
    let target_hp_pct = 100 * target.hp as u32 / target.max_hp as u32;
    if target_hp_pct <= 70 {
        growl_score -= 2; // :1142
    }
    // AttackDown4: Bulbasaur is Grass/Poison, not in the physical-type list
    // (:1153-1160), so roll B at :1149 always fires.
    if stream() % 256 >= 50 {
        growl_score -= 2; // :1150
    }

    // AI_TryToFaint (battle_ai_scripts.s:2767-2782): if Scratch's simulated
    // damage -- AI_CalcDmg with no crit (battle_script_commands.c:1225-1235)
    // scaled by simulatedRNG for its slot, minimum 1
    // (battle_ai_script_commands.c:1475-1500) -- would faint us, score +4.
    // Otherwise it is the most powerful move and unchanged. Growl's power is
    // < 2, so if_can_faint falls through and get_how_powerful_move_is
    // returns MOVE_POWER_DISCOURAGED (not NOT_MOST_POWERFUL,
    // battle_ai_script_commands.c:1022-1025): Growl is NOT penalised here.
    // The AI's Scratch slot is 0 (moves [SCRATCH, GROWL]).
    let sim_damage =
        (base_damage(rival, target, Move::Scratch, false) * simulated[0] as i32 / 100).max(1);
    if target.hp as i32 <= sim_damage {
        scratch_score += 4;
    }

    // Tie-break: Random() % numOfBestMoves, consumed even for one candidate
    // (battle_ai_script_commands.c:408).
    let tie = stream();
    if scratch_score == growl_score {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulbasaur() -> Mon {
        // Measured from gBattleMons on the committed battle (battle-truth).
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
    fn base_damage_matches_hand_arithmetic() {
        // Our Tackle vs Charmander: 11*35=385, *4=1540, /9=171, /50=3, +2=5.
        assert_eq!(
            base_damage(&bulbasaur(), &charmander(), Move::Tackle, false),
            5
        );
        // Crit doubles after the +2: 5*2=10.
        assert_eq!(
            base_damage(&bulbasaur(), &charmander(), Move::Tackle, true),
            10
        );
        // Rival Scratch vs us: 11*40=440, *4=1760, /10=176, /50=3, +2=5.
        assert_eq!(
            base_damage(&charmander(), &bulbasaur(), Move::Scratch, false),
            5
        );
    }

    #[test]
    fn variance_bounds_and_min_one() {
        for roll in 0..=u16::MAX {
            let out = apply_variance(5, roll);
            assert!((4..=5).contains(&out), "roll {roll} gave {out}");
        }
        assert_eq!(apply_variance(1, 15), 1, "85% of 1 floors to the minimum");
    }

    /// The committed battle, no emulator involved: from the battle-start
    /// `gRngValue` the tier-1 replay reports (`examples/fit-pacing.rs`), the
    /// engine's leaf set must contain the committed result -- 2409 frames,
    /// won, with the commit gates resolving to 13 on all three turns.
    #[test]
    fn engine_reproduces_the_committed_battle() {
        let leaves = crate::engine::simulate(
            &[4, 3, 3, 3],
            frlg_rng::Rng(0xed94271d),
            bulbasaur(),
            charmander(),
        );
        assert!(
            leaves.iter().any(|l| l.commit_durs == [13, 13, 13]
                && l.result == crate::engine::SimResult::Win { frames: 2409 }),
            "committed leaf missing from {leaves:?}"
        );
    }
}
