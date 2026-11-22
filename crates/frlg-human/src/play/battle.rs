//! S4, battles and catches: the controller's battle loop. It reads the real battle from RAM
//! ([`super::battle_ram`]), converts it into the simulator's types ([`crate::battle::Mon`]), asks the
//! simulator's own policy what to do, and drives the battle menus with human inputs.
//!
//! # API
//!
//! ```ignore
//! use frlg_human::play::battle::{fight, BattleCfg, BattleEnd, WildPlan};
//! let plan = frlg_human::eval::Plan::load(dex, &plan_text)?;
//! let ctx = plan.ctx();
//! // A battle is on screen (or about to start: `fight` mashes A until it is).
//! let cfg = BattleCfg::new(&ctx, "misty");           // plan segment: tactics + Torrent phase
//! let rep = fight(&mut feed, &obs, &mut log, &cfg)?;  // returns on CB2_Overworld
//! assert_eq!(rep.end, BattleEnd::Won);
//! // Wild battles: the route script says what the plan wants with this encounter.
//! let cfg = BattleCfg { wild: WildPlan::Catch, ..BattleCfg::new(&ctx, "mt-moon") };
//! ```
//!
//! `fight` returns once `gMain.callback2` is `CB2_Overworld` again, after any evolution; the
//! post-battle field text (a leader's badge speech) is the route script's job.
//!
//! # One decision source
//!
//! | choice | simulator code |
//! | --- | --- |
//! | move / X item / heal / sacrifice | [`crate::battle::decide`] (greedy + lookahead, `choose`) |
//! | tactics of a fight | [`crate::sim::fight_tactics`] (plan `fight_tactics`, Torrent phase, X stock) |
//! | ball type | [`crate::sim::ball_tier`] + [`crate::sim::bag_ball`] |
//! | give up throwing | [`crate::sim::MAX_THROWS`] |
//! | delete a move for a new one | [`crate::sim::replace_slot`] |
//!
//! Choices the simulator never makes are logged as `model-vs-actual` and handled by a fixed rule
//! here: the lead fainting (send the highest-level mon left), weakening a wild mon before a throw
//! ([`crate::battle::weaken_move`]: the sim abstracts it), a heal or ball the bag does not hold.
//!
//! # Human timing
//!
//! Every menu press is late by `U{0..N}` ([`HumanFeed::press`]); text is mashed with the same
//! jitter. A reaction delay `U{lo..hi}` ([`HumanFeed::react`]) is paid at an action menu only after
//! a turn the human could not predict (a miss or a move that did nothing, a hit above the expected
//! max roll, a new status on either side, an enemy heal), and before choosing a send-out after a
//! faint: the sim's `Outcome::events`.
//!
//! # Logged
//!
//! `battle` per action menu (action, policy action, HP both sides, PP left, danger, sure kill,
//! win probability / expected turns from [`crate::battle::outlook`], lookahead table, last turn's
//! damage dealt and taken, the reaction reason), the trainer's send-outs against the model's
//! `most_suitable`, learn/skip decisions; `encounter` and `catch` (ball, odds, catch chance) for
//! wild mons; `model-vs-actual` wherever the game leaves the model (above, plus a trainer party
//! or stats that differ from the dex's trainer data, a bag without heals).
use frlg_emu::keys;
use frlg_route::{Feed, Observer, RouteError};
use serde_json::json;

use super::battle_ram::{self, BattleRam, RawBattleMon, RawPartyMon};
use super::events::EventLog;
use super::human::HumanFeed;
use super::ids;
use crate::battle::{self as sim_battle, Action, Fs, Mon, Status1, Tactics, XItem};
use crate::data::Dex;
use crate::rng::GameRng;
use crate::sim::{self, Ctx};

/// `B_OUTCOME_*` (`decompiled/include/constants/battle.h:76-86`).
const B_OUTCOME_WON: u8 = 1;
const B_OUTCOME_LOST: u8 = 2;
const B_OUTCOME_DREW: u8 = 3;
const B_OUTCOME_RAN: u8 = 4;
const B_OUTCOME_CAUGHT: u8 = 7;
/// `BATTLE_TYPE_TRAINER`, `BATTLE_TYPE_SAFARI` (`include/constants/battle.h:45-54`).
const BATTLE_TYPE_TRAINER: u32 = 1 << 3;
const BATTLE_TYPE_SAFARI: u32 = 1 << 7;
/// Battle-script opcodes (`decompiled/src/battle_script_commands.c:404-557`): `trygivecaughtmonnick`
/// asks for a nickname (answered NO with B).
const OP_TRYGIVECAUGHTMONNICK: u8 = 0xF3;
/// `STATUS1_*`, `STATUS2_CONFUSION`, `STATUS2_FOCUS_ENERGY` (`include/constants/battle.h:90-120`).
const STATUS1_SLEEP: u32 = 0x7;
const STATUS1_POISON: u32 = 1 << 3;
const STATUS1_BURN: u32 = 1 << 4;
const STATUS1_FREEZE: u32 = 1 << 5;
const STATUS1_PARALYSIS: u32 = 1 << 6;
const STATUS1_TOXIC_POISON: u32 = 1 << 7;
const STATUS2_CONFUSION: u32 = 0x7;
const STATUS2_FOCUS_ENERGY: u32 = 1 << 20;
/// Items (`decompiled/include/constants/items.h`).
const ITEM_MASTER_BALL: u16 = 1;
const ITEM_ULTRA_BALL: u16 = 2;
const ITEM_GREAT_BALL: u16 = 3;
const ITEM_POKE_BALL: u16 = 4;
const ITEM_POTION: u16 = 13;
const ITEM_AWAKENING: u16 = 17;
const ITEM_FULL_RESTORE: u16 = 19;
const ITEM_MAX_POTION: u16 = 20;
const ITEM_HYPER_POTION: u16 = 21;
const ITEM_SUPER_POTION: u16 = 22;
const ITEM_GUARD_SPEC: u16 = 73;
const ITEM_DIRE_HIT: u16 = 74;
const ITEM_X_ATTACK: u16 = 75;
const ITEM_X_DEFEND: u16 = 76;
const ITEM_X_SPEED: u16 = 77;
const ITEM_X_ACCURACY: u16 = 78;
const ITEM_X_SPECIAL: u16 = 79;
const ITEM_SITRUS_BERRY: u16 = 142;
const ITEM_LEFTOVERS: u16 = 200;
const ITEM_BLACK_GLASSES: u16 = 206;
const ITEM_MYSTIC_WATER: u16 = 209;
const ITEM_NEVER_MELT_ICE: u16 = 212;
/// Rollouts behind the logged win probability (the policy itself uses `Tactics::rollouts`).
const OUTLOOK_N: u32 = 200;

/// What the plan wants with a wild encounter (the route script knows: a hunt target, a forced
/// fight for exp, or neither).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WildPlan {
    Flee,
    Catch,
    Fight,
    /// Throw the Master Ball on the first turn (a legendary the plan catches, `Kind::Mewtwo`).
    Master,
}

/// How [`fight`] plays one battle.
pub struct BattleCfg<'a> {
    pub ctx: &'a Ctx<'a>,
    /// The plan segment the battle belongs to: a trainer fight's tactics key
    /// (`Params::fight_tactics`) and, for any battle, the Torrent phase (`Params::torrent_until`).
    /// Wild battles use the simulator's own key `"wild"` for tactics.
    pub seg: String,
    pub wild: WildPlan,
    /// Frames before the battle is declared stuck.
    pub budget: usize,
}

impl<'a> BattleCfg<'a> {
    pub fn new(ctx: &'a Ctx<'a>, seg: &str) -> Self {
        BattleCfg {
            ctx,
            seg: seg.to_string(),
            wild: WildPlan::Flee,
            budget: 60_000,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BattleEnd {
    Won,
    Lost,
    Ran,
    Caught,
    /// Any other `gBattleOutcome` (draw, teleport, the mon fled).
    Other(u8),
}

#[derive(Clone, Debug)]
pub struct BattleReport {
    pub end: BattleEnd,
    /// Action menus answered.
    pub turns: u32,
    pub throws: u32,
    /// Frames from the call to the overworld.
    pub frames: usize,
    pub trainer: Option<String>,
}

/// A battler converted to the simulator's type (`None` for a species or move the dex lacks).
pub fn sim_battler(dex: &Dex, r: &RawBattleMon, badges: u8, player: bool) -> Option<Mon> {
    let sp = species_index(dex, r.species)?;
    let mut m = sim_battle::make_mon(
        dex,
        sp,
        r.level as u32,
        [0; 6],
        [0; 6],
        0,
        if player { badges } else { 0 },
    );
    m.hp = r.hp as i32;
    m.max_hp = r.max_hp as i32;
    m.stat = r.stats.map(|s| s as i32);
    // `statStages[]`: HP, Atk, Def, Spe, SpA, SpD, Acc, Eva with 6 neutral; the sim keeps Atk..Eva.
    m.stages = std::array::from_fn(|i| (r.stages[i + 1] - 6).clamp(-6, 6));
    m.types = r.types;
    let (moves, pp) = moves_of(dex, &r.moves, &r.pp)?;
    m.moves = moves;
    m.pp = pp;
    m.status = status_of(r.status1);
    m.confusion = (r.status2 & STATUS2_CONFUSION) as u8;
    m.focus_energy = r.status2 & STATUS2_FOCUS_ENERGY != 0;
    held(&mut m, r.item);
    Some(m)
}

/// A party slot converted to the simulator's type (`slot` = party index).
pub fn sim_party_mon(
    dex: &Dex,
    r: &RawPartyMon,
    badges: u8,
    player: bool,
    slot: usize,
) -> Option<Mon> {
    let sp = species_index(dex, r.species)?;
    let mut m = sim_battle::make_mon(
        dex,
        sp,
        r.level as u32,
        r.ivs,
        r.evs,
        (r.personality % 25) as u8,
        if player { badges } else { 0 },
    );
    m.hp = r.hp as i32;
    m.max_hp = r.max_hp as i32;
    m.stat = r.stats.map(|s| s as i32);
    let (moves, pp) = moves_of(dex, &r.moves, &r.pp)?;
    m.moves = moves;
    m.pp = pp;
    m.status = status_of(r.status);
    m.slot = slot;
    held(&mut m, r.item);
    Some(m)
}

pub(crate) fn species_index(dex: &Dex, id: u16) -> Option<usize> {
    let name = ids::SPECIES.get(id as usize).filter(|n| !n.is_empty())?;
    dex.has_species(name).then(|| dex.id(name))
}

pub(crate) fn move_index(dex: &Dex, id: u16) -> Option<usize> {
    let name = ids::MOVES.get(id as usize).filter(|n| !n.is_empty())?;
    dex.has_move(name).then(|| dex.move_id(name))
}

pub(crate) fn moves_of(
    dex: &Dex,
    moves: &[u16; 4],
    pp: &[u8; 4],
) -> Option<(Vec<usize>, Vec<i32>)> {
    let mut m = vec![];
    let mut p = vec![];
    for (i, &id) in moves.iter().enumerate() {
        if id == 0 {
            continue;
        }
        m.push(move_index(dex, id)?);
        p.push(pp[i] as i32);
    }
    Some((m, p))
}

fn status_of(s: u32) -> Status1 {
    if s & STATUS1_SLEEP != 0 {
        Status1::Sleep((s & STATUS1_SLEEP) as u8)
    } else if s & STATUS1_TOXIC_POISON != 0 {
        Status1::Toxic(((s >> 8) & 0xF) as u8)
    } else if s & STATUS1_POISON != 0 {
        Status1::Poison
    } else if s & STATUS1_BURN != 0 {
        Status1::Burn
    } else if s & STATUS1_FREEZE != 0 {
        Status1::Freeze
    } else if s & STATUS1_PARALYSIS != 0 {
        Status1::Paralysis
    } else {
        Status1::None
    }
}

/// Held items the engine models (`Mon::boost`, `sitrus`, `leftovers`; type numbers from
/// `include/constants/pokemon.h:96-114`, boost +10% `holdEffectParam`).
fn held(m: &mut Mon, item: u16) {
    match item {
        ITEM_MYSTIC_WATER => m.boost = Some((11, 10)),
        ITEM_NEVER_MELT_ICE => m.boost = Some((15, 10)),
        ITEM_BLACK_GLASSES => m.boost = Some((17, 10)),
        ITEM_LEFTOVERS => m.leftovers = true,
        ITEM_SITRUS_BERRY => m.sitrus = true,
        _ => {}
    }
}

fn move_name(dex: &Dex, mv: usize) -> String {
    dex.moves[mv].name.trim_start_matches("MOVE_").to_string()
}

fn species_name(dex: &Dex, sp: usize) -> String {
    dex.species[sp]
        .name
        .trim_start_matches("SPECIES_")
        .to_string()
}

fn x_item_id(x: XItem) -> Option<u16> {
    Some(match x {
        XItem::Attack => ITEM_X_ATTACK,
        XItem::Defend => ITEM_X_DEFEND,
        XItem::Speed => ITEM_X_SPEED,
        XItem::Special => ITEM_X_SPECIAL,
        XItem::DireHit => ITEM_DIRE_HIT,
        XItem::Accuracy => ITEM_X_ACCURACY,
        XItem::GuardSpec => ITEM_GUARD_SPEC,
        XItem::CalmMind | XItem::Move(_) => return None,
    })
}

/// Everything the policy reads, from RAM.
pub struct Scene {
    pub me: Mon,
    pub foe: Mon,
    /// The trainer's mons neither on the field nor fainted.
    pub bench: Vec<Mon>,
    pub party: Vec<RawPartyMon>,
    /// Party slot on the field.
    pub active: usize,
    pub items: Vec<(u16, u16)>,
    pub balls: Vec<(u16, u16)>,
    pub badges: u8,
}

/// Read the battle as the policy needs it.
pub fn read_scene(
    dex: &Dex,
    obs: &Observer,
    ram: &BattleRam,
    emu: &mut frlg_emu::Emu,
    trainer: bool,
) -> Result<Scene, String> {
    let badges = battle_ram::badges(obs, emu);
    let rme = ram.battle_mon(emu, 0);
    let rfoe = ram.battle_mon(emu, 1);
    let mut me = sim_battler(dex, &rme, badges, true)
        .ok_or_else(|| format!("player battler not in the dex: {rme:?}"))?;
    let foe = sim_battler(dex, &rfoe, badges, false)
        .ok_or_else(|| format!("enemy battler not in the dex: {rfoe:?}"))?;
    let (dis, timer) = ram.disabled(emu, 0);
    if dis != 0 && timer > 0 {
        if let Some(mv) = move_index(dex, dis) {
            me.disabled = Some((mv, timer));
        }
    }
    let foe_slot = ram.party_index(emu, 1);
    let bench = if trainer {
        ram.enemy_party(emu)
            .iter()
            .enumerate()
            .filter(|&(i, m)| i != foe_slot && m.hp > 0 && !m.is_egg)
            .filter_map(|(i, m)| sim_party_mon(dex, m, 0, false, i))
            .collect()
    } else {
        vec![]
    };
    Ok(Scene {
        me,
        foe: Mon {
            slot: foe_slot,
            ..foe
        },
        bench,
        party: ram.player_party(emu),
        active: ram.party_index(emu, 0),
        items: battle_ram::bag_pocket(obs, emu, battle_ram::POCKET_ITEMS),
        balls: battle_ram::bag_pocket(obs, emu, battle_ram::POCKET_POKE_BALLS),
        badges,
    })
}

fn count(items: &[(u16, u16)], id: u16) -> u32 {
    items
        .iter()
        .filter(|&&(i, _)| i == id)
        .map(|&(_, q)| q as u32)
        .sum()
}

/// X items in the bag in `TacIn::x_stock` order.
fn x_stock(items: &[(u16, u16)]) -> [u32; 7] {
    [
        ITEM_X_ATTACK,
        ITEM_X_DEFEND,
        ITEM_X_SPEED,
        ITEM_X_SPECIAL,
        ITEM_DIRE_HIT,
        ITEM_X_ACCURACY,
        ITEM_GUARD_SPEC,
    ]
    .map(|id| count(items, id))
}

/// Heal items a fight may spend: potions of any tier and Full Restores.
fn heal_count(items: &[(u16, u16)]) -> u32 {
    [
        ITEM_POTION,
        ITEM_SUPER_POTION,
        ITEM_HYPER_POTION,
        ITEM_MAX_POTION,
        ITEM_FULL_RESTORE,
    ]
    .iter()
    .map(|&id| count(items, id))
    .sum()
}

/// The fight's tactics from the plan and the real party and bag (`sim::fight_tactics`).
pub fn tactics_for(ctx: &Ctx, seg: &str, wild: bool, sc: &Scene) -> Tactics {
    let league = sim::is_league(seg);
    let seg_i = ctx.segs.iter().position(|s| s.name == seg).unwrap_or(0);
    let alive_others = sc
        .party
        .iter()
        .enumerate()
        .filter(|&(i, m)| i != sc.active && m.hp > 0 && !m.is_egg)
        .count() as u32;
    let mewtwo = ctx.dex.id("SPECIES_MEWTWO");
    let mut tac = sim::fight_tactics(
        ctx,
        if wild { "wild" } else { seg },
        league,
        &sim::TacIn {
            seg_i,
            torrent_lost: false,
            level: sc.me.level,
            moves: &sc.me.moves,
            league_items: league || count(&sc.items, ITEM_FULL_RESTORE) > 0,
            x_stock: x_stock(&sc.items),
            fodder: alive_others,
            fodder_at_start: alive_others,
            heal_items: heal_count(&sc.items),
            mewtwo_now: sc.me.species == mewtwo,
            awakenings: count(&sc.items, ITEM_AWAKENING),
        },
    );
    // The real bag rations the heals (the sim leaves them unlimited outside the League), and the
    // potion it holds is the one the policy can count on.
    tac.heal_stock = tac.heal_stock.min(heal_count(&sc.items));
    if let (_, Some(id)) = heal_pick(&sc.items, sc.me.level, false) {
        tac.heal = item_hp(id);
    }
    tac
}

/// HP an item restores: Potion 20, Super Potion 50, Hyper Potion 200, Max Potion / Full Restore
/// all (`decompiled/src/data/pokemon/item_effects.h`; the sim's `heal_amount` tiers).
fn item_hp(id: u16) -> i32 {
    match id {
        ITEM_POTION => 20,
        ITEM_SUPER_POTION => 50,
        ITEM_HYPER_POTION => 200,
        _ => 999,
    }
}

/// The heal item the model means (`heal_amount`'s tier for this level, or a Full Restore), and the
/// one the bag can pay: that tier, else the nearest other.
fn heal_pick(items: &[(u16, u16)], level: u32, full: bool) -> (u16, Option<u16>) {
    let want: [u16; 3] = if full {
        [ITEM_FULL_RESTORE, ITEM_MAX_POTION, ITEM_HYPER_POTION]
    } else {
        match sim_battle::heal_amount(level, false) {
            20 => [ITEM_POTION, ITEM_SUPER_POTION, ITEM_HYPER_POTION],
            50 => [ITEM_SUPER_POTION, ITEM_HYPER_POTION, ITEM_POTION],
            _ => [ITEM_HYPER_POTION, ITEM_SUPER_POTION, ITEM_POTION],
        }
    };
    (
        want[0],
        want.iter().copied().find(|&id| count(items, id) > 0),
    )
}

/// One frame-by-frame human mash whose key depends on the state: `key(emu)` is pressed, each
/// press late by `U{0..N}` like [`HumanFeed::human_mash_until`].
fn mash_dyn(
    f: &mut HumanFeed,
    what: &str,
    budget: usize,
    mut key: impl FnMut(&mut frlg_emu::Emu) -> u16,
    mut until: impl FnMut(&mut frlg_emu::Emu) -> bool,
) -> Result<(), RouteError> {
    let start = f.fed();
    loop {
        if until(f.emu()) {
            return Ok(());
        }
        if f.fed() - start >= budget {
            return Err(RouteError::Timeout {
                what: what.to_string(),
                budget,
                frames: f.fed(),
            });
        }
        let n = f.model.jitter_n;
        let d = if n == 0 { 0 } else { f.below(n + 1) };
        for _ in 0..d {
            f.step(0)?;
            if until(f.emu()) {
                return Ok(());
            }
        }
        let k = key(f.emu());
        f.step(k)?;
        if until(f.emu()) {
            return Ok(());
        }
        f.step(0)?;
    }
}

/// A deliberate press, repeated (a human tries again when nothing moved) until `moved` holds:
/// up to `tries` presses, each followed by up to `wait` frames of watching.
fn press_until(
    f: &mut HumanFeed,
    what: &str,
    key: u16,
    tries: u32,
    wait: usize,
    mut moved: impl FnMut(&mut frlg_emu::Emu) -> bool,
) -> Result<(), RouteError> {
    for _ in 0..tries {
        f.press(key)?;
        for _ in 0..wait {
            if moved(f.emu()) {
                return Ok(());
            }
            f.step(0)?;
        }
        if moved(f.emu()) {
            return Ok(());
        }
    }
    Err(RouteError::Timeout {
        what: what.to_string(),
        budget: tries as usize * wait,
        frames: f.fed(),
    })
}

/// Steer a 2x2 menu cursor (bit 0 column, bit 1 row: the action and move menus,
/// `battle_controller_player.c` `HandleInputChooseAction` / `HandleInputChooseMove`).
fn nav_2x2(
    f: &mut HumanFeed,
    what: &str,
    target: u8,
    cursor: impl Fn(&mut frlg_emu::Emu) -> u8,
) -> Result<(), RouteError> {
    for _ in 0..4 {
        let cur = cursor(f.emu());
        if cur == target {
            return Ok(());
        }
        let key = if (cur ^ target) & 1 != 0 {
            if target & 1 != 0 {
                keys::RIGHT
            } else {
                keys::LEFT
            }
        } else if target & 2 != 0 {
            keys::DOWN
        } else {
            keys::UP
        };
        press_until(f, what, key, 4, 12, |e| cursor(e) != cur)?;
    }
    if cursor(f.emu()) == target {
        Ok(())
    } else {
        Err(RouteError::Timeout {
            what: what.to_string(),
            budget: 4,
            frames: f.fed(),
        })
    }
}

/// Both sides as the human saw them at an action menu.
#[derive(Clone, Copy, Debug)]
struct Seen {
    me_slot: usize,
    me_hp: i32,
    me_status: Status1,
    foe_slot: usize,
    foe_hp: i32,
    foe_status: Status1,
    /// The foe's best no-crit hit as the policy read it (0 = not computed).
    danger: i32,
    /// The player used a damaging move.
    attacked: bool,
}

impl Seen {
    fn of(sc: &Scene) -> Seen {
        Seen {
            me_slot: sc.active,
            me_hp: sc.me.hp,
            me_status: sc.me.status,
            foe_slot: sc.foe.slot,
            foe_hp: sc.foe.hp,
            foe_status: sc.foe.status,
            danger: 0,
            attacked: false,
        }
    }

    /// What about the turn since `self` a human did not see coming (the sim's reaction events:
    /// a miss or a move that did nothing, a hit above the expected max, a status, an enemy heal).
    fn surprise(&self, now: &Seen) -> Option<&'static str> {
        let same_me = self.me_slot == now.me_slot;
        let same_foe = self.foe_slot == now.foe_slot;
        if (same_me && !same_status(self.me_status, now.me_status))
            || (same_foe && !same_status(self.foe_status, now.foe_status))
        {
            Some("status")
        } else if same_foe && now.foe_hp > self.foe_hp {
            Some("enemy healed")
        } else if same_foe && self.attacked && now.foe_hp == self.foe_hp {
            Some("no damage dealt")
        } else if same_me && self.danger > 0 && self.me_hp - now.me_hp > self.danger {
            Some("hit above the expected max")
        } else {
            None
        }
    }
}

/// Same status kind (a sleep counter or Toxic counter ticking down is no news).
fn same_status(a: Status1, b: Status1) -> bool {
    std::mem::discriminant(&a) == std::mem::discriminant(&b)
}

/// Where the loop stands between presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Point {
    /// `gBattleOutcome` set.
    Over,
    /// The player's action menu takes input.
    Action,
    /// The party menu takes input (a faint, or the shift prompt).
    Party,
    /// The summary screen asks which move to forget.
    Forget,
}

struct Ctl<'a, 'b> {
    obs: &'a Observer,
    ram: BattleRam,
    dex: &'a Dex,
    cfg: &'a BattleCfg<'b>,
    trainer: Option<String>,
    ai: u32,
    ai_heals: u32,
    wild: bool,
    safari: bool,
    tac: Option<Tactics>,
    fs: Option<Fs>,
    fighter: Option<usize>,
    turns: u32,
    throws: u32,
    rng: GameRng,
    armed: std::cell::Cell<bool>,
    /// Both sides at the last action menu and what the policy expected, for the turn-result note
    /// and the reaction rule of the next menu.
    last: Option<Seen>,
    /// The foe on the field at the last menu, its bench, and the fighter then (the send-out check).
    last_foe: Option<(Mon, Vec<Mon>, Mon)>,
}

impl Ctl<'_, '_> {
    fn point(&self, emu: &mut frlg_emu::Emu) -> Option<Point> {
        let obs = self.obs;
        // `gBattleOutcome` is stale from the last battle until `BattleStartClearSetData` zeroes it
        // (`decompiled/src/battle_main.c`): only an outcome seen after a zero counts.
        let outcome = obs.battle_outcome(emu);
        if outcome == 0 {
            self.armed.set(true);
        } else if self.armed.get() {
            return Some(Point::Over);
        }
        if obs.battle_controller_is(emu, 0, "HandleInputChooseAction") {
            return Some(Point::Action);
        }
        if obs.callback2_is(emu, "CB2_UpdatePartyMenu")
            && obs.task_active(emu, "Task_HandleChooseMonInput")
        {
            return Some(Point::Party);
        }
        if obs.task_active(emu, "Task_InputHandler_SelectOrForgetMove") {
            return Some(Point::Forget);
        }
        None
    }

    /// Mash through text to the next decision point: A, except B at the nickname prompt.
    fn advance(&self, f: &mut HumanFeed, what: &str) -> Result<Point, RouteError> {
        let ram = self.ram.clone();
        let mut at = None;
        mash_dyn(
            f,
            what,
            self.cfg.budget,
            |e| {
                if ram.script_opcode(e) == OP_TRYGIVECAUGHTMONNICK {
                    keys::B
                } else {
                    keys::A
                }
            },
            |e| {
                at = self.point(e);
                at.is_some()
            },
        )?;
        Ok(at.unwrap())
    }
}

/// Play the battle on screen to its end with the simulator's policy; see the module doc.
pub fn fight(
    f: &mut HumanFeed,
    obs: &Observer,
    log: &mut EventLog,
    cfg: &BattleCfg,
) -> Result<BattleReport, RouteError> {
    let dex = cfg.ctx.dex;
    let ram = BattleRam::new(obs.symbols()).map_err(|e| RouteError::Timeout {
        what: format!("battle symbols: {e}"),
        budget: 0,
        frames: 0,
    })?;
    let start = f.fed();
    if !obs.in_battle(f.emu()) {
        f.human_mash_until("the battle to start", keys::A, 6000, |e| obs.in_battle(e))?;
    }
    let flags = obs.battle_type_flags(f.emu());
    let trainer = flags & BATTLE_TYPE_TRAINER != 0;
    let seed = (f.below(1 << 16) << 16) | f.below(1 << 16);
    let mut c = Ctl {
        obs,
        ram,
        dex,
        cfg,
        trainer: None,
        ai: 0,
        ai_heals: 0,
        wild: !trainer,
        safari: flags & BATTLE_TYPE_SAFARI != 0,
        tac: None,
        fs: None,
        fighter: None,
        turns: 0,
        throws: 0,
        rng: GameRng::from_u32(seed),
        armed: std::cell::Cell::new(false),
        last: None,
        last_foe: None,
    };
    if trainer {
        let id = c.ram.trainer_id(f.emu());
        let name = ids::TRAINERS
            .get(id as usize)
            .copied()
            .unwrap_or("")
            .to_string();
        let fight = dex.fight(&name);
        c.ai = fight.ai;
        c.ai_heals = fight.heals;
        c.trainer = Some(name);
    }
    if !battle_ram::battle_style_set(obs, f.emu()) && trainer {
        log.push(
            f.frame(),
            "warn",
            "battle style SHIFT: the 'change POKeMON?' prompt is answered NO",
            json!({}),
        );
    }
    let mut point = c.advance(f, "the first action menu")?;
    let mut started = false;
    loop {
        match point {
            Point::Over => break,
            Point::Action => {
                if !started {
                    started = true;
                    on_start(f, &mut c, log)?;
                }
                turn(f, &mut c, log)?;
            }
            Point::Party => party_choice(f, &mut c, log)?,
            Point::Forget => forget_choice(f, &mut c, log)?,
        }
        point = c.advance(f, "the next battle menu or the end")?;
    }
    let outcome = obs.battle_outcome(f.emu());
    let end = match outcome {
        B_OUTCOME_WON => BattleEnd::Won,
        B_OUTCOME_LOST | B_OUTCOME_DREW => BattleEnd::Lost,
        B_OUTCOME_RAN => BattleEnd::Ran,
        B_OUTCOME_CAUGHT => BattleEnd::Caught,
        o => BattleEnd::Other(o),
    };
    let (me_hp, me_max) = obs.party_lead_hp(f.emu());
    log.push(
        f.frame(),
        "battle",
        format!("battle over: {end:?} after {} turns", c.turns),
        json!({"outcome": outcome, "turns": c.turns, "throws": c.throws,
               "lead_hp": me_hp, "lead_max_hp": me_max}),
    );
    // Back to the field: B through the closing text (A in the evolution scene, where B would cancel
    // it; `decompiled/src/evolution_scene.c`), the move-forget screen answered by the policy.
    loop {
        let mut forget = false;
        let ram = c.ram.clone();
        mash_dyn(
            f,
            "the overworld after the battle",
            cfg.budget,
            |e| {
                if obs.callback2_is(e, "CB2_EvolutionSceneUpdate") || ram.script_opcode(e) == 0x5A {
                    keys::A
                } else {
                    keys::B
                }
            },
            |e| {
                forget = obs.task_active(e, "Task_InputHandler_SelectOrForgetMove");
                forget || obs.callback2_is(e, "CB2_Overworld")
            },
        )?;
        if forget {
            forget_choice(f, &mut c, log)?;
        } else {
            break;
        }
    }
    Ok(BattleReport {
        end,
        turns: c.turns,
        throws: c.throws,
        frames: f.fed() - start,
        trainer: c.trainer,
    })
}

/// First action menu: the encounter line, the trainer's party against the dex.
fn on_start(f: &mut HumanFeed, c: &mut Ctl, log: &mut EventLog) -> Result<(), RouteError> {
    let dex = c.dex;
    let sc = read_scene(dex, c.obs, &c.ram, f.emu(), c.trainer.is_some()).map_err(|e| {
        RouteError::Timeout {
            what: e,
            budget: 0,
            frames: 0,
        }
    })?;
    if let Some(name) = c.trainer.clone() {
        let fight = dex.fight(&name);
        let real: Vec<(usize, u32)> = c
            .ram
            .enemy_party(f.emu())
            .iter()
            .filter_map(|m| species_index(dex, m.species).map(|s| (s, m.level as u32)))
            .collect();
        let model: Vec<(usize, u32)> = fight.full.iter().map(|m| (m.species, m.level)).collect();
        log.push(
            f.frame(),
            "battle",
            format!(
                "{} sends out {} Lv{} ({} mons)",
                name.trim_start_matches("TRAINER_"),
                species_name(dex, sc.foe.species),
                sc.foe.level,
                real.len()
            ),
            json!({"trainer": name, "ai": c.ai, "ai_heals": c.ai_heals,
                   "party": real.iter().map(|&(s, l)| format!("{} {l}", species_name(dex, s))).collect::<Vec<_>>()}),
        );
        if real != model {
            log.push(
                f.frame(),
                "model-vs-actual",
                format!("{name}: party in RAM differs from the dex's trainer data"),
                json!({"real": format!("{real:?}"), "model": format!("{model:?}")}),
            );
        }
        // The model builds trainer mons with `enemy_mon` (neutral nature, `trainers.h` IV): compare.
        for (i, pm) in fight.full.iter().enumerate() {
            let Some(raw) = c.ram.enemy_party(f.emu()).get(i).cloned() else {
                break;
            };
            let m = sim_battle::enemy_mon(dex, pm);
            let real_stat = raw.stats.map(|s| s as i32);
            if m.stat != real_stat || m.max_hp != raw.max_hp as i32 {
                log.push(
                    f.frame(),
                    "model-vs-actual",
                    format!(
                        "{} #{i} {}: model stats {:?} hp {} vs RAM {:?} hp {}",
                        name.trim_start_matches("TRAINER_"),
                        species_name(dex, m.species),
                        m.stat,
                        m.max_hp,
                        real_stat,
                        raw.max_hp
                    ),
                    json!({"slot": i}),
                );
            }
        }
    } else {
        log.push(
            f.frame(),
            "encounter",
            format!(
                "wild {} Lv{}: {}",
                species_name(dex, sc.foe.species),
                sc.foe.level,
                match c.cfg.wild {
                    WildPlan::Flee => "fleeing",
                    WildPlan::Catch => "catching",
                    WildPlan::Fight => "fighting",
                    WildPlan::Master => "Master Ball",
                }
            ),
            json!({"species": species_name(dex, sc.foe.species), "level": sc.foe.level,
                   "hp": sc.foe.hp, "plan": format!("{:?}", c.cfg.wild)}),
        );
    }
    let tac = tactics_for(c.cfg.ctx, &c.cfg.seg, c.wild, &sc);
    if heal_count(&sc.items) == 0 && !c.wild {
        log.push(
            f.frame(),
            "model-vs-actual",
            "no heal item in the bag (the sim assumes unlimited potions outside the League)",
            json!({}),
        );
    }
    c.fs = Some(Fs::new(&tac, c.ai_heals));
    c.tac = Some(tac);
    c.fighter = Some(sc.active);
    Ok(())
}

/// One action menu: decide, log, drive.
fn turn(f: &mut HumanFeed, c: &mut Ctl, log: &mut EventLog) -> Result<(), RouteError> {
    let dex = c.dex;
    c.turns += 1;
    let sc = read_scene(dex, c.obs, &c.ram, f.emu(), c.trainer.is_some()).map_err(|e| {
        RouteError::Timeout {
            what: e,
            budget: 0,
            frames: 0,
        }
    })?;
    // A new mon on the field (a switch, a send-out after a faint): its own tactics, same fight state.
    if c.fighter != Some(sc.active) {
        c.fighter = Some(sc.active);
        c.tac = Some(tactics_for(c.cfg.ctx, &c.cfg.seg, c.wild, &sc));
    }
    // A human decides after watching the turn; only an unpredictable one costs a reaction
    // (`react`, the sim's `out.events`), the rest is the menu jitter of each press.
    let now = Seen::of(&sc);
    let surprise = c.last.and_then(|l| l.surprise(&now));
    if surprise.is_some() {
        f.react()?;
    }
    if c.wild && c.cfg.wild != WildPlan::Fight {
        c.last = Some(now);
        return wild_turn(f, c, log, &sc);
    }
    let tac = c.tac.clone().unwrap();
    let fs = c.fs.as_mut().unwrap();
    let others = sc
        .party
        .iter()
        .enumerate()
        .filter(|&(i, m)| i != sc.active && m.hp > 0 && !m.is_egg)
        .count() as u32;
    fs.set_others_alive(others);
    fs.set_screens(c.ram.screens(f.emu(), 0), c.ram.screens(f.emu(), 1));
    let d = sim_battle::decide(
        dex, &sc.me, &sc.foe, &sc.bench, &tac, fs, c.ai, c.ai_heals, c.wild, &mut c.rng,
    );
    let (win, exp_turns, hp_left) = sim_battle::outlook(
        dex,
        &sc.me,
        &sc.foe,
        &sc.bench,
        &tac,
        fs,
        c.ai,
        c.ai_heals,
        c.wild,
        OUTLOOK_N,
        c.turns.wrapping_mul(0x2545_F491),
    );
    let taken = execute(f, c, log, &sc, d.action)?;
    let what = describe(dex, &sc, taken);
    let mv_pp = match taken {
        Action::Move(mv) => sc
            .me
            .moves
            .iter()
            .position(|&m| m == mv)
            .map(|i| sc.me.pp[i]),
        _ => None,
    };
    // A new foe after a faint: is it the one the model's `most_suitable` sends?
    if let Some((old, old_bench, old_me)) = c.last_foe.take() {
        if old.slot != sc.foe.slot && old_bench.len() > 1 {
            let fainted = Mon { hp: 0, ..old };
            let i = sim_battle::predict_next(
                dex,
                &old_me,
                &fainted,
                c.fs.as_ref().unwrap(),
                &old_bench,
            );
            let model = old_bench[i].species;
            let text = format!(
                "trainer sent {} (model's most_suitable: {})",
                species_name(dex, sc.foe.species),
                species_name(dex, model)
            );
            if model == sc.foe.species {
                log.push(f.frame(), "battle", text, json!({"match": true}));
            } else {
                log.push(f.frame(), "model-vs-actual", text, json!({"match": false}));
            }
        }
    }
    c.last_foe = Some((sc.foe.clone(), sc.bench.clone(), sc.me.clone()));
    // What the last turn did, as the human saw it: HP lost on both sides (same mons only).
    let (dealt, took) = match c.last {
        Some(l) => (
            (l.foe_slot == sc.foe.slot).then_some(l.foe_hp - sc.foe.hp),
            (l.me_slot == sc.active).then_some(l.me_hp - sc.me.hp),
        ),
        None => (None, None),
    };
    c.last = Some(Seen {
        danger: d.read.danger,
        attacked: matches!(taken, Action::Move(mv) if dex.moves[mv].power > 0),
        ..now
    });
    let st = |m: &Mon| match m.status {
        Status1::None => String::new(),
        s => format!(" [{s:?}]"),
    };
    log.push(
        f.frame(),
        "battle",
        format!(
            "turn {}: {what}{} vs {} {}/{}{} (danger {}{}, win {:.0}%)",
            c.turns,
            st(&sc.me),
            species_name(dex, sc.foe.species),
            sc.foe.hp,
            sc.foe.max_hp,
            st(&sc.foe),
            d.read.danger,
            if d.read.sure_kill { ", sure kill" } else { "" },
            win * 100.0
        ),
        json!({
            "dealt_last": dealt, "took_last": took, "react": surprise,
            "my_status": format!("{:?}", sc.me.status), "enemy_status": format!("{:?}", sc.foe.status),
            "turn": c.turns, "action": what, "policy": format!("{:?}", d.action),
            "enemy": species_name(dex, sc.foe.species), "enemy_hp": sc.foe.hp, "enemy_max_hp": sc.foe.max_hp,
            "my_hp": sc.me.hp, "my_max_hp": sc.me.max_hp, "my_level": sc.me.level,
            "my_stats": sc.me.stat, "heal_hp": tac.heal, "heals_left": c.fs.as_ref().map(|f| f.heals_left()), "pp_left": mv_pp,
            "danger": d.read.danger, "sure_kill": d.read.sure_kill, "stall": d.read.stalled,
            "win_prob": win, "exp_turns": exp_turns, "hp_left_on_win": hp_left,
            "rollouts": d.rollouts.iter().map(|(a, w)| format!("{} {w}", describe(dex, &sc, *a))).collect::<Vec<_>>(),
            "bench": sc.bench.len(),
        }),
    );
    let fs = c.fs.as_mut().unwrap();
    sim_battle::note(fs, &tac, &d, taken);
    if let Action::Move(mv) = taken {
        fs.set_last_move(Some(mv));
    }
    Ok(())
}

fn describe(dex: &Dex, _sc: &Scene, a: Action) -> String {
    match a {
        Action::Move(mv) => move_name(dex, mv),
        Action::XItem(x) => match x.setup_move(dex) {
            Some(mv) => move_name(dex, mv),
            None => format!("{x:?}"),
        },
        Action::Heal => "potion".to_string(),
        Action::FullRestore => "Full Restore".into(),
        Action::Awaken => "Awakening".into(),
        Action::Switch => "switch back".into(),
        Action::Sac(_) => "sacrifice switch".into(),
    }
}

/// Carry out a policy action through the menus; returns the action actually taken (a heal the
/// bag cannot pay becomes the best attack).
fn execute(
    f: &mut HumanFeed,
    c: &mut Ctl,
    log: &mut EventLog,
    sc: &Scene,
    a: Action,
) -> Result<Action, RouteError> {
    let dex = c.dex;
    match a {
        Action::Move(mv) => {
            use_move(f, c, sc, mv)?;
            Ok(a)
        }
        Action::XItem(x) => {
            if let Some(mv) = x.setup_move(dex) {
                use_move(f, c, sc, mv)?;
                return Ok(a);
            }
            let id = x_item_id(x).unwrap();
            use_item(f, c, sc, battle_ram::POCKET_ITEMS, id, false)?;
            Ok(a)
        }
        Action::Heal | Action::FullRestore => {
            let (model, pick) = heal_pick(&sc.items, sc.me.level, a == Action::FullRestore);
            // A potion on a fighter at full HP "won't have any effect" and the bag stays open: the
            // policy's Full Restore for a status (Freeze) with only HP potions in the bag. Attack
            // (the frozen turn passes, with its thaw chance) instead.
            let pick = pick.filter(|&id| sc.me.hp < sc.me.max_hp || id == ITEM_FULL_RESTORE);
            if pick.is_none() && sc.me.hp >= sc.me.max_hp {
                log.push(
                    f.frame(),
                    "model-vs-actual",
                    "Full Restore chosen for a status at full HP, the bag has only HP potions \
                     (league_heals are Max Potions): attacking instead",
                    json!({"status": format!("{:?}", sc.me.status)}),
                );
                let mv = best_move(dex, sc);
                use_move(f, c, sc, mv)?;
                return Ok(Action::Move(mv));
            }
            match pick {
                Some(id) => {
                    if id != model {
                        log.push(
                            f.frame(),
                            "model-vs-actual",
                            format!("heal: item {id} instead of {model} (bag)"),
                            json!({"item": id, "model": model}),
                        );
                    }
                    use_item(f, c, sc, battle_ram::POCKET_ITEMS, id, true)?;
                    Ok(a)
                }
                None => {
                    log.push(
                        f.frame(),
                        "model-vs-actual",
                        "heal chosen but no heal item in the bag: attacking instead",
                        json!({}),
                    );
                    let mv = best_move(dex, sc);
                    use_move(f, c, sc, mv)?;
                    Ok(Action::Move(mv))
                }
            }
        }
        Action::Awaken => {
            // `tactics_for` counts the bag's Awakenings, so the policy only picks one the bag holds.
            use_item(f, c, sc, battle_ram::POCKET_ITEMS, ITEM_AWAKENING, true)?;
            Ok(a)
        }
        Action::Sac(x) => {
            if x.is_some() {
                log.push(
                    f.frame(),
                    "model-vs-actual",
                    "sacrifice with a free X item from the back (sac_free): the game has no such action",
                    json!({}),
                );
            }
            let helper = sc
                .party
                .iter()
                .enumerate()
                .filter(|&(i, m)| i != sc.active && m.hp > 0 && !m.is_egg)
                .min_by_key(|&(_, m)| m.level)
                .map(|(i, _)| i);
            match helper {
                Some(slot) => {
                    switch_to(f, c, slot)?;
                    Ok(a)
                }
                None => {
                    let mv = best_move(dex, sc);
                    use_move(f, c, sc, mv)?;
                    Ok(Action::Move(mv))
                }
            }
        }
        Action::Switch => {
            let slot = c.fighter.unwrap_or(0);
            switch_to(f, c, slot)?;
            Ok(a)
        }
    }
}

/// The highest-PP damaging move with PP, for the fallbacks.
fn best_move(dex: &Dex, sc: &Scene) -> usize {
    sc.me
        .moves
        .iter()
        .enumerate()
        .filter(|&(i, &mv)| sc.me.pp.get(i).is_none_or(|&p| p > 0) && dex.moves[mv].power > 0)
        .max_by_key(|&(_, &mv)| dex.moves[mv].power)
        .map(|(_, &mv)| mv)
        .unwrap_or(sc.me.moves[0])
}

/// FIGHT, steer to the move's slot, A; then mash to the end of the selection.
fn use_move(f: &mut HumanFeed, c: &Ctl, sc: &Scene, mv: usize) -> Result<(), RouteError> {
    let obs = c.obs;
    let slot = sc.me.moves.iter().position(|&m| m == mv).unwrap_or(0) as u8;
    nav_2x2(f, "action cursor on FIGHT", 0, |e| obs.action_cursor(e, 0))?;
    press_until(f, "the move menu", keys::A, 4, 30, |e| {
        obs.battle_controller_is(e, 0, "HandleInputChooseMove")
    })?;
    nav_2x2(f, "move cursor", slot, |e| obs.move_cursor(e, 0))?;
    press_until(f, "the move chosen", keys::A, 4, 30, |e| {
        !obs.battle_controller_is(e, 0, "HandleInputChooseMove")
    })
}

/// BAG, pocket, row, A, USE (and the party slot for a potion).
fn use_item(
    f: &mut HumanFeed,
    c: &Ctl,
    sc: &Scene,
    pocket: u16,
    item: u16,
    on_mon: bool,
) -> Result<(), RouteError> {
    let obs = c.obs;
    let ram = &c.ram;
    let list = if pocket == battle_ram::POCKET_POKE_BALLS {
        &sc.balls
    } else {
        &sc.items
    };
    let row = list.iter().position(|&(i, _)| i == item).unwrap_or(0) as u16;
    nav_2x2(f, "action cursor on BAG", 1, |e| obs.action_cursor(e, 0))?;
    press_until(f, "the bag", keys::A, 4, 120, |e| {
        obs.task_active(e, "Task_BagMenu_HandleInput")
    })?;
    for _ in 0..4 {
        let (p, _) = ram.bag_cursor(f.emu());
        if p == pocket {
            break;
        }
        let key = if pocket > p { keys::RIGHT } else { keys::LEFT };
        press_until(f, "the bag pocket", key, 4, 40, |e| {
            ram.bag_cursor(e).0 != p && obs.task_active(e, "Task_BagMenu_HandleInput")
        })?;
    }
    for _ in 0..64 {
        let (_, r) = ram.bag_cursor(f.emu());
        if r == row {
            break;
        }
        let key = if row > r { keys::DOWN } else { keys::UP };
        press_until(f, "the bag row", key, 4, 12, |e| ram.bag_cursor(e).1 != r)?;
    }
    press_until(f, "the item's context menu", keys::A, 4, 30, |e| {
        obs.task_active(e, "Task_FieldItemContextMenuHandleInput")
    })?;
    press_until(f, "USE", keys::A, 4, 30, |e| {
        !obs.task_active(e, "Task_FieldItemContextMenuHandleInput")
    })?;
    if on_mon {
        f.wait_until("the party menu for the item", 600, |e| {
            obs.task_active(e, "Task_HandleChooseMonInput")
        })?;
        let slot = c.ram.menu_slot_of(f.emu(), sc.active);
        party_nav(f, c, slot as i8)?;
        press_until(f, "the item used", keys::A, 4, 30, |e| {
            !obs.task_active(e, "Task_HandleChooseMonInput")
        })?;
    }
    Ok(())
}

/// Steer the party menu cursor (`gPartyMenu.slotId`) to `slot`.
fn party_nav(f: &mut HumanFeed, c: &Ctl, slot: i8) -> Result<(), RouteError> {
    let ram = &c.ram;
    for _ in 0..12 {
        let cur = ram.party_menu_slot(f.emu());
        if cur == slot {
            return Ok(());
        }
        let key = if slot > cur && cur >= 0 {
            keys::DOWN
        } else {
            keys::UP
        };
        press_until(f, "the party cursor", key, 4, 12, |e| {
            ram.party_menu_slot(e) != cur
        })?;
    }
    Err(RouteError::Timeout {
        what: format!("party cursor to slot {slot}"),
        budget: 12,
        frames: f.fed(),
    })
}

/// POKEMON, the mon at field party index `field`, A, SHIFT.
fn switch_to(f: &mut HumanFeed, c: &Ctl, field: usize) -> Result<(), RouteError> {
    let obs = c.obs;
    nav_2x2(f, "action cursor on POKEMON", 2, |e| {
        obs.action_cursor(e, 0)
    })?;
    press_until(f, "the party menu", keys::A, 4, 120, |e| {
        obs.task_active(e, "Task_HandleChooseMonInput")
    })?;
    let slot = c.ram.menu_slot_of(f.emu(), field);
    pick_party(f, c, slot)
}

/// In the party menu: steer to menu `slot` (battle order), A, then A on the first entry of the
/// selection menu (SHIFT / SEND OUT; `party_menu.c` `Task_HandleSelectionMenuInput`).
fn pick_party(f: &mut HumanFeed, c: &Ctl, slot: usize) -> Result<(), RouteError> {
    let obs = c.obs;
    party_nav(f, c, slot as i8)?;
    press_until(f, "the selection menu", keys::A, 4, 40, |e| {
        obs.task_active(e, "Task_HandleSelectionMenuInput")
    })?;
    press_until(f, "the mon sent", keys::A, 4, 60, |e| {
        !obs.task_active(e, "Task_HandleSelectionMenuInput")
            && !obs.task_active(e, "Task_HandleChooseMonInput")
    })
}

/// Wild battle under `WildPlan::Flee` / `Catch`.
fn wild_turn(
    f: &mut HumanFeed,
    c: &mut Ctl,
    log: &mut EventLog,
    sc: &Scene,
) -> Result<(), RouteError> {
    let dex = c.dex;
    let ctx = c.cfg.ctx;
    let obs = c.obs;
    if c.cfg.wild == WildPlan::Master && count(&sc.balls, ITEM_MASTER_BALL) > 0 {
        c.throws += 1;
        log.push(
            f.frame(),
            "catch",
            format!(
                "throw {}: Master Ball at {} Lv{}, catch chance 1.00",
                c.throws,
                species_name(dex, sc.foe.species),
                sc.foe.level
            ),
            json!({"ball": ITEM_MASTER_BALL, "chance": 1.0, "throw": c.throws,
                   "species": species_name(dex, sc.foe.species)}),
        );
        return use_item(
            f,
            c,
            sc,
            battle_ram::POCKET_POKE_BALLS,
            ITEM_MASTER_BALL,
            false,
        );
    }
    if c.cfg.wild == WildPlan::Catch && !c.safari {
        let sp = &dex.species[sc.foe.species];
        // Weaken first (plan `weaken`): only while the foe is above half HP.
        if ctx.p.weaken && c.throws == 0 && sc.foe.hp * 2 > sc.foe.max_hp {
            if let Some(mv) = sim_battle::weaken_move(dex, &sc.me, &sc.foe) {
                let (lo, hi) = sim_battle::damage_range(dex, &sc.me, &sc.foe, mv);
                log.push(
                    f.frame(),
                    "catch",
                    format!(
                        "weaken {} with {} ({lo}-{hi} of {} HP)",
                        species_name(dex, sc.foe.species),
                        move_name(dex, mv),
                        sc.foe.hp
                    ),
                    json!({"move": move_name(dex, mv), "lo": lo, "hi": hi, "hp": sc.foe.hp}),
                );
                if c.turns == 1 {
                    log.push(
                        f.frame(),
                        "model-vs-actual",
                        "weakening is a rule of the controller: the sim abstracts it (1-2 turns to weaken_hp)",
                        json!({}),
                    );
                }
                return use_move(f, c, sc, mv);
            }
        }
        let have =
            [ITEM_POKE_BALL, ITEM_GREAT_BALL, ITEM_ULTRA_BALL].map(|id| count(&sc.balls, id));
        let want = sim::ball_tier(ctx, sp.catch_rate);
        let tier = sim::bag_ball(want, have);
        if c.throws < sim::MAX_THROWS {
            if let Some(t) = tier {
                if t != want {
                    log.push(
                        f.frame(),
                        "model-vs-actual",
                        format!("ball tier {t} thrown, the plan wants {want} (bag {have:?})"),
                        json!({"want": want, "tier": t}),
                    );
                }
                let item = [ITEM_POKE_BALL, ITEM_GREAT_BALL, ITEM_ULTRA_BALL][t];
                let odds = sim::catch_odds(
                    sp.catch_rate,
                    sim::BALL_MULT[t],
                    sc.foe.hp as u32,
                    sc.foe.max_hp as u32,
                    sc.foe.status,
                );
                c.throws += 1;
                log.push(
                    f.frame(),
                    "catch",
                    format!(
                        "throw {}: {} at {} Lv{} ({}/{} HP), catch chance {:.2}",
                        c.throws,
                        ["Poke Ball", "Great Ball", "Ultra Ball"][t],
                        species_name(dex, sc.foe.species),
                        sc.foe.level,
                        sc.foe.hp,
                        sc.foe.max_hp,
                        sim::catch_prob(odds)
                    ),
                    json!({"ball": item, "chance": sim::catch_prob(odds), "odds": odds,
                           "hp": sc.foe.hp, "max_hp": sc.foe.max_hp, "throw": c.throws,
                           "species": species_name(dex, sc.foe.species)}),
                );
                return use_item(f, c, sc, battle_ram::POCKET_POKE_BALLS, item, false);
            }
            log.push(
                f.frame(),
                "model-vs-actual",
                format!(
                    "no ball in the bag (master balls {}): fleeing",
                    count(&sc.balls, ITEM_MASTER_BALL)
                ),
                json!({"balls": format!("{:?}", sc.balls)}),
            );
        } else if c.throws == sim::MAX_THROWS {
            log.push(
                f.frame(),
                "catch",
                format!("{} balls missed: giving up", sim::MAX_THROWS),
                json!({"throws": c.throws}),
            );
        }
    }
    // Run (`TryRunFromBattle`, `decompiled/src/battle_main.c`); a failed escape gives a new menu.
    nav_2x2(f, "action cursor on RUN", 3, |e| obs.action_cursor(e, 0))?;
    press_until(f, "RUN chosen", keys::A, 4, 30, |e| {
        !obs.battle_controller_is(e, 0, "HandleInputChooseAction")
    })
}

/// The party menu took input: a send-out after a faint (the sim's fighter never faints and
/// wins; a lost lead is a model-vs-actual), or the SHIFT-style prompt (cancelled).
fn party_choice(f: &mut HumanFeed, c: &mut Ctl, log: &mut EventLog) -> Result<(), RouteError> {
    let dex = c.dex;
    // The menu is open: `gPlayerParty` is in battle order, the mon on the field at its slot.
    let party = c.ram.player_party(f.emu());
    let field_active = c.ram.party_index(f.emu(), 0);
    let active = c.ram.menu_slot_of(f.emu(), field_active);
    let active_alive = party.get(active).is_some_and(|m| m.hp > 0);
    if active_alive {
        // Not forced: SHIFT style's "Will you change?" answered YES by the mash. Back out.
        return press_until(f, "out of the party menu", keys::B, 4, 60, |e| {
            !c.obs.task_active(e, "Task_HandleChooseMonInput")
        });
    }
    f.react()?;
    let pick = party
        .iter()
        .enumerate()
        .filter(|&(i, m)| i != active && m.hp > 0 && !m.is_egg)
        .max_by_key(|&(i, m)| (m.level, usize::MAX - i))
        .map(|(i, _)| i);
    let Some(slot) = pick else {
        return Err(RouteError::Timeout {
            what: "a mon to send out (none alive)".into(),
            budget: 0,
            frames: f.fed(),
        });
    };
    let name = |m: &RawPartyMon| {
        species_index(dex, m.species)
            .map(|s| species_name(dex, s))
            .unwrap_or_default()
    };
    log.push(
        f.frame(),
        "model-vs-actual",
        format!(
            "{} fainted: sending out {} Lv{} (the sim's fighter never faints and wins)",
            name(&party[active]),
            name(&party[slot]),
            party[slot].level
        ),
        json!({"fainted": active, "send": slot}),
    );
    pick_party(f, c, slot)
}

/// The summary screen asks which move to forget for `gMoveToLearn`: [`sim::replace_slot`] picks
/// the slot, or slot 4 (the new move) to give it up.
fn forget_choice(f: &mut HumanFeed, c: &mut Ctl, log: &mut EventLog) -> Result<(), RouteError> {
    let dex = c.dex;
    let ctx = c.cfg.ctx;
    let new = c.ram.move_to_learn(f.emu());
    let party = c.ram.player_party(f.emu());
    let Some(mv) = move_index(dex, new) else {
        return Err(RouteError::Timeout {
            what: format!("move {new} not in the dex"),
            budget: 0,
            frames: f.fed(),
        });
    };
    // The learner: the party mon with four moves whose learnset offers `mv` at its level.
    let learner = party
        .iter()
        .filter_map(|m| species_index(dex, m.species).map(|s| (s, m)))
        .find(|(s, m)| {
            m.moves.iter().all(|&x| x != 0)
                && dex.species[*s]
                    .learnset
                    .iter()
                    .any(|&(l, x)| l == m.level as u32 && x == mv)
        })
        .or_else(|| {
            party
                .first()
                .and_then(|m| species_index(dex, m.species).map(|s| (s, m)))
        });
    let Some((sp, m)) = learner else {
        return Err(RouteError::Timeout {
            what: "the mon learning a move".into(),
            budget: 0,
            frames: f.fed(),
        });
    };
    let Some((slots, _)) = moves_of(dex, &m.moves, &m.pp) else {
        return Err(RouteError::Timeout {
            what: "the learner's moves".into(),
            budget: 0,
            frames: f.fed(),
        });
    };
    let pick = sim::replace_slot(ctx, sp, &slots, mv, false);
    let target = pick.unwrap_or(4) as u8;
    log.push(
        f.frame(),
        "battle",
        match pick {
            Some(i) => format!(
                "{} learns {} over {}",
                species_name(dex, sp),
                move_name(dex, mv),
                move_name(dex, slots[i])
            ),
            None => format!(
                "{} skips {} (worth {:.0} vs the weakest slot)",
                species_name(dex, sp),
                move_name(dex, mv),
                sim::move_utility(ctx, sp, mv)
            ),
        },
        json!({"move": move_name(dex, mv), "slot": pick,
               "utility": slots.iter().map(|&s| sim::move_utility(ctx, sp, s)).collect::<Vec<_>>(),
               "new_utility": sim::move_utility(ctx, sp, mv)}),
    );
    let ram = c.ram.clone();
    let obs = c.obs;
    for _ in 0..8 {
        let cur = ram.summary_move_cursor(f.emu());
        if cur == target {
            break;
        }
        let key = if target > cur { keys::DOWN } else { keys::UP };
        press_until(f, "the forget cursor", key, 6, 20, |e| {
            ram.summary_move_cursor(e) != cur
        })?;
    }
    press_until(f, "the move to forget chosen", keys::A, 6, 60, |e| {
        !obs.task_active(e, "Task_InputHandler_SelectOrForgetMove")
            || obs.callback2_is(e, "BattleMainCB2")
    })
}
