//! One attempt of a plan: run the segments, sampling the human's jitter and the game's
//! randomness, applying the reset rules, and report what it cost.
//!
//! Model constants marked `guess` are uncalibrated (see `human-run/plans/*.md`).

use crate::battle::{Status1, XItem};
use crate::data::{Area, Caught, Dex, Slot};
use crate::params::{Hunt, Params};
use crate::rng::{hash_str, GameRng, Rng};
use crate::route::{Era, Kind, Seg};
use frlg_mon::stats::{calc_stats, exp_for_level, exp_gain, level_from_exp, Growth};

/// Frames of a wild encounter before the first action is possible (transition, text). guess.
const ENC_INTRO: u32 = 220;
/// Running from a wild battle once it is decided. guess.
const FLEE: u32 = 150;
/// One turn of a trainer or wild battle: move menu, animations off, enemy action, text. guess.
const TURN: u32 = 330;
/// Sending out / defeating one enemy mon: faint, exp text, next send-out. guess.
const MON_OVERHEAD: u32 = 450;
/// Trainer battle outro: money text and walking on. guess.
const TRAINER_OUTRO: u32 = 300;
/// Stat screen and move prompts per level gained. guess.
const LEVEL_UP: u32 = 300;
/// Paying at the Safari gate, the Warden's text and walking in. guess.
const SAFARI_ENTRY: u32 = 1500;
/// The Warden calling time and walking out. guess.
const SAFARI_EXIT: u32 = 900;
/// A Poké Ball throw with its shakes. guess.
const THROW: u32 = 420;
/// Dex entry and nickname prompt after a catch. guess.
const CATCH_END: u32 = 380;
/// Whiting out: text, Pokémon Center, walking back. guess.
const WHITEOUT: u32 = 2500;
/// Chance per turn the enemy crits or our move misses, each forcing a decision. guess.
const P_EVENT: f64 = 0.10;

pub const FPS: f64 = 59.7275;

pub struct Ctx<'a> {
    pub dex: &'a Dex,
    pub p: &'a Params,
    pub segs: &'a [Seg],
    /// Best-case frames to finish from just after segment i (no jitter, no events).
    pub floor_after: Vec<u64>,
    /// Mean frames remaining after segment i, from a no-reset pilot (for `reset_aggr`).
    pub mean_after: Vec<f64>,
    /// The starter and its two evolutions.
    pub starter_line: [usize; 3],
    /// Levels at which the starter evolves (`decompiled/src/data/pokemon/evolution.h:3-8`).
    pub evo_at: [u32; 2],
    /// First level with a damaging STAB move (`level_up_learnsets.h`: Bubble/Ember 7, Vine Whip 10).
    pub stab_from: u32,
    /// Reset aggressiveness at each segment (the global value unless overridden).
    pub aggr: Vec<f64>,
    /// Moves the carry never forgets (battle_sim), resolved from `Params::keep_moves`.
    pub keep_ids: Vec<usize>,
    /// Per-segment tactics (battle_sim), resolved from `Params::fight_tactics`:
    /// (X items, heal threshold, sacrificial teammates).
    pub tactics: std::collections::HashMap<String, (Vec<XItem>, f64, u32)>,
    /// Index of Oak's check, and (from the no-reset pilot) the fill time as `base + slope * missing
    /// species`, the mean species still to be caught before Oak after segment i, and the mean remaining
    /// frames after segment i without the fill (for `dex_resets`).
    pub oak_idx: usize,
    pub fill_base: f64,
    pub fill_slope: f64,
    pub gain_after: Vec<f64>,
    pub mean_nofill_after: Vec<f64>,
}

#[derive(Default, Clone)]
pub struct SegStats {
    pub n: u64,
    pub sum: f64,
    pub sumsq: f64,
    pub resets: u64,
    pub whiteouts: u64,
    pub abandons: u64,
    pub level_sum: f64,
    pub caught_sum: f64,
    /// Money model: prize money and balls in the bag after the segment (summed).
    pub money_sum: f64,
    pub balls_sum: f64,
}

/// Optional extra measurements from an attempt.
#[derive(Default)]
pub struct Probe {
    /// Kanto species caught when Oak's check starts.
    pub oak_count: u32,
    /// Per fill hunt: (frames spent, species gained, times run).
    pub fill: Vec<(f64, f64, u64)>,
    /// Kanto species caught after each segment (filled when a probe is passed).
    pub caught_at: Vec<u32>,
}

pub struct Outcome {
    pub wall: u64,
    pub run: u64,
    pub finished: bool,
    pub caught: u32,
    pub whiteouts: u32,
}

struct State {
    clock: u64,
    off: u64,
    exp: u32,
    carry: u32,
    strength: f64,
    power: f64,
    caught: Caught,
    abandoned: bool,
    whiteouts: u32,
    /// The game's RNG stream (re-keyed per segment, advanced per frame by `bump`).
    game: GameRng,
    /// The carry's four move slots, their PP left, and the level up to which level-up
    /// moves were offered.
    slots: Vec<usize>,
    slot_pp: Vec<i32>,
    learned_to: u32,
    /// Status that outlived the last battle (cured by item or Pokémon Center).
    carry_status: Status1,
    /// X items in the bag: Attack, Defend, Speed, Special, Dire Hit.
    x_stock: [u32; 5],
    /// Mewtwo once caught (battle_sim): its rolled IVs and nature, and its move slots.
    mewtwo: Option<([u32; 6], u8)>,
    mewtwo_moves: Vec<usize>,
    /// The first-evolution stat gate was already checked.
    evo_checked: bool,
    /// Living expendable teammates (sacrifices burn them; a Centre revives them).
    fodder: u32,
    /// TM and tutor moves already taught: Gen 3 TMs are single-use and each tutor
    /// teaches once (`FLAG_TUTOR_*`, `data/scripts/move_tutors.inc`); HMs are reusable.
    taught_once: Vec<usize>,
    /// Elixirs and Max Elixirs in the bag.
    elixirs: u32,
    max_elixirs: u32,
    revives: u32,
    candies: u32,
    /// Inside the Safari Zone on the current entry (steps and balls left).
    safari_open: bool,
    safari_steps: u32,
    safari_balls: u32,
    /// Money model: prize money, ball stock (Poké, Great, Ultra) and the best ball a mart on the
    /// route so far sells.
    money: u32,
    balls: [u32; 3],
    max_tier: usize,
    /// Repel steps left, and whether a mart on the route has sold them yet.
    repel_left: u32,
    repel_ok: bool,
    /// Battle simulation: the carry's IVs and nature (rolled at the starter), effort values earned,
    /// HP as a fraction, badges (bit 0 Boulder, 4 Soul, 6 Volcano) and whether Surf is known.
    ivs: [u32; 6],
    nature: u8,
    evs: [u32; 6],
    hp_frac: f64,
    badges: u8,
    league_items: bool,
}

impl Ctx<'_> {
    /// A model constant, overridable through `Params::tune` for sensitivity studies.
    fn k(&self, name: &str, default: f64) -> f64 {
        if self.p.tune.is_empty() {
            default
        } else {
            self.p.tune.get(name).copied().unwrap_or(default)
        }
    }

    /// Frames from the battle starting to the first action menu. Measured on libmgba with A
    /// mashed: 358 (`human-run/calibration.md`), times 1.25 for a human; the older guess was 220.
    fn enc_intro(&self) -> f64 {
        if self.p.calib_enc {
            450.0
        } else {
            ENC_INTRO as f64
        }
    }

    /// One ball thrown and its result back at the menu: 397-416 measured, x1.2.
    fn throw_cost(&self) -> f64 {
        if self.p.calib_enc {
            480.0
        } else {
            THROW as f64
        }
    }

    /// After the last shake: Gotcha text, dex page, nickname prompt. The measured throw-to-overworld
    /// total is 409-549, so what comes after the throw cycle is small.
    fn catch_end(&self) -> f64 {
        let base = if self.p.calib_enc {
            150.0
        } else {
            CATCH_END as f64 * self.p.text_factor
        };
        // The caught-jingle wait spans catch_end and the tail of the throw animation; the
        // measured whole-catch saving is credited here (clamped so the segment stays positive).
        if self.p.clemi_skip {
            (base - 140.0).max(10.0)
        } else {
            base
        }
    }

    /// One battle turn. `calib_battle`: 385 = the 280-406 frame floor measured on libmgba
    /// (mean ~320 with A mashed) times 1.2 (`human-run/calibration.md`).
    fn turn(&self) -> f64 {
        if self.p.calib_battle {
            385.0
        } else {
            TURN as f64 * self.p.battle_scale
        }
    }

    /// Faint, exp text and the next send-out per enemy mon beyond the turn itself. `calib_battle`:
    /// 810 = (975 measured wild kill cycle - 320 turn) x 1.2.
    fn mon_overhead(&self) -> f64 {
        if self.p.calib_battle {
            810.0
        } else {
            MON_OVERHEAD as f64 * self.p.battle_scale
        }
    }

    /// Frames from a trainer battle starting to its first menu. `calib_battle`: at least
    /// 1250 = the 1040 frames measured for the lab rival x 1.2.
    fn trainer_intro(&self, intro: u32) -> f64 {
        let t = intro as f64 * self.p.text_factor;
        if self.p.calib_battle {
            t.max(1250.0)
        } else {
            t
        }
    }

    pub fn floor_frames(&self, seg: &Seg) -> f64 {
        let p = self.p;
        match &seg.kind {
            Kind::Text { frames } => *frames as f64 * p.text_factor,
            Kind::Walk { tiles, era, .. } => {
                *tiles as f64 * era.frames_per_tile() * p.path_overhead
            }
            Kind::Trainer { party, intro, .. } => {
                self.trainer_intro(*intro)
                    + party.len() as f64 * (self.turn() + self.mon_overhead())
                    + self.k("trainer_outro", TRAINER_OUTRO as f64)
            }
            Kind::Parties { parties, .. } => parties
                .iter()
                .map(|p| {
                    self.trainer_intro(600)
                        + p.len() as f64 * (self.turn() + self.mon_overhead())
                        + self.k("trainer_outro", TRAINER_OUTRO as f64)
                })
                .sum(),
            Kind::Group { n, mons, .. } => {
                *n as f64
                    * (self.trainer_intro(600)
                        + *mons as f64 * (self.turn() + self.mon_overhead())
                        + self.k("trainer_outro", TRAINER_OUTRO as f64))
            }
            Kind::Wander { .. }
            | Kind::Hunt { .. }
            | Kind::OakCheck
            | Kind::Stones
            | Kind::Mewtwo => 0.0,
            Kind::Special { .. } | Kind::Mart { .. } => 0.0,
            Kind::Teach { teach } => teach.frames as f64 * self.p.text_factor,
            Kind::XBuy { xbuy } => xbuy.frames as f64 * self.p.text_factor,
            Kind::Elixir { pickup } => pickup.frames as f64 * self.p.text_factor,
            Kind::TeleportOrWalk { frames, .. } => *frames as f64 * self.p.text_factor,
            Kind::Grind { .. } => 0.0,
        }
    }
}

fn bump(st: &mut State, f: f64) {
    let n = f.round() as u64;
    st.clock += n;
    // The VBlank advance (`decompiled/src/main.c:412`): time moves the game's stream.
    st.game.frames(n.min(u32::MAX as u64) as u32);
}

fn jitter(ctx: &Ctx, rng: &mut Rng, inputs: u32) -> u32 {
    let n = ctx.p.jitter_n;
    if n == 0 {
        return 0;
    }
    (0..inputs).map(|_| rng.below(n + 1)).sum()
}

/// Mistakes over `units` hundreds of tiles or inputs.
fn mistakes(ctx: &Ctx, rng: &mut Rng, units: f64) -> f64 {
    let rate = ctx.p.mistake_rate;
    if rate <= 0.0 {
        return 0.0;
    }
    let p = (rate / 100.0).min(0.5);
    let n = (units * 100.0) as u64;
    let mut total = 0.0;
    let mut left = n;
    loop {
        let gap = rng.geometric(p);
        if gap >= left {
            break;
        }
        left -= gap + 1;
        total += rng.range(60, 300) as f64 + react(ctx, rng) as f64;
    }
    total
}

fn react(ctx: &Ctx, rng: &mut Rng) -> u32 {
    rng.range(ctx.p.react_lo, ctx.p.react_hi)
}

fn effective_level(st: &State, p: &Params) -> f64 {
    (st.carry as f64 * p.team_bonus).max(st.power)
}

/// A move slot's worth when something has to go: damage against a neutral dummy with
/// STAB for the current stage; keep-listed moves are effectively permanent.
fn slot_utility(ctx: &Ctx, st: &State, mv: usize) -> f64 {
    if let Some(pos) = ctx.keep_ids.iter().position(|&k| k == mv) {
        return 1_000_000.0 - pos as f64;
    }
    let m = &ctx.dex.moves[mv];
    if m.power == 0 {
        return 0.0;
    }
    let types = ctx.dex.species[carry_species(ctx, st.carry)].types;
    let stab = if types.0 == m.mtype || types.1 == m.mtype {
        1.5
    } else {
        1.0
    };
    let acc = if m.acc == 0 { 100 } else { m.acc } as f64 / 100.0;
    m.power as f64 * stab * acc
}

/// Learn a move into the four slots: fill a free slot, else replace the lowest-utility
/// slot if the newcomer is worth more (`forced` teaches always find a victim).
fn offer_move(ctx: &Ctx, st: &mut State, mv: usize, forced: bool) {
    if st.slots.contains(&mv) {
        return;
    }
    if st.slots.len() < 4 {
        st.slots.push(mv);
        st.slot_pp.push(ctx.dex.moves[mv].pp as i32);
        return;
    }
    let (worst_i, worst_u) = st
        .slots
        .iter()
        .enumerate()
        .map(|(i, &s)| (i, slot_utility(ctx, st, s)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .unwrap();
    let new_u = slot_utility(ctx, st, mv);
    if forced || new_u > worst_u {
        st.slots[worst_i] = mv;
        st.slot_pp[worst_i] = ctx.dex.moves[mv].pp as i32;
    }
}

/// Offer every level-up move between the last processed level and the current one,
/// using the learnset of the stage the carry had at each level.
fn learn_moves(ctx: &Ctx, st: &mut State) {
    while st.learned_to < st.carry {
        st.learned_to += 1;
        let l = st.learned_to;
        let sp = carry_species(ctx, l);
        let learn: Vec<usize> = ctx.dex.species[sp]
            .learnset
            .iter()
            .filter(|&&(ll, _)| ll == l)
            .map(|&(_, m)| m)
            .collect();
        for m in learn {
            offer_move(ctx, st, m, false);
        }
    }
}

fn add_exp(ctx: &Ctx, st: &mut State, e: u32) -> u32 {
    st.exp = st.exp.saturating_add(e);
    let new = level_from_exp(Growth::MediumSlow, st.exp) as u32;
    let gained = new.saturating_sub(st.carry);
    st.carry = new.max(st.carry);
    let [_sq, war, bla] = ctx.starter_line;
    if st.carry >= ctx.evo_at[0] {
        st.caught.insert(ctx.dex.species[war].dex);
    }
    if st.carry >= ctx.evo_at[1] {
        st.caught.insert(ctx.dex.species[bla].dex);
    }
    if gained > 0 {
        learn_moves(ctx, st);
    }
    gained
}

/// Weighted slot pick (weights sum to 100).
fn pick_slot(rng: &mut Rng, weights: &[u32]) -> usize {
    let total: u32 = weights.iter().sum();
    let mut r = rng.below(total);
    for (i, w) in weights.iter().enumerate() {
        if r < *w {
            return i;
        }
        r -= w;
    }
    weights.len() - 1
}

fn isqrt(n: u32) -> u32 {
    (n as f64).sqrt() as u32
}

fn ball_mult(p: &Params) -> u32 {
    match p.ball.as_str() {
        "great" => 15,
        "ultra" => 20,
        _ => 10,
    }
}

/// The four shake checks of `Cmd_handleballthrow`
/// (`decompiled/src/battle_script_commands.c:9549-9640`), on the game's own stream.
fn shakes_catch(g: &mut GameRng, odds: u32) -> bool {
    if odds > 254 {
        return true;
    }
    let o = odds.max(1);
    let s = isqrt(isqrt(16_711_680 / o)).max(1);
    let thr = 1_048_560 / s;
    (0..4).all(|_| (g.roll() as u32) < thr)
}

/// The status bonus of the same function: sleep/freeze double the odds, any other
/// status is x1.5 (`battle_script_commands.c:9553-9556`).
fn status_odds(odds: u32, status: Status1) -> u32 {
    match status {
        Status1::Sleep(_) | Status1::Freeze => odds * 2,
        Status1::None => odds,
        _ => odds * 15 / 10,
    }
}

/// Safari Zone encounter: `safari_rocks` rocks first, then Safari Balls until caught, out of
/// balls or the mon flees. A rock doubles the catch factor and the flee factor for 2-6 turns (the
/// counter runs down once per enemy turn, then the catch factor resets), a ball uses the current
/// factor (`decompiled/src/battle_main.c:4339-4408`, `battle_ai_script_commands.c:1715-1732`,
/// `battle_script_commands.c:9497`, ball multiplier 15 at `:808-814`).
fn safari_catch(ctx: &Ctx, st: &mut State, rng: &mut Rng, sp: &crate::data::Species) -> bool {
    let base = sp.catch_rate * 100 / 1275;
    let mut factor = base;
    let esc = (sp.safari_flee * 100 / 1275).max(2);
    let mut counter = 0u32;
    let mut bait = 0u32;
    let mut rocks = ctx.p.safari_rocks;
    let mut bait_pending = ctx.p.safari_bait_from > 0 && esc >= ctx.p.safari_bait_from;
    loop {
        if bait_pending {
            bait_pending = false;
            bait = (bait + rng.range(2, 6)).min(6);
            counter = 0;
            bump(st, THROW as f64 * 0.6);
            factor = (factor >> 1).max(3);
        } else if rocks > 0 {
            rocks -= 1;
            bump(st, THROW as f64 * 0.6);
            counter = (counter + rng.range(2, 6)).min(6);
            factor = (factor * 2).min(20);
        } else {
            if st.safari_balls == 0 {
                return false;
            }
            st.safari_balls -= 1;
            bump(st, ctx.throw_cost());
            let rate = (factor * 1275 / 100).min(255);
            let odds = (rate * 15 / 10) * 100 / 300;
            let mut g = st.game;
            let caught = shakes_catch(&mut g, odds);
            st.game = g;
            if caught {
                bump(st, ctx.catch_end());
                return true;
            }
            bump(st, react(ctx, rng) as f64);
        }
        let flee_factor = if counter > 0 {
            (esc * 2).min(20)
        } else if bait > 0 {
            (esc / 4).max(1)
        } else {
            esc
        };
        if rng.below(100) < flee_factor * 5 {
            bump(st, FLEE as f64);
            return false;
        }
        if counter > 0 {
            counter -= 1;
            if counter == 0 {
                factor = base;
            }
        } else {
            bait = bait.saturating_sub(1);
        }
    }
}

/// Mart prices, `decompiled/src/data/items.json`.
const BALL_PRICE: [u32; 3] = [200, 600, 1200];
const BALL_MULT: [u32; 3] = [10, 15, 20];
/// Walking into a mart, buying, walking out. guess.
const MART: u32 = 900;

/// The ball for the next throw under the money model: the best type the plan allows and a
/// mart on the route has sold so far, bought in batches of `ball_batch` as money allows, else a
/// cheaper one. `None` when not even a Poké Ball is affordable.
fn take_ball(ctx: &Ctx, st: &mut State) -> Option<u32> {
    let mut t = match ctx.p.ball.as_str() {
        "great" => 1,
        "ultra" => 2,
        _ => 0,
    }
    .min(st.max_tier);
    loop {
        if st.balls[t] > 0 {
            st.balls[t] -= 1;
            return Some(BALL_MULT[t]);
        }
        if !ctx.p.marts.is_empty() {
            // Scheduled mart stops only: use whatever is in the bag.
            if t == 0 {
                return None;
            }
            t -= 1;
            continue;
        }
        let qty = (st.money / BALL_PRICE[t]).min(ctx.p.ball_batch);
        if qty > 0 {
            st.money -= qty * BALL_PRICE[t];
            st.balls[t] += qty;
            bump(st, MART as f64 * ctx.p.text_factor);
            continue;
        }
        if t == 0 {
            return None;
        }
        t -= 1;
    }
}

/// Throws until caught or patience (8 throws) runs out. Formula:
/// `decompiled/src/battle_script_commands.c:9549-9640`, status bonus included.
#[allow(clippy::too_many_arguments)]
fn throw_balls(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    catch_rate: u32,
    hp_frac: f64,
    status: Status1,
    ball: Option<u32>,
    max_throws: u32,
) -> bool {
    let max_hp = 100u32;
    let hp = ((hp_frac * max_hp as f64) as u32).max(1);
    for _ in 0..max_throws {
        let b = match ball {
            Some(b) => b,
            None => match take_ball(ctx, st) {
                Some(b) => b,
                None => return false,
            },
        };
        let odds = status_odds(
            (catch_rate * b / 10) * (max_hp * 3 - hp * 2) / (3 * max_hp),
            status,
        );
        bump(st, ctx.throw_cost());
        let mut g = st.game;
        let caught = shakes_catch(&mut g, odds);
        st.game = g;
        if caught {
            bump(st, ctx.catch_end());
            return true;
        }
        bump(st, react(ctx, rng) as f64);
    }
    false
}

#[allow(clippy::too_many_arguments)]
/// One wild encounter on arrival: catch if it is a new Kanto species the human wants, else
/// flee or fight. `skip` lists species left alone.
fn encounter(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    slot: &Slot,
    level: u32,
    skip: &[usize],
    want_catch: bool,
    safari: bool,
) {
    let sp = &ctx.dex.species[slot.species];
    bump(st, ctx.enc_intro() + react(ctx, rng) as f64);
    let wanted =
        want_catch && sp.dex != 0 && !st.caught.has(sp.dex) && !skip.contains(&slot.species);
    if wanted && safari {
        if safari_catch(ctx, st, rng, sp) {
            st.caught.insert(sp.dex);
        }
    } else if wanted {
        let mut hp = 1.0;
        if ctx.p.weaken {
            let turns = 1 + rng.chance(0.5) as u32;
            bump(st, turns as f64 * ctx.turn());
            if rng.chance(ctx.k("weaken_ko", 0.05) * turns as f64) {
                return;
            }
            hp = ctx.k("weaken_hp", 0.3);
        }
        let ball = (!ctx.p.money_model).then(|| ball_mult(ctx.p));
        if throw_balls(ctx, st, rng, sp.catch_rate, hp, Status1::None, ball, 8) {
            st.caught.insert(sp.dex);
        }
    } else if (ctx.p.fight_wild || st.carry < ctx.p.fight_until_level) && ctx.p.battle_sim {
        let foe = wild_mon(ctx, st, slot.species, level);
        // A lost wild fight is a whiteout like any other; the human does not give up the run over it.
        sim_battle(
            ctx,
            st,
            rng,
            &BattleArgs {
                name: "wild",
                foes: &[foe],
                trainer: false,
                intro: 0,
                ai: 0,
                heals: 0,
                yield_: None,
                league: false,
                wild: true,
            },
        );
    } else if ctx.p.fight_wild || st.carry < ctx.p.fight_until_level {
        let e = exp_gain(sp.exp_yield as u8, level as u8, false);
        let t = turns_per_mon(level as f64, effective_level(st, ctx.p), st.strength, 1.0);
        bump(st, t * ctx.turn() + ctx.mon_overhead());
        let g = add_exp(ctx, st, e);
        bump(st, g as f64 * ctx.k("level_up", LEVEL_UP as f64));
    } else {
        flee(ctx, st, rng);
    }
}

/// Running from a wild battle. Measured on libmgba with a level-6 starter in Viridian Forest:
/// a successful Run costs 400-1013 frames from the menu (mean 598 with retries), a failed one
/// 277-585 (mean 374); 10 of 54 attempts worked. The escape formula guarantees an escape when the
/// player is faster, which a carry past level 15 nearly always is vs a wild mon of the area
/// (`decompiled/src/battle_main.c`, `TryRunFromBattle`). guess for the 15 and the 0.2.
fn flee(ctx: &Ctx, st: &mut State, rng: &mut Rng) {
    if !ctx.p.calib_enc {
        bump(st, FLEE as f64);
        return;
    }
    let p_ok = if st.carry < 15 { 0.2 } else { 1.0 };
    loop {
        if rng.chance(p_ok) {
            bump(st, 500.0);
            return;
        }
        bump(st, 470.0);
    }
}

fn turns_per_mon(le: f64, lp: f64, strength: f64, adv: f64) -> f64 {
    ((1.0 + 3.0 * (le / lp - 0.7).max(0.0)) / (strength * adv)).max(1.0)
}

/// A wild mon with the game's generation: nature from the PID (`Random32() % 25`),
/// IVs from two 16-bit rolls in 5-bit fields (`decompiled/src/pokemon.c`, `CreateBoxMon`:
/// personality at `:1793`, IVs at `:1834-1853`), default level-up moves.
fn wild_mon(ctx: &Ctx, st: &mut State, species: usize, level: u32) -> crate::battle::Mon {
    let mut g = st.game;
    let pid = (g.roll() as u32) | ((g.roll() as u32) << 16);
    let iv1 = g.roll() as u32;
    let iv2 = g.roll() as u32;
    st.game = g;
    let ivs = [
        iv1 & 0x1F,
        (iv1 >> 5) & 0x1F,
        (iv1 >> 10) & 0x1F,
        iv2 & 0x1F,
        (iv2 >> 5) & 0x1F,
        (iv2 >> 10) & 0x1F,
    ];
    let nature = (pid % 25) as u8;
    let mut foe = crate::battle::make_mon(ctx.dex, species, level, ivs, [0; 6], nature, 0);
    foe.moves = ctx.dex.default_moves(species, level);
    foe
}

/// Opening the bag and using a Repel. guess.
const REPEL_USE: f64 = 350.0;

/// Tiles of a walk covered by Repels, using new ones as the old run out. Pays for them when the
/// money model is on; stops when it cannot.
fn use_repels(ctx: &Ctx, st: &mut State, seg: &Seg, tiles: u32) -> u32 {
    if !st.repel_ok || !ctx.p.repel_on.contains(&seg.name) {
        return 0;
    }
    let (mut covered, mut left) = (0, tiles);
    while left > 0 {
        if st.repel_left == 0 {
            let (dur, price) = match st.max_tier {
                0 => (100, 350),
                1 => (200, 500),
                _ => (250, 700),
            };
            if ctx.p.money_model {
                if st.money < price {
                    break;
                }
                st.money -= price;
            }
            bump(st, REPEL_USE * ctx.p.text_factor);
            st.repel_left = dur;
        }
        let take = left.min(st.repel_left);
        st.repel_left -= take;
        left -= take;
        covered += take;
    }
    covered
}

struct EncArgs<'a> {
    site: &'a str,
    area: Area,
    steps: u32,
    era: Era,
    skip: &'a [usize],
    stop_when_done: bool,
    /// A Repel is active: encounters below the lead's level never start.
    repel: bool,
}

fn run_encounters(ctx: &Ctx, st: &mut State, rng: &mut Rng, a: &EncArgs) -> u32 {
    let EncArgs {
        site,
        area,
        steps,
        era,
        skip,
        stop_when_done,
        repel,
    } = *a;
    let Some((rate, slots, weights)) = ctx.dex.area(site, area, ctx.p.version == "LG", &ctx.p.rod)
    else {
        return 0;
    };
    let mut p = rate as f64 * 16.0 / 1600.0;
    if era == Era::Bike {
        p *= 0.8;
    }
    let mut left = steps as u64;
    let mut used = 0u32;
    loop {
        if stop_when_done
            && slots.iter().all(|s| {
                let sp = &ctx.dex.species[s.species];
                sp.dex == 0 || st.caught.has(sp.dex) || skip.contains(&s.species)
            })
        {
            break;
        }
        let gap = rng.geometric(p);
        if gap >= left {
            used += left as u32;
            break;
        }
        left -= gap + 1;
        used += gap as u32 + 1;
        let slot = &slots[pick_slot(rng, &weights)];
        let level = rng.range(slot.min, slot.max);
        if repel && level < st.carry {
            continue;
        }
        encounter(
            ctx,
            st,
            rng,
            slot,
            level,
            skip,
            stop_when_done
                || (ctx.p.catch_on_pass
                    && ctx.dex.species[slot.species].catch_rate >= ctx.p.pass_min_catch),
            ctx.p.safari_model && site.starts_with("MAP_SAFARI_ZONE"),
        );
        if stop_when_done && left == 0 {
            break;
        }
    }
    used
}

/// Type-matchup multiplier on the fighter's effective level against one enemy species: its best
/// damaging type against the enemy over a neutral baseline, divided by the root of the enemy's
/// best STAB against it. Move power, accuracy and AI choices are ignored. guess.
fn matchup(ctx: &Ctx, st: &State, enemy: usize) -> f64 {
    if ctx.p.type_strength == 0.0 {
        return 1.0;
    }
    let e = ctx.dex.species[enemy].types;
    let dex = ctx.dex;
    let (mine, off, neutral) = if st.power > st.carry as f64 * ctx.p.team_bonus {
        // Mewtwo: Psychic STAB plus TM coverage.
        let off = (1.5 * dex.eff(14, e))
            .max(dex.eff(15, e))
            .max(dex.eff(13, e))
            .max(dex.eff(10, e));
        ((14, 14), off, 1.5)
    } else {
        let stage = (st.carry >= ctx.evo_at[0]) as usize + (st.carry >= ctx.evo_at[1]) as usize;
        let mine = dex.species[ctx.starter_line[stage]].types;
        let normal = dex.eff(0, e);
        if st.carry >= ctx.stab_from {
            let mut stab = dex.eff(mine.0, e);
            if stage > 0 {
                stab = stab.max(dex.eff(mine.1, e));
            }
            ((mine.0, mine.1), (1.5 * stab).max(normal), 1.5)
        } else {
            (mine, normal, 1.0)
        }
    };
    let inc = dex.eff(e.0, mine).max(dex.eff(e.1, mine)).clamp(0.5, 2.0);
    ((off / neutral).clamp(0.5, 2.0) / inc.sqrt())
        .clamp(0.5, 2.0)
        .powf(ctx.p.type_strength)
        .max(ctx.p.type_floor)
}

fn enemy_list(ctx: &Ctx, st: &State, party: &[(u32, usize)]) -> Vec<(u32, u32, f64)> {
    party
        .iter()
        .map(|&(l, s)| (l, ctx.dex.species[s].exp_yield, matchup(ctx, st, s)))
        .collect()
}

fn p_lose(r: f64, center: f64, slope: f64) -> f64 {
    1.0 / (1.0 + (-(r - center) / slope).exp())
}

/// A set of enemy mons fought in sequence by the best fighter. Returns false if the human
/// gives up (whited out too often).
fn battle(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    enemies: &[(u32, u32, f64)],
    trainer: bool,
    intro: u32,
    adv: f64,
) -> bool {
    let lp0 = effective_level(st, ctx.p);
    let lp_of = |a: f64| lp0 * (a * adv).sqrt();
    let mut r = enemies
        .iter()
        .map(|e| e.0 as f64 / lp_of(e.2))
        .fold(0.0, f64::max);
    if r > 0.9 && st.power <= st.carry as f64 * ctx.p.team_bonus && st.carry >= ctx.stab_from {
        r /= 1.0 + ctx.p.ability_bonus;
    }
    let mut tries = 0;
    loop {
        let start = st.clock;
        bump(st, ctx.trainer_intro(intro));
        bump(st, jitter(ctx, rng, 8) as f64);
        let mut total_exp = 0u32;
        for &(lvl, yld, a) in enemies {
            let mean = turns_per_mon(lvl as f64, lp_of(a), st.strength, 1.0);
            let mut turns = mean.floor() + rng.chance(mean - mean.floor()) as u32 as f64;
            for _ in 0..turns as u32 {
                if rng.chance(ctx.k("p_event", P_EVENT)) {
                    bump(st, react(ctx, rng) as f64);
                    if rng.chance(0.5) {
                        turns += 1.0;
                    }
                }
            }
            bump(st, turns * ctx.turn() + ctx.mon_overhead());
            bump(st, jitter(ctx, rng, turns as u32 * 2 + 3) as f64);
            total_exp += exp_gain(yld as u8, lvl as u8, trainer);
        }
        let heal_turns = ((r - 0.95).max(0.0) * enemies.len() as f64 * 2.0).round();
        bump(st, heal_turns * ctx.turn());
        if trainer {
            bump(st, ctx.k("trainer_outro", TRAINER_OUTRO as f64));
        }
        let danger = p_lose(r, ctx.k("p_lose_center", 1.25), ctx.k("p_lose_slope", 0.05))
            * 0.5f64.powi(tries);
        if rng.chance(danger) {
            let spent = st.clock - start;
            st.whiteouts += 1;
            bump(st, ctx.k("whiteout", WHITEOUT as f64) + spent as f64 * 0.0);
            tries += 1;
            if tries >= 3 {
                st.abandoned = true;
                return false;
            }
            continue;
        }
        let g = add_exp(ctx, st, total_exp);
        bump(st, g as f64 * ctx.k("level_up", LEVEL_UP as f64));
        return true;
    }
}

/// A TM/HM or tutor move the species can be taught (`tmhm_learnsets.h`,
/// `tutor_learnsets.h`).
fn can_learn(ctx: &Ctx, species: usize, mv: usize) -> bool {
    let sp = &ctx.dex.species[species];
    sp.tmhm.contains(&mv) || sp.tutor.contains(&mv)
}

/// The carry's species at `level` (`starter_line`, evolving at `evo_at`).
fn carry_species(ctx: &Ctx, level: u32) -> usize {
    let stage = (level >= ctx.evo_at[0]) as usize + (level >= ctx.evo_at[1]) as usize;
    ctx.starter_line[stage]
}

/// One `battle_sim` fight, described.
struct BattleArgs<'a> {
    /// Segment name, for the per-fight tactics lookup.
    name: &'a str,
    foes: &'a [crate::battle::Mon],
    trainer: bool,
    intro: u32,
    /// The trainer's AI bits and heal items (`trainers.h`).
    ai: u32,
    heals: u32,
    /// Exp-yield override (the guessed trainer groups).
    yield_: Option<u32>,
    league: bool,
    wild: bool,
}

/// `battle_sim` fight against `a.foes` with the exact engine. Returns false when the
/// human gives up (three whiteouts, or one in the League where a loss means walking the
/// rooms again).
fn sim_battle(ctx: &Ctx, st: &mut State, rng: &mut Rng, a: &BattleArgs) -> bool {
    use crate::battle::{fight, heal_amount, make_mon, Status1, Tactics};
    let dex = ctx.dex;
    if a.trainer {
        bump(st, ctx.trainer_intro(a.intro));
        bump(st, jitter(ctx, rng, 8) as f64);
    }
    let (x_plan, heal_override, sacs) = ctx
        .tactics
        .get(a.name)
        .map(|(x, h, s)| (x.clone(), *h, *s))
        .unwrap_or((vec![], -1.0, 0));
    // Sacrifices burn real party members; the budget spans a Centre-less chain, with
    // Revives (bag trip each) bringing fodder back between fights.
    while st.fodder < sacs && st.revives > 0 {
        st.revives -= 1;
        st.fodder += 1;
        bump(st, 450.0 * ctx.p.text_factor);
    }
    let sacs = sacs.min(st.fodder);
    // Low on attacking PP before a trainer fight: drink an Elixir (bag trip ~500f).
    // Before a League fight any empty slot is worth topping up — the next Centre is
    // several fights away.
    let pp_low = st.slot_pp.iter().sum::<i32>() < 12 || (a.league && st.slot_pp.contains(&0));
    if a.trainer && pp_low {
        if st.max_elixirs > 0 {
            st.max_elixirs -= 1;
            bump(st, 500.0 * ctx.p.text_factor);
            for (k, &m) in st.slots.clone().iter().enumerate() {
                st.slot_pp[k] = ctx.dex.moves[m].pp as i32;
            }
        } else if st.elixirs > 0 {
            st.elixirs -= 1;
            bump(st, 500.0 * ctx.p.text_factor);
            for (k, &m) in st.slots.clone().iter().enumerate() {
                st.slot_pp[k] = (st.slot_pp[k] + 10).min(ctx.dex.moves[m].pp as i32);
            }
        }
    }
    let heal_below = if heal_override >= 0.0 {
        heal_override
    } else {
        ctx.p.heal_below
    };
    let mut tries = 0;
    loop {
        // Mewtwo takes over once it is caught; otherwise the starter line carries.
        let mewtwo_now = ctx.p.mewtwo && st.mewtwo.is_some() && !a.wild;
        let mut me = if let (true, Some((ivs, nature))) = (mewtwo_now, st.mewtwo) {
            let mut m = make_mon(
                dex,
                dex.id("SPECIES_MEWTWO"),
                70,
                ivs,
                [0; 6],
                nature,
                st.badges,
            );
            m.moves = st.mewtwo_moves.clone();
            m
        } else {
            let mut m = make_mon(
                dex,
                carry_species(ctx, st.carry),
                st.carry,
                st.ivs,
                st.evs,
                st.nature,
                st.badges,
            );
            m.moves = st.slots.clone();
            m.pp = st.slot_pp.clone();
            m.status = st.carry_status;
            m
        };
        // Heal with items before the fight when low (each use is a bag trip).
        if st.hp_frac < 0.5 && !mewtwo_now {
            let uses = (((0.85 - st.hp_frac) * me.max_hp as f64)
                / heal_amount(me.level, st.league_items) as f64)
                .ceil();
            bump(st, uses * ctx.turn());
            st.hp_frac = 0.85;
        }
        me.hp = ((me.max_hp as f64 * if mewtwo_now { 1.0 } else { st.hp_frac }).round() as i32)
            .clamp(1, me.max_hp);
        // X items limited by the bag.
        let mut stock = st.x_stock;
        let xs: Vec<XItem> = x_plan
            .iter()
            .filter(|&&x| {
                let i = x_index(x);
                if stock[i] > 0 {
                    stock[i] -= 1;
                    true
                } else {
                    false
                }
            })
            .copied()
            .collect();
        let tac = Tactics {
            x_items: xs.clone(),
            heal_below,
            heal: heal_amount(me.level, st.league_items),
            full_restore: st.league_items,
            sacs,
        };
        let mut g = st.game;
        let out = fight(dex, &mut me, &tac, a.foes, a.ai, a.heals, a.wild, &mut g);
        st.game = g;
        if a.trainer && std::env::var_os("FRLG_SIM_TRACE").is_some() {
            eprintln!(
                "fight {} L{} hp{:.2} x{:?} sacs{} pp{:?} moves{:?} -> won {} turns {} x_used {} sacs_used {}",
                a.name, me.level, st.hp_frac, stock, sacs, me.pp,
                me.moves.iter().map(|&m| dex.moves[m].name.trim_start_matches("MOVE_").to_string()).collect::<Vec<_>>(),
                out.won, out.turns, out.x_used, out.sacs_used
            );
        }
        for x in xs.iter().take(out.x_used as usize) {
            st.x_stock[x_index(*x)] -= 1;
        }
        bump(st, out.turns as f64 * ctx.turn());
        bump(st, out.kos.len() as f64 * ctx.mon_overhead());
        // A sacrifice costs its faint animation and the send-outs both ways. guess.
        bump(st, out.sacs_used as f64 * 900.0);
        st.fodder = st.fodder.saturating_sub(out.sacs_used);
        for _ in 0..out.events {
            bump(st, react(ctx, rng) as f64);
        }
        bump(st, jitter(ctx, rng, out.turns * 2 + 3) as f64);
        if out.won {
            st.hp_frac = out.hp_frac;
            if !mewtwo_now {
                st.carry_status = out.end_status;
                st.slot_pp = me.pp.clone();
                cure_status(ctx, st);
            }
            for &(sp, lvl) in &out.kos {
                let yld = a.yield_.unwrap_or(dex.species[sp].exp_yield);
                if !mewtwo_now {
                    let e = exp_gain(yld as u8, lvl as u8, a.trainer);
                    let g = add_exp(ctx, st, e);
                    bump(st, g as f64 * ctx.k("level_up", LEVEL_UP as f64));
                    let ev = dex.species[sp].ev_yield;
                    for (e, y) in st.evs.iter_mut().zip(ev) {
                        *e = (*e + y).min(255);
                    }
                }
            }
            if a.trainer {
                bump(st, ctx.k("trainer_outro", TRAINER_OUTRO as f64));
            }
            return true;
        }
        // A wild battle the engine ended without a win and without fainting is a flee
        // (a wild Abra's Teleport): carry on.
        if a.wild && out.hp_frac > 0.0 {
            st.hp_frac = out.hp_frac;
            st.carry_status = out.end_status;
            cure_status(ctx, st);
            return true;
        }
        st.whiteouts += 1;
        bump(st, ctx.k("whiteout", WHITEOUT as f64));
        st.hp_frac = 1.0;
        st.carry_status = Status1::None;
        // A whiteout lands at a Pokémon Center: PP comes back too.
        for (k, &m) in st.slots.clone().iter().enumerate() {
            st.slot_pp[k] = ctx.dex.moves[m].pp as i32;
        }
        tries += 1;
        if a.wild {
            // The wild mon is gone; the run continues.
            return true;
        }
        // A whiteout in the League sends the player back to the first room: the human resets.
        if tries >= 3 || a.league {
            st.abandoned = true;
            return false;
        }
    }
}

fn x_index(x: XItem) -> usize {
    match x {
        XItem::Attack => 0,
        XItem::Defend => 1,
        XItem::Speed => 2,
        XItem::Special => 3,
        XItem::DireHit => 4,
    }
}

/// Cure a status that outlived a battle with the matching item: frames and (money model)
/// the mart price (`items.json`: Antidote 100, Paralyze Heal 200, Awakening 250, Burn
/// Heal 250, Ice Heal 250). Poison that stays would otherwise hurt in the overworld.
fn cure_status(ctx: &Ctx, st: &mut State) {
    use crate::battle::Status1;
    if !ctx.p.cure_status || !st.carry_status.any() {
        return;
    }
    let price = match st.carry_status {
        Status1::Poison | Status1::Toxic(_) => 100,
        Status1::Paralysis => 200,
        _ => 250,
    };
    if ctx.p.money_model {
        if st.money < price {
            return; // live with it until the next Pokémon Center
        }
        st.money -= price;
    }
    bump(st, 400.0 * ctx.p.text_factor);
    st.carry_status = Status1::None;
}

fn is_league(name: &str) -> bool {
    let n = name.trim_end_matches("-2");
    matches!(n, "lorelei" | "bruno" | "agatha" | "lance" | "champion")
}

fn group_battle(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    n: u32,
    mons: u32,
    level: u32,
    yld: u32,
) -> bool {
    for _ in 0..n {
        if ctx.p.battle_sim {
            let foes: Vec<crate::battle::Mon> = (0..mons)
                .map(|_| crate::battle::generic_mon(ctx.dex, level))
                .collect();
            let args = BattleArgs {
                name: "group",
                foes: &foes,
                trainer: true,
                intro: 600,
                ai: 1,
                heals: 0,
                yield_: Some(yld),
                league: false,
                wild: false,
            };
            if !sim_battle(ctx, st, rng, &args) {
                return false;
            }
            continue;
        }
        let enemies: Vec<(u32, u32, f64)> = (0..mons).map(|_| (level, yld, 1.0)).collect();
        if !battle(ctx, st, rng, &enemies, true, 600, 1.0) {
            return false;
        }
    }
    true
}

fn hunt(ctx: &Ctx, st: &mut State, rng: &mut Rng, h: &Hunt, era: Era) {
    let area = match h.area.as_str() {
        "water" => Area::Water,
        "fish" => Area::Fish,
        _ => Area::Land,
    };
    let mut skip: Vec<usize> = h.skip.iter().map(|s| ctx.dex.id(s)).collect();
    if ctx.p.hunt_min_catch > 0 {
        skip.extend(
            (0..ctx.dex.species.len())
                .filter(|&i| ctx.dex.species[i].catch_rate < ctx.p.hunt_min_catch),
        );
    }
    let safari = ctx.p.safari_model && h.site.starts_with("MAP_SAFARI_ZONE");
    let mut steps = h.steps;
    if !safari {
        st.safari_open = false;
    } else {
        if !st.safari_open {
            if ctx.p.money_model {
                // 500 at the gate (`decompiled/data/maps/FuchsiaCity_SafariZone_Entrance/scripts.inc`).
                if st.money < 500 {
                    return;
                }
                st.money -= 500;
            }
            bump(st, SAFARI_ENTRY as f64 * ctx.p.text_factor);
            st.safari_open = true;
            st.safari_steps = 600;
            st.safari_balls = 30;
        }
        st.safari_steps = st.safari_steps.saturating_sub(h.detour_tiles * 2);
        if area == Area::Land {
            steps = steps.min(st.safari_steps);
        }
    }
    let walk_tiles = if ctx.p.detour_map {
        ctx.dex
            .fly_tiles(&h.site, &h.area)
            .unwrap_or(h.detour_tiles)
    } else {
        h.detour_tiles
    };
    // On the bike where the map allows it (the Power Plant forbids cycling).
    let walk_era = if ctx.p.detour_bike && ctx.p.bike && h.site != "MAP_POWER_PLANT" {
        Era::Bike
    } else {
        Era::Run
    };
    // An Escape Rope replaces the walk back out of a dungeon site (rope 550,
    // `items.json`; warps to the entrance, `EscapeRopeFieldEffect`).
    let dungeon = [
        "SEAFOAM",
        "VICTORY_ROAD",
        "POKEMON_TOWER",
        "DIGLETTS",
        "CAVE",
        "TUNNEL",
        "MT_MOON",
    ]
    .iter()
    .any(|d| h.site.contains(d));
    let ways = if ctx.p.rope_fills && dungeon {
        if ctx.p.money_model && st.money >= 550 {
            st.money -= 550;
        }
        bump(st, 350.0 * ctx.p.text_factor);
        1.0
    } else {
        2.0
    };
    bump(
        st,
        walk_tiles as f64 * ways * walk_era.frames_per_tile() * ctx.p.path_overhead,
    );
    let per_step = if area == Area::Land {
        era.frames_per_tile() * ctx.p.path_overhead
    } else {
        // Casting and waiting for a bite. guess.
        180.0
    };
    let args = EncArgs {
        site: &h.site,
        area,
        steps,
        era,
        skip: &skip,
        stop_when_done: true,
        repel: false,
    };
    let used = run_encounters(ctx, st, rng, &args);
    bump(st, used as f64 * per_step);
    if safari {
        if area == Area::Land {
            st.safari_steps = st.safari_steps.saturating_sub(used);
        }
        if st.safari_steps == 0 || st.safari_balls == 0 {
            bump(st, SAFARI_EXIT as f64 * ctx.p.text_factor);
            st.safari_open = false;
        }
    }
}

fn starter_ok(ctx: &Ctx, rng: &mut Rng) -> (bool, f64, [u32; 6], u8) {
    let squirtle = frlg_mon::species::by_id(match ctx.p.starter.as_str() {
        "bulbasaur" => frlg_mon::species::BULBASAUR,
        "charmander" => frlg_mon::species::CHARMANDER,
        _ => frlg_mon::species::SQUIRTLE,
    })
    .expect("starter base stats");
    let mut g = frlg_rng::Rng(rng.next_u64() as u32);
    let genome = frlg_mon::create::gift_mon(&mut g);
    let s5 = calc_stats(squirtle, genome.ivs, 5, genome.nature());
    let total = s5.hp as u32
        + s5.atk as u32
        + s5.def as u32
        + s5.spe as u32
        + s5.spa as u32
        + s5.spd as u32;
    let s50 = calc_stats(squirtle, genome.ivs, 50, genome.nature());
    let base = calc_stats(
        squirtle,
        frlg_mon::stats::Ivs {
            hp: 15,
            atk: 15,
            def: 15,
            spe: 15,
            spa: 15,
            spd: 15,
        },
        50,
        0,
    );
    let score = |s: &frlg_mon::stats::Stats| s.spa as f64 + 0.6 * s.hp as f64;
    let strength = (score(&s50) / score(&base)).clamp(0.85, 1.15);
    let m = &ctx.p.starter_min_stats;
    let stats_ok = [s5.hp, s5.atk, s5.def, s5.spe, s5.spa, s5.spd]
        .iter()
        .zip(m)
        .all(|(&v, &lo)| v as u32 >= lo);
    let ok = total >= ctx.p.starter_min_total && stats_ok;
    let iv = genome.ivs;
    (
        ok,
        strength,
        [
            iv.hp as u32,
            iv.atk as u32,
            iv.def as u32,
            iv.spe as u32,
            iv.spa as u32,
            iv.spd as u32,
        ],
        genome.nature(),
    )
}

/// Runs one attempt. `target` is the active T in frames; `None` disables resets.
pub fn attempt(
    ctx: &Ctx,
    seed: u64,
    idx: u64,
    target: Option<u64>,
    mut rec: Option<&mut Vec<SegStats>>,
    mut split_clock: Option<&mut Vec<u64>>,
    mut probe: Option<&mut Probe>,
) -> Outcome {
    let mut st = State {
        clock: 0,
        off: 0,
        exp: exp_for_level(Growth::MediumSlow, 5),
        carry: 5,
        strength: 1.0,
        power: 0.0,
        caught: Caught::default(),
        abandoned: false,
        safari_open: false,
        safari_steps: 0,
        safari_balls: 0,
        // `decompiled/src/new_game.c:128`, five Poké Balls from Oak
        // (`PalletTown_ProfessorOaksLab/scripts.inc:660`).
        money: 3000,
        balls: [5, 0, 0],
        max_tier: 0,
        repel_left: 0,
        repel_ok: false,
        ivs: [15; 6],
        nature: 0,
        evs: [0; 6],
        hp_frac: 1.0,
        badges: 0,
        league_items: false,
        whiteouts: 0,
        game: GameRng::from_u32(0),
        slots: vec![],
        slot_pp: vec![],
        learned_to: 5,
        carry_status: Status1::None,
        x_stock: [0; 5],
        mewtwo: None,
        mewtwo_moves: vec![],
        evo_checked: false,
        fodder: 0,
        taught_once: vec![],
        elixirs: 0,
        max_elixirs: 0,
        revives: 0,
        candies: 0,
    };
    st.caught.insert(ctx.dex.species[ctx.starter_line[0]].dex);
    // The starter's level-5 moves (`level_up_learnsets.h`).
    for &(l, m) in &ctx.dex.species[ctx.starter_line[0]].learnset {
        if l <= 5 && !st.slots.contains(&m) {
            st.slots.push(m);
            st.slot_pp.push(ctx.dex.moves[m].pp as i32);
        }
    }
    for (i, seg) in ctx.segs.iter().enumerate() {
        let mut rng = Rng::new(seed, idx, hash_str(&seg.name));
        // The game's stream, re-keyed per segment (see `GameRng`).
        st.game = GameRng::from_u32(rng.next_u64() as u32);
        let before = st.clock;
        let wo_before = st.whiteouts;
        match &seg.kind {
            Kind::Text { frames } => {
                bump(&mut st, *frames as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                bump(&mut st, mistakes(ctx, &mut rng, seg.inputs as f64 / 100.0));
            }
            Kind::Walk { tiles, era, enc } => {
                bump(
                    &mut st,
                    *tiles as f64 * era.frames_per_tile() * ctx.p.path_overhead,
                );
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                bump(&mut st, mistakes(ctx, &mut rng, *tiles as f64 / 100.0));
                if let Some(e) = enc {
                    let covered = use_repels(ctx, &mut st, seg, *tiles);
                    let guarded = (e.steps as u64 * covered as u64 / (*tiles).max(1) as u64) as u32;
                    for (steps, repel) in [(guarded, true), (e.steps - guarded, false)] {
                        let args = EncArgs {
                            site: &e.site,
                            area: e.area,
                            steps,
                            era: *era,
                            skip: &[],
                            stop_when_done: false,
                            repel,
                        };
                        run_encounters(ctx, &mut st, &mut rng, &args);
                    }
                }
            }
            Kind::Wander { enc, era } => {
                let covered = use_repels(ctx, &mut st, seg, enc.steps);
                let guarded = covered.min(enc.steps);
                for (steps, repel) in [(guarded, true), (enc.steps - guarded, false)] {
                    let args = EncArgs {
                        site: &enc.site,
                        area: enc.area,
                        steps,
                        era: *era,
                        skip: &[],
                        stop_when_done: false,
                        repel,
                    };
                    run_encounters(ctx, &mut st, &mut rng, &args);
                }
            }
            Kind::Trainer {
                party,
                intro,
                adv,
                money,
                full,
                ai,
                heals,
            } => {
                if ctx.p.battle_sim && !full.is_empty() {
                    let foes: Vec<_> = full
                        .iter()
                        .map(|m| crate::battle::enemy_mon(ctx.dex, m))
                        .collect();
                    let args = BattleArgs {
                        name: &seg.name,
                        foes: &foes,
                        trainer: true,
                        intro: *intro,
                        ai: *ai,
                        heals: *heals,
                        yield_: None,
                        league: is_league(&seg.name),
                        wild: false,
                    };
                    if !sim_battle(ctx, &mut st, &mut rng, &args) {
                        break;
                    }
                } else {
                    let enemies = enemy_list(ctx, &st, party);
                    let hand = if ctx.p.type_strength > 0.0 { 1.0 } else { *adv };
                    if !battle(ctx, &mut st, &mut rng, &enemies, true, *intro, hand) {
                        break;
                    }
                }
                st.money += *money;
            }
            Kind::Parties {
                parties,
                money,
                fulls,
                ais,
            } => {
                let mut ok = true;
                for (pi, p) in parties.iter().enumerate() {
                    if ctx.p.battle_sim && fulls.get(pi).is_some_and(|f| !f.is_empty()) {
                        let foes: Vec<_> = fulls[pi]
                            .iter()
                            .map(|m| crate::battle::enemy_mon(ctx.dex, m))
                            .collect();
                        let args = BattleArgs {
                            name: &seg.name,
                            foes: &foes,
                            trainer: true,
                            intro: 600,
                            ai: ais.get(pi).copied().unwrap_or(1),
                            heals: 0,
                            yield_: None,
                            league: is_league(&seg.name),
                            wild: false,
                        };
                        if !sim_battle(ctx, &mut st, &mut rng, &args) {
                            ok = false;
                            break;
                        }
                        continue;
                    }
                    let enemies = enemy_list(ctx, &st, p);
                    if !battle(ctx, &mut st, &mut rng, &enemies, true, 600, 1.0) {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    break;
                }
                st.money += *money;
            }
            Kind::Group {
                n,
                mons,
                level,
                yield_,
            } => {
                if !group_battle(ctx, &mut st, &mut rng, *n, *mons, *level, *yield_) {
                    break;
                }
                // Class value 5 is a guess; the formula is `battle_script_commands.c:5373`.
                st.money += n * 4 * level * 5;
            }
            Kind::Hunt { hunt: h, era } => hunt(ctx, &mut st, &mut rng, h, *era),
            Kind::Grind {
                battles,
                level,
                yield_,
            } => {
                for _ in 0..*battles {
                    let gap = rng.geometric(0.07) + 1;
                    bump(
                        &mut st,
                        gap as f64 * Era::Run.frames_per_tile() + ctx.enc_intro(),
                    );
                    let t =
                        turns_per_mon(*level as f64, effective_level(&st, ctx.p), st.strength, 1.0);
                    bump(&mut st, t * ctx.turn() + ctx.mon_overhead());
                    let g = add_exp(ctx, &mut st, exp_gain(*yield_ as u8, *level as u8, false));
                    bump(&mut st, g as f64 * ctx.k("level_up", LEVEL_UP as f64));
                }
            }
            Kind::OakCheck => {
                if let Some(pr) = probe.as_deref_mut() {
                    pr.oak_count = st.caught.count();
                }
                // Once the list is exhausted the human goes round it again (at most 6 times).
                let rounds = if ctx.p.fill.is_empty() { 0 } else { 6 };
                for (hi, h) in (0..rounds).flat_map(|_| ctx.p.fill.iter().enumerate()) {
                    if st.caught.count() >= 60 {
                        break;
                    }
                    let (c0, n0) = (st.clock, st.caught.count());
                    // The Power Plant forbids cycling (`allow_cycling` in its map.json).
                    let era = if ctx.p.hunt_bike && ctx.p.bike && h.site != "MAP_POWER_PLANT" {
                        Era::Bike
                    } else {
                        Era::Run
                    };
                    hunt(ctx, &mut st, &mut rng, h, era);
                    if let Some(pr) = probe.as_deref_mut() {
                        if pr.fill.len() <= hi {
                            pr.fill.resize(hi + 1, (0.0, 0.0, 0));
                        }
                        let e = &mut pr.fill[hi];
                        e.0 += (st.clock - c0) as f64;
                        e.1 += (st.caught.count() - n0) as f64;
                        e.2 += 1;
                    }
                    if let Some(t) = target {
                        let left = if ctx.p.dex_resets {
                            ctx.fill_slope * (60.0 - st.caught.count() as f64).max(0.0)
                        } else {
                            0.0
                        };
                        let rem = ctx.floor_after[i] as f64
                            + ctx.aggr[i]
                                * (ctx.mean_after[i] + left - ctx.floor_after[i] as f64).max(0.0);
                        if st.caught.count() < 60 && st.clock as f64 + rem > t as f64 {
                            if let Some(r) = rec.as_deref_mut() {
                                r[i].resets += 1;
                            }
                            return Outcome {
                                wall: st.clock + st.off,
                                run: st.clock,
                                finished: false,
                                caught: st.caught.count(),
                                whiteouts: st.whiteouts,
                            };
                        }
                    }
                }
                if st.caught.count() < 60 {
                    st.abandoned = true;
                    break;
                }
            }
            Kind::Stones => {
                for s in &ctx.p.stones {
                    let want = format!("ITEM_{s}_STONE");
                    // Bought in Celadon (Fire/Thunder/Water/Leaf) or picked up (Moon). guess frames.
                    bump(
                        &mut st,
                        if s == "MOON" { 500.0 } else { 1500.0 } * ctx.p.text_factor,
                    );
                    'found: for sp in &ctx.dex.species {
                        for (k, param, target) in &sp.evo {
                            if k == "EVO_ITEM"
                                && *param == want
                                && sp.dex != 0
                                && st.caught.has(sp.dex)
                            {
                                let t = ctx.dex.species[*target].dex;
                                if !st.caught.has(t) {
                                    st.caught.insert(t);
                                    bump(&mut st, 700.0 * ctx.p.text_factor);
                                    break 'found;
                                }
                            }
                        }
                    }
                }
            }
            Kind::Special { special }
                if special.requires.iter().any(|r| {
                    let d = ctx.dex.species[ctx.dex.id(r)].dex;
                    !st.caught.has(d)
                }) => {}
            Kind::Special { special }
                if special.kind == "buy"
                    && (special
                        .species
                        .iter()
                        .all(|n| st.caught.has(ctx.dex.species[ctx.dex.id(n)].dex))
                        || (ctx.p.money_model && st.money < special.cost)) => {}
            Kind::Special { special } => {
                if special.kind == "buy" && ctx.p.money_model {
                    st.money -= special.cost;
                }
                bump(&mut st, special.frames as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                for n in &special.species {
                    let id = ctx.dex.id(n);
                    let sp = &ctx.dex.species[id];
                    if special.kind == "gift" || special.kind == "trade" || special.kind == "buy" {
                        st.caught.insert(sp.dex);
                    } else {
                        bump(&mut st, ctx.enc_intro() + react(ctx, &mut rng) as f64);
                        let turns = (((1.0 - special.hp_frac) / 0.25).ceil() as u32).max(1);
                        bump(&mut st, turns as f64 * ctx.turn());
                        if rng.chance((0.04 * turns as f64).min(0.9)) {
                            continue;
                        }
                        let ball = match special.ball.as_str() {
                            "poke" => 10,
                            "great" => 15,
                            _ => 20,
                        };
                        if throw_balls(
                            ctx,
                            &mut st,
                            &mut rng,
                            sp.catch_rate,
                            special.hp_frac,
                            Status1::None,
                            Some(ball),
                            special.max_throws,
                        ) {
                            st.caught.insert(sp.dex);
                            if sp.dex != 0
                                && (special.level as f64 * ctx.p.power_bonus) > st.power
                                && special.level >= 60
                            {
                                st.power = special.level as f64 * ctx.p.power_bonus;
                            }
                        }
                    }
                }
            }
            Kind::Mart { mart } => {
                bump(&mut st, MART as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                for (t, want) in [(0, mart.poke), (1, mart.great), (2, mart.ultra)] {
                    if t > st.max_tier {
                        continue;
                    }
                    let qty = want.min(st.money / BALL_PRICE[t]);
                    st.money -= qty * BALL_PRICE[t];
                    st.balls[t] += qty;
                }
            }
            Kind::TeleportOrWalk { frames, tiles } => {
                // Abra is national dex 63.
                if st.caught.has(63) {
                    bump(&mut st, *frames as f64 * ctx.p.text_factor);
                } else {
                    bump(
                        &mut st,
                        *tiles as f64 * Era::Run.frames_per_tile() * ctx.p.path_overhead,
                    );
                }
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
            }
            Kind::Mewtwo => {
                bump(&mut st, 1500.0 * ctx.p.text_factor);
                bump(&mut st, ctx.throw_cost() + ctx.catch_end());
                st.caught.insert(150);
                st.power = 70.0 * ctx.p.power_bonus;
                // Roll its IVs and nature like any wild mon (`pokemon.c`, `CreateBoxMon`:
                // personality then two IV words); its default level-70 moves are
                // Swift/Recover/Safeguard/Psychic (`level_up_learnsets.h`), extended by
                // `tms` entries with `mewtwo: true`.
                let mut g = st.game;
                let pid = (g.roll() as u32) | ((g.roll() as u32) << 16);
                let iv1 = g.roll() as u32;
                let iv2 = g.roll() as u32;
                st.game = g;
                let ivs = [
                    iv1 & 0x1F,
                    (iv1 >> 5) & 0x1F,
                    (iv1 >> 10) & 0x1F,
                    iv2 & 0x1F,
                    (iv2 >> 5) & 0x1F,
                    (iv2 >> 10) & 0x1F,
                ];
                st.mewtwo = Some((ivs, (pid % 25) as u8));
                if st.mewtwo_moves.is_empty() {
                    st.mewtwo_moves = ctx.dex.default_moves(ctx.dex.id("SPECIES_MEWTWO"), 70);
                }
            }
            Kind::Teach { teach } => {
                let mv = ctx.dex.move_id(&teach.mv);
                let mewtwo_sp = ctx.dex.id("SPECIES_MEWTWO");
                let learner = if teach.mewtwo {
                    mewtwo_sp
                } else {
                    carry_species(ctx, st.carry)
                };
                let affordable = !ctx.p.money_model || st.money >= teach.cost;
                let is_hm = matches!(
                    ctx.dex.moves[mv].name.as_str(),
                    "MOVE_CUT"
                        | "MOVE_FLY"
                        | "MOVE_SURF"
                        | "MOVE_STRENGTH"
                        | "MOVE_FLASH"
                        | "MOVE_ROCK_SMASH"
                        | "MOVE_WATERFALL"
                        | "MOVE_DIVE"
                );
                let possible = can_learn(ctx, learner, mv)
                    && (!teach.mewtwo || st.mewtwo.is_some())
                    && (is_hm || !st.taught_once.contains(&mv));
                if affordable && possible {
                    if ctx.p.money_model {
                        st.money -= teach.cost;
                    }
                    if !is_hm {
                        st.taught_once.push(mv);
                    }
                    bump(&mut st, teach.frames as f64 * ctx.p.text_factor);
                    bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                    if teach.mewtwo {
                        if st.mewtwo_moves.len() < 4 {
                            st.mewtwo_moves.push(mv);
                        } else {
                            // Psychic and Recover stay; the weakest other slot goes
                            // (Safeguard, then Swift, then an earlier teach).
                            let keep = [
                                ctx.dex.move_id("MOVE_PSYCHIC"),
                                ctx.dex.move_id("MOVE_RECOVER"),
                            ];
                            if let Some(i) = st
                                .mewtwo_moves
                                .iter()
                                .enumerate()
                                .filter(|(_, m)| !keep.contains(m))
                                .min_by_key(|(_, &m)| ctx.dex.moves[m].power)
                                .map(|(i, _)| i)
                            {
                                st.mewtwo_moves[i] = mv;
                            }
                        }
                    } else {
                        offer_move(ctx, &mut st, mv, true);
                    }
                }
            }
            Kind::Elixir { pickup } => {
                bump(&mut st, pickup.frames as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                match pickup.kind.as_str() {
                    "max" => st.max_elixirs += 1,
                    "candy" => st.candies += 1,
                    _ => st.elixirs += 1,
                }
            }
            Kind::XBuy { xbuy } => {
                bump(&mut st, xbuy.frames as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                // Prices (`items.json`): X Attack 500, X Defend 550, X Speed 350,
                // X Special 350, Dire Hit 650.
                // Bought scarcest-first so a short purse fills the specials before
                // the cheaper nice-to-haves (prices `items.json`).
                {
                    let qty = if ctx.p.money_model {
                        xbuy.revives.min(st.money / 1500)
                    } else {
                        xbuy.revives
                    };
                    if ctx.p.money_model {
                        st.money -= qty * 1500;
                    }
                    st.revives += qty;
                }
                for (i, want, price) in [
                    (3usize, xbuy.special, 350),
                    (2, xbuy.speed, 350),
                    (0, xbuy.attack, 500),
                    (1, xbuy.defend, 550),
                    (4, xbuy.dire, 650),
                ] {
                    let qty = if ctx.p.money_model {
                        want.min(st.money / price)
                    } else {
                        want
                    };
                    if ctx.p.money_model {
                        st.money -= qty * price;
                    }
                    st.x_stock[i] += qty;
                }
            }
        }
        if st.abandoned {
            if let Some(r) = rec.as_deref_mut() {
                r[i].abandons += 1;
                r[i].whiteouts += (st.whiteouts - wo_before) as u64;
            }
            break;
        }
        // Marts that sell better balls: Lavender (Great), Fuchsia (Ultra)
        // (`decompiled/data/maps/LavenderTown_Mart/scripts.inc:29`, `FuchsiaCity_Mart/scripts.inc:25`).
        match seg.name.as_str() {
            "brock" => {
                st.repel_ok = true;
                st.badges |= 1;
            }
            "koga-badge" => {
                st.max_tier = 2;
                st.badges |= 1 << 4;
            }
            // Thunder Badge boosts the player's battle speed by 10%
            // (`GetWhoStrikesFirst`, `battle_main.c:3444-3446`).
            "surge-badge" => st.badges |= 1 << 2,
            "blaine-badge-and-sevii-scene" => st.badges |= 1 << 6,
            // Giovanni hands over TM26 Earthquake
            // (`ViridianCity_Gym/scripts.inc:28-30`); teaching it is a plan choice via
            // `tms`, not automatic (a taught move displaces a slot).
            "indigo-heal" => {
                st.league_items = true;
                // Rare Candies kept for the League go down now: a level each, worth the
                // most exp this late (~400 frames per use).
                while st.candies > 0 && st.carry < 100 {
                    st.candies -= 1;
                    st.carry += 1;
                    st.exp = st
                        .exp
                        .max(exp_for_level(Growth::MediumSlow, st.carry as u8));
                    bump(&mut st, 400.0 * ctx.p.text_factor);
                }
                learn_moves(ctx, &mut st);
            }
            // HM03 Surf is taught to the carry after the Safari trip (the route needs a
            // surfer anyway; `plan.md` HM gates).
            "safari-walk" => {
                let surf = ctx.dex.move_id("MOVE_SURF");
                if can_learn(ctx, carry_species(ctx, st.carry), surf) {
                    offer_move(ctx, &mut st, surf, true);
                }
            }
            "lavender-heal" => st.max_tier = st.max_tier.max(1),
            _ => {}
        }
        // league-entry is the Indigo Plateau Centre before the rematch chain.
        if seg.name.contains("heal") || seg.name == "league-entry" {
            // The party fillers (gift mons, caught fodder) stand revived; five slots
            // beside the carry once the party has filled out (guess: by Misty).
            st.fodder = if st.carry >= 20 { 5 } else { 2 };
            st.hp_frac = 1.0;
            st.carry_status = Status1::None;
            for (k, &m) in st.slots.clone().iter().enumerate() {
                st.slot_pp[k] = ctx.dex.moves[m].pp as i32;
            }
        }
        if seg.off_timer {
            let d = st.clock - before;
            st.off += d;
            st.clock = before;
        }
        // The evolution stat gate: the human opens the summary at the first evolution
        // and resets unless the minima hold (finer IV information than level 5 gives).
        if !st.evo_checked
            && st.carry >= ctx.evo_at[0]
            && ctx.p.evo_min_stats.iter().any(|&m| m > 0)
        {
            st.evo_checked = true;
            bump(&mut st, 250.0 * ctx.p.text_factor);
            let m = crate::battle::make_mon(
                ctx.dex,
                carry_species(ctx, st.carry),
                st.carry,
                st.ivs,
                st.evs,
                st.nature,
                0,
            );
            let stats = [
                m.max_hp, m.stat[0], m.stat[1], m.stat[2], m.stat[3], m.stat[4],
            ];
            let ok = stats
                .iter()
                .zip(ctx.p.evo_min_stats)
                .all(|(&v, lo)| v as u32 >= lo);
            if !ok {
                if let Some(r) = rec.as_deref_mut() {
                    r[i].resets += 1;
                }
                return Outcome {
                    wall: st.clock + st.off,
                    run: st.clock,
                    finished: false,
                    caught: st.caught.count(),
                    whiteouts: st.whiteouts,
                };
            }
        }
        if seg.starter_check {
            let (mut ok, mut strength, mut ivs, mut nat) =
                starter_ok(ctx, &mut Rng::new(seed, idx, 0x5747));
            if !ok && ctx.p.starter_save_reroll {
                // Saved in the lab before choosing: soft reset, Continue, choose again. The timer
                // keeps running, so every re-roll is run time but not a lost attempt.
                let mut k = 0u64;
                while !ok && k < 20_000 {
                    k += 1;
                    bump(&mut st, ctx.p.reroll_frames as f64 * ctx.p.text_factor);
                    (ok, strength, ivs, nat) =
                        starter_ok(ctx, &mut Rng::new(seed, idx, 0x5747 + k * 7919));
                }
            }
            st.strength = strength;
            st.ivs = ivs;
            st.nature = nat;
            if !ok {
                let wall = st.clock + st.off;
                if let Some(r) = rec.as_deref_mut() {
                    r[i].resets += 1;
                }
                return Outcome {
                    wall,
                    run: st.clock,
                    finished: false,
                    caught: st.caught.count(),
                    whiteouts: st.whiteouts,
                };
            }
        }
        if let Some(r) = rec.as_deref_mut() {
            let d = (st.clock - before) as f64;
            r[i].n += 1;
            r[i].sum += d;
            r[i].sumsq += d * d;
            r[i].whiteouts += (st.whiteouts - wo_before) as u64;
            r[i].level_sum += st.carry as f64;
            r[i].caught_sum += st.caught.count() as f64;
            r[i].money_sum += st.money as f64;
            r[i].balls_sum += st.balls.iter().sum::<u32>() as f64;
        }
        if let Some(sc) = split_clock.as_deref_mut() {
            sc[i] = st.clock;
        }
        if let Some(pr) = probe.as_deref_mut() {
            if pr.caught_at.len() <= i {
                pr.caught_at.resize(i + 1, 0);
            }
            pr.caught_at[i] = st.caught.count();
        }
        if seg.split {
            if let Some(t) = target {
                let mean = if ctx.p.dex_resets && i < ctx.oak_idx {
                    // The fill still to come depends on how many species are in the bag by then.
                    let dex_proj = st.caught.count() as f64 + ctx.gain_after[i];
                    ctx.mean_nofill_after[i]
                        + ctx.fill_base
                        + ctx.fill_slope * (60.0 - dex_proj).max(0.0)
                } else {
                    ctx.mean_after[i]
                };
                let remaining = ctx.floor_after[i] as f64
                    + ctx.aggr[i] * (mean - ctx.floor_after[i] as f64).max(0.0);
                if st.clock as f64 + remaining > t as f64 {
                    if let Some(r) = rec.as_deref_mut() {
                        r[i].resets += 1;
                    }
                    return Outcome {
                        wall: st.clock + st.off,
                        run: st.clock,
                        finished: false,
                        caught: st.caught.count(),
                        whiteouts: st.whiteouts,
                    };
                }
            }
        }
    }
    let finished = !st.abandoned;
    Outcome {
        wall: st.clock + st.off,
        run: st.clock,
        finished,
        caught: st.caught.count(),
        whiteouts: st.whiteouts,
    }
}
