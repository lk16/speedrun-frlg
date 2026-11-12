//! A mechanics-exact battle engine: the decomp's damage pipeline, stat stages, status
//! conditions, move effects, trainer AI and trainer items, driven by the game's own LCG
//! ([`GameRng`]).
//!
//! What is exact (each with its source):
//!
//! * stats `((2 base + IV + EV/4) L / 100) + 5` (HP `+ L + 10`), nature ±10%
//!   (`decompiled/src/pokemon.c:2093-2170`);
//! * the damage pipeline of `CalculateBaseDamage` (`decompiled/src/pokemon.c:2385-2666`):
//!   badge boosts ×1.1 before stat stages, stat stages from `gStatStageRatios`
//!   (`pokemon.c:1442`), `(A × power × (2L/5 + 2) / D) / 50`, burn halves physical, screens
//!   halve non-crits, `+2` at the end; then crit ×2 (`Cmd_damagecalc`), STAB ×15/10 and the
//!   type chart (`Cmd_typecalc`), and the random factor `(100 - Random() % 16)` / 100 with
//!   a minimum of 1 (`ApplyRandomDmgMultiplier`, `battle_script_commands.c:1557-1570`);
//! * crits: `Random() % sCriticalHitChance[n]`, table {16, 8, 4, 3, 2}, n = +1 for
//!   high-crit moves, +2 for Focus Energy / Dire Hit (`battle_script_commands.c:588,1170-1207`);
//! * accuracy: `Random() % 100 + 1 > acc × sAccuracyStageRatios[stage]`
//!   (`battle_script_commands.c:570-586,1003-1106`);
//! * speed: stat stages, ×1.1 Thunder-Badge boost for the player, /4 when paralysed,
//!   ties by `Random() & 1` (`GetWhoStrikesFirst`, `battle_main.c:3400-3499`);
//! * status: sleep 2-5 turns decremented per move attempt (`battle_script_commands.c:2356`,
//!   `battle_util.c` CANCELLER_ASLEEP), freeze with a 20% thaw (`battle_util.c:1311-1315`),
//!   full paralysis 1 in 4 (`battle_util.c:1440`), confusion 2-5 turns with a 50% 40-power
//!   typeless self-hit (`battle_script_commands.c:2413`, `battle_util.c` CANCELLER_CONFUSED),
//!   burn/poison 1/8 and Toxic n/16 per turn end, multi-hit 2-5 with the real distribution
//!   (`battle_script_commands.c:6857-6871`);
//! * trainer AI: scores start at 100, highest wins, ties broken by `Random() % n`
//!   (`BattleAI_ChooseMoveOrAction`, `battle_ai_script_commands.c:363-408`); the
//!   CHECK_BAD_MOVE vetoes, TRY_TO_FAINT bonus and the CHECK_VIABILITY handlers for the
//!   effects route trainers carry are ported from `data/battle_ai_scripts.s`; the AI's
//!   damage estimate uses its own rolled variance per move
//!   (`battle_ai_script_commands.c:310,980-1026`); trainers use a Full Restore below 1/4 HP
//!   (`ShouldUseItem`, `battle_ai_switch_items.c`);
//! * X items raise one stage, Dire Hit is Focus Energy
//!   (`src/data/pokemon/item_effects.h:235-260`).
//!
//! Approximations, all listed: no PP (enemies rarely run out before the fight decides; the
//! player's healing policy keeps fights short), no enemy switching (the game's party order),
//! Attract ignores gender, Roar/Whirlwind/Teleport are no-ops in trainer battles (Teleport
//! ends a wild one), Future Sight / Spikes / Rapid Spin side effects / Foresight / Encore /
//! Nightmare are no-ops, Bide and Counter hold only the last turn's physical damage, weather
//! is modelled only as the fire/water multipliers, Thunder accuracy and sand/hail chip.

use crate::data::{Ability, Dex, PartyMon};
use crate::rng::GameRng;

/// `gStatStageRatios` (`decompiled/src/pokemon.c:1442-1457`), index = stage + 6.
const STAGE_RATIO: [(i32, i32); 13] = [
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

/// `sAccuracyStageRatios` (`decompiled/src/battle_script_commands.c:570-586`).
const ACC_RATIO: [(u32, u32); 13] = [
    (33, 100),
    (36, 100),
    (43, 100),
    (50, 100),
    (60, 100),
    (75, 100),
    (1, 1),
    (133, 100),
    (166, 100),
    (2, 1),
    (233, 100),
    (133, 50),
    (3, 1),
];

/// `sCriticalHitChance` (`decompiled/src/battle_script_commands.c:588`).
const CRIT_CHANCE: [u32; 5] = [16, 8, 4, 3, 2];

/// How the engine interprets a move's `EFFECT_*` (`battle_moves.h`). Everything that the
/// route's trainers, wilds and the player can carry maps to one of these; unmodelled
/// effects fall into `Nop` (status moves that then do nothing) or `PlainHit`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Effect {
    PlainHit,
    HighCrit,
    AlwaysHit,
    /// Secondary status on hit: (status, is `_HIT` secondary).
    SecStatus(SecStatus),
    FlinchHit,
    /// Stat change: (stat index 0-6, delta, on self).
    StatChange(u8, i8, bool),
    /// Stat-down secondary on hit (stat, -1).
    StatDownHit(u8),
    AllStatsUpHit,
    TwoStat(u8, u8, i8),
    MultiHit,
    DoubleHit,
    Twineedle,
    Recoil4,
    Recoil3,
    Absorb,
    DreamEater,
    Explosion,
    SonicBoom,
    DragonRage,
    LevelDamage,
    SuperFang,
    Ohko,
    Counter,
    Recharge,
    Charge,
    SemiInvulnDig,
    SemiInvulnFly,
    Rampage,
    Earthquake,
    Gust,
    Thunder,
    SkyUppercut,
    BrickBreak,
    QuickAttack,
    VitalThrow,
    RestoreHp,
    Rest,
    Sleep,
    Paralyze,
    Poison,
    Toxic,
    PoisonFang,
    Confuse,
    WillOWisp,
    Yawn,
    LeechSeed,
    Attract,
    Swagger,
    Disable,
    Reflect,
    LightScreen,
    Mist,
    Haze,
    Safeguard,
    FocusEnergy,
    Protect,
    Trap,
    Curse,
    Snore,
    Facade,
    Flail,
    Nop,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SecStatus {
    Poison,
    Burn,
    Freeze,
    Paralysis,
    Confusion,
}

pub fn effect_from(e: &str) -> Effect {
    use Effect::*;
    match e {
        "EFFECT_HIT"
        | "EFFECT_TRIPLE_KICK"
        | "EFFECT_PAY_DAY"
        | "EFFECT_PURSUIT"
        | "EFFECT_REVENGE"
        | "EFFECT_RAGE"
        | "EFFECT_RAPID_SPIN"
        | "EFFECT_SMELLINGSALT"
        | "EFFECT_FALSE_SWIPE" => PlainHit,
        "EFFECT_HIGH_CRITICAL" | "EFFECT_BLAZE_KICK" | "EFFECT_POISON_TAIL" => HighCrit,
        "EFFECT_ALWAYS_HIT" | "EFFECT_VITAL_THROW" => AlwaysHit,
        "EFFECT_POISON_HIT" => SecStatus(self::SecStatus::Poison),
        "EFFECT_BURN_HIT" | "EFFECT_THAW_HIT" => SecStatus(self::SecStatus::Burn),
        "EFFECT_FREEZE_HIT" => SecStatus(self::SecStatus::Freeze),
        "EFFECT_PARALYZE_HIT" => SecStatus(self::SecStatus::Paralysis),
        "EFFECT_CONFUSE_HIT" | "EFFECT_FLATTER" => SecStatus(self::SecStatus::Confusion),
        "EFFECT_POISON_FANG" => PoisonFang,
        "EFFECT_FLINCH_HIT" | "EFFECT_FLINCH_MINIMIZE_HIT" | "EFFECT_TWISTER" | "EFFECT_SNORE_" => {
            FlinchHit
        }
        "EFFECT_ATTACK_UP" => StatChange(0, 1, true),
        "EFFECT_ATTACK_UP_2" => StatChange(0, 2, true),
        "EFFECT_DEFENSE_UP" | "EFFECT_DEFENSE_CURL" | "EFFECT_HARDEN" => StatChange(1, 1, true),
        "EFFECT_DEFENSE_UP_2" => StatChange(1, 2, true),
        "EFFECT_SPEED_UP" => StatChange(2, 1, true),
        "EFFECT_SPEED_UP_2" => StatChange(2, 2, true),
        "EFFECT_SPECIAL_ATTACK_UP" => StatChange(3, 1, true),
        "EFFECT_SPECIAL_ATTACK_UP_2" => StatChange(3, 2, true),
        "EFFECT_SPECIAL_DEFENSE_UP_2" => StatChange(4, 2, true),
        "EFFECT_ACCURACY_UP" => StatChange(5, 1, true),
        "EFFECT_EVASION_UP" | "EFFECT_MINIMIZE" => StatChange(6, 1, true),
        "EFFECT_ATTACK_DOWN" => StatChange(0, -1, false),
        "EFFECT_ATTACK_DOWN_2" => StatChange(0, -2, false),
        "EFFECT_DEFENSE_DOWN" => StatChange(1, -1, false),
        "EFFECT_DEFENSE_DOWN_2" => StatChange(1, -2, false),
        "EFFECT_SPEED_DOWN" => StatChange(2, -1, false),
        "EFFECT_SPEED_DOWN_2" => StatChange(2, -2, false),
        "EFFECT_SPECIAL_ATTACK_DOWN" => StatChange(3, -1, false),
        "EFFECT_SPECIAL_DEFENSE_DOWN" => StatChange(4, -1, false),
        "EFFECT_ACCURACY_DOWN" => StatChange(5, -1, false),
        "EFFECT_EVASION_DOWN" => StatChange(6, -1, false),
        "EFFECT_DEFENSE_DOWN_HIT" => StatDownHit(1),
        "EFFECT_SPEED_DOWN_HIT" => StatDownHit(2),
        "EFFECT_SPECIAL_ATTACK_DOWN_HIT" => StatDownHit(3),
        "EFFECT_SPECIAL_DEFENSE_DOWN_HIT" => StatDownHit(4),
        "EFFECT_ACCURACY_DOWN_HIT" => StatDownHit(5),
        "EFFECT_ATTACK_DOWN_HIT" => StatDownHit(0),
        "EFFECT_ALL_STATS_UP_HIT" => AllStatsUpHit,
        "EFFECT_ATTACK_UP_HIT" => PlainHit,
        "EFFECT_CALM_MIND" => TwoStat(3, 4, 1),
        "EFFECT_BULK_UP" => TwoStat(0, 1, 1),
        "EFFECT_DRAGON_DANCE" => TwoStat(0, 2, 1),
        "EFFECT_COSMIC_POWER" => TwoStat(1, 4, 1),
        "EFFECT_GROWTH" => StatChange(3, 1, true),
        "EFFECT_MULTI_HIT" => MultiHit,
        "EFFECT_DOUBLE_HIT" => DoubleHit,
        "EFFECT_TWINEEDLE" => Twineedle,
        "EFFECT_RECOIL" => Recoil4,
        "EFFECT_DOUBLE_EDGE" => Recoil3,
        "EFFECT_ABSORB" => Absorb,
        "EFFECT_DREAM_EATER" => DreamEater,
        "EFFECT_EXPLOSION" => Explosion,
        "EFFECT_SONICBOOM" => SonicBoom,
        "EFFECT_DRAGON_RAGE" => DragonRage,
        "EFFECT_LEVEL_DAMAGE" => LevelDamage,
        "EFFECT_SUPER_FANG" => SuperFang,
        "EFFECT_OHKO" => Ohko,
        "EFFECT_COUNTER" => Counter,
        "EFFECT_RECHARGE" => Recharge,
        "EFFECT_RAZOR_WIND" | "EFFECT_SKY_ATTACK" | "EFFECT_SOLAR_BEAM" | "EFFECT_SKULL_BASH" => {
            Charge
        }
        "EFFECT_SEMI_INVULNERABLE" => SemiInvulnFly,
        "EFFECT_RAMPAGE" => Rampage,
        "EFFECT_EARTHQUAKE" | "EFFECT_MAGNITUDE" => Earthquake,
        "EFFECT_GUST" => Gust,
        "EFFECT_THUNDER" => Thunder,
        "EFFECT_SKY_UPPERCUT" => SkyUppercut,
        "EFFECT_BRICK_BREAK" => BrickBreak,
        "EFFECT_QUICK_ATTACK" => QuickAttack,
        "EFFECT_RESTORE_HP" | "EFFECT_SOFTBOILED" | "EFFECT_MORNING_SUN" | "EFFECT_SYNTHESIS"
        | "EFFECT_MOONLIGHT" => RestoreHp,
        "EFFECT_REST" => Rest,
        "EFFECT_SLEEP" => Sleep,
        "EFFECT_PARALYZE" => Paralyze,
        "EFFECT_POISON" => Poison,
        "EFFECT_TOXIC" => Toxic,
        "EFFECT_CONFUSE" => Confuse,
        "EFFECT_WILL_O_WISP" => WillOWisp,
        "EFFECT_YAWN" => Yawn,
        "EFFECT_LEECH_SEED" => LeechSeed,
        "EFFECT_ATTRACT" => Attract,
        "EFFECT_SWAGGER" => Swagger,
        "EFFECT_DISABLE" => Disable,
        "EFFECT_REFLECT" => Reflect,
        "EFFECT_LIGHT_SCREEN" => LightScreen,
        "EFFECT_MIST" => Mist,
        "EFFECT_HAZE" => Haze,
        "EFFECT_SAFEGUARD" => Safeguard,
        "EFFECT_FOCUS_ENERGY" => FocusEnergy,
        "EFFECT_PROTECT" | "EFFECT_ENDURE" => Protect,
        "EFFECT_TRAP" | "EFFECT_MEAN_LOOK" => Trap,
        "EFFECT_CURSE" => Curse,
        "EFFECT_SNORE" => Snore,
        "EFFECT_FACADE" => Facade,
        "EFFECT_FLAIL" => Flail,
        "EFFECT_OVERHEAT" => PlainHit,
        _ => Nop,
    }
}

/// Is the move's type physical (`IS_TYPE_PHYSICAL`, `decompiled/include/battle.h:475`:
/// types below `TYPE_MYSTERY` = 9).
fn is_physical(t: u8) -> bool {
    t < 9
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Status1 {
    #[default]
    None,
    /// Turns of sleep left (decremented per move attempt).
    Sleep(u8),
    Poison,
    /// Toxic with its rising counter.
    Toxic(u8),
    Burn,
    Freeze,
    Paralysis,
}

impl Status1 {
    pub fn any(&self) -> bool {
        !matches!(self, Status1::None)
    }
}

#[derive(Clone, Debug)]
pub struct Mon {
    pub species: usize,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    /// Atk, Def, Spe, SpA, SpD — raw stats with nature, no badge, no stages.
    pub stat: [i32; 5],
    /// Atk, Def, Spe, SpA, SpD, Acc, Eva: -6..=6.
    pub stages: [i8; 7],
    pub types: (u8, u8),
    pub ability: Ability,
    pub moves: Vec<usize>,
    pub status: Status1,
    pub confusion: u8,
    pub infatuated: bool,
    pub flinch: bool,
    /// Two-turn move in progress (second call executes it).
    pub charging: Option<usize>,
    /// Semi-invulnerable this turn (Dig/Fly vanish).
    pub underground: bool,
    pub airborne: bool,
    pub recharging: bool,
    pub trap: u8,
    pub seeded: bool,
    pub focus_energy: bool,
    pub disabled: Option<(usize, u8)>,
    pub protected: bool,
    pub protect_count: u8,
    pub rampage: u8,
    /// Player badge boosts ×100: Atk, Def, Spe, SpA, SpD (enemy: all 100).
    pub badge: [i32; 5],
    /// Physical damage taken this turn (Counter).
    pub counter_dmg: i32,
    pub yawned: bool,
}

/// Side state that outlives one enemy mon (screens guard the trainer's whole party).
#[derive(Default, Clone, Copy)]
struct Side {
    reflect: u8,
    light_screen: u8,
    mist: u8,
    safeguard: u8,
}

pub fn make_mon(
    dex: &Dex,
    species: usize,
    level: u32,
    iv: [u32; 6],
    ev: [u32; 6],
    nature: u8,
    badges: u8,
) -> Mon {
    let b = dex.species[species].base;
    let l = level as i32;
    let hp = ((2 * b[0] as i32 + iv[0] as i32 + ev[0] as i32 / 4) * l) / 100 + l + 10;
    let raw = |i: usize| ((2 * b[i] as i32 + iv[i] as i32 + ev[i] as i32 / 4) * l) / 100 + 5;
    let nat = frlg_mon::stats::NATURE_STAT_TABLE[nature as usize];
    let with = |v: i32, col: usize| match nat[col] {
        1 => v * 110 / 100,
        -1 => v * 90 / 100,
        _ => v,
    };
    // Badges: bit 0 Boulder (Atk), bit 2 Thunder (Spe), bit 4 Soul (Def), bit 6 Volcano
    // (SpA and SpD) — `CalculateBaseDamage` (`pokemon.c`) and `GetWhoStrikesFirst`
    // (`battle_main.c:3444-3446`).
    let bb = |bit: u8| if badges & (1 << bit) != 0 { 110 } else { 100 };
    Mon {
        species,
        level,
        hp,
        max_hp: hp,
        stat: [
            with(raw(1), 0),
            with(raw(2), 1),
            with(raw(3), 2),
            with(raw(4), 3),
            with(raw(5), 4),
        ],
        stages: [0; 7],
        types: dex.species[species].types,
        ability: dex.species[species].ability,
        moves: vec![],
        status: Status1::None,
        confusion: 0,
        infatuated: false,
        flinch: false,
        charging: None,
        underground: false,
        airborne: false,
        recharging: false,
        trap: 0,
        seeded: false,
        focus_energy: false,
        disabled: None,
        protected: false,
        protect_count: 0,
        rampage: 0,
        badge: [bb(0), bb(4), bb(2), bb(6), bb(6)],
        counter_dmg: 0,
        yawned: false,
    }
}

/// Mon from a trainer's party entry: the party IV applies to every stat, no EVs, neutral
/// nature (`decompiled/src/battle_main.c`, `CreateNPCTrainerParty`: a fixed personality
/// per party — the model uses neutral, an approximation worth one stat point either way).
pub fn enemy_mon(dex: &Dex, m: &PartyMon) -> Mon {
    let mut e = make_mon(dex, m.species, m.level, [m.iv; 6], [0; 6], 0, 0);
    e.moves = m.moves.clone();
    e
}

/// A mon with no species data (the guessed trainer groups): all base stats 55, Normal type.
pub fn generic_mon(dex: &Dex, level: u32) -> Mon {
    let sp = dex.id("SPECIES_RATTATA");
    let l = level as i32;
    let r = |b: i32| ((2 * b + 15) * l) / 100 + 5;
    let hp = ((2 * 55 + 15) * l) / 100 + l + 10;
    let mut m = make_mon(dex, sp, level, [15; 6], [0; 6], 0, 0);
    m.hp = hp;
    m.max_hp = hp;
    m.stat = [r(55); 5];
    m.types = (0, 0);
    m.ability = Ability::None;
    m.moves = vec![dex.move_id("MOVE_TACKLE")];
    m
}

fn staged(stat: i32, stage: i8) -> i32 {
    let (n, d) = STAGE_RATIO[(stage + 6) as usize];
    stat * n / d
}

/// Effective speed (`GetWhoStrikesFirst`, `battle_main.c:3428-3450`): stage ratio, the
/// player's Thunder-Badge ×1.1, paralysis /4.
fn speed(m: &Mon) -> u32 {
    let mut s = staged(m.stat[2], m.stages[2]) as u32;
    s = s * m.badge[2] as u32 / 100;
    if m.status == Status1::Paralysis {
        s /= 4;
    }
    s
}

/// `CalculateBaseDamage` + crit + STAB + type chart; `random` applies the 85-100 roll.
#[allow(clippy::too_many_arguments)]
fn calc_damage(
    dex: &Dex,
    att: &Mon,
    def: &Mon,
    mv: usize,
    power_override: u32,
    crit: bool,
    def_side: &Side,
    rng: Option<&mut GameRng>,
) -> i32 {
    let m = &dex.moves[mv];
    let mut power = if power_override > 0 {
        power_override
    } else {
        m.power
    } as i32;
    if power == 0 {
        return 0;
    }
    // Torrent/Blaze/Overgrow at 1/3 HP (`pokemon.c`, `CalculateBaseDamage`).
    let boosted = match att.ability {
        Ability::Torrent => m.mtype == 11,
        Ability::Blaze => m.mtype == 10,
        Ability::Overgrow => m.mtype == 12,
        _ => false,
    };
    if boosted && att.hp * 3 <= att.max_hp {
        power = power * 150 / 100;
    }
    let phys = is_physical(m.mtype);
    let (a_raw, d_raw, a_badge, d_badge, a_stage, d_stage) = if phys {
        (
            att.stat[0],
            def.stat[1],
            att.badge[0],
            def.badge[1],
            att.stages[0],
            def.stages[1],
        )
    } else {
        (
            att.stat[3],
            def.stat[4],
            att.badge[3],
            def.badge[4],
            att.stages[3],
            def.stages[4],
        )
    };
    let mut a = a_raw * a_badge / 100;
    let mut d = d_raw * d_badge / 100;
    // Thick Fat halves Fire/Ice special attack (`CalculateBaseDamage`).
    if def.ability == Ability::ThickFat && (m.mtype == 10 || m.mtype == 15) {
        a /= 2;
    }
    // Explosion halves defense.
    if effect_from(&m.effect) == Effect::Explosion {
        d /= 2;
    }
    // Crits ignore the attacker's drops and the defender's gains (`CalculateBaseDamage`).
    let a_mod = if crit && a_stage < 0 {
        a
    } else {
        staged(a, a_stage)
    };
    let d_mod = if crit && d_stage > 0 {
        d
    } else {
        staged(d, d_stage)
    };
    let l = att.level as i32;
    let mut dmg = a_mod.max(1);
    dmg *= power;
    dmg *= 2 * l / 5 + 2;
    dmg /= d_mod.max(1);
    dmg /= 50;
    if phys && att.status == Status1::Burn && att.ability != Ability::Guts {
        dmg /= 2;
    }
    if phys && def_side.reflect > 0 && !crit {
        dmg /= 2;
    }
    if !phys && def_side.light_screen > 0 && !crit {
        dmg /= 2;
    }
    if phys && dmg == 0 {
        dmg = 1;
    }
    dmg += 2;
    if crit {
        dmg *= 2;
    }
    if att.types.0 == m.mtype || att.types.1 == m.mtype {
        dmg = dmg * 15 / 10;
    }
    let eff = dex.eff(m.mtype, def.types);
    dmg = (dmg as f64 * eff) as i32;
    if let Some(rng) = rng {
        // `ApplyRandomDmgMultiplier` (`battle_script_commands.c:1557-1570`).
        let pct = 100 - rng.modn(16) as i32;
        if dmg != 0 {
            dmg = dmg * pct / 100;
            if dmg == 0 {
                dmg = 1;
            }
        }
    }
    if eff > 0.0 && dmg < 1 {
        dmg = 1;
    }
    dmg
}

/// Type effectiveness with Levitate (`Cmd_typecalc`).
fn effectiveness(dex: &Dex, mtype: u8, def: &Mon) -> f64 {
    if def.ability == Ability::Levitate && mtype == 4 {
        return 0.0;
    }
    dex.eff(mtype, def.types)
}

/// `Cmd_accuracycheck`: stage-adjusted accuracy, `Random() % 100 + 1 > calc` misses.
fn accuracy_check(att: &Mon, def: &Mon, move_acc: u32, rng: &mut GameRng) -> bool {
    if move_acc == 0 {
        return true; // Swift, Vital Throw, Feint Attack class.
    }
    let buff = (att.stages[5] - def.stages[6]).clamp(-6, 6);
    let (n, d) = ACC_RATIO[(buff + 6) as usize];
    let mut calc = move_acc * n / d;
    if att.ability == Ability::CompoundEyes {
        calc = calc * 130 / 100;
    }
    (rng.modn(100) + 1) <= calc
}

fn can_status(def: &Mon, s: SecStatus) -> bool {
    if def.status.any() {
        return false;
    }
    match s {
        SecStatus::Poison => {
            def.types.0 != 3
                && def.types.1 != 3
                && def.types.0 != 8
                && def.types.1 != 8
                && def.ability != Ability::Immunity
        }
        SecStatus::Burn => def.types.0 != 10 && def.types.1 != 10,
        SecStatus::Freeze => def.types.0 != 15 && def.types.1 != 15,
        SecStatus::Paralysis => def.ability != Ability::Limber,
        SecStatus::Confusion => true, // handled apart (confusion is status2)
    }
}

fn apply_stat_change(m: &mut Mon, stat: u8, delta: i8, mist: bool) -> bool {
    if delta < 0 && mist {
        return false;
    }
    if delta < 0
        && ((stat == 0 && m.ability == Ability::HyperCutter)
            || (stat == 5 && m.ability == Ability::KeenEye))
    {
        return false;
    }
    let s = &mut m.stages[stat as usize];
    let before = *s;
    *s = (*s + delta).clamp(-6, 6);
    *s != before
}

/// What the player brings to one fight.
pub struct Tactics {
    /// X items used on the opening turns while above half HP, in order
    /// (+1 stage each, Dire Hit = Focus Energy; `item_effects.h:235-260`).
    pub x_items: Vec<XItem>,
    /// Use a heal item when HP falls below this fraction.
    pub heal_below: f64,
    /// HP restored by the carried potion tier.
    pub heal: i32,
    /// Full Restores available (League): heal = full and cures status.
    pub full_restore: bool,
    /// Expendable teammates: against an enemy whose best move out-damages the fighter's
    /// maximum HP, one is sent out to take the hit while the fighter uses its next X item
    /// from the back (the universal speedrun sacrifice; each costs a faint + send-out).
    pub sacs: u32,
}

impl Default for Tactics {
    fn default() -> Self {
        Tactics {
            x_items: vec![],
            heal_below: 0.4,
            heal: 50,
            full_restore: false,
            sacs: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum XItem {
    Attack,
    Defend,
    Speed,
    Special,
    DireHit,
}

/// What happened in a fight.
#[derive(Default, Debug)]
pub struct Outcome {
    pub won: bool,
    /// Player turns (moves and item uses) and how many were item uses.
    pub turns: u32,
    pub item_turns: u32,
    /// Surprises that cost the human a reaction delay: misses, crits taken, full
    /// paralysis, confusion self-hits, status inflictions, enemy items.
    pub events: u32,
    /// Enemy mons defeated: (species, level).
    pub kos: Vec<(usize, u32)>,
    pub hp_frac: f64,
    pub end_status: Status1,
    pub x_used: u32,
    pub heals_used: u32,
    pub full_restores_used: u32,
    /// Sacrificial faints spent (each costs a faint animation and a send-out).
    pub sacs_used: u32,
}

enum Action {
    Move(usize),
    XItem(XItem),
    Heal,
    FullRestore,
    /// A teammate eats this turn's hit while the fighter uses its next X item.
    Sac(XItem),
}

/// Expected damage for the player's (full-knowledge) choice: no crit, mean roll, scaled
/// by accuracy.
fn player_score(dex: &Dex, att: &Mon, def: &Mon, mv: usize, side: &Side) -> f64 {
    let m = &dex.moves[mv];
    let fx = effect_from(&m.effect);
    let base = match fx {
        Effect::SonicBoom => {
            20.0 * if effectiveness(dex, m.mtype, def) > 0.0 {
                1.0
            } else {
                0.0
            }
        }
        Effect::DragonRage => 40.0,
        Effect::LevelDamage => att.level as f64,
        _ => {
            if m.power == 0 {
                return 0.0;
            }
            // Two-turn and recharge moves pay double time; halve their value.
            let time = match fx {
                Effect::Charge
                | Effect::SemiInvulnDig
                | Effect::SemiInvulnFly
                | Effect::Recharge => 0.5,
                Effect::Explosion | Effect::Ohko | Effect::Counter => return 0.0,
                _ => 1.0,
            };
            let d = calc_damage(dex, att, def, mv, 0, false, side, None) as f64;
            d * 0.925 * time
        }
    };
    let acc = if matches!(fx, Effect::AlwaysHit) || m.acc == 0 {
        1.0
    } else {
        let buff = (att.stages[5] - def.stages[6]).clamp(-6, 6);
        let (n, d) = ACC_RATIO[(buff + 6) as usize];
        ((m.acc * n / d) as f64 / 100.0).min(1.0)
    };
    base * acc
}

/// The AI's damage estimate for one move: real calc, no crit, its own rolled variance
/// (`battle_ai_script_commands.c:980-1026`; `simulatedRNG` at `:310`).
fn ai_damage(dex: &Dex, att: &Mon, def: &Mon, mv: usize, side: &Side, rng: &mut GameRng) -> i32 {
    let m = &dex.moves[mv];
    if m.power <= 1 {
        return 0;
    }
    let fx = effect_from(&m.effect);
    // `sDiscouragedPowerfulMoveEffects`: two-turn, recharge and self-KO moves are never
    // "most powerful".
    if matches!(
        fx,
        Effect::Explosion
            | Effect::Charge
            | Effect::SemiInvulnDig
            | Effect::SemiInvulnFly
            | Effect::Recharge
            | Effect::DreamEater
    ) {
        return 0;
    }
    let sim = 100 - rng.modn(16) as i32;
    let d = calc_damage(dex, att, def, mv, 0, false, side, None);
    let eff = effectiveness(dex, m.mtype, def);
    let d = (d as f64 * if eff > 0.0 { 1.0 } else { 0.0 }) as i32;
    (d * sim / 100).max(if eff > 0.0 { 1 } else { 0 })
}

/// The ported AI (`data/battle_ai_scripts.s`): CHECK_BAD_MOVE vetoes, TRY_TO_FAINT,
/// and the CHECK_VIABILITY handlers for the effects route trainers carry. Returns the
/// chosen move index into `user.moves`.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::if_same_then_else, clippy::collapsible_if)]
fn ai_choose(
    dex: &Dex,
    user: &Mon,
    target: &Mon,
    ai_flags: u32,
    user_side: &Side,
    target_side: &Side,
    _turn: u32,
    rng: &mut GameRng,
) -> Option<usize> {
    if user.moves.is_empty() {
        return None;
    }
    let n = user.moves.len();
    let mut score = vec![100i32; n];
    let mut dmg = vec![0i32; n];
    let user_hp = user.hp * 100 / user.max_hp.max(1);
    let target_hp = target.hp * 100 / target.max_hp.max(1);
    let target_faster = speed(target) > speed(user);
    for i in 0..n {
        let mv = user.moves[i];
        if let Some((d, _)) = user.disabled {
            if d == mv {
                score[i] = -20;
                continue;
            }
        }
        let m = &dex.moves[mv];
        let fx = effect_from(&m.effect);
        dmg[i] = ai_damage(dex, user, target, mv, target_side, rng);
        let eff = effectiveness(dex, m.mtype, target);

        // AI_CheckBadMove (always on: every trainer has the flag).
        if m.power > 1 && eff == 0.0 {
            score[i] -= 10;
            continue;
        }
        match fx {
            Effect::Sleep | Effect::Yawn => {
                if target.status.any()
                    || matches!(target.ability, Ability::Insomnia | Ability::VitalSpirit)
                {
                    score[i] -= 10;
                }
            }
            Effect::Poison | Effect::Toxic => {
                if !can_status(target, SecStatus::Poison) {
                    score[i] -= 10;
                }
            }
            Effect::Paralyze => {
                if target.status.any()
                    || !can_status(target, SecStatus::Paralysis)
                    || effectiveness(dex, m.mtype, target) == 0.0
                {
                    score[i] -= 10;
                }
            }
            Effect::WillOWisp => {
                if target.status.any() || !can_status(target, SecStatus::Burn) {
                    score[i] -= 10;
                }
            }
            Effect::Confuse | Effect::Swagger => {
                if target.confusion > 0 {
                    score[i] -= 10;
                }
            }
            Effect::StatChange(stat, _, on_self) => {
                if on_self && user.stages[stat as usize] >= 6 {
                    score[i] -= 10;
                }
                if !on_self && (target.stages[stat as usize] <= -6 || target_side.mist > 0) {
                    score[i] -= 10;
                }
            }
            Effect::TwoStat(a, b, _) => {
                if user.stages[a as usize] >= 6 && user.stages[b as usize] >= 6 {
                    score[i] -= 10;
                }
            }
            Effect::Ohko => {
                if eff == 0.0 || target.ability == Ability::Sturdy || user.level < target.level {
                    score[i] -= 10;
                }
            }
            Effect::DreamEater => {
                if !matches!(target.status, Status1::Sleep(_)) {
                    score[i] -= 8;
                }
            }
            Effect::RestoreHp | Effect::Rest => {
                if user.hp == user.max_hp {
                    score[i] -= 10;
                }
            }
            Effect::LeechSeed => {
                if target.seeded || target.types.0 == 12 || target.types.1 == 12 {
                    score[i] -= 10;
                }
            }
            Effect::FocusEnergy => {
                if user.focus_energy {
                    score[i] -= 10;
                }
            }
            Effect::Reflect => {
                if user_side.reflect > 0 {
                    score[i] -= 10;
                }
            }
            Effect::LightScreen => {
                if user_side.light_screen > 0 {
                    score[i] -= 10;
                }
            }
            Effect::Mist => {
                if user_side.mist > 0 {
                    score[i] -= 10;
                }
            }
            Effect::Safeguard => {
                if user_side.safeguard > 0 {
                    score[i] -= 10;
                }
            }
            Effect::Trap => {
                if target.trap > 0 {
                    score[i] -= 10;
                }
            }
            Effect::Disable if user.disabled.is_some() => {
                score[i] -= 10;
            }
            _ => {}
        }
        if score[i] < 0 {
            continue;
        }

        // AI_CheckViability, ported per effect (`data/battle_ai_scripts.s`).
        if ai_flags & 2 != 0 {
            match fx {
                Effect::Paralyze => {
                    // AI_CV_Paralyze.
                    if target_faster {
                        if !(rng.modn(256) < 20) {
                            score[i] += 3;
                        }
                    } else if user_hp <= 70 {
                        score[i] -= 1;
                    }
                }
                Effect::Confuse | Effect::Swagger => {
                    // AI_CV_Confuse.
                    if target_hp <= 70 {
                        if rng.modn(256) >= 128 {
                            score[i] -= 1;
                        } else {
                            if target_hp <= 50 {
                                score[i] -= 1;
                            }
                            if target_hp <= 30 {
                                score[i] -= 1;
                            }
                        }
                    }
                }
                Effect::Toxic | Effect::LeechSeed => {
                    // AI_CV_Toxic (leech seed shares it).
                    let has_attack = user.moves.iter().any(|&m2| dex.moves[m2].power > 1);
                    if has_attack {
                        if user_hp <= 50 && rng.modn(256) >= 50 {
                            score[i] -= 3;
                        } else if target_hp <= 50 && rng.modn(256) >= 50 {
                            score[i] -= 3;
                        }
                    }
                }
                Effect::StatChange(6, d, true) if d > 0 => {
                    // AI_CV_EvasionUp (Double Team, Minimize), trimmed to the
                    // statuses the model tracks.
                    if user_hp >= 90 && rng.modn(256) >= 100 {
                        score[i] += 3;
                    }
                    if user.stages[6] >= 3 && rng.modn(256) >= 128 {
                        score[i] -= 1;
                    }
                    if matches!(target.status, Status1::Toxic(_))
                        && user_hp > 50
                        && rng.modn(256) >= 50
                    {
                        score[i] += 3;
                    }
                    if target.seeded && rng.modn(256) >= 70 {
                        score[i] += 3;
                    }
                    if user_hp <= 70 && user.stages[6] != 6 {
                        if user_hp <= 40 || target_hp <= 40 {
                            score[i] -= 2;
                        } else if rng.modn(256) >= 70 {
                            score[i] -= 2;
                        }
                    }
                }
                Effect::StatChange(5, d, false) if d < 0 => {
                    // AI_CV_AccuracyDown, trimmed the same way.
                    if user_hp < 70 && rng.modn(256) >= 100 {
                        score[i] -= 1;
                    }
                    if user.stages[5] <= -2 && rng.modn(256) >= 80 {
                        score[i] -= 2;
                    }
                    if matches!(target.status, Status1::Toxic(_)) && rng.modn(256) >= 70 {
                        score[i] += 2;
                    }
                    if target.seeded && rng.modn(256) >= 70 {
                        score[i] += 2;
                    }
                    if user_hp <= 70 && target.stages[5] != 6 && (user_hp <= 40 || target_hp <= 40)
                    {
                        score[i] -= 2;
                    }
                }
                Effect::StatChange(0, d, true) | Effect::StatChange(3, d, true) if d > 0 => {
                    // AI_CV_AttackUp / AI_CV_SpAtkUp.
                    if user.stages[if matches!(fx, Effect::StatChange(0, _, _)) {
                        0
                    } else {
                        3
                    }] >= 3
                        && rng.modn(256) >= 100
                    {
                        score[i] -= 1;
                    } else if user_hp == 100 && rng.modn(256) >= 128 {
                        score[i] += 2;
                    }
                    if user_hp <= 70 {
                        if user_hp <= 40 {
                            score[i] -= 2;
                        } else if rng.modn(256) >= 70 {
                            score[i] -= 2;
                        }
                    }
                }
                Effect::StatChange(1, d, true)
                | Effect::StatChange(4, d, true)
                | Effect::TwoStat(_, _, d)
                    if d > 0 =>
                {
                    // AI_CV_DefenseUp shape; Calm Mind and friends are unhandled in
                    // FRLG's script and keep 100, so only the generic HP discourage.
                    if user_hp <= 40 {
                        score[i] -= 2;
                    }
                }
                Effect::StatChange(s, d, false) if d < 0 && s < 5 => {
                    // AI_CV_AttackDown family: discouraged off full HP and vs low targets.
                    if target.stages[s as usize] > -6 {
                        score[i] -= 1;
                        if user_hp <= 90 {
                            score[i] -= 1;
                        }
                        if target.stages[s as usize] < -3 && rng.modn(256) >= 50 {
                            score[i] -= 2;
                        }
                        if target_hp <= 70 {
                            score[i] -= 2;
                        }
                    }
                }
                Effect::RestoreHp => {
                    // AI_CV_Heal.
                    if user_hp == 100 {
                        score[i] -= 3;
                    } else if !target_faster {
                        score[i] -= 8;
                    } else if user_hp >= 70 && rng.modn(256) >= 30 {
                        score[i] -= 3;
                    } else if rng.modn(256) >= 20 {
                        score[i] += 2;
                    }
                }
                Effect::Rest => {
                    // AI_CV_Rest.
                    if !target_faster {
                        if user_hp == 100 {
                            score[i] -= 8;
                        } else if user_hp >= 40 && (user_hp > 50 || rng.modn(256) >= 70) {
                            score[i] -= 3;
                        } else if rng.modn(256) >= 10 {
                            score[i] += 3;
                        }
                    } else if user_hp >= 60 && (user_hp > 70 || rng.modn(256) >= 50) {
                        score[i] -= 3;
                    } else if rng.modn(256) >= 10 {
                        score[i] += 3;
                    }
                }
                Effect::HighCrit => {
                    // AI_CV_HighCrit.
                    if eff >= 2.0 && rng.modn(256) >= 128 {
                        score[i] += 1;
                    }
                }
                Effect::Explosion => {
                    // AI_CV_SelfKO, trimmed (no evasion bookkeeping on the player).
                    if user_hp >= 80 && !target_faster {
                        if rng.modn(256) >= 50 {
                            score[i] -= 3;
                        }
                    } else if user_hp <= 50 {
                        if rng.modn(256) >= 128 {
                            score[i] += 1;
                        }
                        if user_hp <= 30 && rng.modn(256) >= 50 {
                            score[i] += 1;
                        }
                    } else if rng.modn(256) >= 50 {
                        score[i] -= 1;
                    }
                }
                Effect::Recharge => {
                    // AI_CV_Recharge.
                    if eff < 1.0
                        || (!target_faster && user_hp > 40)
                        || (target_faster && user_hp >= 60)
                    {
                        score[i] -= 1;
                    }
                }
                Effect::Trap => {
                    // AI_CV_Trap.
                    if matches!(target.status, Status1::Toxic(_)) && rng.modn(256) >= 128 {
                        score[i] += 1;
                    }
                }
                Effect::AlwaysHit => {
                    // AI_CV_AlwaysHit.
                    if target.stages[6] >= 5 || user.stages[5] <= -4 {
                        score[i] += 1;
                    } else if (target.stages[6] >= 3 || user.stages[5] <= -2)
                        && rng.modn(256) >= 100
                    {
                        score[i] += 1;
                    }
                }
                Effect::Counter => {
                    // AI_CV_Counter, trimmed: discouraged at low HP, encouraged against
                    // physical attackers.
                    if matches!(target.status, Status1::Sleep(_)) || target.confusion > 0 {
                        score[i] -= 1;
                    }
                    if user_hp <= 30 && rng.modn(256) >= 10 {
                        score[i] -= 1;
                    }
                    if user_hp <= 50 && rng.modn(256) >= 100 {
                        score[i] -= 1;
                    }
                    let phys_target = is_physical(target.types.0) || is_physical(target.types.1);
                    if phys_target {
                        if rng.modn(256) >= 100 {
                            score[i] += 1;
                        }
                    } else if rng.modn(256) >= 50 {
                        score[i] -= 1;
                    }
                }
                Effect::Protect => {
                    // AI_CV_Protect, trimmed.
                    if user.protect_count >= 2 {
                        score[i] -= 2;
                    } else if matches!(target.status, Status1::Toxic(_)) || target.seeded {
                        score[i] += 2;
                    }
                    if user.protect_count >= 1 {
                        score[i] -= 1;
                        if rng.modn(256) >= 128 {
                            score[i] -= 1;
                        }
                    }
                }
                Effect::Sleep => {
                    // AI_CV_Sleep: +1 only when the *target* knows Dream Eater or
                    // Nightmare (the FRLG script checks AI_TARGET) — effectively never.
                }
                _ => {}
            }
        }

        // AI_TryToFaint.
        if ai_flags & 4 != 0 && dmg[i] > 0 && dmg[i] >= target.hp {
            score[i] += if matches!(fx, Effect::QuickAttack) {
                6
            } else {
                4
            };
        }
    }
    // TRY_TO_FAINT's -1 for damaging moves that are not the most powerful.
    if ai_flags & 4 != 0 {
        let best = dmg.iter().copied().max().unwrap_or(0);
        for i in 0..n {
            if dmg[i] > 0 && dmg[i] < best && dmg[i] < target.hp {
                score[i] -= 1;
            }
        }
    }
    // Highest score wins; ties broken by Random() % n
    // (`BattleAI_ChooseMoveOrAction`, `battle_ai_script_commands.c:389-408`).
    let best = *score.iter().max().unwrap();
    let tied: Vec<usize> = (0..n).filter(|&i| score[i] == best).collect();
    Some(tied[rng.modn(tied.len() as u32) as usize])
}

struct Exec<'a> {
    dex: &'a Dex,
    wild: bool,
}

/// One mon acts. Returns true if the battle should end (wild Teleport flee).
#[allow(clippy::too_many_arguments)]
fn do_move(
    ex: &Exec,
    att: &mut Mon,
    def: &mut Mon,
    mv: usize,
    att_side: &mut Side,
    def_side: &mut Side,
    att_is_player: bool,
    acted_first: bool,
    out: &mut Outcome,
    rng: &mut GameRng,
) -> bool {
    let dex = ex.dex;

    // --- attack cancelers (`battle_util.c`, AtkCanceller_UnableToUseMove order).
    if att.recharging {
        att.recharging = false;
        return false;
    }
    if let Status1::Sleep(t) = att.status {
        if t > 1 {
            att.status = Status1::Sleep(t - 1);
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
        att.status = Status1::None; // woke up, acts this turn
    }
    if att.status == Status1::Freeze {
        // 20% thaw (`battle_util.c:1311-1315`).
        if rng.modn(5) != 0 {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
        att.status = Status1::None;
    }
    if att.flinch {
        att.flinch = false;
        if att_is_player {
            out.events += 1;
        }
        return false;
    }
    if att.confusion > 0 {
        att.confusion -= 1;
        if att.confusion > 0 && rng.modn(2) == 0 {
            // 50% 40-power typeless physical self-hit (CANCELLER_CONFUSED).
            let a = staged(att.stat[0], att.stages[0]) * att.badge[0] / 100;
            let d = staged(att.stat[1], att.stages[1]) * att.badge[1] / 100;
            let l = att.level as i32;
            let mut dmg = (a.max(1) * 40 * (2 * l / 5 + 2) / d.max(1)) / 50 + 2;
            dmg = dmg * (100 - rng.modn(16) as i32) / 100;
            att.hp -= dmg.max(1);
            out.events += 1;
            return false;
        }
    }
    if att.status == Status1::Paralysis && rng.modn(4) == 0 {
        // Full paralysis (`battle_util.c:1440`).
        out.events += 1;
        return false;
    }
    if att.infatuated && rng.modn(2) == 0 {
        out.events += 1;
        return false;
    }

    let m = dex.moves[mv].clone();
    let fx = effect_from(&m.effect);

    // --- two-turn moves.
    if att.charging == Some(mv) {
        att.charging = None;
        att.underground = false;
        att.airborne = false;
    } else {
        match fx {
            Effect::Charge => {
                att.charging = Some(mv);
                return false;
            }
            Effect::SemiInvulnDig => {
                att.charging = Some(mv);
                att.underground = true;
                return false;
            }
            Effect::SemiInvulnFly => {
                att.charging = Some(mv);
                att.airborne = true;
                return false;
            }
            _ => {}
        }
    }
    // Dig and Fly are distinguished by the move, not the effect.
    if m.name == "MOVE_DIG" && att.charging == Some(mv) {
        att.underground = true;
        att.airborne = false;
    }

    // --- pure self/field moves need no accuracy roll.
    match fx {
        Effect::StatChange(stat, delta, true) => {
            apply_stat_change(att, stat, delta, false);
            return false;
        }
        Effect::TwoStat(a, b, d) => {
            apply_stat_change(att, a, d, false);
            apply_stat_change(att, b, d, false);
            return false;
        }
        Effect::RestoreHp => {
            att.hp = (att.hp + att.max_hp / 2).min(att.max_hp);
            return false;
        }
        Effect::Rest => {
            att.hp = att.max_hp;
            att.status = Status1::Sleep(3);
            return false;
        }
        Effect::Reflect => {
            att_side.reflect = 5;
            return false;
        }
        Effect::LightScreen => {
            att_side.light_screen = 5;
            return false;
        }
        Effect::Mist => {
            att_side.mist = 5;
            return false;
        }
        Effect::Safeguard => {
            att_side.safeguard = 5;
            return false;
        }
        Effect::Haze => {
            att.stages = [0; 7];
            def.stages = [0; 7];
            return false;
        }
        Effect::FocusEnergy => {
            att.focus_energy = true;
            return false;
        }
        Effect::Protect => {
            // 50% fail per consecutive use (approximation of the 1/X counter).
            if att.protect_count == 0 || rng.modn(2) == 0 {
                att.protected = true;
                att.protect_count += 1;
            } else {
                att.protect_count = 0;
            }
            return false;
        }
        Effect::Curse => {
            // Non-ghost Curse for every user on the routes.
            apply_stat_change(att, 0, 1, false);
            apply_stat_change(att, 1, 1, false);
            apply_stat_change(att, 2, -1, false);
            return false;
        }
        Effect::Nop => {
            if m.name == "MOVE_TELEPORT" && ex.wild && !att_is_player {
                return true; // the wild mon (an Abra) flees
            }
            return false;
        }
        _ => {}
    }

    // --- targeted moves from here.
    if def.protected {
        return false;
    }
    // Semi-invulnerable target (`Cmd_accuracycheck` STATUS3 check + the moves that pierce).
    let mut power_mult = 1u32;
    if def.underground {
        if matches!(fx, Effect::Earthquake) {
            power_mult = 2;
        } else {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
    }
    if def.airborne {
        if matches!(fx, Effect::Gust | Effect::Thunder | Effect::SkyUppercut) {
            if matches!(fx, Effect::Gust) {
                power_mult = 2;
            }
        } else {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
    }

    // --- status-only targeted moves (their own accuracy, no damage pipeline).
    let status_only = matches!(
        fx,
        Effect::Sleep
            | Effect::Paralyze
            | Effect::Poison
            | Effect::Toxic
            | Effect::WillOWisp
            | Effect::Confuse
            | Effect::Swagger
            | Effect::Yawn
            | Effect::LeechSeed
            | Effect::Attract
            | Effect::Disable
            | Effect::Trap
            | Effect::StatChange(_, _, false)
    );
    if status_only {
        // Thunder Wave respects type immunity (`Cmd_typecalc` runs for it).
        if m.name == "MOVE_THUNDER_WAVE" && effectiveness(dex, m.mtype, def) == 0.0 {
            return false;
        }
        if !accuracy_check(att, def, m.acc, rng) {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
        if def_side.safeguard > 0
            && matches!(
                fx,
                Effect::Sleep
                    | Effect::Paralyze
                    | Effect::Poison
                    | Effect::Toxic
                    | Effect::WillOWisp
                    | Effect::Confuse
                    | Effect::Yawn
            )
        {
            return false;
        }
        let hit_player = !att_is_player;
        match fx {
            Effect::Sleep => {
                if !def.status.any()
                    && !matches!(def.ability, Ability::Insomnia | Ability::VitalSpirit)
                {
                    // 2-5 turns (`battle_script_commands.c:2356`).
                    def.status = Status1::Sleep(((rng.roll() & 3) + 2) as u8);
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::Yawn => {
                if !def.status.any() {
                    def.yawned = true;
                }
            }
            Effect::Paralyze => {
                if can_status(def, SecStatus::Paralysis) {
                    def.status = Status1::Paralysis;
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::Poison => {
                if can_status(def, SecStatus::Poison) {
                    def.status = Status1::Poison;
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::Toxic => {
                if can_status(def, SecStatus::Poison) {
                    def.status = Status1::Toxic(0);
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::WillOWisp => {
                if can_status(def, SecStatus::Burn) {
                    def.status = Status1::Burn;
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::Confuse | Effect::Swagger => {
                if def.confusion == 0 {
                    // 2-5 turns (`battle_script_commands.c:2413`).
                    def.confusion = ((rng.modn(4)) + 2) as u8;
                    if matches!(fx, Effect::Swagger) {
                        apply_stat_change(def, 0, 2, def_side.mist > 0);
                    }
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::LeechSeed => {
                if !def.seeded && def.types.0 != 12 && def.types.1 != 12 {
                    def.seeded = true;
                }
            }
            Effect::Attract => {
                // Gender unmodelled; infatuation applies (50% immobile per turn).
                def.infatuated = true;
                if hit_player {
                    out.events += 1;
                }
            }
            Effect::Disable => {
                if def.disabled.is_none() && !def.moves.is_empty() {
                    let k = rng.modn(def.moves.len() as u32) as usize;
                    // 2-5 turns (`battle_script_commands.c:7632`).
                    def.disabled = Some((def.moves[k], ((rng.roll() & 3) + 2) as u8));
                    if hit_player {
                        out.events += 1;
                    }
                }
            }
            Effect::Trap => {
                if def.trap == 0 {
                    // 3-6 turns (`battle_script_commands.c:2490`).
                    def.trap = ((rng.roll() & 3) + 3) as u8;
                }
            }
            Effect::StatChange(stat, delta, false) => {
                if def.ability == Ability::ShieldDust && delta < 0 {
                    // Shield Dust blocks secondaries, not status moves; no-op here.
                }
                apply_stat_change(def, stat, delta, def_side.mist > 0);
            }
            _ => {}
        }
        return false;
    }

    // --- damaging moves.
    if effectiveness(dex, m.mtype, def) == 0.0 {
        if att_is_player {
            out.events += 1;
        }
        return false;
    }
    // OHKO accuracy: `(user level - target level) + 30`, fails from below
    // (`AccuracyCalcHelper`, `battle_script_commands.c`).
    if matches!(fx, Effect::Ohko) {
        if att.level < def.level || def.ability == Ability::Sturdy {
            return false;
        }
        let acc = (att.level - def.level + 30).min(100);
        if rng.modn(100) + 1 > acc {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
        let d = def.hp;
        def.hp = 0;
        if is_physical(m.mtype) {
            def.counter_dmg = d;
        }
        if !att_is_player {
            out.events += 1;
        }
        return false;
    }
    if matches!(fx, Effect::Counter) {
        if att.counter_dmg > 0 {
            def.hp -= att.counter_dmg * 2;
            if !att_is_player {
                out.events += 1;
            }
        }
        return false;
    }
    if !matches!(fx, Effect::AlwaysHit) && !accuracy_check(att, def, m.acc, rng) {
        if att_is_player {
            out.events += 1;
        }
        return false;
    }

    let hits = match fx {
        // 2-5 with the real distribution (`Cmd_setmultihitcounter`).
        Effect::MultiHit => {
            let r = rng.roll() & 3;
            if r > 1 {
                ((rng.roll() & 3) + 2) as u32
            } else {
                (r + 2) as u32
            }
        }
        Effect::DoubleHit | Effect::Twineedle => 2,
        _ => 1,
    };

    let mut total = 0i32;
    for _ in 0..hits {
        if def.hp <= 0 {
            break;
        }
        // Crit (`Cmd_critcalc`).
        let mut crit_idx = 0usize;
        if matches!(fx, Effect::HighCrit) {
            crit_idx += 1;
        }
        if att.focus_energy {
            crit_idx += 2;
        }
        let crit = !matches!(def.ability, Ability::BattleArmor | Ability::ShellArmor)
            && rng.modn(CRIT_CHANCE[crit_idx.min(4)]) == 0;
        let dmg = match fx {
            Effect::SonicBoom => 20,
            Effect::DragonRage => 40,
            Effect::LevelDamage => att.level as i32,
            Effect::SuperFang => (def.hp / 2).max(1),
            _ => calc_damage(
                dex,
                att,
                def,
                mv,
                m.power * power_mult,
                crit,
                def_side,
                Some(rng),
            ),
        };
        if crit && !att_is_player {
            out.events += 1;
        }
        if crit && att_is_player {
            out.events += 1;
        }
        def.hp -= dmg;
        total += dmg;
        if matches!(fx, Effect::Twineedle) && can_status(def, SecStatus::Poison) && rng.pct(20) {
            def.status = Status1::Poison;
        }
    }
    if is_physical(m.mtype) {
        def.counter_dmg = total.min(def.hp.max(0) + total);
    }

    // Secondary effects (`Cmd_seteffectwithchance`: `Random() % 100 < chance`).
    let sec_ok = def.hp > 0 && def.ability != Ability::ShieldDust && def_side.safeguard == 0;
    match fx {
        Effect::SecStatus(s) if sec_ok && rng.pct(m.chance) => {
            if s == SecStatus::Confusion {
                if def.confusion == 0 {
                    def.confusion = ((rng.modn(4)) + 2) as u8;
                    if !att_is_player {
                        out.events += 1;
                    }
                }
            } else if can_status(def, s) {
                def.status = match s {
                    SecStatus::Poison => Status1::Poison,
                    SecStatus::Burn => Status1::Burn,
                    SecStatus::Freeze => Status1::Freeze,
                    SecStatus::Paralysis => Status1::Paralysis,
                    SecStatus::Confusion => unreachable!(),
                };
                if !att_is_player {
                    out.events += 1;
                }
            }
        }
        Effect::PoisonFang if sec_ok && rng.pct(m.chance.max(30)) => {
            if can_status(def, SecStatus::Poison) {
                def.status = Status1::Toxic(0);
                if !att_is_player {
                    out.events += 1;
                }
            }
        }
        Effect::FlinchHit if sec_ok && acted_first && rng.pct(m.chance) => {
            if def.ability != Ability::InnerFocus {
                def.flinch = true;
            }
        }
        Effect::StatDownHit(stat) if sec_ok && rng.pct(m.chance) => {
            apply_stat_change(def, stat, -1, def_side.mist > 0);
        }
        Effect::AllStatsUpHit if rng.pct(m.chance) => {
            for s in 0..5 {
                apply_stat_change(att, s, 1, false);
            }
        }
        Effect::Recoil4 => {
            if att.ability != Ability::RockHead {
                att.hp -= (total / 4).max(1);
            }
        }
        Effect::Recoil3 => {
            if att.ability != Ability::RockHead {
                att.hp -= (total / 3).max(1);
            }
        }
        Effect::Absorb | Effect::DreamEater => {
            if !matches!(fx, Effect::DreamEater) || matches!(def.status, Status1::Sleep(_)) {
                att.hp = (att.hp + (total / 2).max(1)).min(att.max_hp);
            }
        }
        Effect::Explosion => {
            att.hp = 0;
        }
        Effect::Recharge => {
            att.recharging = true;
        }
        Effect::BrickBreak => {
            def_side.reflect = 0;
            def_side.light_screen = 0;
        }
        _ => {}
    }
    // Static / Poison Point / Effect Spore on contact: physical moves, 30%/30%/10%
    // (`battle_util.c`, ABILITYEFFECT_ON_DAMAGE). Modelled for the defender's ability.
    if def.hp > 0 && total > 0 && is_physical(m.mtype) {
        match def.ability {
            Ability::Static if rng.pct(30) && can_status(att, SecStatus::Paralysis) => {
                att.status = Status1::Paralysis;
                if att_is_player {
                    out.events += 1;
                }
            }
            Ability::PoisonPoint if rng.pct(30) && can_status(att, SecStatus::Poison) => {
                att.status = Status1::Poison;
                if att_is_player {
                    out.events += 1;
                }
            }
            Ability::EffectSpore if rng.pct(10) => {
                let r = rng.modn(3);
                if r == 0 && can_status(att, SecStatus::Poison) {
                    att.status = Status1::Poison;
                } else if r == 1 && can_status(att, SecStatus::Paralysis) {
                    att.status = Status1::Paralysis;
                } else if r == 2
                    && !att.status.any()
                    && !matches!(att.ability, Ability::Insomnia | Ability::VitalSpirit)
                {
                    att.status = Status1::Sleep(((rng.roll() & 3) + 2) as u8);
                }
                if att_is_player && att.status.any() {
                    out.events += 1;
                }
            }
            _ => {}
        }
    }
    false
}

/// End-of-turn residual damage and timers for one mon.
fn end_of_turn(m: &mut Mon, other: &mut Mon, side: &mut Side, out: &mut Outcome, is_player: bool) {
    if m.hp <= 0 {
        return;
    }
    match m.status {
        Status1::Burn | Status1::Poison => {
            m.hp -= (m.max_hp / 8).max(1);
            if is_player {
                out.events += out.turns & 1; // occasional reaction, not every tick
            }
        }
        Status1::Toxic(n) => {
            let n = (n + 1).min(15);
            m.hp -= ((m.max_hp * n as i32) / 16).max(1);
            m.status = Status1::Toxic(n);
        }
        _ => {}
    }
    if m.seeded && other.hp > 0 {
        let d = (m.max_hp / 8).max(1);
        m.hp -= d;
        other.hp = (other.hp + d).min(other.max_hp);
    }
    if m.trap > 0 {
        m.trap -= 1;
        m.hp -= (m.max_hp / 16).max(1);
    }
    if m.yawned && !m.status.any() {
        m.yawned = false;
        m.status = Status1::Sleep(3);
        if is_player {
            out.events += 1;
        }
    }
    if let Some((mv, t)) = m.disabled {
        m.disabled = if t > 1 { Some((mv, t - 1)) } else { None };
    }
    m.protected = false;
    m.counter_dmg = 0;
    side.reflect = side.reflect.saturating_sub(1);
    side.light_screen = side.light_screen.saturating_sub(1);
    side.mist = side.mist.saturating_sub(1);
    side.safeguard = side.safeguard.saturating_sub(1);
}

/// Fights the trainer's party (or one wild mon) in order. `me` carries its state in and
/// out (stages reset between enemy mons only for the enemy side; the player's X-item
/// stages persist through the trainer's whole party, like the real game).
#[allow(clippy::too_many_arguments)]
pub fn fight(
    dex: &Dex,
    me: &mut Mon,
    tac: &Tactics,
    foes: &[Mon],
    ai_flags: u32,
    mut ai_heals: u32,
    wild: bool,
    rng: &mut GameRng,
) -> Outcome {
    let mut out = Outcome::default();
    let mut my_side = Side::default();
    let mut en_side = Side::default();
    let ex = Exec { dex, wild };
    let mut setup_idx = 0usize;
    let mut sacs_left = tac.sacs;
    for foe in foes {
        let mut e = foe.clone();
        let mut guard = 0;
        while e.hp > 0 && me.hp > 0 && guard < 120 {
            guard += 1;
            out.turns += 1;

            // --- player action.
            let my_frac = me.hp as f64 / me.max_hp as f64;
            let forced = me.recharging || me.charging.is_some() || me.rampage > 0;
            // The human knows the enemy's strongest move: heal when inside its max-roll
            // KO range (no crit) and no sure kill is on the table this turn.
            let danger = e
                .moves
                .iter()
                .map(|&mv| calc_damage(dex, &e, me, mv, 0, false, &my_side, None))
                .max()
                .unwrap_or(0);
            let sure_kill = me.moves.iter().any(|&mv| {
                let m = &dex.moves[mv];
                let fx = effect_from(&m.effect);
                m.power > 0
                    && (m.acc == 0 || m.acc >= 95 || matches!(fx, Effect::AlwaysHit))
                    && !matches!(
                        fx,
                        Effect::Charge | Effect::SemiInvulnDig | Effect::SemiInvulnFly
                    )
                    && (calc_damage(dex, me, &e, mv, 0, false, &en_side, None) * 85) / 100 >= e.hp
            });
            // Healing is only worth a turn if it actually leaves the KO range.
            let in_danger = me.hp <= danger && me.max_hp > danger && !sure_kill;
            let my_action = if forced {
                Action::Move(me.charging.unwrap_or(me.moves[0]))
            } else if danger >= me.max_hp
                && sacs_left > 0
                && setup_idx < tac.x_items.len()
                && !sure_kill
            {
                sacs_left -= 1;
                let x = tac.x_items[setup_idx];
                setup_idx += 1;
                Action::Sac(x)
            } else if tac.full_restore
                && (matches!(me.status, Status1::Sleep(_) | Status1::Freeze)
                    || (my_frac < tac.heal_below && me.max_hp > danger)
                    || in_danger)
            {
                Action::FullRestore
            } else if (my_frac < tac.heal_below && tac.heal as f64 + me.hp as f64 > danger as f64)
                || (in_danger && me.hp + tac.heal > danger)
            {
                Action::Heal
            } else if setup_idx < tac.x_items.len() && me.hp * 2 > me.max_hp && !sure_kill {
                let x = tac.x_items[setup_idx];
                setup_idx += 1;
                Action::XItem(x)
            } else {
                // Full-knowledge pick: kill if a min-roll kill exists, else best expected.
                let mut best: Option<(usize, f64)> = None;
                let mut killer: Option<(usize, f64)> = None;
                for &mv in &me.moves {
                    if let Some((d, _)) = me.disabled {
                        if d == mv {
                            continue;
                        }
                    }
                    let s = player_score(dex, me, &e, mv, &en_side);
                    if s <= 0.0 {
                        continue;
                    }
                    let m = &dex.moves[mv];
                    let min_roll =
                        (calc_damage(dex, me, &e, mv, 0, false, &en_side, None) * 85) / 100;
                    let acc_ok = m.acc == 0
                        || m.acc >= 95
                        || matches!(effect_from(&m.effect), Effect::AlwaysHit);
                    if min_roll >= e.hp
                        && acc_ok
                        && !matches!(
                            effect_from(&m.effect),
                            Effect::Charge | Effect::SemiInvulnDig | Effect::SemiInvulnFly
                        )
                    {
                        let spd = dex.moves[mv].prio as f64;
                        if killer.is_none() || spd > killer.unwrap().1 {
                            killer = Some((mv, spd));
                        }
                    }
                    if best.is_none() || s > best.unwrap().1 {
                        best = Some((mv, s));
                    }
                }
                match killer.or(best) {
                    Some((mv, _)) => Action::Move(mv),
                    None => Action::Move(me.moves[0]),
                }
            };

            // --- enemy action.
            let en_forced = e.recharging || e.charging.is_some();
            let en_move = if en_forced {
                e.charging
            } else if ai_heals > 0 && e.hp * 4 < e.max_hp {
                // `ShouldUseItem`: Full Restore under 1/4 (`battle_ai_switch_items.c`).
                None
            } else {
                ai_choose(dex, &e, me, ai_flags, &en_side, &my_side, out.turns, rng)
                    .map(|i| e.moves[i])
            };
            let enemy_heals_now = !en_forced && en_move.is_none() && ai_heals > 0;

            // --- order (`GetWhoStrikesFirst`): priority, then speed, ties Random() & 1.
            let my_prio = match my_action {
                Action::Move(mv) => dex.moves[mv].prio,
                _ => 6, // item use happens before moves
            };
            let en_prio = if enemy_heals_now {
                6
            } else {
                en_move.map_or(0, |mv| dex.moves[mv].prio)
            };
            let me_first = if my_prio != en_prio {
                my_prio > en_prio
            } else {
                let (s1, s2) = (speed(me), speed(&e));
                if s1 != s2 {
                    s1 > s2
                } else {
                    rng.roll() & 1 == 0
                }
            };

            let act_player = |me: &mut Mon,
                              e: &mut Mon,
                              my_side: &mut Side,
                              en_side: &mut Side,
                              first: bool,
                              out: &mut Outcome,
                              rng: &mut GameRng| {
                match my_action {
                    Action::Move(mv) => {
                        do_move(&ex, me, e, mv, my_side, en_side, true, first, out, rng);
                    }
                    Action::XItem(x) => {
                        out.item_turns += 1;
                        out.x_used += 1;
                        match x {
                            XItem::Attack => {
                                apply_stat_change(me, 0, 1, false);
                            }
                            XItem::Defend => {
                                apply_stat_change(me, 1, 1, false);
                            }
                            XItem::Speed => {
                                apply_stat_change(me, 2, 1, false);
                            }
                            XItem::Special => {
                                apply_stat_change(me, 3, 1, false);
                            }
                            XItem::DireHit => me.focus_energy = true,
                        }
                    }
                    Action::Heal => {
                        out.item_turns += 1;
                        out.heals_used += 1;
                        me.hp = (me.hp + tac.heal).min(me.max_hp);
                    }
                    Action::FullRestore => {
                        out.item_turns += 1;
                        out.full_restores_used += 1;
                        me.hp = me.max_hp;
                        me.status = Status1::None;
                        me.confusion = 0;
                    }
                    Action::Sac(x) => {
                        out.sacs_used += 1;
                        out.x_used += 1;
                        match x {
                            XItem::Attack => {
                                apply_stat_change(me, 0, 1, false);
                            }
                            XItem::Defend => {
                                apply_stat_change(me, 1, 1, false);
                            }
                            XItem::Speed => {
                                apply_stat_change(me, 2, 1, false);
                            }
                            XItem::Special => {
                                apply_stat_change(me, 3, 1, false);
                            }
                            XItem::DireHit => me.focus_energy = true,
                        }
                    }
                }
            };
            let mut act_enemy = |me: &mut Mon,
                                 e: &mut Mon,
                                 my_side: &mut Side,
                                 en_side: &mut Side,
                                 first: bool,
                                 out: &mut Outcome,
                                 rng: &mut GameRng|
             -> bool {
                if enemy_heals_now {
                    ai_heals -= 1;
                    e.hp = e.max_hp;
                    e.status = Status1::None;
                    e.confusion = 0;
                    out.events += 1;
                    return false;
                }
                match en_move {
                    Some(mv) => do_move(&ex, e, me, mv, en_side, my_side, false, first, out, rng),
                    None => false,
                }
            };

            let sac_turn = matches!(my_action, Action::Sac(_));
            if std::env::var_os("FRLG_BATTLE_TRACE").is_some() {
                let an = match &my_action {
                    Action::Move(mv) => dex.moves[*mv].name.clone(),
                    Action::XItem(x) => format!("X{x:?}"),
                    Action::Heal => "Heal".into(),
                    Action::FullRestore => "FullRestore".into(),
                    Action::Sac(x) => format!("Sac+{x:?}"),
                };
                eprintln!(
                    "t{} me {}/{} st{:?} stg{:?} | foe {} {}/{} | act {} | en {:?} first {} danger {} kill {} sacs {}",
                    out.turns, me.hp, me.max_hp, me.status, me.stages, foe.species, e.hp, e.max_hp,
                    an, en_move.map(|m| dex.moves[m].name.clone()), me_first, danger, sure_kill, sacs_left
                );
            }
            let fled = if me_first {
                act_player(me, &mut e, &mut my_side, &mut en_side, true, &mut out, rng);
                if e.hp > 0 && me.hp > 0 && !sac_turn {
                    act_enemy(me, &mut e, &mut my_side, &mut en_side, false, &mut out, rng)
                } else {
                    false
                }
            } else if sac_turn {
                // The enemy's move falls on the sacrifice; the fighter acts freely.
                act_player(me, &mut e, &mut my_side, &mut en_side, false, &mut out, rng);
                false
            } else {
                let f = act_enemy(me, &mut e, &mut my_side, &mut en_side, true, &mut out, rng);
                if me.hp > 0 && e.hp > 0 && !f {
                    act_player(me, &mut e, &mut my_side, &mut en_side, false, &mut out, rng);
                }
                f
            };
            if fled {
                // A wild Teleport: the battle just ends (no catch).
                out.won = false;
                out.hp_frac = (me.hp.max(0)) as f64 / me.max_hp as f64;
                out.end_status = me.status;
                return out;
            }

            end_of_turn(me, &mut e, &mut my_side, &mut out, true);
            end_of_turn(&mut e, me, &mut en_side, &mut out, false);
        }
        if me.hp <= 0 || e.hp > 0 {
            out.won = false;
            out.hp_frac = 0.0;
            out.end_status = me.status;
            return out;
        }
        out.kos.push((foe.species, foe.level));
        // The next enemy mon enters fresh; the enemy side's screens persist.
        me.flinch = false;
        me.counter_dmg = 0;
        me.infatuated = false;
    }
    out.won = true;
    out.hp_frac = me.hp.max(0) as f64 / me.max_hp as f64;
    out.end_status = me.status;
    out
}

/// Heal item used mid-fight: Potion (20) to level 14, Super Potion (50) to 29, Hyper
/// Potion (200) after; a Full Restore (999) once the Indigo Plateau mart is reached.
pub fn heal_amount(level: u32, full: bool) -> i32 {
    if full {
        999
    } else if level >= 30 {
        200
    } else if level >= 15 {
        50
    } else {
        20
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rng(k: u32) -> GameRng {
        GameRng::from_u32(k.wrapping_mul(2654435761).wrapping_add(1))
    }

    #[test]
    fn damage_follows_the_decomp_pipeline() {
        // Hand case: L50, power 95, A=100, D=100, no stab, neutral type, no crit:
        // 100*95*22/100/50 = 418 -> +2 = 420... integer: (100*95=9500; *22=209000;
        // /100=2090; /50=41) + 2 = 43.
        let dex = Dex::load();
        let mut a = make_mon(&dex, dex.id("SPECIES_RATTATA"), 50, [0; 6], [0; 6], 0, 0);
        let mut d = a.clone();
        a.stat = [100; 5];
        d.stat = [100; 5];
        a.types = (9, 9); // no STAB
        d.types = (0, 0);
        let tackle = dex.move_id("MOVE_POUND"); // 40 power
        let side = Side::default();
        let dmg = calc_damage(&dex, &a, &d, tackle, 95, false, &side, None);
        assert_eq!(dmg, 43);
    }

    #[test]
    fn stages_and_badges_change_damage() {
        let dex = Dex::load();
        let sq = dex.id("SPECIES_SQUIRTLE");
        let mut a = make_mon(&dex, sq, 20, [15; 6], [0; 6], 0, 0);
        let d = make_mon(&dex, dex.id("SPECIES_RATTATA"), 20, [15; 6], [0; 6], 0, 0);
        let mv = dex.move_id("MOVE_MEGA_PUNCH");
        let side = Side::default();
        let base = calc_damage(&dex, &a, &d, mv, 0, false, &side, None);
        a.stages[0] = 2; // +2 attack = x2
        let boosted = calc_damage(&dex, &a, &d, mv, 0, false, &side, None);
        assert!(boosted > base * 3 / 2, "{base} -> {boosted}");
    }

    #[test]
    fn lab_rival_is_a_close_fight_the_carry_usually_wins() {
        let dex = Dex::load();
        let sq = dex.id("SPECIES_SQUIRTLE");
        let tackle = dex.move_id("MOVE_TACKLE");
        let f = dex.fight("TRAINER_RIVAL_OAKS_LAB_BULBASAUR");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let (mut won, mut turns) = (0u32, 0u32);
        for i in 0..400 {
            let mut me = make_mon(&dex, sq, 5, [20; 6], [0; 6], 0, 0);
            me.moves = vec![tackle];
            let mut g = rng(i);
            let o = fight(
                &dex,
                &mut me,
                &Tactics::default(),
                &foes,
                f.ai,
                0,
                false,
                &mut g,
            );
            won += o.won as u32;
            turns += o.turns;
        }
        // Measured on libmgba with Tackle mashed: 5 turns (`human-run/calibration.md`).
        assert!(won > 300, "won {won}/400");
        let mean = turns as f64 / 400.0;
        assert!((3.5..9.0).contains(&mean), "mean turns {mean}");
    }

    #[test]
    fn dig_beats_surge_with_x_items() {
        // The WR route's answer to Raichu: Dig plus X items on a Wartortle.
        let dex = Dex::load();
        let war = dex.id("SPECIES_WARTORTLE");
        let f = dex.fight("TRAINER_LEADER_LT_SURGE");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let mut won = 0;
        for i in 0..400 {
            let mut me = make_mon(&dex, war, 26, [20; 6], [0; 6], 0, 1 | 4);
            me.moves = vec![
                dex.move_id("MOVE_DIG"),
                dex.move_id("MOVE_WATER_GUN"),
                dex.move_id("MOVE_BITE"),
            ];
            let tac = Tactics {
                x_items: vec![XItem::Attack, XItem::Attack],
                heal_below: 0.35,
                heal: 50,
                full_restore: false,
            };
            let mut g = rng(i + 7000);
            let o = fight(&dex, &mut me, &tac, &foes, f.ai, f.heals, false, &mut g);
            won += o.won as u32;
        }
        // The lone no-TM carry won ~5% of these in the old engine; Dig + two X Attacks
        // has to make the fight roughly a coin flip or better at level 26 (higher level
        // or a third X item pushes it well past that — the optimiser's call).
        assert!(won > 180, "won {won}/400");
    }
}
