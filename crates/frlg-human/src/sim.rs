//! One attempt of a plan: run the segments, sampling the human's jitter and the game's
//! randomness, applying the reset rules, and report what it cost.
//!
//! Model constants marked `guess` are uncalibrated (see `human-run/plans/*.md`).

use crate::data::{Area, Caught, Dex, Slot};
use crate::params::{Hunt, Params};
use crate::rng::{hash_str, Rng};
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
}

/// Optional extra measurements from an attempt.
#[derive(Default)]
pub struct Probe {
    /// Kanto species caught when Oak's check starts.
    pub oak_count: u32,
    /// Per fill hunt: (frames spent, species gained, times run).
    pub fill: Vec<(f64, f64, u64)>,
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
}

impl Ctx<'_> {
    fn turn(&self) -> f64 {
        TURN as f64 * self.p.battle_scale
    }

    fn mon_overhead(&self) -> f64 {
        MON_OVERHEAD as f64 * self.p.battle_scale
    }

    pub fn floor_frames(&self, seg: &Seg) -> f64 {
        let p = self.p;
        match &seg.kind {
            Kind::Text { frames } => *frames as f64 * p.text_factor,
            Kind::Walk { tiles, era, .. } => {
                *tiles as f64 * era.frames_per_tile() * p.path_overhead
            }
            Kind::Trainer { party, intro, .. } => {
                (*intro as f64 * p.text_factor)
                    + party.len() as f64 * (self.turn() + self.mon_overhead())
                    + TRAINER_OUTRO as f64
            }
            Kind::Parties { parties } => parties
                .iter()
                .map(|p| {
                    600.0
                        + p.len() as f64 * (self.turn() + self.mon_overhead())
                        + TRAINER_OUTRO as f64
                })
                .sum(),
            Kind::Group { n, mons, .. } => {
                *n as f64
                    * (600.0
                        + *mons as f64 * (self.turn() + self.mon_overhead())
                        + TRAINER_OUTRO as f64)
            }
            Kind::Wander { .. }
            | Kind::Hunt { .. }
            | Kind::OakCheck
            | Kind::Stones
            | Kind::Mewtwo => 0.0,
            Kind::Special { .. } => 0.0,
            Kind::TeleportOrWalk { frames, .. } => *frames as f64 * self.p.text_factor,
            Kind::Grind { .. } => 0.0,
        }
    }
}

fn bump(st: &mut State, f: f64) {
    st.clock += f.round() as u64;
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

/// Throws until caught or patience (8 throws) runs out. Formula:
/// `decompiled/src/battle_script_commands.c:9549-9640`.
fn throw_balls(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    catch_rate: u32,
    hp_frac: f64,
    ball: u32,
    max_throws: u32,
) -> bool {
    let max_hp = 100u32;
    let hp = ((hp_frac * max_hp as f64) as u32).max(1);
    let odds = (catch_rate * ball / 10) * (max_hp * 3 - hp * 2) / (3 * max_hp);
    for _ in 0..max_throws {
        bump(st, THROW as f64);
        let caught = if odds > 254 {
            true
        } else {
            let o = odds.max(1);
            let s = isqrt(isqrt(16_711_680 / o)).max(1);
            let thr = 1_048_560 / s;
            (0..4).all(|_| rng.below(65536) < thr)
        };
        if caught {
            bump(st, CATCH_END as f64 * ctx.p.text_factor);
            return true;
        }
        bump(st, react(ctx, rng) as f64);
    }
    false
}

/// One wild encounter on arrival: catch if it is a new Kanto species the human wants, else
/// flee or fight. `skip` lists species left alone.
fn encounter(
    ctx: &Ctx,
    st: &mut State,
    rng: &mut Rng,
    slot: &Slot,
    skip: &[usize],
    want_catch: bool,
) {
    let sp = &ctx.dex.species[slot.species];
    let level = rng.range(slot.min, slot.max);
    bump(st, ENC_INTRO as f64 + react(ctx, rng) as f64);
    let wanted =
        want_catch && sp.dex != 0 && !st.caught.has(sp.dex) && !skip.contains(&slot.species);
    if wanted {
        let mut hp = 1.0;
        if ctx.p.weaken {
            let turns = 1 + rng.chance(0.5) as u32;
            bump(st, turns as f64 * ctx.turn());
            if rng.chance(0.05 * turns as f64) {
                return;
            }
            hp = 0.3;
        }
        if throw_balls(ctx, st, rng, sp.catch_rate, hp, ball_mult(ctx.p), 8) {
            st.caught.insert(sp.dex);
        }
    } else if ctx.p.fight_wild || st.carry < ctx.p.fight_until_level {
        let e = exp_gain(sp.exp_yield as u8, level as u8, false);
        let t = turns_per_mon(level as f64, effective_level(st, ctx.p), st.strength, 1.0);
        bump(st, t * ctx.turn() + ctx.mon_overhead());
        let g = add_exp(ctx, st, e);
        bump(st, (g * LEVEL_UP) as f64);
    } else {
        bump(st, FLEE as f64);
    }
}

fn turns_per_mon(le: f64, lp: f64, strength: f64, adv: f64) -> f64 {
    ((1.0 + 3.0 * (le / lp - 0.7).max(0.0)) / (strength * adv)).max(1.0)
}

struct EncArgs<'a> {
    site: &'a str,
    area: Area,
    steps: u32,
    era: Era,
    skip: &'a [usize],
    stop_when_done: bool,
}

fn run_encounters(ctx: &Ctx, st: &mut State, rng: &mut Rng, a: &EncArgs) -> u32 {
    let EncArgs {
        site,
        area,
        steps,
        era,
        skip,
        stop_when_done,
    } = *a;
    let Some((rate, slots, weights)) = ctx.dex.area(site, area, ctx.p.version == "LG") else {
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
        let slot = &slots[pick_slot(rng, weights)];
        encounter(
            ctx,
            st,
            rng,
            slot,
            skip,
            stop_when_done
                || (ctx.p.catch_on_pass
                    && ctx.dex.species[slot.species].catch_rate >= ctx.p.pass_min_catch),
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

fn p_lose(r: f64) -> f64 {
    1.0 / (1.0 + (-(r - 1.25) / 0.05).exp())
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
        bump(st, intro as f64 * ctx.p.text_factor);
        bump(st, jitter(ctx, rng, 8) as f64);
        let mut total_exp = 0u32;
        for &(lvl, yld, a) in enemies {
            let mean = turns_per_mon(lvl as f64, lp_of(a), st.strength, 1.0);
            let mut turns = mean.floor() + rng.chance(mean - mean.floor()) as u32 as f64;
            for _ in 0..turns as u32 {
                if rng.chance(P_EVENT) {
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
            bump(st, TRAINER_OUTRO as f64);
        }
        let danger = p_lose(r) * 0.5f64.powi(tries);
        if rng.chance(danger) {
            let spent = st.clock - start;
            st.whiteouts += 1;
            bump(st, WHITEOUT as f64 + spent as f64 * 0.0);
            tries += 1;
            if tries >= 3 {
                st.abandoned = true;
                return false;
            }
            continue;
        }
        let g = add_exp(ctx, st, total_exp);
        bump(st, (g * LEVEL_UP) as f64);
        return true;
    }
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
    let skip: Vec<usize> = h.skip.iter().map(|s| ctx.dex.id(s)).collect();
    bump(
        st,
        h.detour_tiles as f64 * 2.0 * Era::Run.frames_per_tile() * ctx.p.path_overhead,
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
        steps: h.steps,
        era,
        skip: &skip,
        stop_when_done: true,
    };
    let used = run_encounters(ctx, st, rng, &args);
    bump(st, used as f64 * per_step);
}

fn starter_ok(ctx: &Ctx, rng: &mut Rng) -> (bool, f64) {
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
    (ok, strength)
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
        whiteouts: 0,
    };
    st.caught.insert(ctx.dex.species[ctx.starter_line[0]].dex);
    for (i, seg) in ctx.segs.iter().enumerate() {
        let mut rng = Rng::new(seed, idx, hash_str(&seg.name));
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
                    let args = EncArgs {
                        site: &e.site,
                        area: e.area,
                        steps: e.steps,
                        era: *era,
                        skip: &[],
                        stop_when_done: false,
                    };
                    run_encounters(ctx, &mut st, &mut rng, &args);
                }
            }
            Kind::Wander { enc, era } => {
                let args = EncArgs {
                    site: &enc.site,
                    area: enc.area,
                    steps: enc.steps,
                    era: *era,
                    skip: &[],
                    stop_when_done: false,
                };
                run_encounters(ctx, &mut st, &mut rng, &args);
            }
            Kind::Trainer { party, intro, adv } => {
                let enemies = enemy_list(ctx, &st, party);
                let hand = if ctx.p.type_strength > 0.0 { 1.0 } else { *adv };
                if !battle(ctx, &mut st, &mut rng, &enemies, true, *intro, hand) {
                    break;
                }
            }
            Kind::Parties { parties } => {
                let mut ok = true;
                for p in parties {
                    let enemies = enemy_list(ctx, &st, p);
                    if !battle(ctx, &mut st, &mut rng, &enemies, true, 600, 1.0) {
                        ok = false;
                        break;
                    }
                }
                if !ok {
                    break;
                }
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
                        gap as f64 * Era::Run.frames_per_tile() + ENC_INTRO as f64,
                    );
                    let t =
                        turns_per_mon(*level as f64, effective_level(&st, ctx.p), st.strength, 1.0);
                    bump(&mut st, t * ctx.turn() + ctx.mon_overhead());
                    let g = add_exp(ctx, &mut st, exp_gain(*yield_ as u8, *level as u8, false));
                    bump(&mut st, (g * LEVEL_UP) as f64);
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
                    hunt(ctx, &mut st, &mut rng, h, Era::Run);
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
                        let rem = ctx.floor_after[i] as f64
                            + ctx.aggr[i]
                                * (ctx.mean_after[i] - ctx.floor_after[i] as f64).max(0.0);
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
            Kind::Special { special } => {
                bump(&mut st, special.frames as f64 * ctx.p.text_factor);
                bump(&mut st, jitter(ctx, &mut rng, seg.inputs) as f64);
                for n in &special.species {
                    let id = ctx.dex.id(n);
                    let sp = &ctx.dex.species[id];
                    if special.kind == "gift" || special.kind == "trade" {
                        st.caught.insert(sp.dex);
                    } else {
                        bump(&mut st, ENC_INTRO as f64 + react(ctx, &mut rng) as f64);
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
                            ball,
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
                bump(&mut st, THROW as f64 + CATCH_END as f64);
                st.caught.insert(150);
                st.power = 70.0 * ctx.p.power_bonus;
            }
        }
        if st.abandoned {
            if let Some(r) = rec.as_deref_mut() {
                r[i].abandons += 1;
                r[i].whiteouts += (st.whiteouts - wo_before) as u64;
            }
            break;
        }
        if seg.off_timer {
            let d = st.clock - before;
            st.off += d;
            st.clock = before;
        }
        if seg.starter_check {
            let (ok, strength) = starter_ok(ctx, &mut Rng::new(seed, idx, 0x5747));
            st.strength = strength;
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
        }
        if let Some(sc) = split_clock.as_deref_mut() {
            sc[i] = st.clock;
        }
        if seg.split {
            if let Some(t) = target {
                let remaining = ctx.floor_after[i] as f64
                    + ctx.aggr[i] * (ctx.mean_after[i] - ctx.floor_after[i] as f64).max(0.0);
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
