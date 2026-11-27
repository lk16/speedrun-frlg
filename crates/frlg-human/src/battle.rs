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
//!   (`ShouldUseItem`, `battle_ai_switch_items.c`); `if_can_faint` counts every move of power 2+ (a finishing
//!   Hyper Beam earns TRY_TO_FAINT's +4, `Cmd_if_can_faint`); a trainer sends out its next mon by
//!   `GetMostSuitableMonToSwitchInto` (`most_suitable`, which reproduces the orders of the WR video) and may
//!   switch mid-fight by `ShouldSwitch` (`ai_should_switch`);
//! * X items raise one stage, Dire Hit is Focus Energy
//!   (`src/data/pokemon/item_effects.h:235-260`).
//!
//! Approximations, all listed: no PP (enemies rarely run out before the fight decides; the
//! player's healing policy keeps fights short), no switching by Natural Cure / Perish Song / Wonder Guard,
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

/// Weather numbers (`gBattleWeather`, `include/constants/battle.h` `B_WEATHER_*`).
pub const W_RAIN: u8 = 1;
pub const W_SUN: u8 = 2;
pub const W_SAND: u8 = 3;
pub const W_HAIL: u8 = 4;
/// `turns` value of a permanent weather (Sand Stream: `B_WEATHER_SANDSTORM_PERMANENT`).
const W_FOREVER: u8 = 255;

/// Moves whose base PP is at most this are "scarce" slots the human rations.
const SCARCE_PP: u32 = 5;
/// A scarce move's value is scaled by this unless it kills.
const SCARCE_FACTOR: f64 = 0.6;

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
    /// Rain Dance / Sunny Day / Sandstorm / Hail (weather number, `W_*`).
    Weather(u8),
    /// Overheat: the user's Special Attack falls by 2 after a hit (`EFFECT_OVERHEAT`).
    Overheat,
    /// Future Sight: damage lands two turns after the end of the turn it is used.
    FutureSight,
    /// Roar / Whirlwind: forces the target's switch in trainer battles.
    Roar,
    /// Foresight / Odor Sleuth: only its AI score matters here.
    Foresight,
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
        "EFFECT_OVERHEAT" => Overheat,
        "EFFECT_RAIN_DANCE" => Weather(W_RAIN),
        "EFFECT_SUNNY_DAY" => Weather(W_SUN),
        "EFFECT_SANDSTORM" => Weather(W_SAND),
        "EFFECT_HAIL" => Weather(W_HAIL),
        "EFFECT_FUTURE_SIGHT" => FutureSight,
        "EFFECT_ROAR" => Roar,
        "EFFECT_FORESIGHT" => Foresight,
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
    /// PP left per move slot (parallel to `moves`); empty = unlimited (callers that do
    /// not track PP). `set_pp` fills it from the move data.
    pub pp: Vec<i32>,
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
    /// The move a Thrash/Outrage lock repeats (`gLockedMoves`, `battle_script_commands.c:2727`).
    pub lock_move: Option<usize>,
    /// The move this mon used last (`gLastMoves`; the AI's Counter / Defense Up scripts read it).
    pub last_move: Option<usize>,
    /// Holds a Sitrus Berry (`HOLD_EFFECT_RESTORE_HP`, 30 HP at <= 1/2, `battle_util.c:2562`).
    pub sitrus: bool,
    /// Holds a Persim Berry (`HOLD_EFFECT_CURE_CONFUSION`, `battle_util.c:2766,2913`): the first confusion
    /// is cured right after the move that caused it, before the holder acts again.
    pub persim: bool,
    /// Holds Leftovers (`HOLD_EFFECT_LEFTOVERS`, `battle_util.c:2626-2639`): max HP / 16 (at least 1) at the
    /// end of every turn while below max HP.
    pub leftovers: bool,
    /// Held type booster (`sHoldEffectToType`, `decompiled/src/pokemon.c:1461-1480`): moves of this type
    /// use `stat * (100 + pct) / 100` (Black Glasses = Dark, +10: `items.json` `holdEffectParam`).
    pub boost: Option<(u8, i32)>,
    /// Player badge boosts ×100: Atk, Def, Spe, SpA, SpD (enemy: all 100).
    pub badge: [i32; 5],
    /// Physical damage taken this turn (Counter).
    pub counter_dmg: i32,
    pub yawned: bool,
    /// The starter ability's 1/3-HP boost applies whatever the HP (`FightTactic::free_torrent`): the WR carry
    /// walks into most gym and League fights at or under a third of its HP (`v2-mid-fights.md`), which the
    /// timing lane's own HP does not reproduce.
    pub force_torrent: bool,
    /// Party slot of a trainer's mon (`fight` numbers them): a mon the AI switches out goes back in its place.
    pub slot: usize,
    /// The last move that landed on this mon (`gLastLandedMoves`, `MOVEEND_UPDATE_LAST_MOVES`): the AI's
    /// switching reads it; 0 (None) after a miss, an immunity, or on switch-in.
    pub last_landed: Option<usize>,
}

/// Side state that outlives one enemy mon (screens guard the trainer's whole party).
#[derive(Default, Clone, Copy)]
struct Side {
    reflect: u8,
    light_screen: u8,
    mist: u8,
    safeguard: u8,
    /// Field weather, mirrored on both sides so `calc_damage` and the AI can read it from
    /// either (`gBattleWeather`; the 5-turn counter is `gWishFutureKnock.weatherDuration`).
    weather: u8,
    weather_turns: u8,
    /// A Future Sight aimed at this side's mon: turns left and the damage fixed at cast
    /// (`Cmd_trysetfutureattack`, `battle_script_commands.c:8544`).
    future: u8,
    future_dmg: i32,
}

/// Start `w` on both sides for `turns` turns (the turn it is set counts: the counter is
/// decremented at that turn's end, `battle_util.c:654-710`).
fn set_weather(a: &mut Side, b: &mut Side, w: u8, turns: u8) {
    for s in [a, b] {
        s.weather = w;
        s.weather_turns = turns;
    }
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
        pp: vec![],
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
        lock_move: None,
        last_move: None,
        sitrus: false,
        persim: false,
        leftovers: false,
        boost: None,
        badge: [bb(0), bb(4), bb(2), bb(6), bb(6)],
        counter_dmg: 0,
        yawned: false,
        force_torrent: false,
        slot: 0,
        last_landed: None,
    }
}

/// Mon from a trainer's party entry: the party IV applies to every stat, no EVs, neutral
/// nature (`decompiled/src/battle_main.c`, `CreateNPCTrainerParty`: a fixed personality
/// per party — the model uses neutral, an approximation worth one stat point either way).
/// Confuse `m` for `turns` turns, unless it holds a Persim Berry: the berry cures the confusion right
/// after the move that caused it (`HOLD_EFFECT_CURE_CONFUSION`, `decompiled/src/battle_util.c:2913`
/// `ITEMEFFECT_MOVE_END`; `:2766` at the end of the turn for the rampage case) and is used up.
fn confuse(m: &mut Mon, turns: u8) {
    if m.persim {
        m.persim = false;
        m.confusion = 0;
    } else {
        m.confusion = turns;
    }
}

pub fn enemy_mon(dex: &Dex, m: &PartyMon) -> Mon {
    let mut e = make_mon(dex, m.species, m.level, [m.iv; 6], [0; 6], m.nature, 0);
    e.sitrus = m.sitrus;
    e.moves = m.moves.clone();
    e.set_pp(dex);
    e
}

impl Mon {
    /// Full PP from the move data (`battle_moves.h` `.pp`).
    pub fn set_pp(&mut self, dex: &Dex) {
        self.pp = self.moves.iter().map(|&m| dex.moves[m].pp as i32).collect();
    }

    fn has_pp(&self, mv: usize) -> bool {
        self.pp_left(mv) > 0
    }

    fn pp_left(&self, mv: usize) -> i32 {
        if self.pp.is_empty() {
            return 1;
        }
        match self.moves.iter().position(|&m| m == mv) {
            Some(i) => self.pp.get(i).copied().unwrap_or(1),
            None => 1,
        }
    }

    fn spend_pp(&mut self, mv: usize) {
        if let Some(i) = self.moves.iter().position(|&m| m == mv) {
            if let Some(p) = self.pp.get_mut(i) {
                *p = (*p - 1).max(0);
            }
        }
    }
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

fn legacy_types() -> bool {
    static L: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *L.get_or_init(|| std::env::var_os("FRLG_LEGACY_TYPE").is_some())
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
        Ability::Swarm => m.mtype == 6,
        _ => false,
    };
    if boosted && (att.hp * 3 <= att.max_hp || att.force_torrent) {
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
    // Huge Power / Pure Power double the raw Attack before the badge (`CalculateBaseDamage`).
    let a_raw = if phys && att.ability == Ability::HugePower {
        a_raw * 2
    } else {
        a_raw
    };
    let mut a = a_raw * a_badge / 100;
    let mut d = d_raw * d_badge / 100;
    // Held type booster, after the badge and before the ability halvings (`CalculateBaseDamage`).
    if let Some((t, pct)) = att.boost {
        if t == m.mtype {
            a = a * (100 + pct) / 100;
        }
    }
    // Thick Fat halves Fire/Ice special attack (`CalculateBaseDamage`).
    if def.ability == Ability::ThickFat && (m.mtype == 10 || m.mtype == 15) {
        a /= 2;
    }
    // Hustle and Guts: x1.5 Attack (a status of any kind for Guts); Marvel Scale: x1.5 Defense while statused
    // (`CalculateBaseDamage`, in this order after Thick Fat).
    if phys && att.ability == Ability::Hustle {
        a = a * 150 / 100;
    }
    if phys && att.ability == Ability::Guts && att.status.any() {
        a = a * 150 / 100;
    }
    if phys && def.ability == Ability::MarvelScale && def.status.any() {
        d = d * 150 / 100;
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
    // Weather acts on special-type moves only, after the screens and before the final +2
    // (`CalculateBaseDamage`, `decompiled/src/pokemon.c:2605-2640`).
    if !phys && def_side.weather != 0 {
        let w = def_side.weather;
        let t = m.mtype;
        if w == W_RAIN {
            if t == 10 {
                dmg /= 2;
            } else if t == 11 {
                dmg = dmg * 15 / 10;
            }
        }
        if matches!(w, W_RAIN | W_SAND | W_HAIL) && m.name == "MOVE_SOLAR_BEAM" {
            dmg /= 2;
        }
        if w == W_SUN {
            if t == 10 {
                dmg = dmg * 15 / 10;
            } else if t == 11 {
                dmg /= 2;
            }
        }
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
    dmg = if legacy_types() {
        (dmg as f64 * eff) as i32
    } else {
        dex.modulate(dmg, m.mtype, def.types)
    };
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

/// The damage a move does without a crit: `(min roll, max roll)` (`ApplyRandomDmgMultiplier` is 85..=100%),
/// for the probes and `frlg-human dmg`.
pub fn damage_range(dex: &Dex, att: &Mon, def: &Mon, mv: usize) -> (i32, i32) {
    let hi = calc_damage(dex, att, def, mv, 0, false, &Side::default(), None);
    let lo = if hi == 0 { 0 } else { (hi * 85 / 100).max(1) };
    (lo, hi)
}

/// `Some(heals)` when the defender's ability swallows a move of this type.
fn absorbs(ab: Ability, mtype: u8) -> Option<bool> {
    match (ab, mtype) {
        (Ability::WaterAbsorb, 11) | (Ability::VoltAbsorb, 13) => Some(true),
        (Ability::FlashFire, 10) => Some(false),
        _ => None,
    }
}

/// `CalculateBaseDamage` alone: no STAB, no type chart, no roll, no crit (what Future
/// Sight stores at cast, `Cmd_trysetfutureattack`).
fn base_damage(dex: &Dex, att: &Mon, def: &Mon, mv: usize, def_side: &Side) -> i32 {
    let mut a = att.clone();
    a.types = (9, 9);
    let mut d = def.clone();
    d.types = (9, 9);
    calc_damage(dex, &a, &d, mv, 0, false, def_side, None)
}

/// Type effectiveness with Levitate (`Cmd_typecalc`).
fn effectiveness(dex: &Dex, mtype: u8, def: &Mon) -> f64 {
    if def.ability == Ability::Levitate && mtype == 4 {
        return 0.0;
    }
    dex.eff(mtype, def.types)
}

/// `Cmd_accuracycheck`: stage-adjusted accuracy, `Random() % 100 + 1 > calc` misses.
fn accuracy_check(att: &Mon, def: &Mon, move_acc: u32, mtype: u8, rng: &mut GameRng) -> bool {
    if move_acc == 0 {
        return true; // Swift, Vital Throw, Feint Attack class.
    }
    let buff = (att.stages[5] - def.stages[6]).clamp(-6, 6);
    let (n, d) = ACC_RATIO[(buff + 6) as usize];
    let mut calc = move_acc * n / d;
    if att.ability == Ability::CompoundEyes {
        calc = calc * 130 / 100;
    }
    // Hustle costs a fifth of the accuracy of physical moves (`Cmd_accuracycheck`).
    if att.ability == Ability::Hustle && is_physical(mtype) {
        calc = calc * 80 / 100;
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
#[derive(Clone)]
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
    /// With `false` (honest sacrifice, `Params::sac_honest`) the sacrifice turn is only the hit absorbed: the
    /// switch to the helper is the player's action, so the fighter neither attacks nor uses an X item that
    /// turn (items and moves act on the active mon; after the helper faints the fighter's send-out is free
    /// but takes no action). `true` is the older model (fighter acts from behind the sacrifice).
    pub sac_free: bool,
    /// HP of the typical helper (`Params::helper_hp`). A sacrifice whose foe `danger` stays below it leaves the
    /// helper alive: the foe's move did not faint it, so the fighter has to be switched back in on the next
    /// turn, and the foe's move lands on the fighter then (the helper absorbed nothing that mattered).
    /// 0 = every helper faints (older plans).
    pub helper_hp: i32,
    /// Torrent tactic (the WR runner keeps the starter at <= 1/3 HP): Withdraw when the foe's max
    /// roll would KO and one Defense stage saves the carry, and a stall turn (Tackle) while above
    /// 1/3 HP when one hit is survivable and lands the carry in the boost.
    pub torrent: bool,
    /// With `torrent`: allow the stall turns (Withdraw is always allowed).
    pub torrent_stall: bool,
    /// With `torrent`: Withdraw also against a foe that moves first (its hit this turn lands before the
    /// Defense stage does) when the stage buys the carry another survivable hit of the foe's best move
    /// and the foe would otherwise need at most two of them (Onix's Rock Tomb at 10 HP; not the Geodude).
    pub torrent_early: bool,
    /// Living teammates besides the fighter (they take Roar/Whirlwind's drag-in).
    pub others_alive: u32,
    /// Heal items (Full Restore / potion uses) the fighter can still spend in this fight;
    /// `u32::MAX` = unlimited (outside the League the model does not ration them).
    pub heal_stock: u32,
    /// Lookahead rollouts per candidate action when a fight is contested (0 = greedy human);
    /// see [`fight`].
    pub rollouts: u32,
    /// An X item is used only against a foe the fighter's best attack needs this many hits to
    /// drop (1 = always set up; see `turns_to_kill`).
    pub setup_min: u32,
    /// Probability that one chosen move is a menu slip: another usable move of the set is used
    /// instead (`Params::menu_slip`).
    pub menu_slip: f64,
    /// Value factor of a scarce-PP move (Blizzard, Fire Blast) unless it kills (`Params::scarce_factor`;
    /// 1.0 = no rationing).
    pub scarce: f64,
    /// MODEL CHANGE (`Params::sleep_no_potion`, battle bridge 2026-11-22): no potion while the fighter
    /// is asleep. Using an item does not run the sleeper's move attempt, so the sleep counter does not
    /// tick (`decompiled/src/battle_util.c` `AtkCanceller_UnableToUseMove`, `CANCELLER_ASLEEP` decrements
    /// only on a move attempt): the potion buys one foe hit and leaves the fighter just as asleep (s13 on
    /// the S.S. Anne rival state healed twice asleep and fainted). Unless the potion outheals the foe's best
    /// hit (a Super Potion against Razor Leaf: alternate turns still gain HP). A Full Restore still goes.
    pub sleep_no_potion: bool,
    /// Awakenings in the bag (MODEL CHANGE `Params::awakenings`, bought at the Cerulean mart,
    /// `CeruleanCity_Mart/scripts.inc:30`): a sleeping fighter is woken with one at once (the human
    /// cannot see the sleep counter; the expected sleep is 2.5 lost turns for one item turn).
    pub awakenings: u32,
    /// Poison cures in the bag (PLAN `Params::full_heals`, model honesty 4: Full Heal, `FuchsiaCity_Mart/
    /// scripts.inc:28`, cures every status, `item_effects.h`): a poisoned fighter that is not about to
    /// finish the foe and survives its next hit spends an item turn on one (Koga's Muk / Koffing Toxic).
    pub cures: u32,
    /// MODEL CHANGE `Params::league_bag_honest`: the heal items are real items. `heal_stock` counts every HP
    /// item; `fr_stock` of them are Full Restores and `max_stock` Max Potions, the rest heal `heal` (Hyper
    /// Potions, 200); `full_heals` cure every status. Off = the old rule (with `full_restore` every heal is a
    /// Full Restore).
    pub bag_honest: bool,
    pub fr_stock: u32,
    pub max_stock: u32,
    pub full_heals: u32,
}

impl Default for Tactics {
    fn default() -> Self {
        Tactics {
            x_items: vec![],
            heal_below: 0.4,
            heal: 50,
            full_restore: false,
            sacs: 0,
            sac_free: true,
            helper_hp: 0,
            torrent: false,
            torrent_stall: true,
            torrent_early: false,
            others_alive: 0,
            heal_stock: u32::MAX,
            rollouts: 0,
            setup_min: 1,
            menu_slip: 0.0,
            scarce: SCARCE_FACTOR,
            sleep_no_potion: false,
            awakenings: 0,
            cures: 0,
            bag_honest: false,
            fr_stock: u32::MAX,
            max_stock: 0,
            full_heals: 0,
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
    /// X Accuracy: +1 accuracy stage (`item_effects.h`).
    Accuracy,
    /// Guard Spec: Mist for five turns on the player's side (`ITEM_GUARD_SPEC`, `item_effects.h`).
    GuardSpec,
    /// Not an item: the fighter's own Calm Mind (TM04, Sabrina's gift, `SaffronCity_Gym/scripts.inc:22-24`;
    /// +1 Sp. Atk and +1 Sp. Def, `EFFECT_CALM_MIND`). A normal move turn (priority 0, one PP), no bag
    /// trip and no stock; the sim only queues it for a fighter that knows the move.
    CalmMind,
    /// Any other self-targeting set-up move the fighter knows (`move:MOVE_BULK_UP`, `move:MOVE_SWORDS_DANCE`,
    /// `move:MOVE_LIGHT_SCREEN`, ...): a normal move turn like `CalmMind`, queued only when known.
    Move(usize),
}

impl XItem {
    /// The move behind a set-up queue entry (not an item), if it is one.
    pub fn setup_move(self, dex: &Dex) -> Option<usize> {
        match self {
            XItem::CalmMind => Some(dex.move_id("MOVE_CALM_MIND")),
            XItem::Move(m) => Some(m),
            _ => None,
        }
    }
}

/// What happened in a fight.
#[derive(Default, Debug)]
pub struct Outcome {
    pub won: bool,
    /// HP the fighter regained from items this fight (running total).
    pub hp_up: i32,
    /// The fighter's own HP bar movement, in fractions of its max HP: damage taken plus HP healed.
    /// The bar moves 1/384 of its length per frame, one `MoveBattleBar` call per frame
    /// (`decompiled/src/battle_interface.c:1844-1845,1955-2012` `CalcNewBarValue`, 48 px x 8;
    /// `battle_controller_player.c:999-1013`), so a fraction costs 384 frames of waiting.
    pub hp_bar: f64,
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
    /// `Tactics::bag_honest`: Max Potions among `heals_used`, and Full Heals spent.
    pub maxes_used: u32,
    pub full_heals_used: u32,
    pub awakenings_used: u32,
    pub cures_used: u32,
    /// Sacrificial faints spent (each costs a faint animation and a send-out).
    pub sacs_used: u32,
    /// Sacrifices whose helper survived the hit (see `Tactics::helper_hp`): two switch turns, no faint.
    pub sacs_survived: u32,
    /// Healthy party mons besides the fighter (Roar/Whirlwind needs one to drag in;
    /// `AI_CBM_Roar` counts them, `data/battle_ai_scripts.s`).
    pub others_alive: u32,
    /// A Roar/Whirlwind dragged the fighter out this turn.
    pub forced_out: bool,
    /// Times the fighter was dragged out.
    pub dragged: u32,
}

/// One player turn as the policy chooses it (`decide` hands it to the playing controller).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Move(usize),
    /// Switch the fighter back in after a drag-out: no attack, the foe's move lands on it.
    Switch,
    XItem(XItem),
    Heal,
    FullRestore,
    /// Awakening on the sleeping fighter (`Tactics::awakenings`; `ITEM_AWAKENING` cures sleep,
    /// `item_effects.h`): an item turn, the fighter acts again from the next turn.
    Awaken,
    /// Full Heal on the poisoned fighter (`Tactics::cures`): an item turn, the status is gone.
    Cure,
    /// A teammate eats this turn's hit while the fighter acts freely: uses its next X
    /// item, or simply attacks when no setup is pending.
    Sac(Option<XItem>),
}

/// Expected damage for the player's (full-knowledge) choice: no crit, mean roll, scaled
/// by accuracy.
fn player_score(dex: &Dex, att: &Mon, def: &Mon, mv: usize, side: &Side) -> f64 {
    let m = &dex.moves[mv];
    let fx = effect_from(&m.effect);
    if m.power > 0 && absorbs(def.ability, m.mtype).is_some() {
        return 0.0;
    }
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

/// Torrent tactic. `stall = false`: Withdraw when the foe's max-roll hit would KO, the carry moves
/// first and one Defense stage leaves it alive (Rock Tomb 8-10 -> 5-7 in the WR's Brock fight,
/// `STRATEGY.md` 4). `stall = true`: while above 1/3 HP, no kill on the table and one max-roll hit
/// survivable, spend the turn on the best non-Water attack (Tackle) so the foe's hit puts the carry
/// inside the Torrent boost for the Water move after it.
#[allow(clippy::too_many_arguments)]
fn torrent_action(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    my_side: &Side,
    danger: i32,
    sure_kill: bool,
    stall: bool,
    early: bool,
) -> Option<Action> {
    if me.ability != Ability::Torrent || sure_kill {
        return None;
    }
    let usable = |mv: usize| {
        me.has_pp(mv) && me.disabled.is_none_or(|(d, _)| d != mv) && dex.moves[mv].mtype != 11
    };
    if !stall {
        let slower = speed(me) <= speed(e);
        if me.stages[1] >= 6 || (slower && !(early && me.stages[1] == 0)) {
            return None;
        }
        if !slower && danger < me.hp {
            return None;
        }
        let mut m2 = me.clone();
        m2.stages[1] += 1;
        let d2 = e
            .moves
            .iter()
            .map(|&x| calc_damage(dex, e, &m2, x, 0, false, my_side, None))
            .max()
            .unwrap_or(0);
        if d2 >= me.hp {
            return None;
        }
        if slower {
            // The foe moves first: worth a turn only when it buys another hit and the foe is close.
            let hits = |d: i32| if d <= 0 { 99 } else { (me.hp + d - 1) / d };
            let (n0, n1) = (hits(danger), hits(d2));
            if n1 <= n0 || n0 > 2 {
                return None;
            }
        }
        return me
            .moves
            .iter()
            .copied()
            .find(|&mv| {
                me.has_pp(mv)
                    && me.disabled.is_none_or(|(d, _)| d != mv)
                    && effect_from(&dex.moves[mv].effect) == Effect::StatChange(1, 1, true)
            })
            .map(Action::Move);
    }
    let on = me.hp * 3 <= me.max_hp;
    let after_min = me.hp - danger * 85 / 100;
    if on || danger == 0 || danger >= me.hp || after_min * 3 > me.max_hp {
        return None;
    }
    let mut best: Option<(usize, f64)> = None;
    for &mv in &me.moves {
        if !usable(mv) {
            continue;
        }
        let s = player_score(dex, me, e, mv, &Side::default());
        if s > 0.0 && best.is_none_or(|(_, bs)| s > bs) {
            best = Some((mv, s));
        }
    }
    best.map(|(mv, _)| Action::Move(mv))
}

/// The best hit any of `foes` can land on `me` (max roll, no crit), counting one Withdraw-type
/// Defense stage when the carry knows such a move (Rock Tomb 8-10 -> 5-7 on the WR's Brock fight).
pub fn entry_floor(dex: &Dex, me: &Mon, foes: &[Mon]) -> i32 {
    let mut m = me.clone();
    if me.moves.iter().any(|&mv| {
        effect_from(&dex.moves[mv].effect) == Effect::StatChange(1, 1, true)
            && dex.moves[mv].mtype != 11
    }) {
        m.stages[1] = (m.stages[1] + 1).min(6);
    }
    let side = Side::default();
    foes.iter()
        .flat_map(|f| {
            f.moves
                .iter()
                .map(|&x| calc_damage(dex, f, &m, x, 0, false, &side, None))
        })
        .max()
        .unwrap_or(0)
}

/// The AI's damage estimate for one move: real calc, no crit, its own rolled variance
/// (`battle_ai_script_commands.c:980-1026`; `simulatedRNG` at `:310`). Returns the estimate and whether the
/// move is on `sDiscouragedPowerfulMoveEffects` (`battle_ai_script_commands.c:245`: two-turn, recharge,
/// self-KO, Overheat, Dream Eater): such a move is never "the most powerful" (`get_how_powerful_move_is`),
/// but `if_can_faint` counts its damage like any other move's (`Cmd_if_can_faint`, `:1475`: only
/// `power < 2` is excluded), so a Hyper Beam or Solar Beam that would finish the target earns TRY_TO_FAINT's
/// +4. Moves of power 0 or 1 (Dragon Rage, Sonic Boom, status moves) estimate 0 either way.
fn ai_damage(
    dex: &Dex,
    att: &Mon,
    def: &Mon,
    mv: usize,
    side: &Side,
    rng: &mut GameRng,
) -> (i32, bool) {
    let m = &dex.moves[mv];
    if m.power <= 1 {
        return (0, false);
    }
    let fx = effect_from(&m.effect);
    let discouraged = matches!(
        fx,
        Effect::Explosion
            | Effect::Charge
            | Effect::Recharge
            | Effect::DreamEater
            | Effect::Overheat
    );
    let sim = 100 - rng.modn(16) as i32;
    let d = calc_damage(dex, att, def, mv, 0, false, side, None);
    let eff = effectiveness(dex, m.mtype, def);
    let d = (d as f64 * if eff > 0.0 { 1.0 } else { 0.0 }) as i32;
    (
        (d * sim / 100).max(if eff > 0.0 { 1 } else { 0 }),
        discouraged,
    )
}

/// Everything the viability scripts read about the two mons.
struct CvCtx<'a> {
    user: &'a Mon,
    target: &'a Mon,
    user_side: &'a Side,
    user_hp: i32,
    target_hp: i32,
    /// `if_target_faster`: the target would move before the user.
    target_faster: bool,
    /// Effectiveness of the considered move on the target.
    eff: f64,
    /// The target's last used move (`get_last_used_move AI_TARGET`).
    target_last: Option<usize>,
}

/// `AI_CheckViability` (`data/battle_ai_scripts.s:653-2300`): the score change for one move,
/// ported script by script. `if_random_less_than n` is `Random() % 256 < n`; stat levels
/// are the game's 0..12 with 6 neutral (our stages + 6). Effects without a script change
/// nothing.
fn ai_cv(dex: &Dex, c: &CvCtx, fx: Effect, mv: usize, rng: &mut GameRng) -> i32 {
    let rl = |rng: &mut GameRng, n: u32| rng.modn(256) < n;
    let (user, target) = (c.user, c.target);
    let lvl = |m: &Mon, s: usize| m.stages[s] as i32 + 6;
    let (uh, th) = (c.user_hp, c.target_hp);
    let mut sc = 0i32;
    // The "last move" scripts: power and type of the target's last used move.
    let last = c
        .target_last
        .map(|m| (dex.moves[m].power, dex.moves[m].mtype));
    match fx {
        // AI_CV_AttackUp / SpAtkUp.
        Effect::StatChange(s @ (0 | 3), d, true) if d > 0 => {
            if lvl(user, s as usize) < 9 {
                if uh == 100 && !rl(rng, 128) {
                    sc += 2;
                }
            } else if !rl(rng, 100) {
                sc -= 1;
            }
            // AU3: AttackUp discourages with rl(40), SpAtkUp with rl(70).
            if uh <= 70 && (uh < 40 || !rl(rng, if s == 0 { 40 } else { 70 })) {
                sc -= 2;
            }
        }
        // AI_CV_DefenseUp / SpDefUp (Bulk Up, Calm Mind and Cosmic Power share them).
        Effect::StatChange(s @ (1 | 4), d, true) if d > 0 => {
            sc += cv_def_up(c, s as usize, last, rng)
        }
        Effect::TwoStat(0, 1, d) if d > 0 => sc += cv_def_up(c, 1, last, rng),
        Effect::TwoStat(3, 4, d) | Effect::TwoStat(1, 4, d) if d > 0 => {
            sc += cv_def_up(c, 4, last, rng)
        }
        // AI_CV_DragonDance.
        Effect::TwoStat(0, 2, d) if d > 0 => {
            if c.target_faster {
                if !rl(rng, 128) {
                    sc += 1;
                }
            } else if uh <= 50 && !rl(rng, 70) {
                sc -= 1;
            }
        }
        // AI_CV_SpeedUp.
        Effect::StatChange(2, d, true) if d > 0 => {
            if c.target_faster {
                if !rl(rng, 70) {
                    sc += 3;
                }
            } else {
                sc -= 3;
            }
        }
        // AI_CV_AccuracyUp.
        Effect::StatChange(5, d, true) if d > 0 => {
            if lvl(user, 5) >= 9 && !rl(rng, 50) {
                sc -= 2;
            }
            if uh <= 70 {
                sc -= 2;
            }
        }
        // AI_CV_EvasionUp (Double Team, Minimize).
        Effect::StatChange(6, d, true) if d > 0 => {
            if uh >= 90 && !rl(rng, 100) {
                sc += 3;
            }
            if lvl(user, 6) >= 9 && !rl(rng, 128) {
                sc -= 1;
            }
            if matches!(target.status, Status1::Toxic(_)) {
                let skip = if uh > 50 { false } else { rl(rng, 80) };
                if !skip && !rl(rng, 50) {
                    sc += 3;
                }
            }
            if target.seeded && !rl(rng, 70) {
                sc += 3;
            }
            if uh <= 70 && lvl(user, 6) != 6 && (uh < 40 || th < 40 || !rl(rng, 70)) {
                sc -= 2;
            }
        }
        // AI_CV_AttackDown.
        Effect::StatChange(0, d, false) if d < 0 => {
            if lvl(target, 0) != 6 {
                sc -= 1;
                if uh <= 90 {
                    sc -= 1;
                }
                if lvl(target, 0) <= 3 && !rl(rng, 50) {
                    sc -= 2;
                }
            }
            if th <= 70 {
                sc -= 2;
            }
            let phys = |t: u8| matches!(t, 0 | 1 | 4 | 5 | 6 | 8);
            if !(phys(target.types.0) || phys(target.types.1)) && !rl(rng, 50) {
                sc -= 2;
            }
        }
        // AI_CV_SpAtkDown (the neutral test reads the target's *Attack*, as the script does).
        Effect::StatChange(3, d, false) if d < 0 => {
            if lvl(target, 0) != 6 {
                sc -= 1;
                if uh <= 90 {
                    sc -= 1;
                }
                if lvl(target, 3) <= 3 && !rl(rng, 50) {
                    sc -= 2;
                }
            }
            if th <= 70 {
                sc -= 2;
            }
            let special = |t: u8| t >= 10;
            if !(special(target.types.0) || special(target.types.1)) && !rl(rng, 50) {
                sc -= 2;
            }
        }
        // AI_CV_DefenseDown / SpDefDown / EvasionDown share one shape.
        Effect::StatChange(s @ (1 | 4 | 6), d, false) if d < 0 => {
            let go2 = uh < 70 || lvl(target, s as usize) <= 3;
            if go2 && !rl(rng, 50) {
                sc -= 2;
            }
            if th <= 70 {
                sc -= 2;
            }
        }
        // AI_CV_SpeedDown (also Icy Wind / Rock Tomb / Mud Shot via `SpeedDownFromChance`).
        Effect::StatChange(2, d, false) if d < 0 => sc += cv_speed_down(c, rng),
        Effect::StatDownHit(2)
            if matches!(
                dex.moves[mv].name.as_str(),
                "MOVE_ICY_WIND" | "MOVE_ROCK_TOMB" | "MOVE_MUD_SHOT"
            ) =>
        {
            sc += cv_speed_down(c, rng)
        }
        // AI_CV_AccuracyDown.
        Effect::StatChange(5, d, false) if d < 0 => {
            if (uh < 70 || th <= 70) && !rl(rng, 100) {
                sc -= 1;
            }
            if lvl(user, 5) <= 4 && !rl(rng, 80) {
                sc -= 2;
            }
            if matches!(target.status, Status1::Toxic(_)) && !rl(rng, 70) {
                sc += 2;
            }
            if target.seeded && !rl(rng, 70) {
                sc += 2;
            }
            if uh <= 70 && lvl(target, 5) != 6 && (uh < 40 || th < 40 || !rl(rng, 70)) {
                sc -= 2;
            }
        }
        // AI_CV_Roar: -3 unless the target is boosted to +3 or more in a stat the AI cares
        // about, then +2 half the time.
        Effect::Roar => {
            if [0usize, 1, 3, 4, 6].iter().any(|&k| target.stages[k] >= 3) {
                if !rl(rng, 128) {
                    sc += 2;
                }
            } else {
                sc -= 3;
            }
        }
        // AI_CV_Heal (Recover, Softboiled); Synthesis / Morning Sun / Moonlight first lose 2
        // in hail, rain or sandstorm (AI_CV_HealWeather).
        Effect::RestoreHp => {
            if matches!(
                dex.moves[mv].effect.as_str(),
                "EFFECT_SYNTHESIS" | "EFFECT_MORNING_SUN" | "EFFECT_MOONLIGHT"
            ) && matches!(c.user_side.weather, W_RAIN | W_SAND | W_HAIL)
            {
                sc -= 2;
            }
            if uh == 100 {
                sc -= 3;
            } else if !c.target_faster {
                sc -= 8;
            } else if uh >= 70 && !rl(rng, 30) {
                sc -= 3;
            } else if !rl(rng, 20) {
                sc += 2;
            }
        }
        // AI_CV_Rest.
        Effect::Rest => {
            if !c.target_faster {
                if uh == 100 {
                    sc -= 8;
                } else if uh >= 40 && (uh > 50 || !rl(rng, 70)) {
                    sc -= 3;
                } else if !rl(rng, 10) {
                    sc += 3;
                }
            } else if uh >= 60 && (uh > 70 || !rl(rng, 50)) {
                sc -= 3;
            } else if !rl(rng, 10) {
                sc += 3;
            }
        }
        // AI_CV_Paralyze.
        Effect::Paralyze => {
            if c.target_faster {
                if !rl(rng, 20) {
                    sc += 3;
                }
            } else if uh <= 70 {
                sc -= 1;
            }
        }
        // AI_CV_Confuse (Swagger shares it; Flatter adds a coin flip).
        Effect::Confuse | Effect::Swagger => {
            if th <= 70 {
                if !rl(rng, 128) {
                    sc -= 1;
                }
                if th <= 50 {
                    sc -= 1;
                    if th <= 30 {
                        sc -= 1;
                    }
                }
            }
        }
        // AI_CV_Toxic (Leech Seed shares it).
        Effect::Toxic | Effect::LeechSeed => {
            let has_attack = user.moves.iter().any(|&m2| dex.moves[m2].power > 1);
            if has_attack {
                if uh <= 50 && !rl(rng, 50) {
                    sc -= 3;
                }
                if th <= 50 && !rl(rng, 50) {
                    sc -= 3;
                }
            }
            let protects = user
                .moves
                .iter()
                .any(|&m2| matches!(effect_from(&dex.moves[m2].effect), Effect::Protect));
            if protects && !rl(rng, 60) {
                sc += 2;
            }
        }
        // AI_CV_Poison.
        Effect::Poison => {
            if uh < 50 || th <= 50 {
                sc -= 1;
            }
        }
        // AI_CV_Trap.
        Effect::Trap => {
            if (matches!(target.status, Status1::Toxic(_)) || target.infatuated) && !rl(rng, 128) {
                sc += 1;
            }
        }
        // AI_CV_HighCrit.
        Effect::HighCrit => {
            if c.eff >= 1.0 {
                let two = c.eff >= 2.0;
                if (two || !rl(rng, 128)) && !rl(rng, 128) {
                    sc += 1;
                }
            }
        }
        // AI_CV_Absorb: Giga Drain and friends are discouraged when resisted.
        Effect::Absorb => {
            if c.eff > 0.0 && c.eff < 1.0 && !rl(rng, 50) {
                sc -= 3;
            }
        }
        // AI_CV_SelfKO, trimmed (no evasion bookkeeping on the player).
        Effect::Explosion => {
            if uh >= 80 && !c.target_faster {
                if rl(rng, 50) {
                    // no change
                } else {
                    sc -= 3;
                }
            } else if uh <= 50 {
                if !rl(rng, 128) {
                    sc += 1;
                }
                if uh <= 30 && !rl(rng, 50) {
                    sc += 1;
                }
            } else if uh > 50 && !rl(rng, 50) {
                sc -= 1;
            }
        }
        // AI_CV_Recharge.
        Effect::Recharge => {
            if c.eff < 1.0 || (!c.target_faster && uh > 40) || (c.target_faster && uh >= 60) {
                sc -= 1;
            }
        }
        // AI_CV_AlwaysHit.
        Effect::AlwaysHit => {
            if lvl(target, 6) > 10 || lvl(user, 5) < 2 {
                sc += 1;
                if !rl(rng, 100) {
                    sc += 1;
                }
            } else if (lvl(target, 6) > 8 || lvl(user, 5) < 4) && !rl(rng, 100) {
                sc += 1;
            }
        }
        // AI_CV_Counter, trimmed to its type/HP terms.
        Effect::Counter => {
            if matches!(target.status, Status1::Sleep(_))
                || target.confusion > 0
                || target.infatuated
            {
                sc -= 1;
            } else {
                if uh <= 30 && !rl(rng, 10) {
                    sc -= 1;
                }
                if uh <= 50 && !rl(rng, 100) {
                    sc -= 1;
                }
                match last {
                    Some((pw, ty)) if pw > 0 => {
                        if !is_physical(ty) {
                            sc -= 1;
                        } else if !rl(rng, 100) {
                            sc += 1;
                        }
                    }
                    _ => {
                        let phys = is_physical(target.types.0) || is_physical(target.types.1);
                        if !phys && !rl(rng, 50) {
                            sc -= 1;
                        }
                    }
                }
            }
        }
        // AI_CV_Protect, trimmed.
        Effect::Protect => {
            if user.protect_count >= 2 {
                sc -= 2;
            } else {
                if matches!(target.status, Status1::Toxic(_)) || target.seeded {
                    sc += 2;
                }
                if user.protect_count >= 1 {
                    sc -= 1;
                    if !rl(rng, 128) {
                        sc -= 1;
                    }
                }
            }
        }
        // AI_CV_Sleep: +1 only when the target knows Dream Eater or Nightmare (never the
        // player's moves here).
        Effect::Sleep => {}
        Effect::Reflect => {
            // AI_CV_Reflect: -2 below half HP; else only worth it against a physical-type
            // target (80% -2 against anything else).
            let phys = is_physical(target.types.0) || is_physical(target.types.1);
            if uh < 50 || (!phys && !rl(rng, 50)) {
                sc -= 2;
            }
        }
        Effect::LightScreen => {
            // AI_CV_LightScreen: the mirror image (special-type targets).
            let special = target.types.0 >= 10 || target.types.1 >= 10;
            if uh < 50 || (!special && !rl(rng, 50)) {
                sc -= 2;
            }
        }
        Effect::Charge => {
            // AI_CV_ChargeUpMove: -2 when resisted (or the target Protects), -1 at 38% HP
            // or less.
            if c.eff < 1.0 {
                sc -= 2;
            } else if uh <= 38 {
                sc -= 1;
            }
        }
        Effect::Foresight => {
            // AI_CV_Foresight: -2 for a non-Ghost user without raised evasion.
            let ghost = user.types.0 == 7 || user.types.1 == 7;
            if ghost || user.stages[6] > 2 {
                if !rl(rng, 80) {
                    sc += 2;
                }
            } else {
                sc -= 2;
            }
        }
        // AI_CV_RainDance / SunnyDay / Hail: -1 below 40% HP, else +1 when a different
        // weather is up.
        Effect::Weather(w) => {
            if w != W_SAND {
                if uh < 40 {
                    sc -= 1;
                } else if c.user_side.weather != 0 && c.user_side.weather != w {
                    sc += 1;
                }
            }
        }
        // AI_CV_Overheat.
        Effect::Overheat => {
            if c.eff < 1.0 {
                sc -= 1;
            } else if c.target_faster {
                if uh <= 80 {
                    sc -= 1;
                }
            } else if uh <= 60 {
                sc -= 1;
            }
        }
        _ => {}
    }
    sc
}

/// `AI_CV_SpeedDown`: worth +2 (72%) when the target is faster, else -3.
fn cv_speed_down(c: &CvCtx, rng: &mut GameRng) -> i32 {
    if c.target_faster {
        if rng.modn(256) >= 70 {
            2
        } else {
            0
        }
    } else {
        -3
    }
}

/// `AI_CV_DefenseUp` / `AI_CV_SpDefUp` for the stat `s` (1 Def, 4 SpD).
fn cv_def_up(c: &CvCtx, s: usize, last: Option<(u32, u8)>, rng: &mut GameRng) -> i32 {
    let rl = |rng: &mut GameRng, n: u32| rng.modn(256) < n;
    let uh = c.user_hp;
    let mut sc = 0;
    if c.user.stages[s] as i32 + 6 < 9 {
        if uh == 100 && !rl(rng, 128) {
            sc += 2;
        }
    } else if !rl(rng, 100) {
        sc -= 1;
    }
    // DU3: at 70%+ HP, 200/256 of the time that is all.
    if uh >= 70 && rl(rng, 200) {
        return sc;
    }
    // DU4: below 40% never; then it depends on what the target last did.
    if uh < 40 {
        return sc - 2;
    }
    if let Some((pw, ty)) = last {
        if pw > 0 {
            let phys = is_physical(ty);
            // Defense Up wants physical hitters, Sp. Def Up special ones.
            if (s == 1 && !phys) || (s == 4 && phys) {
                return sc - 2;
            }
            if rl(rng, 60) {
                return sc;
            }
        }
    }
    // DU5.
    if rl(rng, 60) {
        return sc;
    }
    sc - 2
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
    target_others: u32,
    rng: &mut GameRng,
) -> Option<usize> {
    if user.moves.is_empty() {
        return None;
    }
    let n = user.moves.len();
    let mut score = vec![100i32; n];
    let mut dmg = vec![0i32; n];
    let mut faint_dmg = vec![0i32; n];
    let user_hp = user.hp * 100 / user.max_hp.max(1);
    let target_hp = target.hp * 100 / target.max_hp.max(1);
    let target_faster = speed(target) > speed(user);
    for i in 0..n {
        let mv = user.moves[i];
        if !user.has_pp(mv) {
            score[i] = -30;
            continue;
        }
        if let Some((d, _)) = user.disabled {
            if d == mv {
                score[i] = -20;
                continue;
            }
        }
        let m = &dex.moves[mv];
        let fx = effect_from(&m.effect);
        let (raw, discouraged) = ai_damage(dex, user, target, mv, target_side, rng);
        faint_dmg[i] = raw;
        dmg[i] = if discouraged { 0 } else { raw };
        let eff = effectiveness(dex, m.mtype, target);

        // AI_CheckBadMove (always on: every trainer has the flag).
        if m.power > 1 && eff == 0.0 {
            score[i] -= 10;
            continue;
        }
        if m.power > 1 && absorbs(target.ability, m.mtype).is_some() {
            score[i] -= 12;
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
                // `AI_CBM_AttackDown` & co. test the target's stage (0 = -6) and its ability only: the AI
                // does not look at Mist, so a trainer keeps trying Sand Attack / Feather Dance into the
                // WR's Guard Spec ("A is protected by MIST!" three times in the Champion's first turns,
                // 2:46:23-2:46:40 of the VOD) instead of switching to its attack.
                if !on_self && target.stages[stat as usize] <= -6 {
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
            // `AI_CBM_Mist`: Score_Minus8 when its own side already has Mist up.
            Effect::Mist => {
                if user_side.mist > 0 {
                    score[i] -= 8;
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
            // AI_CBM_RainDance / SunnyDay / Hail / Sandstorm: -8 when it is already up.
            Effect::Weather(w) => {
                if user_side.weather == w {
                    score[i] -= 8;
                }
            }
            // AI_CBM_FutureSight: -10 when one is already pending on the target's side.
            Effect::FutureSight => {
                if target_side.future > 0 {
                    score[i] -= 10;
                }
            }
            // AI_CBM_Roar: -10 when the target has no other mon alive.
            Effect::Roar if target_others == 0 => {
                score[i] -= 10;
            }
            _ => {}
        }
        if score[i] < 0 {
            continue;
        }

        // AI_CheckViability (`data/battle_ai_scripts.s`).
        if ai_flags & 2 != 0 {
            let cv = CvCtx {
                user,
                target,
                user_side,
                user_hp,
                target_hp,
                target_faster,
                eff,
                target_last: target.last_move,
            };
            score[i] += ai_cv(dex, &cv, fx, mv, rng);
        }

        // AI_TryToFaint.
        // `AI_TryToFaint_TryToEncourageQuickAttack`: an Explosion that would finish the target is skipped.
        if ai_flags & 4 != 0
            && faint_dmg[i] > 0
            && faint_dmg[i] >= target.hp
            && !matches!(fx, Effect::Explosion)
        {
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

/// `ModulateByTypeEffectiveness` (`battle_ai_switch_items.c:405`): a `u8` multiplied by every matching
/// table entry in table order, `*var = *var * mult / 10`.
fn modulate_u8(dex: &Dex, atk: u8, def: (u8, u8), var: &mut u8) {
    for &(d, m) in &dex.chart_ord[atk as usize] {
        if d == def.0 {
            *var = (*var as u32 * m / 10) as u8;
        }
        if d == def.1 && def.0 != def.1 {
            *var = (*var as u32 * m / 10) as u8;
        }
    }
}

/// Does `mv` carry the super-effective flag against `def` (`TypeCalc` / `ModulateDmgByType2`,
/// `battle_script_commands.c:1421-1511`)? Moves without power never do; a 2x and a 0.5x entry cancel.
fn super_effective_flag(dex: &Dex, mv: usize, def: &Mon) -> bool {
    let m = &dex.moves[mv];
    if m.power == 0 {
        return false;
    }
    if def.ability == Ability::Levitate && m.mtype == 4 {
        return false;
    }
    let (mut se, mut nve, mut none) = (false, false, false);
    for &(d, mult) in &dex.chart_ord[m.mtype as usize] {
        let hit1 = d == def.types.0;
        let hit2 = d == def.types.1 && def.types.0 != def.types.1;
        for _ in 0..(hit1 as u32 + hit2 as u32) {
            match mult {
                0 => {
                    none = true;
                    nve = false;
                    se = false;
                }
                5 if !none => {
                    if se {
                        se = false;
                    } else {
                        nve = true;
                    }
                }
                20 if !none => {
                    if nve {
                        nve = false;
                    } else {
                        se = true;
                    }
                }
                _ => {}
            }
        }
    }
    se
}

/// `GetMostSuitableMonToSwitchInto` (`battle_ai_switch_items.c:428`), which the trainer runs whenever a mon
/// faints (`OpponentHandleChoosePokemon`, `battle_controller_opponent.c:1411`): the party order is **not** the
/// order the mons come out in. Pass one: the bench mon whose typing the player's two types (the single type
/// twice for a mono-type mon) hit hardest (`bestDmg < typeDmg`: the first of equals), taken only if it has a move
/// that is super effective on the player, else it is struck off and the next best is looked at. Pass two, when
/// no mon qualified: the highest `bestDmg` (a `u8`, so values past 255 wrap) over every bench mon's moves
/// (power other than 1) of `AI_CalcDmg(fainted mon, player)` with the **last move used in the battle**
/// (`gCurrentMove`, not the candidate's move) then `TypeCalc(candidate's move)`: the fainted mon's STAB on the
/// candidate's move type and the move's effectiveness on the player. Ties keep the lowest party slot. The WR
/// video's orders follow from this (Agatha: Gengar, Golbat, Arbok, Gengar, Haunter; Bruno 2: Steelix, Steelix,
/// Hitmonlee, Hitmonchan, Machamp). Returns an index into `bench` (the party order of what is left).
fn most_suitable(
    dex: &Dex,
    me: &Mon,
    fainted: &Mon,
    last_move: Option<usize>,
    my_side: &Side,
    bench: &[Mon],
) -> usize {
    if bench.len() <= 1 {
        return 0;
    }
    let mut invalid = vec![false; bench.len()];
    loop {
        let mut best = 0u8;
        let mut best_id: Option<usize> = None;
        for (i, c) in bench.iter().enumerate() {
            if invalid[i] {
                continue;
            }
            let mut type_dmg = 10u8;
            modulate_u8(dex, me.types.0, c.types, &mut type_dmg);
            modulate_u8(dex, me.types.1, c.types, &mut type_dmg);
            if best < type_dmg {
                best = type_dmg;
                best_id = Some(i);
            }
        }
        match best_id {
            Some(i) => {
                if bench[i]
                    .moves
                    .iter()
                    .any(|&mv| super_effective_flag(dex, mv, me))
                {
                    return i;
                }
                invalid[i] = true;
            }
            None => break,
        }
    }
    let d0 = last_move.map_or(0, |mv| base_damage(dex, fainted, me, mv, my_side));
    let mut best = 0u8;
    let mut best_id = 0usize;
    for (i, c) in bench.iter().enumerate() {
        for &mv in &c.moves {
            let m = &dex.moves[mv];
            let mut dmg = 0i32;
            if m.power != 1 {
                dmg = d0;
                if fainted.types.0 == m.mtype || fainted.types.1 == m.mtype {
                    dmg = dmg * 15 / 10;
                }
                if !(me.ability == Ability::Levitate && m.mtype == 4) {
                    dmg = dex.modulate(dmg, m.mtype, me.types);
                }
            }
            if (best as i32) < dmg {
                best = dmg as u8;
                best_id = i;
            }
        }
    }
    best_id
}

/// The trainer's turn-start switching (`ShouldSwitch`, `battle_ai_switch_items.c:296-355`, run for every trainer
/// by `AI_TrySwitchOrUseItem` before it looks at items or moves). Returns the bench slot to switch to.
/// In order: a bench mon whose ability absorbs the type of the move that last landed (Lapras and Water Absorb
/// against Surf, Arcanine and Flash Fire against Fire Blast; skipped 2 times in 3 when the active mon has a
/// super-effective move, then a coin flip per candidate); nothing at all while the active mon has a
/// super-effective move (a 10% blind spot per move) or has raised its stats by more than 3 stages; otherwise a
/// bench mon that the last landed move cannot hurt (immune, 1 in 2) or only weakly (1 in 3) and that has a
/// super-effective move on the player. Natural Cure, Perish Song and Wonder Guard (`ShouldSwitchIf*`) are not
/// on any route trainer's or the player's mon. Nothing happens while the active mon is trapped.
fn ai_should_switch(
    dex: &Dex,
    e: &Mon,
    me: &Mon,
    bench: &[Mon],
    rng: &mut GameRng,
) -> Option<usize> {
    if bench.is_empty() || e.trap > 0 {
        return None;
    }
    let has_se = |rng: &mut GameRng, no_rng: bool| -> bool {
        for &mv in &e.moves {
            if super_effective_flag(dex, mv, me) && (no_rng || rng.modn(10) != 0) {
                return true;
            }
        }
        false
    };
    // FindMonThatAbsorbsOpponentsMove.
    if let Some(lm) = e.last_landed {
        let m = &dex.moves[lm];
        let blocked = has_se(rng, true) && rng.modn(3) != 0;
        if !blocked && m.power != 0 {
            let want = match m.mtype {
                10 => Some(Ability::FlashFire),
                11 => Some(Ability::WaterAbsorb),
                13 => Some(Ability::VoltAbsorb),
                _ => None,
            };
            if let Some(w) = want {
                if e.ability != w {
                    for (i, c) in bench.iter().enumerate() {
                        if c.ability == w && rng.roll() & 1 == 1 {
                            return Some(i);
                        }
                    }
                }
            }
        }
    }
    let raised: i32 = e.stages.iter().map(|&s| (s as i32).max(0)).sum();
    if has_se(rng, false) || raised > 3 {
        return None;
    }
    // FindMonWithFlagsAndSuperEffective(DOESNT_AFFECT_FOE, 2) then (NOT_VERY_EFFECTIVE, 3).
    let lm = e.last_landed?;
    if dex.moves[lm].power == 0 {
        return None;
    }
    for (flag_none, modulo) in [(true, 2u32), (false, 3u32)] {
        for (i, c) in bench.iter().enumerate() {
            let eff = if c.ability == Ability::Levitate && dex.moves[lm].mtype == 4 {
                0.0
            } else {
                dex.eff(dex.moves[lm].mtype, c.types)
            };
            let hit = if flag_none {
                eff == 0.0
            } else {
                eff > 0.0 && eff < 1.0
            };
            if hit {
                for &mv in &c.moves {
                    if super_effective_flag(dex, mv, me) && rng.modn(modulo) == 0 {
                        return Some(i);
                    }
                }
            }
        }
    }
    None
}

/// Entry abilities (`battle_util.c` `ABILITYEFFECT_ON_SWITCHIN`): Intimidate lowers the fighter's Attack,
/// Sand Stream raises a permanent sandstorm.
fn enter_effects(e: &Mon, me: &mut Mon, fs: &mut Fs) {
    if e.ability == Ability::Intimidate && me.ability != Ability::HyperCutter {
        apply_stat_change(me, 0, -1, false);
    }
    if e.ability == Ability::SandStream && fs.my_side.weather != W_SAND {
        set_weather(&mut fs.my_side, &mut fs.en_side, W_SAND, W_FOREVER);
    }
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

    // PP: one per use, charged when the move starts (`Cmd_ppreduce`).
    if att.charging != Some(mv) {
        att.spend_pp(mv);
    }
    att.last_move = Some(mv);

    // --- two-turn moves.
    if att.charging == Some(mv) {
        att.charging = None;
        att.underground = false;
        att.airborne = false;
    } else {
        match fx {
            // Solar Beam needs no charge turn in sun (`BattleScript_EffectSolarBeam`).
            Effect::Charge if !(m.name == "MOVE_SOLAR_BEAM" && att_side.weather == W_SUN) => {
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
        Effect::Weather(w) => {
            // Fails when that weather is already up (`AI_CBM_*` mirrors it).
            if att_side.weather != w {
                set_weather(att_side, def_side, w, 5);
            }
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
        Effect::Nop | Effect::Foresight => {
            if m.name == "MOVE_TELEPORT" && ex.wild && !att_is_player {
                return true; // the wild mon (an Abra) flees
            }
            return false;
        }
        _ => {}
    }

    // --- targeted moves from here. `gLastLandedMoves[target]` is 0 until the move connects.
    def.last_landed = None;
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

    // Future Sight: no accuracy roll, damage fixed now (`Cmd_trysetfutureattack`).
    if matches!(fx, Effect::FutureSight) {
        if def_side.future == 0 {
            def_side.future = 3;
            def_side.future_dmg = base_damage(dex, att, def, mv, def_side);
        }
        return false;
    }

    // --- status-only targeted moves (their own accuracy, no damage pipeline).
    let status_only = matches!(
        fx,
        Effect::Roar
            | Effect::Sleep
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
        if !accuracy_check(att, def, m.acc, m.mtype, rng) {
            if att_is_player {
                out.events += 1;
            }
            return false;
        }
        def.last_landed = Some(mv);
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
                    confuse(def, ((rng.modn(4)) + 2) as u8);
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
            // Trainer battle: the target is replaced by a random healthy party mon
            // (`Cmd_forcerandomswitch`, `battle_script_commands.c:6905`); it fails when
            // the target has no teammate left.
            Effect::Roar => {
                if !ex.wild && !att_is_player && out.others_alive > 0 {
                    out.forced_out = true;
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
    // Water Absorb / Volt Absorb heal a quarter and take the move; Flash Fire takes it
    // (`battle_util.c` `ABILITYEFFECT_MOVES_BLOCK`).
    if let Some(heal) = absorbs(def.ability, m.mtype) {
        if heal {
            def.hp = (def.hp + def.max_hp / 4).min(def.max_hp);
        }
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
    if !matches!(fx, Effect::AlwaysHit) && !accuracy_check(att, def, m.acc, m.mtype, rng) {
        if att_is_player {
            out.events += 1;
        }
        return false;
    }
    def.last_landed = Some(mv);

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
                    confuse(def, ((rng.modn(4)) + 2) as u8);
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
        // Thrash / Outrage: locked for 2-3 turns in total, confused when it ends
        // (`MOVE_EFFECT_THRASH`, `battle_script_commands.c:2719-2729`; the end-of-turn count
        // and confusion are `ENDTURN_THRASH`, `battle_util.c:952-972`).
        Effect::Rampage if att.rampage == 0 => {
            att.rampage = ((rng.roll() & 1) + 2) as u8;
            att.lock_move = Some(mv);
        }
        Effect::BrickBreak => {
            def_side.reflect = 0;
            def_side.light_screen = 0;
        }
        // `MOVE_EFFECT_SP_ATK_TWO_DOWN | AFFECTS_USER | CERTAIN`.
        Effect::Overheat => {
            apply_stat_change(att, 3, -2, false);
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
fn end_of_turn(
    m: &mut Mon,
    other: &mut Mon,
    side: &mut Side,
    out: &mut Outcome,
    is_player: bool,
    rng: &mut GameRng,
) {
    if m.hp <= 0 {
        return;
    }
    // Weather chip: Hail 1/16 to non-Ice, Sandstorm 1/16 to non-Rock/Ground/Steel
    // (`battle_util.c` `ENDTURN_SANDSTORM` / `ENDTURN_HAIL`, `BattleScript_DamagingWeatherContinues`).
    let immune = |t: u8| m.types.0 == t || m.types.1 == t;
    if (side.weather == W_HAIL && !immune(15))
        || (side.weather == W_SAND && !(immune(5) || immune(4) || immune(8)))
    {
        m.hp -= (m.max_hp / 16).max(1);
    }
    // Future Sight lands when its counter runs out (`ENDTURN_FUTURE_SIGHT`).
    if side.future > 0 {
        side.future -= 1;
        if side.future == 0 {
            m.hp -= side.future_dmg.max(1);
            side.future_dmg = 0;
            if is_player {
                out.events += 1;
            }
        }
    }
    if m.hp <= 0 {
        return;
    }
    if m.sitrus && m.hp > 0 && m.hp <= m.max_hp / 2 {
        m.sitrus = false;
        m.hp = (m.hp + 30).min(m.max_hp);
    }
    if m.rampage > 0 {
        m.rampage -= 1;
        if m.rampage == 0 {
            m.lock_move = None;
            if m.confusion == 0 {
                confuse(m, (rng.modn(4) + 2) as u8);
                if is_player {
                    out.events += 1;
                }
            }
        }
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
    if m.leftovers && m.hp > 0 && m.hp < m.max_hp {
        m.hp = (m.hp + (m.max_hp / 16).max(1)).min(m.max_hp);
    }
}

/// `ShouldUseItem` for a Full Restore: below a quarter of max HP (integer division), and
/// the n-th item (n >= 1) only once the trainer's valid mons are down to
/// `items - n + 1` (`if (i && validMons > (itemsNo - i) + 1) continue;`,
/// `battle_ai_switch_items.c:580`). `valid` counts the active mon and the unsent ones.
fn ai_wants_item(total: u32, left: u32, valid: u32, e: &Mon) -> bool {
    if left == 0 || e.hp <= 0 || e.hp >= e.max_hp / 4 {
        return false;
    }
    let used = total - left;
    used == 0 || valid <= total - used + 1
}

/// `Tactics::bag_honest`: a status alone is cured with a Full Heal only above this HP fraction; lower, the
/// Full Restore's free refill is worth its price (R3f: a Full Heal on a half-HP fighter costs a second item
/// turn later; 0.75 is a guess).
const FULL_HEAL_ABOVE: f64 = 0.75;

/// Mutable state of one fight that a rollout must be able to copy. Public so the playing
/// controller can carry it across the turns of a real fight ([`Fs::new`], [`decide`], [`note`]).
#[derive(Clone)]
pub struct Fs {
    scarce: f64,
    my_side: Side,
    en_side: Side,
    setup_idx: usize,
    sacs_left: u32,
    /// After a Roar/Whirlwind the fighter's next turn is spent switching back in.
    must_switch: bool,
    heals_left: u32,
    awakenings_left: u32,
    cures_left: u32,
    /// `Tactics::bag_honest`: Full Restores and Max Potions among `heals_left`, Full Heals left, and what
    /// the last committed item was (a Max Potion heal, a Full Heal cure).
    frs_left: u32,
    maxes_left: u32,
    fh_left: u32,
    pot_full: bool,
    cure_all: bool,
    last_heal: bool,
    /// Full Restores the trainer still holds.
    ai_heals: u32,
    others_alive: u32,
    /// Torrent stall turns used so far (max 2 per fight).
    stalls: u32,
    /// The last move either side used (`gCurrentMove`), which the trainer's choice of its next mon reads.
    last_move: Option<usize>,
}

/// What the human saw when choosing: the foe's best-move damage and whether a sure kill exists.
#[derive(Clone, Copy, Debug)]
pub struct Read {
    pub danger: i32,
    pub sure_kill: bool,
    pub stalled: bool,
}

impl Fs {
    /// The state at the start of a fight, exactly as [`fight`] sets it up.
    pub fn new(tac: &Tactics, ai_heals: u32) -> Fs {
        Fs {
            my_side: Side::default(),
            en_side: Side::default(),
            setup_idx: 0,
            sacs_left: tac.sacs,
            must_switch: false,
            heals_left: tac.heal_stock,
            awakenings_left: tac.awakenings,
            cures_left: tac.cures,
            frs_left: tac.fr_stock.min(tac.heal_stock),
            maxes_left: tac
                .max_stock
                .min(tac.heal_stock.saturating_sub(tac.fr_stock)),
            fh_left: tac.full_heals,
            pot_full: false,
            cure_all: false,
            last_heal: false,
            ai_heals,
            others_alive: tac.others_alive,
            stalls: 0,
            last_move: None,
            scarce: tac.scarce,
        }
    }

    /// X items / set-up moves of `Tactics::x_items` already used.
    pub fn setup_idx(&self) -> usize {
        self.setup_idx
    }

    /// Heal items the fighter may still spend, and sacrifices left.
    pub fn heals_left(&self) -> u32 {
        self.heals_left
    }

    pub fn sacs_left(&self) -> u32 {
        self.sacs_left
    }

    /// Living teammates besides the fighter (the controller re-reads them from the party).
    pub fn set_others_alive(&mut self, n: u32) {
        self.others_alive = n;
    }

    /// The last move either side used (`gCurrentMove`), which the trainer's choice of its next mon reads.
    pub fn set_last_move(&mut self, mv: Option<usize>) {
        self.last_move = mv;
    }

    /// The trainer's Full Restores still unused (the controller counts the heals it saw: the sim's own
    /// fight decrements this itself, `act_enemy`).
    pub fn set_ai_heals(&mut self, left: u32) {
        self.ai_heals = left;
    }

    /// Screens up on either side as (Reflect, Light Screen) turns left (`gSideTimers`).
    pub fn set_screens(&mut self, mine: (u8, u8), theirs: (u8, u8)) {
        self.my_side.reflect = mine.0;
        self.my_side.light_screen = mine.1;
        self.en_side.reflect = theirs.0;
        self.en_side.light_screen = theirs.1;
    }
}

/// One choice of the policy, with the numbers behind it.
#[derive(Clone, Debug)]
pub struct Decision {
    pub action: Action,
    pub read: Read,
    /// The lookahead ran: each candidate action with its wins out of `Tactics::rollouts`.
    pub rollouts: Vec<(Action, u32)>,
}

/// Fights the trainer's party (or one wild mon) in order. `me` carries its state in and
/// out (stages reset between enemy mons only for the enemy side; the player's X-item
/// stages persist through the trainer's whole party, like the real game).
///
/// With `tac.rollouts > 0` the human is a *lookahead* player: when the fight is contested
/// (the foe can reach the fighter within about two hits) it tries each sensible action —
/// the greedy choice, the next-best attack, a heal, the next X item, a sacrifice — by
/// playing the rest of the fight `tac.rollouts` times with fresh random numbers (no peeking
/// at the real game's future) and the greedy policy, and takes the action that wins most
/// often. A top runner knows these fights by heart; this is the model of that skill.
#[allow(clippy::too_many_arguments)]
pub fn fight(
    dex: &Dex,
    me: &mut Mon,
    tac: &Tactics,
    foes: &[Mon],
    ai_flags: u32,
    ai_heals: u32,
    wild: bool,
    rng: &mut GameRng,
) -> Outcome {
    let mut fs = Fs::new(tac, ai_heals);
    // Party slots, so that a mon the AI switches out returns to its place on the bench.
    let numbered: Vec<Mon> = foes
        .iter()
        .enumerate()
        .map(|(i, m)| Mon {
            slot: i,
            ..m.clone()
        })
        .collect();
    run(
        dex, me, tac, &numbered, ai_flags, ai_heals, wild, rng, &mut fs, None, None, 0,
    )
}

/// The greedy human policy for one turn (the pure part: nothing is committed).
#[allow(clippy::too_many_arguments)]
fn greedy_action(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    tac: &Tactics,
    fs: &Fs,
    wild: bool,
    next_foe: Option<&Mon>,
    out: &mut Outcome,
) -> (Action, Read) {
    let my_frac = me.hp as f64 / me.max_hp as f64;
    let forced = me.recharging || me.charging.is_some() || me.rampage > 0;
    // The human knows the enemy's strongest move: heal when inside its max-roll
    // KO range (no crit) and no sure kill is on the table this turn.
    let danger = e
        .moves
        .iter()
        .map(|&mv| calc_damage(dex, e, me, mv, 0, false, &fs.my_side, None))
        .max()
        .unwrap_or(0);
    let sure_kill = me.moves.iter().any(|&mv| {
        if !me.has_pp(mv) {
            return false;
        }
        let m = &dex.moves[mv];
        let fx = effect_from(&m.effect);
        m.power > 0
            && hit_pct(me, e, m) >= 95
            && !matches!(
                fx,
                Effect::Charge | Effect::SemiInvulnDig | Effect::SemiInvulnFly
            )
            && absorbs(e.ability, m.mtype).is_none()
            && (calc_damage(dex, me, e, mv, 0, false, &fs.en_side, None) * 85) / 100 >= e.hp
    });
    // A sure kill the fighter lands first (or survives the wait for) makes any heal pointless
    // — unless what comes next (a +1-priority Extreme Speed, a sandstorm tick) would finish a
    // fighter that low.
    let next_danger = next_foe
        .map(|nf| {
            nf.moves
                .iter()
                .map(|&mv| calc_damage(dex, nf, me, mv, 0, false, &fs.my_side, None))
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    let safe_kill = sure_kill
        && (speed(me) > speed(e) || me.hp > danger)
        && me.hp > next_danger + me.max_hp / 16;
    // Healing is only worth a turn if it actually leaves the KO range.
    let in_danger = me.hp <= danger && me.max_hp > danger && !safe_kill;
    // The foe in front is harmless (or about to fall) but the next one would take the
    // fighter in a single hit: spend the quiet turn on the heal ("two heals between E4
    // fights" — and between mons when the lineup is known by heart).
    let prep_heal = sure_kill
        && next_danger > 0
        && me.hp <= next_danger + me.max_hp / 16
        && me.max_hp > next_danger
        && (danger as f64) * 2.0 < me.hp as f64;
    // ... and never twice in a row for HP alone: after a heal the human takes the
    // free action instead of burning an item per turn against a foe that out-damages
    // the heal (the engine's old Lance loop: 11 Full Restores against Outrage).
    let heal_ok = !fs.last_heal;
    let t_withdraw = (tac.torrent && !forced)
        .then(|| {
            torrent_action(
                dex,
                me,
                e,
                &fs.my_side,
                danger,
                sure_kill,
                false,
                tac.torrent_early,
            )
        })
        .flatten();
    let t_stall = (tac.torrent && tac.torrent_stall && !forced && fs.stalls < 2)
        .then(|| {
            torrent_action(
                dex,
                me,
                e,
                &fs.my_side,
                danger,
                sure_kill,
                true,
                tac.torrent_early,
            )
        })
        .flatten();
    let mut read = Read {
        danger,
        sure_kill,
        stalled: false,
    };
    let next_x = tac.x_items.get(fs.setup_idx).copied();
    // `Tactics::bag_honest`: which items the bag really holds. Potions = Hyper / Max Potions (the cheap HP
    // heal goes first); a status alone takes a Full Heal when there is one, a Full Restore when HP is
    // needed too or nothing else is left.
    let honest = tac.bag_honest;
    let pots = fs.heals_left.saturating_sub(fs.frs_left);
    let hypers = pots.saturating_sub(fs.maxes_left);
    let pot_heal = if !honest || hypers > 0 {
        tac.heal
    } else {
        me.max_hp
    };
    let status_bad = matches!(me.status, Status1::Sleep(_) | Status1::Freeze)
        || (my_frac < 0.6 && (me.status == Status1::Paralysis || me.confusion > 1));
    let hp_need = (my_frac < tac.heal_below && me.hp <= 2 * danger && me.max_hp > danger
        || in_danger)
        && heal_ok
        && !safe_kill;
    let fr_ok = !honest
        || (fs.frs_left > 0
            && ((status_bad && (fs.fh_left == 0 || hp_need || my_frac < FULL_HEAL_ABOVE))
                || (hp_need && pots == 0)
                || (prep_heal && heal_ok && pots == 0)));
    let action = if fs.must_switch {
        Action::Switch
    } else if forced {
        Action::Move(me.charging.or(me.lock_move).unwrap_or(me.moves[0]))
    } else if danger >= me.hp && fs.sacs_left > 0 && (speed(e) > speed(me) || !sure_kill) {
        // A hit would (or could) faint the fighter: a teammate eats it. The free
        // action goes to the next X item, else to the attack itself.
        Action::Sac(if tac.sac_free { next_x } else { None })
    } else if let Some(a) = t_withdraw {
        a
    } else if tac.full_restore
        && fs.heals_left > 0
        && fr_ok
        && (matches!(me.status, Status1::Sleep(_) | Status1::Freeze)
            || (my_frac < 0.6 && (me.status == Status1::Paralysis || me.confusion > 1))
            || ((my_frac < tac.heal_below && me.hp <= 2 * danger && me.max_hp > danger
                || in_danger)
                && heal_ok
                && !safe_kill)
            || (prep_heal && heal_ok))
    {
        Action::FullRestore
    } else if honest && fs.fh_left > 0 && status_bad && my_frac >= FULL_HEAL_ABOVE {
        // `bag_honest`: a Full Heal cures every status, confusion included (`item_effects.h:47-49`).
        Action::Cure
    } else if fs.awakenings_left > 0 && matches!(me.status, Status1::Sleep(_)) && danger > 0 {
        Action::Awaken
    } else if fs.heals_left > 0
        && (!honest || pots > 0)
        && !(tac.sleep_no_potion && matches!(me.status, Status1::Sleep(_)) && pot_heal <= danger)
        && ((my_frac < tac.heal_below
            && me.hp <= 2 * danger
            && pot_heal as f64 + me.hp as f64 > danger as f64)
            || (in_danger && me.hp + pot_heal > danger)
            || prep_heal && me.hp + pot_heal > next_danger)
        && heal_ok
        && !(safe_kill && !prep_heal)
    {
        // A human only burns a heal turn when the foe can actually reach them
        // within ~2 hits (the WR run heals mid-fight only under real pressure).
        Action::Heal
    } else if fs.cures_left + if honest { fs.fh_left } else { 0 } > 0
        && matches!(me.status, Status1::Poison | Status1::Toxic(_))
        && !sure_kill
        && me.hp > danger
        && turns_to_kill(dex, me, e, fs, wild) >= 2
    {
        // PLAN `full_heals`: the poison would tick through a long fight (Muk's Minimize stack).
        Action::Cure
    } else if let (Some(x), true) = (
        next_x,
        me.hp * 2 > me.max_hp && !sure_kill && turns_to_kill(dex, me, e, fs, wild) >= tac.setup_min,
    ) {
        // Setting up costs the foe a free hit; it only pays when the foe would otherwise
        // take three or more attacks to drop (Pidgeot, Alakazam and Tyranitar fall to one
        // or two, so a runner never spends a turn on an X item there).
        Action::XItem(x)
    } else if let Some(a) = t_stall {
        read.stalled = true;
        a
    } else {
        match best_attack(dex, me, e, fs, wild).0 {
            Some(mv) => Action::Move(mv),
            // Struggle stands in when nothing has PP (typeless 50, recoil 1/4:
            // the engine approximates it with the first slot at zero cost —
            // rare enough to log as an event instead of modelling).
            None => {
                out.events += 1;
                Action::Move(me.moves[0])
            }
        }
    };
    (action, read)
}

/// Attacks the fighter's best move needs to drop the foe (mean roll, accuracy included).
fn turns_to_kill(dex: &Dex, me: &Mon, e: &Mon, fs: &Fs, wild: bool) -> u32 {
    let Some(mv) = best_attack(dex, me, e, fs, wild).0 else {
        return 99;
    };
    let m = &dex.moves[mv];
    let d = calc_damage(dex, me, e, mv, 0, false, &fs.en_side, None) as f64
        * 0.925
        * hit_pct(me, e, m) as f64
        / 100.0;
    if d < 1.0 {
        return 99;
    }
    (e.hp as f64 / d).ceil() as u32
}

/// Hit chance of a move in percent after the attacker's accuracy and the target's evasion
/// stages (`Cmd_accuracycheck`, `sAccuracyStageRatios`); 100 for never-miss moves.
fn hit_pct(att: &Mon, def: &Mon, m: &crate::data::MoveData) -> u32 {
    if m.acc == 0 || matches!(effect_from(&m.effect), Effect::AlwaysHit) {
        return 100;
    }
    let buff = (att.stages[5] - def.stages[6]).clamp(-6, 6);
    let (n, d) = ACC_RATIO[(buff + 6) as usize];
    (m.acc * n / d).min(100)
}

/// Full-knowledge attack pick: kill if a min-roll kill exists, else best expected. Returns
/// the choice and the runner-up (a different move worth trying).
/// PP economy (engine-audit 2026-11-13): among kills, and among near-best non-kills
/// (within 8%), prefer the slot with the most PP left — a human saves Surf/Blizzard PP on
/// trash instead of draining the best slot.
fn best_attack(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    fs: &Fs,
    wild: bool,
) -> (Option<usize>, Option<usize>) {
    let mut cands: Vec<(usize, f64)> = vec![];
    let mut best: Option<(usize, f64)> = None;
    let mut killer: Option<(usize, f64, i32, u32)> = None;
    for &mv in &me.moves {
        if !me.has_pp(mv) {
            continue;
        }
        if let Some((d, _)) = me.disabled {
            if d == mv {
                continue;
            }
        }
        let mut s = player_score(dex, me, e, mv, &fs.en_side);
        if s <= 0.0 {
            continue;
        }
        let m = &dex.moves[mv];
        // A slot with a tiny PP pool (Blizzard, Fire Blast: 5) is saved for the
        // mon it decides — kills, or at least ~1.7x the next best — the way the
        // WR run keeps Blizzard for Lance ("Max Elixir between E4 fights"; the
        // exact-engine audit of 2026-11-14 found the pool drained by Agatha).
        if m.pp <= SCARCE_PP && !wild {
            s *= fs.scarce;
        }
        let min_roll = (calc_damage(dex, me, e, mv, 0, false, &fs.en_side, None) * 85) / 100;
        let acc_ok = hit_pct(me, e, m) >= 95;
        if min_roll >= e.hp
            && acc_ok
            && !matches!(
                effect_from(&m.effect),
                Effect::Charge | Effect::SemiInvulnDig | Effect::SemiInvulnFly
            )
        {
            let spd = dex.moves[mv].prio as f64;
            let left = me.pp_left(mv);
            let pow = dex.moves[mv].power;
            let better = match killer {
                None => true,
                Some((_, kspd, kleft, kpow)) => {
                    spd > kspd || (spd == kspd && (left > kleft || (left == kleft && pow < kpow)))
                }
            };
            if better {
                killer = Some((mv, spd, left, pow));
            }
        }
        cands.push((mv, s));
        if best.is_none() || s > best.unwrap().1 {
            best = Some((mv, s));
        }
    }
    let pick = best.map(|(_, bs)| {
        cands
            .iter()
            .filter(|(_, s)| *s >= 0.92 * bs)
            .max_by_key(|(mv, _)| me.pp_left(*mv))
            .map(|&(mv, s)| (mv, s))
            .unwrap()
    });
    let chosen = killer
        .map(|(mv, s, _, _)| (mv, s))
        .or(pick)
        .map(|(mv, _)| mv);
    // The runner-up: the highest-scoring other move.
    let alt = cands
        .iter()
        .filter(|(mv, _)| Some(*mv) != chosen)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|&(mv, _)| mv);
    (chosen, alt)
}

/// Apply the bookkeeping of a chosen action (items spent, teammates fainted).
fn commit(fs: &mut Fs, action: Action, out: &mut Outcome) {
    match action {
        Action::Switch => fs.must_switch = false,
        Action::XItem(_) => fs.setup_idx += 1,
        Action::Heal => {
            // `bag_honest`: a Hyper Potion first, then a Max Potion.
            let hypers = fs
                .heals_left
                .saturating_sub(fs.frs_left)
                .saturating_sub(fs.maxes_left);
            fs.pot_full = hypers == 0 && fs.maxes_left > 0;
            if fs.pot_full {
                fs.maxes_left -= 1;
            }
            fs.heals_left = fs.heals_left.saturating_sub(1);
            if fs.heals_left < fs.frs_left {
                // No potion was left: the item was a Full Restore.
                fs.frs_left = fs.heals_left;
            }
        }
        Action::FullRestore => {
            fs.heals_left = fs.heals_left.saturating_sub(1);
            fs.frs_left = fs.frs_left.saturating_sub(1).min(fs.heals_left);
        }
        Action::Awaken => fs.awakenings_left = fs.awakenings_left.saturating_sub(1),
        Action::Cure => {
            // `bag_honest`: the Full Heals go first (only they cure more than poison).
            fs.cure_all = fs.fh_left > 0;
            if fs.cure_all {
                fs.fh_left -= 1;
            } else {
                fs.cures_left = fs.cures_left.saturating_sub(1);
            }
        }
        Action::Sac(x) => {
            fs.sacs_left -= 1;
            fs.others_alive = fs.others_alive.saturating_sub(1);
            out.others_alive = fs.others_alive;
            if x.is_some() {
                fs.setup_idx += 1;
            }
        }
        Action::Move(_) => {}
    }
    fs.last_heal = matches!(action, Action::Heal | Action::FullRestore);
}

fn same_action(a: &Action, b: &Action) -> bool {
    match (a, b) {
        (Action::Move(x), Action::Move(y)) => x == y,
        (Action::XItem(x), Action::XItem(y)) => x == y,
        (Action::Sac(x), Action::Sac(y)) => x == y,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

#[allow(clippy::too_many_arguments)]
fn run(
    dex: &Dex,
    me: &mut Mon,
    tac: &Tactics,
    foes: &[Mon],
    ai_flags: u32,
    ai_items_total: u32,
    wild: bool,
    rng: &mut GameRng,
    fs: &mut Fs,
    mut resume: Option<Mon>,
    mut forced_first: Option<Action>,
    depth: u8,
) -> Outcome {
    let mut out = Outcome::default();
    let ex = Exec { dex, wild };
    out.others_alive = fs.others_alive;
    // The party is not fought in its own order: after the first mon, every faint lets the trainer pick the
    // next one (`most_suitable`). `bench` holds the mons not yet sent out, in party order; with `resume` the
    // first entry is the foe already on the field (a rollout continues from it).
    let mut bench: Vec<Mon> = foes.to_vec();
    let mut foe_idx = 0usize;
    let mut last_fainted: Option<Mon> = None;
    while !bench.is_empty() {
        let pick = match (&last_fainted, foe_idx) {
            (Some(f), i) if i > 0 => most_suitable(dex, me, f, fs.last_move, &fs.my_side, &bench),
            _ => 0,
        };
        let foe = bench.remove(pick);
        let resumed = foe_idx == 0 && resume.is_some();
        let mut e = match resume.take() {
            Some(m) if foe_idx == 0 => m,
            _ => foe.clone(),
        };
        let mut guard = 0;
        // Entry abilities (`battle_util.c` `ABILITYEFFECT_ON_SWITCHIN`): Intimidate lowers
        // the fighter's Attack, Sand Stream raises a permanent sandstorm.
        if !resumed {
            enter_effects(&e, me, fs);
        }
        while e.hp > 0 && me.hp > 0 && guard < 120 {
            guard += 1;
            out.turns += 1;
            let (hp_turn0, up_turn0) = (me.hp, out.hp_up);

            // --- player action. The human knows the lineup, hence the next mon the trainer will pick.
            let (mut my_action, read, _) = choose(
                dex,
                me,
                &e,
                &bench,
                tac,
                fs,
                ai_flags,
                ai_items_total,
                wild,
                rng,
                depth,
                forced_first.take(),
                &mut out,
            );
            let danger = read.danger;
            let sure_kill = read.sure_kill;
            let mut stalled = read.stalled;
            if tac.menu_slip > 0.0
                && depth == 0
                && me.moves.len() > 1
                && !fs.must_switch
                && !(me.recharging || me.charging.is_some() || me.rampage > 0)
            {
                if let Action::Move(mv) = my_action {
                    // The slip draw comes from the game stream (a mistimed press moves it anyway).
                    if (rng.roll() as f64) < tac.menu_slip * 65536.0 {
                        let k = rng.modn(me.moves.len() as u32) as usize;
                        let alt = me.moves[k];
                        let usable = me.pp.get(k).is_none_or(|&p| p > 0);
                        if alt != mv && usable {
                            my_action = Action::Move(alt);
                            stalled = false;
                            out.events += 1;
                        }
                    }
                }
            }
            let helper_lives = after_choice(fs, tac, my_action, danger, stalled, &mut out);

            // --- enemy action. `AI_TrySwitchOrUseItem` first asks `ShouldSwitch`; a forced move (recharge,
            // two-turn, rampage) never reaches the controller.
            let en_forced = e.recharging || e.charging.is_some() || e.rampage > 0;
            let mut enemy_switched = false;
            if !wild && !en_forced {
                if let Some(bi) = ai_should_switch(dex, &e, me, &bench, rng) {
                    let mut old = e.clone();
                    // `SwitchInClearSetData`: the volatile state does not follow the mon to the bench.
                    old.stages = [0; 7];
                    old.confusion = 0;
                    old.seeded = false;
                    old.focus_energy = false;
                    old.flinch = false;
                    old.infatuated = false;
                    old.disabled = None;
                    old.protected = false;
                    old.last_landed = None;
                    let mut new_e = bench.remove(bi);
                    new_e.last_landed = None;
                    bench.push(old);
                    bench.sort_by_key(|m| m.slot);
                    e = new_e;
                    enter_effects(&e, me, fs);
                    enemy_switched = true;
                    out.events += 1;
                }
            }
            let valid = (bench.len() + 1) as u32;
            let en_move = if enemy_switched {
                None
            } else if en_forced {
                // A recharge or locked turn still "uses" its move: `do_move` clears the
                // recharge flag (the old code skipped it and the flag never cleared, so a
                // Hyper Beam silenced the mon for the rest of its life).
                Some(e.charging.or(e.lock_move).unwrap_or(e.moves[0]))
            } else if ai_wants_item(ai_items_total, fs.ai_heals, valid, &e) {
                // `ShouldUseItem`: Full Restore under 1/4 (`battle_ai_switch_items.c:562-598`).
                None
            } else {
                ai_choose(
                    dex,
                    &e,
                    me,
                    ai_flags,
                    &fs.en_side,
                    &fs.my_side,
                    out.turns,
                    out.others_alive,
                    rng,
                )
                .map(|i| e.moves[i])
            };
            let enemy_heals_now = !en_forced
                && !enemy_switched
                && en_move.is_none()
                && ai_wants_item(ai_items_total, fs.ai_heals, valid, &e);

            // --- order (`GetWhoStrikesFirst`): priority, then speed, ties Random() & 1.
            let my_prio = match my_action {
                Action::Move(mv) => dex.moves[mv].prio,
                Action::XItem(x) | Action::Sac(Some(x)) if x.setup_move(dex).is_some() => {
                    dex.moves[x.setup_move(dex).unwrap()].prio
                }
                _ => 6, // item use happens before moves
            };
            let en_prio = if enemy_heals_now || enemy_switched {
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
                    Action::Switch => {
                        out.item_turns += 1;
                    }
                    Action::XItem(x) if x.setup_move(dex).is_some() => {
                        let mv = x.setup_move(dex).unwrap();
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
                            XItem::Accuracy => {
                                apply_stat_change(me, 5, 1, false);
                            }
                            XItem::GuardSpec => my_side.mist = 5,
                            XItem::CalmMind | XItem::Move(_) => unreachable!("handled above"),
                        }
                    }
                    Action::Heal => {
                        out.item_turns += 1;
                        out.heals_used += 1;
                        let before = me.hp;
                        let amount = if tac.bag_honest && fs.pot_full {
                            out.maxes_used += 1;
                            me.max_hp
                        } else {
                            tac.heal
                        };
                        me.hp = (me.hp + amount).min(me.max_hp);
                        out.hp_up += me.hp - before;
                    }
                    Action::Awaken => {
                        out.item_turns += 1;
                        out.awakenings_used += 1;
                        if matches!(me.status, Status1::Sleep(_)) {
                            me.status = Status1::None;
                        }
                    }
                    Action::Cure => {
                        out.item_turns += 1;
                        if fs.cure_all {
                            out.full_heals_used += 1;
                            me.confusion = 0;
                        } else {
                            out.cures_used += 1;
                        }
                        me.status = Status1::None;
                    }
                    Action::FullRestore => {
                        out.item_turns += 1;
                        out.full_restores_used += 1;
                        out.hp_up += me.max_hp - me.hp.max(0);
                        me.hp = me.max_hp;
                        me.status = Status1::None;
                        me.confusion = 0;
                    }
                    Action::Sac(x) => {
                        if helper_lives {
                            out.sacs_survived += 1;
                        } else {
                            out.sacs_used += 1;
                        }
                        match x {
                            Some(x) if x.setup_move(dex).is_some() => {
                                let mv = x.setup_move(dex).unwrap();
                                do_move(&ex, me, e, mv, my_side, en_side, true, first, out, rng);
                            }
                            Some(x) => {
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
                                    XItem::Accuracy => {
                                        apply_stat_change(me, 5, 1, false);
                                    }
                                    XItem::GuardSpec => my_side.mist = 5,
                                    XItem::CalmMind | XItem::Move(_) => {
                                        unreachable!("handled above")
                                    }
                                }
                            }
                            None if !tac.sac_free => {}
                            None => {
                                // Attack from behind the sacrifice.
                                let mut best: Option<(usize, f64)> = None;
                                for &mv in &me.moves.clone() {
                                    if !me.has_pp(mv) {
                                        continue;
                                    }
                                    let sc = player_score(dex, me, e, mv, en_side);
                                    if sc > 0.0 && (best.is_none() || sc > best.unwrap().1) {
                                        best = Some((mv, sc));
                                    }
                                }
                                if let Some((mv, _)) = best {
                                    do_move(
                                        &ex, me, e, mv, my_side, en_side, true, first, out, rng,
                                    );
                                }
                            }
                        }
                    }
                }
            };
            let mut ai_heals_left = fs.ai_heals;
            let mut act_enemy = |me: &mut Mon,
                                 e: &mut Mon,
                                 my_side: &mut Side,
                                 en_side: &mut Side,
                                 first: bool,
                                 out: &mut Outcome,
                                 rng: &mut GameRng|
             -> bool {
                if enemy_heals_now {
                    ai_heals_left -= 1;
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
            if depth == 0 && std::env::var_os("FRLG_BATTLE_TRACE").is_some() {
                let an = match &my_action {
                    Action::Move(mv) => dex.moves[*mv].name.clone(),
                    Action::Switch => "Switch".into(),
                    Action::XItem(x) => format!("X{x:?}"),
                    Action::Heal => "Heal".into(),
                    Action::FullRestore => "FullRestore".into(),
                    Action::Awaken => "Awaken".into(),
                    Action::Cure => "Cure".into(),
                    Action::Sac(x) => format!("Sac {x:?}"),
                };
                eprintln!(
                    "t{} me {}/{} st{:?} stg{:?} | foe {} {}/{} | act {} | en {:?} first {} danger {} kill {} sacs {} foestg{:?}",
                    out.turns, me.hp, me.max_hp, me.status, me.stages, dex.species[foe.species].name, e.hp, e.max_hp,
                    an, en_move.map(|m| dex.moves[m].name.clone()), me_first, danger, sure_kill, fs.sacs_left, e.stages
                );
            }
            let fled = if me_first {
                act_player(
                    me,
                    &mut e,
                    &mut fs.my_side,
                    &mut fs.en_side,
                    true,
                    &mut out,
                    rng,
                );
                if e.hp > 0 && me.hp > 0 && !sac_turn {
                    act_enemy(
                        me,
                        &mut e,
                        &mut fs.my_side,
                        &mut fs.en_side,
                        false,
                        &mut out,
                        rng,
                    )
                } else {
                    false
                }
            } else if sac_turn {
                // The enemy's move falls on the sacrifice; the fighter acts freely.
                act_player(
                    me,
                    &mut e,
                    &mut fs.my_side,
                    &mut fs.en_side,
                    false,
                    &mut out,
                    rng,
                );
                false
            } else {
                let f = act_enemy(
                    me,
                    &mut e,
                    &mut fs.my_side,
                    &mut fs.en_side,
                    true,
                    &mut out,
                    rng,
                );
                if me.hp > 0 && e.hp > 0 && !f {
                    act_player(
                        me,
                        &mut e,
                        &mut fs.my_side,
                        &mut fs.en_side,
                        false,
                        &mut out,
                        rng,
                    );
                }
                f
            };
            fs.ai_heals = ai_heals_left;
            // `gCurrentMove` after the turn: the second actor's move when it got to act, else the first's.
            {
                let pm = match my_action {
                    Action::Move(mv) => Some(mv),
                    _ => None,
                };
                let em = if enemy_heals_now { None } else { en_move };
                let (first_m, second_m, second_acted) = if me_first {
                    (pm, em, e.hp > 0)
                } else {
                    (em, pm, me.hp > 0)
                };
                if let Some(mv) = if second_acted {
                    second_m.or(first_m)
                } else {
                    first_m
                } {
                    fs.last_move = Some(mv);
                }
            }
            if fled {
                // A wild Teleport: the battle just ends (no catch).
                out.won = false;
                out.hp_frac = (me.hp.max(0)) as f64 / me.max_hp as f64;
                out.end_status = me.status;
                return out;
            }

            end_of_turn(me, &mut e, &mut fs.my_side, &mut out, true, rng);
            end_of_turn(&mut e, me, &mut fs.en_side, &mut out, false, rng);
            {
                let healed = out.hp_up - up_turn0;
                let lost = (hp_turn0 + healed - me.hp.max(0)).max(0);
                out.hp_bar += (healed + lost) as f64 / me.max_hp.max(1) as f64;
            }
            for sd in [&mut fs.my_side, &mut fs.en_side] {
                if sd.weather_turns != 0 && sd.weather_turns != W_FOREVER {
                    sd.weather_turns -= 1;
                    if sd.weather_turns == 0 {
                        sd.weather = 0;
                    }
                }
            }
            if out.forced_out {
                // The drag-out clears the fighter's stat stages and volatile state
                // (`SwitchInClearSetData`); it comes back next turn.
                out.forced_out = false;
                out.dragged += 1;
                if me.hp > 0 {
                    me.stages = [0; 7];
                    me.focus_energy = false;
                    me.confusion = 0;
                    me.seeded = false;
                    me.infatuated = false;
                    fs.must_switch = true;
                    out.events += 1;
                }
            }
        }
        if me.hp <= 0 || e.hp > 0 {
            out.won = false;
            out.hp_frac = 0.0;
            out.end_status = me.status;
            return out;
        }
        out.kos.push((e.species, e.level));
        last_fainted = Some(e.clone());
        foe_idx += 1;
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

/// The player's choice for one turn: the greedy policy ([`greedy_action`]), replaced by the
/// lookahead ([`choose_by_rollouts`]) when the fight is contested, or by `forced` (a rollout's first
/// turn). Shared by the simulator ([`run`]) and the playing controller ([`decide`]); the third value
/// lists the lookahead's candidates with their wins when it ran.
#[allow(clippy::too_many_arguments)]
fn choose(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    bench: &[Mon],
    tac: &Tactics,
    fs: &Fs,
    ai_flags: u32,
    ai_items_total: u32,
    wild: bool,
    rng: &mut GameRng,
    depth: u8,
    forced: Option<Action>,
    out: &mut Outcome,
) -> (Action, Read, Vec<(Action, u32)>) {
    let next_foe = (!bench.is_empty())
        .then(|| &bench[most_suitable(dex, me, e, fs.last_move, &fs.my_side, bench)]);
    let (mut my_action, mut read) = greedy_action(dex, me, e, tac, fs, wild, next_foe, out);
    let mut wins = vec![];
    if let Some(a) = forced {
        my_action = a;
        read.stalled = false;
    } else if depth == 0
        && tac.rollouts > 0
        && !wild
        && !fs.must_switch
        && !(me.recharging || me.charging.is_some() || me.rampage > 0)
        && (me.hp as f64 <= 1.6 * read.danger as f64 || me.hp * 2 < me.max_hp)
    {
        let rest: Vec<Mon> = std::iter::once(e.clone())
            .chain(bench.iter().cloned())
            .collect();
        let (a, w) = choose_by_rollouts(
            dex,
            me,
            e,
            tac,
            fs,
            &rest,
            ai_flags,
            ai_items_total,
            wild,
            rng,
            my_action,
        );
        my_action = a;
        wins = w;
        read.stalled = false;
    }
    (my_action, read, wins)
}

/// Book-keeping after the player's choice: stall turns, items and teammates spent ([`commit`]), and
/// a sacrificed helper that survives the hit (the fighter has to be switched back in next turn).
fn after_choice(
    fs: &mut Fs,
    tac: &Tactics,
    action: Action,
    danger: i32,
    stalled: bool,
    out: &mut Outcome,
) -> bool {
    if stalled {
        fs.stalls += 1;
    }
    commit(fs, action, out);
    // A helper that the foe's hit cannot faint stays on the field: the fighter must be switched back
    // in next turn (`must_switch`: a turn in which the foe's move lands on the fighter).
    let helper_lives = matches!(action, Action::Sac(_)) && tac.helper_hp > danger;
    if helper_lives {
        fs.others_alive += 1;
        out.others_alive = fs.others_alive;
        fs.must_switch = true;
    }
    helper_lives
}

/// The simulator's own choice for one turn of a real fight: the playing controller's entry point.
/// `me` is the fighter on the field, `e` the foe on the field, `bench` the trainer's mons not yet
/// fainted and not on the field (empty for a wild mon), all built from the game's RAM; `fs` is the
/// fight state the controller carries from turn to turn ([`Fs::new`] at the start, [`note`] after
/// each choice). `rng` feeds only the lookahead's rollouts (the human's own imagination, never the
/// game's stream). Nothing is committed: call [`note`] with the action actually taken.
#[allow(clippy::too_many_arguments)]
pub fn decide(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    bench: &[Mon],
    tac: &Tactics,
    fs: &Fs,
    ai_flags: u32,
    ai_items_total: u32,
    wild: bool,
    rng: &mut GameRng,
) -> Decision {
    let mut out = Outcome::default();
    let (action, read, rollouts) = choose(
        dex,
        me,
        e,
        bench,
        tac,
        fs,
        ai_flags,
        ai_items_total,
        wild,
        rng,
        0,
        None,
        &mut out,
    );
    Decision {
        action,
        read,
        rollouts,
    }
}

/// Commit the action the controller took (same book-keeping as the simulator's turn).
pub fn note(fs: &mut Fs, tac: &Tactics, d: &Decision, taken: Action) {
    let mut out = Outcome::default();
    let stalled = d.read.stalled && taken == d.action;
    after_choice(fs, tac, taken, d.read.danger, stalled, &mut out);
}

/// The model's outlook from a real mid-fight state: `n` greedy continuations of the fight (the
/// simulator's [`run`] resumed with `e` on the field and `bench` behind it), each with its own
/// random stream from `seed`. Returns (wins / n, mean player turns, mean HP fraction left on a win).
#[allow(clippy::too_many_arguments)]
pub fn outlook(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    bench: &[Mon],
    tac: &Tactics,
    fs: &Fs,
    ai_flags: u32,
    ai_items_total: u32,
    wild: bool,
    n: u32,
    seed: u32,
) -> (f64, f64, f64) {
    let foes: Vec<Mon> = std::iter::once(e.clone())
        .chain(bench.iter().cloned())
        .collect();
    let (mut wins, mut turns, mut hp) = (0u32, 0u64, 0.0);
    for k in 0..n.max(1) {
        let mut me2 = me.clone();
        let mut fs2 = fs.clone();
        let mut r = GameRng::from_u32(seed.wrapping_add(k.wrapping_mul(0x9E37_79B1)));
        let res = run(
            dex,
            &mut me2,
            tac,
            &foes,
            ai_flags,
            ai_items_total,
            wild,
            &mut r,
            &mut fs2,
            Some(e.clone()),
            None,
            1,
        );
        wins += res.won as u32;
        turns += res.turns as u64;
        if res.won {
            hp += res.hp_frac;
        }
    }
    let n = n.max(1) as f64;
    (
        wins as f64 / n,
        turns as f64 / n,
        if wins > 0 { hp / wins as f64 } else { 0.0 },
    )
}

/// The trainer's next mon after `fainted` went down, as the model predicts it ([`most_suitable`]):
/// an index into `bench` (party order). The playing controller checks it against the game.
pub fn predict_next(dex: &Dex, me: &Mon, fainted: &Mon, fs: &Fs, bench: &[Mon]) -> usize {
    most_suitable(dex, me, fainted, fs.last_move, &fs.my_side, bench)
}

/// Weakening a wild mon before a throw. The simulator abstracts it (`Params::weaken`: one or two
/// turns that leave `weaken_hp` of the HP, `weaken_ko` risk); the playing controller needs a move:
/// the best expected damage ([`player_score`]) among the moves whose max roll leaves the foe at
/// least 1 HP, a crit included (x2, `Cmd_damagecalc`) when any move is that gentle, else without
/// the crit. `None` when no damaging move is gentle enough.
pub fn weaken_move(dex: &Dex, me: &Mon, e: &Mon) -> Option<usize> {
    let gentle = |crit: bool| {
        me.moves
            .iter()
            .filter(|&&mv| me.has_pp(mv))
            .filter_map(|&mv| {
                let (_, hi) = damage_range(dex, me, e, mv);
                // Multi-hit moves: the range is per hit (v3 controller: Double Kick at
                // "27-32 of 43 HP" knocked a Magnemite out instead of weakening it).
                let hits = match dex.moves[mv].effect.as_str() {
                    "EFFECT_DOUBLE_HIT" | "EFFECT_TWINEEDLE" => 2,
                    "EFFECT_MULTI_HIT" => 5,
                    "EFFECT_TRIPLE_KICK" => 3,
                    _ => 1,
                };
                let hi = hi * hits;
                let hi = if crit { 2 * hi } else { hi };
                let s = player_score(dex, me, e, mv, &Side::default());
                (s > 0.0 && hi > 0 && hi < e.hp).then_some((mv, s))
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(mv, _)| mv)
    };
    gentle(true).or_else(|| gentle(false))
}

/// The lookahead human: try the sensible actions and keep the one that wins most often in
/// `tac.rollouts` simulated continuations (the greedy action wins ties).
#[allow(clippy::too_many_arguments)]
fn choose_by_rollouts(
    dex: &Dex,
    me: &Mon,
    e: &Mon,
    tac: &Tactics,
    fs: &Fs,
    foes: &[Mon],
    ai_flags: u32,
    ai_items_total: u32,
    wild: bool,
    rng: &mut GameRng,
    greedy: Action,
) -> (Action, Vec<(Action, u32)>) {
    let mut cands: Vec<Action> = vec![greedy];
    let add = |a: Action, cands: &mut Vec<Action>| {
        if !cands.iter().any(|c| same_action(c, &a)) {
            cands.push(a);
        }
    };
    if let (_, Some(alt)) = best_attack(dex, me, e, fs, wild) {
        add(Action::Move(alt), &mut cands);
    }
    if tac.bag_honest {
        let pots = fs.heals_left.saturating_sub(fs.frs_left);
        if pots > 0 && me.hp < me.max_hp {
            add(Action::Heal, &mut cands);
        }
        if fs.frs_left > 0 && tac.full_restore && (me.hp < me.max_hp || me.status != Status1::None)
        {
            add(Action::FullRestore, &mut cands);
        }
    } else if fs.heals_left > 0 && me.hp < me.max_hp {
        add(
            if tac.full_restore {
                Action::FullRestore
            } else {
                Action::Heal
            },
            &mut cands,
        );
    }
    let next_x = tac.x_items.get(fs.setup_idx).copied();
    if let Some(x) = next_x {
        add(Action::XItem(x), &mut cands);
    }
    if fs.sacs_left > 0 && fs.others_alive > 0 {
        add(
            Action::Sac(if tac.sac_free { next_x } else { None }),
            &mut cands,
        );
    }
    if cands.len() < 2 {
        return (greedy, vec![]);
    }
    let n = tac.rollouts;
    let base = (rng.roll() as u32) << 16 | rng.roll() as u32;
    let mut wins = vec![0u32; cands.len()];
    for (ci, c) in cands.iter().enumerate() {
        for k in 0..n {
            let mut me2 = me.clone();
            let mut fs2 = fs.clone();
            // Common random numbers across candidates: the same seeds for each.
            let mut r2 = GameRng::from_u32(base.wrapping_add(k.wrapping_mul(0x9E37_79B1)));
            // The first turn of the rollout is the candidate itself.
            let res = run(
                dex,
                &mut me2,
                tac,
                foes,
                ai_flags,
                ai_items_total,
                wild,
                &mut r2,
                &mut fs2,
                Some(e.clone()),
                Some(*c),
                1,
            );
            wins[ci] += res.won as u32;
        }
    }
    let (bi, bw) = wins
        .iter()
        .enumerate()
        .max_by_key(|(i, w)| (**w, usize::MAX - *i))
        .map(|(i, w)| (i, *w))
        .unwrap();
    // Noise guard: leave the greedy action unless the alternative is clearly better.
    let table = cands.iter().copied().zip(wins.iter().copied()).collect();
    if bw >= wins[0] + (n / 8).max(1) {
        (cands[bi], table)
    } else {
        (greedy, table)
    }
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

    /// Calm Mind audit (`decompiled/src/pokemon.c:2440-2446` badge boosts, `CalculateBaseDamage` stage ratios):
    /// the Champion-2 specials against Mewtwo L73 (IV 20, Volcano badge) at SpD stage 0 / +1 / +2, hand-derived
    /// with integer math from the sim stats (trainer natures included: Gyarados SpA 126, Tyranitar SpA 147; Gen-3 Ghost is physical, so Shadow Ball ignores Calm Mind). Crits ignore the defender's positive stages.
    #[test]
    fn calm_mind_scales_champion_specials_by_hand() {
        let dex = Dex::load();
        let mut mew = make_mon(&dex, dex.id("SPECIES_MEWTWO"), 73, [20; 6], [0; 6], 0, 255);
        let f = dex.fight("TRAINER_CHAMPION_REMATCH_BULBASAUR");
        let en = |sp: &str| {
            let p = f
                .full
                .iter()
                .find(|m| dex.species[m.species].name == sp)
                .unwrap();
            enemy_mon(&dex, p)
        };
        let cases = [
            (
                "SPECIES_ALAKAZAM",
                "MOVE_SHADOW_BALL",
                [(52, 62), (52, 62), (52, 62)],
                (105, 124),
            ),
            (
                "SPECIES_GYARADOS",
                "MOVE_HYDRO_PUMP",
                [(73, 87), (49, 58), (38, 45)],
                (147, 174),
            ),
            (
                "SPECIES_TYRANITAR",
                "MOVE_CRUNCH",
                [(112, 132), (76, 90), (57, 68)],
                (224, 264),
            ),
        ];
        for (sp, mv, exp, crit) in cases {
            let e = en(sp);
            let mv = dex.move_id(mv);
            for (i, st) in [0i8, 1, 2].into_iter().enumerate() {
                mew.stages[4] = st;
                let (lo, hi) = damage_range(&dex, &e, &mew, mv);
                assert_eq!((lo, hi), exp[i], "{sp} stage {st}");
            }
            let c = calc_damage(&dex, &e, &mew, mv, 0, true, &Side::default(), None);
            assert_eq!((c * 85 / 100, c), crit, "{sp} crit ignores +2");
        }
    }

    /// The Champion-2 hits against the WR's Mewtwo (IVs 5/14, L73, 241 HP, all badges), engine vs
    /// `video-analysis/data/calib/v4-e4r2.md`. The video's 4 s frames catch the HP counter mid-drain, so single
    /// readings (Hydro Pump "55", Extreme Speed "24", Shadow Ball "37") understate; the totals agree with the
    /// engine's one-hit ranges: Shadow Ball 238 -> 176 = 62 (engine 56-66), Hydro Pump 164 -> 55 with one
    /// sandstorm tick (15) = 94 (engine 76-90 + crit-free roll), Extreme Speed 241 -> 178 with one tick = 48
    /// (engine 51-60). Journal 2026-11-29-10-00.
    #[test]
    fn champion2_hits_against_the_wr_mewtwo_match_the_video_totals() {
        let dex = Dex::load();
        let mew = make_mon(
            &dex,
            dex.id("SPECIES_MEWTWO"),
            73,
            [5, 14, 14, 14, 14, 14],
            [0; 6],
            0,
            255,
        );
        let f = dex.fight("TRAINER_CHAMPION_REMATCH_BULBASAUR");
        for (sp, mv, want) in [
            ("SPECIES_GYARADOS", "MOVE_HYDRO_PUMP", (76, 90)),
            ("SPECIES_ARCANINE", "MOVE_EXTREME_SPEED", (51, 60)),
            ("SPECIES_ALAKAZAM", "MOVE_SHADOW_BALL", (56, 66)),
        ] {
            let p = f
                .full
                .iter()
                .find(|m| dex.species[m.species].name == sp)
                .unwrap();
            let e = enemy_mon(&dex, p);
            assert_eq!(damage_range(&dex, &e, &mew, dex.move_id(mv)), want, "{sp}");
        }
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

    /// The WR run's starter (video overlay: Rash, IVs HP15 Atk26 Def9 SpA30 SpD20 Spe29) has the HP the
    /// video shows: 32 at Lv11 (Brock, "10/32") and 56 at Lv20 (Misty, "56/56"). The sim keeps IVs as
    /// HP, Atk, Def, Spe, SpA, SpD (`manip::ManipParams::mins`).
    #[test]
    fn wr_starter_stats_match_the_video() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let ivs = [15, 26, 9, 29, 30, 20];
        let sq = make_mon(&dex, dex.id("SPECIES_SQUIRTLE"), 11, ivs, [0; 6], rash, 0);
        assert_eq!(sq.max_hp, 32);
        let wt = make_mon(&dex, dex.id("SPECIES_WARTORTLE"), 20, ivs, [0; 6], rash, 0);
        assert_eq!(wt.max_hp, 56);
        // Rash: +SpA, -SpD.
        let neutral = make_mon(&dex, dex.id("SPECIES_WARTORTLE"), 20, ivs, [0; 6], 0, 0);
        assert!(wt.stat[3] > neutral.stat[3] && wt.stat[4] < neutral.stat[4]);
    }

    /// A sacrifice is a hit absorbed, not a free action: at 10/32 HP and no potions, two helpers lift Brock to
    /// ~100% only when the fighter may also act from behind them (the older model); honest, they buy little.
    #[test]
    fn honest_sacrifices_give_no_free_action() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let f = dex.fight("TRAINER_LEADER_BROCK");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let rate = |free: bool| {
            let (mut won, n) = (0u32, 1500u32);
            for i in 0..n {
                let mut me = make_mon(
                    &dex,
                    dex.id("SPECIES_SQUIRTLE"),
                    11,
                    [15, 26, 9, 29, 30, 20],
                    [0; 6],
                    rash,
                    0,
                );
                me.moves = vec![dex.move_id("MOVE_TACKLE"), dex.move_id("MOVE_BUBBLE")];
                me.hp = 10;
                let tac = Tactics {
                    torrent: true,
                    sacs: 2,
                    others_alive: 2,
                    heal_stock: 0,
                    sac_free: free,
                    ..Default::default()
                };
                let mut g = rng(i + 41000);
                won += fight(&dex, &mut me, &tac, &foes, f.ai, f.heals, false, &mut g).won as u32;
            }
            won as f64 / n as f64
        };
        let (free, honest) = (rate(true), rate(false));
        assert!(free > 0.9, "free {free}");
        assert!(honest < 0.8, "honest {honest}");
    }

    /// A helper that the foe's hit cannot faint stays on the field and the fighter eats the next hit on the
    /// switch back: against Brock's hits a 24-HP helper makes sacrifices strictly worse than none.
    #[test]
    fn surviving_helper_costs_a_turn_and_a_hit() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let f = dex.fight("TRAINER_LEADER_BROCK");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let rate = |helper_hp: i32| {
            let (mut won, mut surv, n) = (0u32, 0u32, 1500u32);
            for i in 0..n {
                let mut me = make_mon(
                    &dex,
                    dex.id("SPECIES_SQUIRTLE"),
                    11,
                    [15, 26, 9, 29, 30, 20],
                    [0; 6],
                    rash,
                    0,
                );
                me.moves = vec![dex.move_id("MOVE_TACKLE"), dex.move_id("MOVE_BUBBLE")];
                me.hp = 10;
                let tac = Tactics {
                    torrent: true,
                    sacs: 2,
                    others_alive: 2,
                    heal_stock: 0,
                    sac_free: false,
                    helper_hp,
                    ..Default::default()
                };
                let mut g = rng(i + 51000);
                let o = fight(&dex, &mut me, &tac, &foes, f.ai, f.heals, false, &mut g);
                won += o.won as u32;
                surv += o.sacs_survived;
            }
            (won as f64 / n as f64, surv)
        };
        let (dies, none_survive) = rate(0);
        let (lives, survived) = rate(24);
        assert_eq!(none_survive, 0);
        assert!(survived > 0);
        assert!(lives < dies, "lives {lives} dies {dies}");
    }

    /// The Torrent tactic at the WR's Brock state (Lv11, 10/32 HP, Tackle/Bubble/Withdraw): the engine wins
    /// ~97% with about 2 player actions per foe (the video shows 2 for the whole fight).
    #[test]
    fn brock_at_one_third_hp_with_torrent_is_a_short_survivable_fight() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let f = dex.fight("TRAINER_LEADER_BROCK");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let (mut won, mut turns, n) = (0u32, 0u32, 3000u32);
        for i in 0..n {
            let mut me = make_mon(
                &dex,
                dex.id("SPECIES_SQUIRTLE"),
                11,
                [15, 26, 9, 29, 30, 20],
                [0; 6],
                rash,
                0,
            );
            me.moves = vec![
                dex.move_id("MOVE_TACKLE"),
                dex.move_id("MOVE_BUBBLE"),
                dex.move_id("MOVE_WITHDRAW"),
            ];
            me.hp = 10;
            let tac = Tactics {
                torrent: true,
                ..Default::default()
            };
            let mut g = rng(i + 31000);
            let o = fight(&dex, &mut me, &tac, &foes, f.ai, f.heals, false, &mut g);
            won += o.won as u32;
            turns += o.turns;
        }
        let (w, t) = (won as f64 / n as f64, turns as f64 / n as f64);
        assert!(w > 0.9, "win rate {w}");
        assert!(t < 4.5, "actions {t}");
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
                sacs: 0,
                sac_free: true,
                helper_hp: 0,
                torrent: false,
                torrent_stall: true,
                torrent_early: false,
                others_alive: 0,
                heal_stock: u32::MAX,
                rollouts: 0,
                setup_min: 1,
                menu_slip: 0.0,
                scarce: SCARCE_FACTOR,
                sleep_no_potion: false,
                awakenings: 0,
                cures: 0,
                ..Default::default()
            };
            let mut g = rng(i + 7000);
            let o = fight(&dex, &mut me, &tac, &foes, f.ai, f.heals, false, &mut g);
            won += o.won as u32;
        }
        // The lone no-TM carry won ~5% of these in the old engine; Dig + two X Attacks
        // has to make the fight a real chance at level 26 (higher level or a third X item
        // pushes it well past that — the optimiser's call). Exact trainer natures
        // (`CreateNPCTrainerParty`) took this from ~50% to ~39%.
        assert!(won > 120, "won {won}/400");
    }

    /// `ShouldSwitch`: Dewgong has no super-effective move on Blastoise and has just been hit by Surf, so Lapras
    /// (Water Absorb) comes in on a coin flip each turn; with nothing landed yet (`gLastLandedMoves` is 0
    /// after a switch-in) nobody ever switches; a mon that holds a super-effective move (Thunder on Blastoise)
    /// keeps the field in all but a few turns (the 10% blind spot of the SE scan).
    #[test]
    fn trainer_switches_to_an_absorber_after_a_landed_water_move() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let me = make_mon(
            &dex,
            dex.id("SPECIES_BLASTOISE"),
            52,
            [15, 26, 9, 29, 30, 20],
            [0; 6],
            rash,
            0xff,
        );
        let f = dex.fight("TRAINER_ELITE_FOUR_LORELEI");
        let mut dewgong = enemy_mon(&dex, &f.full[0]);
        let lapras = enemy_mon(&dex, &f.full[4]);
        assert_eq!(lapras.ability, Ability::WaterAbsorb);
        let surf = dex.move_id("MOVE_SURF");
        let mut hits = 0;
        for k in 0..400 {
            let mut r = rng(k);
            dewgong.last_landed = None;
            assert_eq!(
                ai_should_switch(&dex, &dewgong, &me, std::slice::from_ref(&lapras), &mut r),
                None
            );
            dewgong.last_landed = Some(surf);
            if ai_should_switch(&dex, &dewgong, &me, std::slice::from_ref(&lapras), &mut r)
                == Some(0)
            {
                hits += 1;
            }
        }
        assert!((140..260).contains(&hits), "{hits}/400");
        // Lapras itself carries Thunder (super effective on Blastoise) and keeps the field.
        let mut sw = 0;
        let mut l2 = lapras.clone();
        l2.last_landed = Some(surf);
        let others = [enemy_mon(&dex, &f.full[1])];
        for k in 0..400 {
            let mut r = rng(k);
            sw += ai_should_switch(&dex, &l2, &me, &others, &mut r).is_some() as u32;
        }
        assert!(sw < 40, "{sw}/400");
    }

    /// The order in which the trainer sends out its mons, replayed from the decomp's own choice
    /// (`most_suitable`: `GetMostSuitableMonToSwitchInto`) against the orders the WR video shows
    /// (`v3-e4r1.md`, `v4-e4r2.md`: they differ from the party order in five of the eight fights seen).
    #[test]
    fn trainer_send_out_order_follows_the_decomp_and_the_video() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let blastoise = |lv: u32| {
            make_mon(
                &dex,
                dex.id("SPECIES_BLASTOISE"),
                lv,
                [15, 26, 9, 29, 30, 20],
                [0; 6],
                rash,
                0xff,
            )
        };
        let mewtwo = make_mon(
            &dex,
            dex.id("SPECIES_MEWTWO"),
            70,
            [5, 31, 14, 14, 14, 14],
            [0; 6],
            0,
            0xff,
        );
        let order = |trainer: &str, me: &Mon, last: &str| -> Vec<String> {
            let mut bench: Vec<Mon> = dex
                .fight(trainer)
                .full
                .iter()
                .map(|p| enemy_mon(&dex, p))
                .collect();
            let mut out = vec![];
            let mut cur = bench.remove(0);
            loop {
                out.push(format!(
                    "{}{}",
                    dex.species[cur.species].name.trim_start_matches("SPECIES_"),
                    cur.level
                ));
                if bench.is_empty() {
                    break;
                }
                cur.hp = 0;
                let i = most_suitable(
                    &dex,
                    me,
                    &cur,
                    Some(dex.move_id(last)),
                    &Side::default(),
                    &bench,
                );
                cur = bench.remove(i);
            }
            out
        };
        let b = blastoise(52);
        // Round 1 (Blastoise, killing move Surf).
        assert_eq!(
            order("TRAINER_ELITE_FOUR_AGATHA", &b, "MOVE_SURF"),
            ["GENGAR54", "GOLBAT54", "ARBOK56", "GENGAR58", "HAUNTER53"]
        );
        assert_eq!(
            order("TRAINER_ELITE_FOUR_BRUNO", &b, "MOVE_SURF"),
            [
                "ONIX51",
                "HITMONCHAN53",
                "HITMONLEE53",
                "MACHAMP56",
                "ONIX54"
            ]
        );
        assert_eq!(
            order("TRAINER_ELITE_FOUR_LANCE", &b, "MOVE_BLIZZARD"),
            [
                "GYARADOS56",
                "DRAGONAIR54",
                "DRAGONAIR54",
                "DRAGONITE60",
                "AERODACTYL58"
            ]
        );
        // Round 2 (Mewtwo, Psychic).
        assert_eq!(
            order("TRAINER_ELITE_FOUR_BRUNO_2", &mewtwo, "MOVE_PSYCHIC"),
            [
                "STEELIX65",
                "STEELIX66",
                "HITMONLEE65",
                "HITMONCHAN65",
                "MACHAMP68"
            ]
        );
        assert_eq!(
            order("TRAINER_ELITE_FOUR_AGATHA_2", &mewtwo, "MOVE_PSYCHIC"),
            [
                "GENGAR66",
                "CROBAT66",
                "GENGAR70",
                "MISDREAVUS65",
                "ARBOK68"
            ]
        );
        assert_eq!(
            order("TRAINER_ELITE_FOUR_LANCE_2", &mewtwo, "MOVE_PSYCHIC"),
            [
                "GYARADOS68",
                "KINGDRA66",
                "DRAGONITE66",
                "AERODACTYL70",
                "DRAGONITE72"
            ]
        );
        // The orders that did not change are reproduced too.
        assert_eq!(
            order("TRAINER_ELITE_FOUR_LORELEI", &b, "MOVE_SURF"),
            ["DEWGONG52", "CLOYSTER51", "SLOWBRO52", "JYNX54", "LAPRAS54"]
        );
        assert_eq!(
            order("TRAINER_CHAMPION_FIRST_BULBASAUR", &b, "MOVE_SURF"),
            [
                "PIDGEOT59",
                "VENUSAUR63",
                "ALAKAZAM57",
                "RHYDON59",
                "GYARADOS59",
                "ARCANINE61"
            ]
        );
        assert_eq!(
            order(
                "TRAINER_CHAMPION_REMATCH_BULBASAUR",
                &mewtwo,
                "MOVE_PSYCHIC"
            ),
            [
                "HERACROSS72",
                "ALAKAZAM73",
                "TYRANITAR72",
                "GYARADOS73",
                "ARCANINE73",
                "VENUSAUR75"
            ]
        );
    }

    /// `if_can_faint` counts a recharge / two-turn / self-KO move's damage (`Cmd_if_can_faint` excludes only
    /// `power < 2`), so Lance's Hyper Beam earns TRY_TO_FAINT's +4 whenever it would finish the target, however
    /// the "most powerful" test treats it. The old estimate returned 0 for such moves and the AI never chose a
    /// finishing Hyper Beam over its other moves.
    #[test]
    fn hyper_beam_that_can_faint_is_chosen() {
        let dex = Dex::load();
        let f = dex.fight("TRAINER_ELITE_FOUR_LANCE");
        let g = enemy_mon(&dex, &f.full[0]); // Gyarados L56: Hyper Beam, Dragon Rage, Twister, Bite
        let rash = crate::manip::nature_index("Rash").unwrap();
        let mut me = make_mon(
            &dex,
            dex.id("SPECIES_BLASTOISE"),
            54,
            [15, 26, 9, 29, 30, 20],
            [0; 6],
            rash,
            0xff,
        );
        let side = Side::default();
        let hb = dex.move_id("MOVE_HYPER_BEAM");
        let others: i32 = g
            .moves
            .iter()
            .filter(|&&m| m != hb)
            .map(|&m| calc_damage(&dex, &g, &me, m, 0, false, &side, None))
            .max()
            .unwrap();
        let hb_max = calc_damage(&dex, &g, &me, hb, 0, false, &side, None);
        // HP above every other move's best hit, at or below Hyper Beam's worst roll.
        me.hp = others + 1;
        assert!(
            hb_max * 85 / 100 >= me.hp,
            "hyper beam {hb_max} vs hp {}",
            me.hp
        );
        let mut picks = 0;
        for k in 0..40 {
            let mut r = rng(k);
            let c = ai_choose(&dex, &g, &me, 7, &side, &side, 1, 1, &mut r).unwrap();
            picks += (g.moves[c] == hb) as u32;
        }
        assert_eq!(
            picks, 40,
            "a Hyper Beam that finishes the target is always the pick"
        );
    }

    /// The AI never reads Mist (`AI_CBM_*` of the stat-drop scripts test the target's stage and ability only): the
    /// Champion's Pidgeot keeps using Sand Attack / Feather Dance into the carry's Guard Spec, as the WR's first
    /// Champion turns show (`Sand-Attack`, `A is protected by MIST!` x2), so its Aerial Ace is not the only pick.
    #[test]
    fn trainer_ai_ignores_mist() {
        let dex = Dex::load();
        let f = dex.fight("TRAINER_CHAMPION_FIRST_BULBASAUR");
        let pidgeot = enemy_mon(&dex, &f.full[0]);
        let me = make_mon(
            &dex,
            dex.id("SPECIES_BLASTOISE"),
            55,
            [15, 26, 9, 29, 30, 20],
            [0; 6],
            0,
            0xff,
        );
        let plain = Side::default();
        let misted = Side {
            mist: 5,
            ..Side::default()
        };
        let sa = dex.move_id("MOVE_SAND_ATTACK");
        let (mut with_mist, mut without) = (0, 0);
        for k in 0..400 {
            let c = ai_choose(&dex, &pidgeot, &me, 7, &plain, &misted, 1, 5, &mut rng(k)).unwrap();
            with_mist += (pidgeot.moves[c] == sa) as u32;
            let c = ai_choose(&dex, &pidgeot, &me, 7, &plain, &plain, 1, 5, &mut rng(k)).unwrap();
            without += (pidgeot.moves[c] == sa) as u32;
        }
        assert_eq!(with_mist, without);
        assert!(
            with_mist > 100,
            "Sand Attack ties with Aerial Ace: {with_mist}/400"
        );
    }

    /// The WR's Mewtwo (`v4-e4r2.md`): 231 HP at Lv70, 235 / 238 / 241 after the level-ups to 71 / 72 / 73. The
    /// stat formula (`pokemon.c:2093-2170`) gives exactly that for an HP IV of 5 and no HP EVs at every one of
    /// the four levels, so the level scaling is right and the WR's Mewtwo has a middling HP roll.
    #[test]
    fn wr_mewtwo_hp_matches_the_video() {
        let dex = Dex::load();
        let ivs = [5, 14, 14, 14, 14, 14];
        for (lv, hp) in [(70, 231), (71, 235), (72, 238), (73, 241)] {
            let m = make_mon(&dex, dex.id("SPECIES_MEWTWO"), lv, ivs, [0; 6], 0, 0);
            assert_eq!(m.max_hp, hp, "Mewtwo L{lv}");
        }
    }

    /// The WR's mid-game fights (`v2-mid-fights.md`) need no power multiplier: with the carry's real level, HP
    /// (inside the Torrent window at Koga 33/120 and Sabrina 43/135), Rash nature, 30-IV SpA, badges and
    /// plain Surf / Water Pulse, the engine's hits-to-kill are the video's. This is what `carry_power` ~1.7-2.0
    /// is made of: Torrent x1.5, times the model carry being 3 levels short at Koga (x1.19 on Surf).
    #[test]
    fn mid_game_hits_to_kill_match_the_video_without_a_power_stand_in() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let ivs = [15, 26, 9, 29, 30, 20];
        let carry = |lv: u32, badges: u8, hp: i32, max: i32| {
            let mut m = make_mon(
                &dex,
                dex.id("SPECIES_BLASTOISE"),
                lv,
                ivs,
                [0; 6],
                rash,
                badges,
            );
            m.hp = hp;
            m.max_hp = max;
            m
        };
        let hits = |me: &Mon, trainer: &str, foe: &str, mv: &str| {
            let f = dex.fight(trainer);
            let p = f.full.iter().find(|p| p.species == dex.id(foe)).unwrap();
            let e = enemy_mon(&dex, p);
            let (lo, hi) = damage_range(&dex, me, &e, dex.move_id(mv));
            let n = |d: i32| (e.max_hp + d - 1) / d;
            (n(hi), n(lo))
        };
        // Koga: Lv40 at 33/120 (Torrent), Boulder + Thunder badges. Koffing falls to one Surf, Muk takes two
        // ("Surf x2 turns"), Weezing two or three turns.
        let me = carry(40, 0b101, 33, 120);
        assert_eq!(
            hits(&me, "TRAINER_LEADER_KOGA", "SPECIES_KOFFING", "MOVE_SURF"),
            (1, 1)
        );
        assert_eq!(
            hits(&me, "TRAINER_LEADER_KOGA", "SPECIES_MUK", "MOVE_SURF"),
            (2, 2)
        );
        assert_eq!(
            hits(&me, "TRAINER_LEADER_KOGA", "SPECIES_WEEZING", "MOVE_SURF"),
            (1, 2)
        );
        // The same carry three levels lower (the model's, 37) at the same fraction of HP: Muk needs 3.
        let low = carry(37, 0b101, 37, 111);
        assert_eq!(
            hits(&low, "TRAINER_LEADER_KOGA", "SPECIES_MUK", "MOVE_SURF"),
            (2, 3)
        );
        // ... and out of the window (full HP) Surf is a third weaker: four hits for Muk.
        let full = carry(40, 0b101, 120, 120);
        assert_eq!(
            hits(&full, "TRAINER_LEADER_KOGA", "SPECIES_MUK", "MOVE_SURF"),
            (3, 3)
        );
        // Sabrina: Lv45 at 43/135 (Torrent), four badges. Water Pulse one-shots Kadabra, takes Mr. Mime two
        // ("Water Pulse ... Surf KOs Mr. Mime"), Surf one-shots Venomoth and Alakazam.
        let me = carry(45, 0b101_0101, 43, 135);
        assert_eq!(
            hits(
                &me,
                "TRAINER_LEADER_SABRINA",
                "SPECIES_KADABRA",
                "MOVE_WATER_PULSE"
            ),
            (1, 1)
        );
        assert_eq!(
            hits(
                &me,
                "TRAINER_LEADER_SABRINA",
                "SPECIES_MR_MIME",
                "MOVE_WATER_PULSE"
            ),
            (2, 2)
        );
        assert_eq!(
            hits(
                &me,
                "TRAINER_LEADER_SABRINA",
                "SPECIES_MR_MIME",
                "MOVE_SURF"
            ),
            (1, 1)
        );
        assert_eq!(
            hits(
                &me,
                "TRAINER_LEADER_SABRINA",
                "SPECIES_ALAKAZAM",
                "MOVE_SURF"
            ),
            (1, 1)
        );
        // `force_torrent` is that window without the HP.
        let mut free = carry(40, 0b101, 120, 120);
        free.force_torrent = true;
        assert_eq!(
            hits(&free, "TRAINER_LEADER_KOGA", "SPECIES_MUK", "MOVE_SURF"),
            (2, 2)
        );
    }

    /// Every modifier of `CalculateBaseDamage` / `Cmd_damagecalc` / `Cmd_typecalc` on hand cases (`pokemon.c:2385-2666`,
    /// `battle_script_commands.c:1209-1330`): badge boosts (player side only, Boulder Atk, Soul Def, Volcano
    /// SpA and SpD), held type booster, Guts / Hustle / Swarm / Marvel Scale / Huge Power, the Torrent threshold
    /// (`hp <= maxHP / 3`), burn, crit (x2, drops and gains ignored), STAB, stat stages.
    #[test]
    fn damage_modifiers_follow_the_decomp() {
        let dex = Dex::load();
        let side = Side::default();
        let mk = |sp: &str, lv: u32, badges: u8| {
            let mut m = make_mon(&dex, dex.id(sp), lv, [0; 6], [0; 6], 0, badges);
            m.stat = [100; 5];
            m.types = (9, 9); // typeless: no STAB, no resistances
            m.ability = Ability::None;
            m
        };
        // Reference: L50, A 100, D 100, power 40 physical (Pound) = (100*40*22/100/50) + 2 = 19.
        let pound = dex.move_id("MOVE_POUND");
        let surf = dex.move_id("MOVE_SURF"); // 95 special
        let bite = dex.move_id("MOVE_BITE"); // 60 special (Dark)
        let calc = |a: &Mon, d: &Mon, mv: usize, crit: bool| {
            calc_damage(&dex, a, d, mv, 0, crit, &side, None)
        };
        let (a0, d0) = (mk("SPECIES_RATTATA", 50, 0), mk("SPECIES_RATTATA", 50, 0));
        assert_eq!(calc(&a0, &d0, pound, false), 19);
        // Badges: Boulder +10% Attack of the badge holder's mon, Soul +10% Defense of the holder's mon; the
        // trainer side never gets any (`ShouldGetStatBadgeBoost`: `GetBattlerSide == B_SIDE_PLAYER`).
        let boulder = mk("SPECIES_RATTATA", 50, 1);
        assert_eq!(
            calc(&boulder, &d0, pound, false),
            100 * 110 / 100 * 40 * 22 / 100 / 50 + 2
        );
        let soul = mk("SPECIES_RATTATA", 50, 1 << 4);
        assert_eq!(
            calc(&a0, &soul, pound, false),
            100 * 40 * 22 / (100 * 110 / 100) / 50 + 2
        );
        assert_eq!(
            calc(&soul, &d0, pound, false),
            19,
            "Soul boosts Defense, not Attack"
        );
        let volcano = mk("SPECIES_RATTATA", 50, 1 << 6);
        assert_eq!(
            calc(&volcano, &d0, surf, false),
            110 * 95 * 22 / 100 / 50 + 2
        );
        assert_eq!(
            calc(&a0, &volcano, surf, false),
            100 * 95 * 22 / 110 / 50 + 2
        );
        assert_eq!(
            calc(&volcano, &d0, pound, false),
            19,
            "Volcano boosts the special stats only"
        );
        // Held type booster: +10% after the badge, only for its type.
        let mut glasses = mk("SPECIES_RATTATA", 50, 0);
        glasses.boost = Some((17, 10));
        assert_eq!(
            calc(&glasses, &d0, bite, false),
            110 * 60 * 22 / 100 / 50 + 2
        );
        assert_eq!(
            calc(&glasses, &d0, surf, false),
            100 * 95 * 22 / 100 / 50 + 2
        );
        // Torrent: power x1.5 when hp <= maxHP / 3 (integer division), Water only.
        let mut torrent = mk("SPECIES_RATTATA", 50, 0);
        torrent.ability = Ability::Torrent;
        for (max, hp, on) in [
            (100, 33, true),
            (100, 34, false),
            (101, 33, true),
            (101, 34, false),
            (102, 34, true),
            (102, 35, false),
        ] {
            torrent.max_hp = max;
            torrent.hp = hp;
            let want = if on {
                100 * (95 * 150 / 100) * 22 / 100 / 50 + 2
            } else {
                100 * 95 * 22 / 100 / 50 + 2
            };
            assert_eq!(calc(&torrent, &d0, surf, false), want, "max {max} hp {hp}");
        }
        assert_eq!(
            calc(&torrent, &d0, bite, false),
            100 * 60 * 22 / 100 / 50 + 2,
            "Torrent is Water only"
        );
        let mut swarm = mk("SPECIES_RATTATA", 50, 0);
        swarm.ability = Ability::Swarm;
        swarm.max_hp = 99;
        swarm.hp = 33;
        let megahorn = dex.move_id("MOVE_MEGAHORN"); // Bug
        assert_eq!(
            calc(&swarm, &d0, megahorn, false),
            100 * (120 * 150 / 100) * 22 / 100 / 50 + 2
        );
        // Guts / Hustle / Marvel Scale / Huge Power.
        let mut guts = mk("SPECIES_RATTATA", 50, 0);
        guts.ability = Ability::Guts;
        assert_eq!(calc(&guts, &d0, pound, false), 19);
        guts.status = Status1::Poison;
        assert_eq!(calc(&guts, &d0, pound, false), 150 * 40 * 22 / 100 / 50 + 2);
        guts.status = Status1::Burn; // burn's halving is waived for Guts
        assert_eq!(calc(&guts, &d0, pound, false), 150 * 40 * 22 / 100 / 50 + 2);
        let mut hustle = mk("SPECIES_RATTATA", 50, 0);
        hustle.ability = Ability::Hustle;
        assert_eq!(
            calc(&hustle, &d0, pound, false),
            150 * 40 * 22 / 100 / 50 + 2
        );
        assert_eq!(
            calc(&hustle, &d0, surf, false),
            100 * 95 * 22 / 100 / 50 + 2,
            "Hustle is physical only"
        );
        let mut scale = mk("SPECIES_RATTATA", 50, 0);
        scale.ability = Ability::MarvelScale;
        scale.status = Status1::Paralysis;
        assert_eq!(
            calc(&a0, &scale, pound, false),
            100 * 40 * 22 / 150 / 50 + 2
        );
        let mut huge = mk("SPECIES_RATTATA", 50, 0);
        huge.ability = Ability::HugePower;
        assert_eq!(calc(&huge, &d0, pound, false), 200 * 40 * 22 / 100 / 50 + 2);
        // Burn halves physical damage before the +2; special is untouched.
        let mut burnt = mk("SPECIES_RATTATA", 50, 0);
        burnt.status = Status1::Burn;
        assert_eq!(
            calc(&burnt, &d0, pound, false),
            100 * 40 * 22 / 100 / 50 / 2 + 2
        );
        assert_eq!(calc(&burnt, &d0, surf, false), 100 * 95 * 22 / 100 / 50 + 2);
        // Crit: (base + 2) x 2, ignoring the attacker's drops and the defender's gains, keeping the rest.
        let mut dropped = mk("SPECIES_RATTATA", 50, 0);
        dropped.stages[0] = -2;
        assert_eq!(
            calc(&dropped, &d0, pound, false),
            50 * 40 * 22 / 100 / 50 + 2
        );
        assert_eq!(calc(&dropped, &d0, pound, true), 19 * 2);
        let mut walled = mk("SPECIES_RATTATA", 50, 0);
        walled.stages[1] = 2;
        assert_eq!(
            calc(&a0, &walled, pound, false),
            100 * 40 * 22 / 200 / 50 + 2
        );
        assert_eq!(calc(&a0, &walled, pound, true), 19 * 2);
        let mut boosted = mk("SPECIES_RATTATA", 50, 0);
        boosted.stages[0] = 2;
        assert_eq!(
            calc(&boosted, &d0, pound, true),
            (200 * 40 * 22 / 100 / 50 + 2) * 2,
            "gains and defender drops count"
        );
        // STAB x1.5 after the crit doubling.
        let mut stab = mk("SPECIES_RATTATA", 50, 0);
        stab.types = (0, 0);
        assert_eq!(calc(&stab, &d0, pound, false), 19 * 15 / 10);
        assert_eq!(calc(&stab, &d0, pound, true), 19 * 2 * 15 / 10);
    }

    /// The WR video's R1 one-shots (`v3-e4r1.md`): the SpA stage the engine's Blastoise (WR IVs, Rash,
    /// 0 EVs, Black Glasses, all badges) needs to one-shot each foe with the observed move at the observed
    /// HP (Torrent when HP <= 1/3), at the maximum and at the minimum damage roll.
    #[test]
    #[ignore]
    fn probe_r1_implied_stage() {
        let dex = Dex::load();
        let rash = crate::manip::nature_index("Rash").unwrap();
        let ivs = [15, 26, 9, 29, 30, 20];
        let obs: [(&str, u32, i32, i32, &str, &str, usize); 14] = [
            (
                "TRAINER_ELITE_FOUR_LORELEI",
                50,
                25,
                150,
                "MOVE_BITE",
                "SPECIES_CLOYSTER",
                1,
            ),
            (
                "TRAINER_ELITE_FOUR_LORELEI",
                51,
                28,
                153,
                "MOVE_BITE",
                "SPECIES_SLOWBRO",
                2,
            ),
            (
                "TRAINER_ELITE_FOUR_LORELEI",
                51,
                28,
                153,
                "MOVE_SURF",
                "SPECIES_JYNX",
                3,
            ),
            (
                "TRAINER_ELITE_FOUR_BRUNO",
                52,
                48,
                156,
                "MOVE_SURF",
                "SPECIES_HITMONCHAN",
                1,
            ),
            (
                "TRAINER_ELITE_FOUR_BRUNO",
                52,
                48,
                156,
                "MOVE_SURF",
                "SPECIES_HITMONLEE",
                2,
            ),
            (
                "TRAINER_ELITE_FOUR_BRUNO",
                52,
                48,
                156,
                "MOVE_SURF",
                "SPECIES_MACHAMP",
                3,
            ),
            (
                "TRAINER_ELITE_FOUR_AGATHA",
                53,
                51,
                159,
                "MOVE_SURF",
                "SPECIES_GENGAR",
                0,
            ),
            (
                "TRAINER_ELITE_FOUR_AGATHA",
                53,
                51,
                159,
                "MOVE_SURF",
                "SPECIES_GOLBAT",
                1,
            ),
            (
                "TRAINER_ELITE_FOUR_AGATHA",
                53,
                51,
                159,
                "MOVE_SURF",
                "SPECIES_ARBOK",
                3,
            ),
            (
                "TRAINER_ELITE_FOUR_AGATHA",
                54,
                54,
                162,
                "MOVE_SURF",
                "SPECIES_GENGAR",
                4,
            ),
            (
                "TRAINER_ELITE_FOUR_AGATHA",
                54,
                54,
                162,
                "MOVE_SURF",
                "SPECIES_HAUNTER",
                2,
            ),
            (
                "TRAINER_CHAMPION_FIRST_BULBASAUR",
                56,
                131,
                167,
                "MOVE_SURF",
                "SPECIES_ALAKAZAM",
                2,
            ),
            (
                "TRAINER_CHAMPION_FIRST_BULBASAUR",
                56,
                131,
                167,
                "MOVE_SURF",
                "SPECIES_RHYDON",
                3,
            ),
            (
                "TRAINER_CHAMPION_FIRST_BULBASAUR",
                55,
                165,
                165,
                "MOVE_SURF",
                "SPECIES_PIDGEOT",
                0,
            ),
        ];
        for (tr, lv, hp, mhp, mv, sp, _slot) in obs {
            let f = dex.fight(tr);
            let sid = dex.id(sp);
            let Some(p) = f.full.iter().find(|p| p.species == sid) else {
                eprintln!("{tr} {sp}: not in party");
                continue;
            };
            let foe = enemy_mon(&dex, p);
            let mut me = make_mon(
                &dex,
                dex.id("SPECIES_BLASTOISE"),
                lv,
                ivs,
                [0; 6],
                rash,
                0xff,
            );
            me.boost = Some((17, 10));
            me.hp = hp;
            me.max_hp = mhp;
            let mvi = dex.move_id(mv);
            let side = Side::default();
            let mut need = [99i8; 2];
            for st in 0..=6i8 {
                me.stages[3] = st;
                let hi = calc_damage(&dex, &me, &foe, mvi, 0, false, &side, None);
                let lo = hi * 85 / 100;
                if need[0] == 99 && hi >= foe.hp {
                    need[0] = st;
                }
                if need[1] == 99 && lo >= foe.hp {
                    need[1] = st;
                }
            }
            me.stages[3] = 0;
            let d0 = calc_damage(&dex, &me, &foe, mvi, 0, false, &side, None);
            eprintln!(
                "{:>34} {:>12} L{} foe hp {:>3} dmg@+0 {:>3} ({:>3}%): stage for max-roll KO +{}, min-roll KO +{}  (me SpA {})",
                tr.trim_start_matches("TRAINER_"), sp.trim_start_matches("SPECIES_"), foe.level, foe.max_hp, d0,
                d0 * 100 / foe.max_hp, need[0], need[1], me.stat[3]
            );
        }
    }

    #[test]
    #[ignore]
    fn probe_oneshots() {
        let dex = Dex::load();
        let mut mt = make_mon(&dex, dex.id("SPECIES_MEWTWO"), 70, [31; 6], [0; 6], 0, 0xff);
        mt.stat[3] = 230;
        eprintln!("mewtwo stats {:?} hp {}", mt.stat, mt.max_hp);
        let psy = dex.move_id("MOVE_PSYCHIC");
        for id in [
            "TRAINER_ELITE_FOUR_LORELEI_2",
            "TRAINER_ELITE_FOUR_LANCE_2",
            "TRAINER_ELITE_FOUR_BRUNO_2",
            "TRAINER_ELITE_FOUR_AGATHA_2",
        ] {
            let f = dex.fight(id);
            for p in &f.full {
                let e = enemy_mon(&dex, p);
                let side = Side::default();
                let lo = calc_damage(&dex, &mt, &e, psy, 0, false, &side, None);
                eprintln!(
                    "{id} {} L{} hp {} spd {} psychic nominal {} ({:.0}%)",
                    dex.species[e.species].name,
                    e.level,
                    e.max_hp,
                    e.stat[4],
                    lo,
                    lo as f64 * 100.0 / e.max_hp as f64
                );
            }
        }
    }
    /// Leftovers (`battle_util.c:2626-2639`): 1/16 of max HP at the end of a turn while below max HP, never
    /// above max HP, nothing without the item.
    #[test]
    fn leftovers_heal_a_sixteenth_per_turn() {
        let dex = Dex::load();
        let mut me = make_mon(&dex, dex.id("SPECIES_BLASTOISE"), 50, [20; 6], [0; 6], 0, 0);
        let mut foe = make_mon(&dex, dex.id("SPECIES_RATTATA"), 5, [0; 6], [0; 6], 0, 0);
        let mut g = rng(1);
        me.hp = me.max_hp / 2;
        let half = me.hp;
        let mut side = Side::default();
        let mut out = Outcome::default();
        end_of_turn(&mut me, &mut foe, &mut side, &mut out, true, &mut g);
        assert_eq!(me.hp, half, "no item, no heal");
        me.leftovers = true;
        end_of_turn(&mut me, &mut foe, &mut side, &mut out, true, &mut g);
        assert_eq!(me.hp, half + me.max_hp / 16);
        me.hp = me.max_hp - 1;
        end_of_turn(&mut me, &mut foe, &mut side, &mut out, true, &mut g);
        assert_eq!(me.hp, me.max_hp);
    }

    /// MODEL CHANGE `sleep_no_potion` / `awakenings` (battle bridge 2026-11-22): a sleeping fighter in KO range
    /// drinks no Potion that the foe's hit out-damages (the item turn does not tick the sleep counter), still
    /// drinks one that outheals the hit, and with an Awakening in the bag wakes up first.
    #[test]
    fn sleeping_fighter_wakes_or_attacks_instead_of_a_losing_potion() {
        let dex = Dex::load();
        let mut me = make_mon(&dex, dex.id("SPECIES_WARTORTLE"), 27, [20; 6], [0; 6], 0, 0);
        me.moves = vec![dex.move_id("MOVE_BITE"), dex.move_id("MOVE_WATER_GUN")];
        me.pp = vec![];
        let mut foe = make_mon(&dex, dex.id("SPECIES_IVYSAUR"), 20, [20; 6], [0; 6], 0, 0);
        foe.moves = vec![dex.move_id("MOVE_RAZOR_LEAF")];
        foe.pp = vec![];
        let danger = calc_damage(
            &dex,
            &foe,
            &me,
            foe.moves[0],
            0,
            false,
            &Side::default(),
            None,
        );
        assert!(danger > 20 && danger < 60, "danger {danger}");
        me.hp = danger;
        me.status = Status1::Sleep(3);
        let pick = |tac: &Tactics| {
            let fs = Fs::new(tac, 0);
            greedy_action(
                &dex,
                &me,
                &foe,
                tac,
                &fs,
                false,
                None,
                &mut Outcome::default(),
            )
            .0
        };
        let old = Tactics {
            heal: 20,
            heal_below: 0.6,
            heal_stock: 4,
            ..Default::default()
        };
        assert!(
            matches!(pick(&old), Action::Heal),
            "old policy heals asleep"
        );
        let new = Tactics {
            sleep_no_potion: true,
            ..old.clone()
        };
        assert!(matches!(pick(&new), Action::Move(_)), "{:?}", pick(&new));
        let big = Tactics {
            heal: 60,
            ..new.clone()
        };
        assert!(
            matches!(pick(&big), Action::Heal),
            "a potion that outheals the hit"
        );
        let wake = Tactics {
            awakenings: 1,
            ..new.clone()
        };
        assert!(matches!(pick(&wake), Action::Awaken));
    }

    /// PLAN `full_heals` (model honesty 4): a poisoned fighter at full HP facing a long fight spends an
    /// item turn on its cure; without one in the bag it attacks.
    #[test]
    fn poisoned_fighter_uses_a_cure() {
        let dex = Dex::load();
        let mut me = make_mon(&dex, dex.id("SPECIES_BLASTOISE"), 42, [20; 6], [0; 6], 0, 0);
        me.moves = vec![dex.move_id("MOVE_WATER_GUN")];
        me.pp = vec![];
        me.status = Status1::Toxic(1);
        let mut foe = make_mon(&dex, dex.id("SPECIES_MUK"), 39, [0; 6], [0; 6], 0, 0);
        foe.moves = vec![dex.move_id("MOVE_SLUDGE")];
        foe.pp = vec![];
        let pick = |tac: &Tactics| {
            let fs = Fs::new(tac, 0);
            greedy_action(
                &dex,
                &me,
                &foe,
                tac,
                &fs,
                false,
                None,
                &mut Outcome::default(),
            )
            .0
        };
        let none = Tactics::default();
        assert!(matches!(pick(&none), Action::Move(_)));
        let cure = Tactics {
            cures: 1,
            ..Default::default()
        };
        assert!(matches!(pick(&cure), Action::Cure), "{:?}", pick(&cure));
    }

    /// MODEL CHANGE `league_bag_honest` (R3f): an asleep fighter at full HP takes a Full Heal, not a Full
    /// Restore; with only Hyper Potions it cannot cure; a low fighter with a Hyper Potion and a Full Restore
    /// drinks the potion. The old pool (every heal a Full Restore) uses the Full Restore for the sleep.
    #[test]
    fn honest_league_bag_picks_the_right_item() {
        let dex = Dex::load();
        let mut me = make_mon(&dex, dex.id("SPECIES_BLASTOISE"), 55, [20; 6], [0; 6], 0, 0);
        me.moves = vec![dex.move_id("MOVE_SURF")];
        me.pp = vec![];
        me.status = Status1::Sleep(3);
        let mut foe = make_mon(&dex, dex.id("SPECIES_GENGAR"), 56, [0; 6], [0; 6], 0, 0);
        foe.moves = vec![dex.move_id("MOVE_SHADOW_BALL")];
        foe.pp = vec![];
        let pick = |me: &Mon, tac: &Tactics| {
            let fs = Fs::new(tac, 0);
            greedy_action(
                &dex,
                me,
                &foe,
                tac,
                &fs,
                false,
                None,
                &mut Outcome::default(),
            )
            .0
        };
        let old = Tactics {
            heal: 999,
            full_restore: true,
            heal_stock: 2,
            ..Default::default()
        };
        assert!(matches!(pick(&me, &old), Action::FullRestore));
        let honest = Tactics {
            heal: 200,
            full_restore: true,
            heal_stock: 2,
            bag_honest: true,
            fr_stock: 1,
            full_heals: 1,
            ..Default::default()
        };
        assert!(
            matches!(pick(&me, &honest), Action::Cure),
            "{:?}",
            pick(&me, &honest)
        );
        let hypers_only = Tactics {
            full_restore: false,
            fr_stock: 0,
            full_heals: 0,
            ..honest.clone()
        };
        assert!(!matches!(
            pick(&me, &hypers_only),
            Action::FullRestore | Action::Cure
        ));
        let mut low = me.clone();
        low.status = Status1::None;
        low.hp = 30;
        let hyper_and_fr = Tactics {
            full_heals: 0,
            ..honest.clone()
        };
        assert!(
            matches!(pick(&low, &hyper_and_fr), Action::Heal),
            "{:?}",
            pick(&low, &hyper_and_fr)
        );
        let mut fs = Fs::new(&hyper_and_fr, 0);
        commit(&mut fs, Action::Heal, &mut Outcome::default());
        assert_eq!((fs.heals_left, fs.frs_left, fs.pot_full), (1, 1, false));
    }

    #[test]
    fn a_held_persim_cures_the_first_confusion_only() {
        let dex = Dex::load();
        let mut me = make_mon(&dex, dex.id("SPECIES_WARTORTLE"), 20, [20; 6], [0; 6], 0, 0);
        me.persim = true;
        confuse(&mut me, 3);
        assert_eq!((me.confusion, me.persim), (0, false));
        confuse(&mut me, 3);
        assert_eq!(me.confusion, 3);
    }

    /// Koga holds Hyper Potion x2 and a Full Heal (`decompiled/src/data/trainers.h`, `TRAINER_LEADER_KOGA`); only
    /// the potions heal HP (`battle_ai_switch_items.c:599-638`), so `ai_cure_items` leaves him two HP heals.
    #[test]
    fn a_full_heal_is_not_an_hp_heal() {
        let dex = Dex::load();
        let f = dex.fight("TRAINER_LEADER_KOGA");
        assert_eq!((f.heals, f.cures), (3, 1));
        let m = dex.fight("TRAINER_LEADER_MISTY");
        assert_eq!(m.heals - m.cures, 1);
    }
}
