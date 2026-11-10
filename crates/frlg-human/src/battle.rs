//! A turn-level battle simulation of the carry against a trainer's party or a wild mon.
//!
//! Replaces the level-ratio shortcuts of the original model (`turns_per_mon`, `p_lose`, the type floor
//! and the team bonus) when `Params::battle_sim` is on. Everything comes from the decomp:
//!
//! * stats `((2 base + IV + EV/4) L / 100) + 5` (HP `+ L + 10`) and the nature
//!   (`decompiled/src/pokemon.c:2093-2170`), badge boosts (`pokemon.c`, `CalculateBaseDamage`);
//! * damage `((2L/5 + 2) Power A / D) / 50 + 2`, physical for types below `TYPE_MYSTERY`
//!   (`IS_TYPE_PHYSICAL`, `battle.h:475`), crit x2 at 1 in 16 (`battle_script_commands.c:588,1199`),
//!   STAB x1.5, type effectiveness (`battle_main.c:312`), random 85-100 percent, Torrent/Blaze/Overgrow
//!   x1.5 power at 1/3 HP (`pokemon.c`, `CalculateBaseDamage`);
//! * moves, base stats, learnsets and the trainers' mons with IVs and moves from `battle_moves.h`,
//!   `species_info.h`, `level_up_learnsets.h`, `trainer_parties.h` (`tools/gen_dex.py`).
//!
//! Not modelled: status and stat-stage moves, abilities other than the starter's, held items, weather,
//! PP, switching, the enemy AI beyond "best expected damage".

use crate::data::{Dex, PartyMon};
use crate::rng::Rng;

/// One combatant.
#[derive(Clone, Debug)]
pub struct Mon {
    pub species: usize,
    pub level: u32,
    pub hp: i32,
    pub max_hp: i32,
    /// Attack, Defense, Speed, Sp. Atk, Sp. Def.
    pub stat: [i32; 5],
    pub types: (u8, u8),
    pub moves: Vec<usize>,
}

/// What the player's side brings: the carry's stats are rebuilt from base, IVs, EVs and nature at
/// the carry's level.
pub struct Player<'a> {
    pub species: usize,
    pub level: u32,
    pub ivs: [u32; 6],
    pub evs: [u32; 6],
    pub nature: u8,
    /// Percent boosts of the four badges that matter (Atk, Def, SpA, SpD): 100 or 110.
    pub badge: [i32; 4],
    pub moves: &'a [usize],
    /// Starter ability type (Torrent Water, Blaze Fire, Overgrow Grass).
    pub ability_type: u8,
}

pub fn make_mon(
    dex: &Dex,
    species: usize,
    level: u32,
    iv: [u32; 6],
    ev: [u32; 6],
    nature: u8,
) -> Mon {
    let b = dex.species[species].base;
    let l = level as i32;
    let hp = ((2 * b[0] as i32 + iv[0] as i32 + ev[0] as i32 / 4) * l) / 100 + l + 10;
    let raw = |i: usize| ((2 * b[i] as i32 + iv[i] as i32 + ev[i] as i32 / 4) * l) / 100 + 5;
    // Attack, Defense, Speed, Sp. Atk, Sp. Def are base indices 1..=5.
    let nat = frlg_mon::stats::NATURE_STAT_TABLE[nature as usize];
    let with = |v: i32, col: usize| match nat[col] {
        1 => v * 110 / 100,
        -1 => v * 90 / 100,
        _ => v,
    };
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
        types: dex.species[species].types,
        moves: vec![],
    }
}

/// Mon from a trainer's party entry: the party IV applies to every stat, no EVs, neutral nature.
pub fn enemy_mon(dex: &Dex, m: &PartyMon) -> Mon {
    let mut e = make_mon(dex, m.species, m.level, [m.iv; 6], [0; 6], 0);
    e.moves = m.moves.clone();
    e
}

/// A mon with no species data (the guessed trainer groups): level `level`, all base stats 55 (about
/// the average of a route trainer's mons), Normal type, one Normal move of power 55.
pub fn generic_mon(dex: &Dex, level: u32) -> Mon {
    // Borrow a mid-range species for the arithmetic, then override types and moves.
    let sp = dex.id("SPECIES_RATTATA");
    let l = level as i32;
    let r = |b: i32| ((2 * b + 15) * l) / 100 + 5;
    let hp = ((2 * 55 + 15) * l) / 100 + l + 10;
    Mon {
        species: sp,
        level,
        hp,
        max_hp: hp,
        stat: [r(55), r(55), r(55), r(55), r(55)],
        types: (0, 0),
        moves: vec![dex.move_id("MOVE_TACKLE")],
    }
}

fn is_physical(t: u8) -> bool {
    t < 9
}

/// Damage of one hit, before the random factor: the decomp's formula, in integer arithmetic.
fn base_damage(level: u32, power: u32, a: i32, d: i32) -> i32 {
    let l = level as i32;
    (((2 * l / 5 + 2) * power as i32 * a.max(1)) / d.max(1)) / 50 + 2
}

struct Hit {
    dmg: i32,
    crit: bool,
}

/// One attack's expected-damage score and its actual roll.
#[allow(clippy::too_many_arguments)]
fn attack(
    dex: &Dex,
    att: &Mon,
    def: &Mon,
    mv: usize,
    power_mult: f64,
    badge: [i32; 4],
    att_is_player: bool,
    rng: &mut Rng,
) -> Option<Hit> {
    let m = &dex.moves[mv];
    if m.acc > 0 && rng.below(100) >= m.acc {
        return None;
    }
    let (a, d) = if is_physical(m.mtype) {
        let a = att.stat[0] * if att_is_player { badge[0] } else { 100 } / 100;
        let d = def.stat[1] * if att_is_player { 100 } else { badge[1] } / 100;
        (a, d)
    } else {
        let a = att.stat[3] * if att_is_player { badge[2] } else { 100 } / 100;
        let d = def.stat[4] * if att_is_player { 100 } else { badge[3] } / 100;
        (a, d)
    };
    let power = (m.power as f64 * power_mult) as u32;
    let crit = rng.below(16) == 0;
    let mut dmg = base_damage(att.level, power, a, d);
    if crit {
        dmg *= 2;
    }
    if att.types.0 == m.mtype || att.types.1 == m.mtype {
        dmg = dmg * 3 / 2;
    }
    let eff = dex.eff(m.mtype, def.types);
    dmg = (dmg as f64 * eff) as i32;
    dmg = dmg * (85 + rng.below(16) as i32) / 100;
    if eff > 0.0 && dmg < 1 {
        dmg = 1;
    }
    Some(Hit { dmg, crit })
}

/// Expected damage of a move without the random factors (for the choice of move).
fn score(dex: &Dex, att: &Mon, def: &Mon, mv: usize, power_mult: f64) -> f64 {
    let m = &dex.moves[mv];
    if m.power == 0 || !m.plain_hit {
        return 0.0;
    }
    let (a, d) = if is_physical(m.mtype) {
        (att.stat[0], def.stat[1])
    } else {
        (att.stat[3], def.stat[4])
    };
    let mut dmg = base_damage(att.level, (m.power as f64 * power_mult) as u32, a, d) as f64;
    if att.types.0 == m.mtype || att.types.1 == m.mtype {
        dmg *= 1.5;
    }
    dmg * dex.eff(m.mtype, def.types) * (m.acc.max(1) as f64 / 100.0).min(1.0)
}

/// What happened in a fight.
#[derive(Default, Debug)]
pub struct Outcome {
    /// The carry stayed up through the whole party.
    pub won: bool,
    /// Player turns (attacks and item uses) and how many of them were item uses.
    pub turns: u32,
    pub item_turns: u32,
    /// Enemy crits and player misses, each a decision for the human (reaction delay).
    pub events: u32,
    /// Enemy mons defeated: (species, level), for exp and effort values.
    pub kos: Vec<(usize, u32)>,
    /// The carry's HP left as a fraction of its maximum.
    pub hp_frac: f64,
}

/// Heal item used mid-fight: Potion (20) to level 14, Super Potion (50) to 29, Hyper Potion (200)
/// after; a Full Restore (999) when `full` (the League, with the Indigo Plateau mart reached).
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

/// Fights the party in order with the carry at `hp_frac` of its maximum. Items are used below 30%
/// HP when the heal beats the next hit.
pub fn fight(
    dex: &Dex,
    pl: &Player,
    hp_frac: f64,
    foes: &[Mon],
    full_heals: bool,
    rng: &mut Rng,
) -> Outcome {
    let mut me = make_mon(dex, pl.species, pl.level, pl.ivs, pl.evs, pl.nature);
    me.hp = ((me.max_hp as f64 * hp_frac).round() as i32).clamp(1, me.max_hp);
    let heal = heal_amount(pl.level, full_heals);
    let mut out = Outcome::default();
    for foe in foes {
        let mut e = foe.clone();
        // Best move of each side against the other (the choice does not change within a fight here).
        let pick = |a: &Mon, d: &Mon, pool: &[usize], mult: f64| -> Option<usize> {
            pool.iter()
                .copied()
                .map(|m| (m, score(dex, a, d, m, mult)))
                .filter(|(_, s)| *s > 0.0)
                .max_by(|x, y| x.1.partial_cmp(&y.1).unwrap())
                .map(|(m, _)| m)
        };
        let mut my_move = pick(&me, &e, pl.moves, 1.0);
        let en_move = pick(&e, &me, &e.moves, 1.0);
        let mut guard = 0;
        while e.hp > 0 && me.hp > 0 && guard < 200 {
            guard += 1;
            let torrent = if me.hp * 3 <= me.max_hp { 1.5 } else { 1.0 };
            let mult_for = |m: usize| {
                if dex.moves[m].mtype == pl.ability_type {
                    torrent
                } else {
                    1.0
                }
            };
            my_move = pick(&me, &e, pl.moves, torrent).or(my_move);
            // Speed: the faster acts first; ties are a coin flip.
            let me_first = me.stat[2] > e.stat[2] || (me.stat[2] == e.stat[2] && rng.chance(0.5));
            let act_me = |me: &mut Mon, e: &mut Mon, out: &mut Outcome, rng: &mut Rng| {
                out.turns += 1;
                if me.hp * 10 < me.max_hp * 4 && en_move.is_some() {
                    // Heal instead of attacking when the potion is worth more than a hit.
                    me.hp = (me.hp + heal).min(me.max_hp);
                    out.item_turns += 1;
                    return;
                }
                let Some(mv) = my_move else { return };
                match attack(dex, me, e, mv, mult_for(mv), pl.badge, true, rng) {
                    Some(h) => e.hp -= h.dmg,
                    None => out.events += 1,
                }
            };
            let act_en = |me: &mut Mon, e: &Mon, out: &mut Outcome, rng: &mut Rng| {
                let Some(mv) = en_move else { return };
                if let Some(h) = attack(dex, e, me, mv, 1.0, pl.badge, false, rng) {
                    me.hp -= h.dmg;
                    if h.crit {
                        out.events += 1;
                    }
                }
            };
            if me_first {
                act_me(&mut me, &mut e, &mut out, rng);
                if e.hp > 0 {
                    act_en(&mut me, &e, &mut out, rng);
                }
            } else {
                act_en(&mut me, &e, &mut out, rng);
                if me.hp > 0 {
                    act_me(&mut me, &mut e, &mut out, rng);
                }
            }
        }
        if me.hp <= 0 || e.hp > 0 {
            out.won = false;
            out.hp_frac = 0.0;
            return out;
        }
        out.kos.push((foe.species, foe.level));
    }
    out.won = true;
    out.hp_frac = me.hp as f64 / me.max_hp as f64;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lab_rival_is_a_close_fight_the_carry_usually_wins() {
        let dex = Dex::load();
        let sq = dex.id("SPECIES_SQUIRTLE");
        let tackle = dex.move_id("MOVE_TACKLE");
        let pl = Player {
            species: sq,
            level: 5,
            ivs: [20; 6],
            evs: [0; 6],
            nature: 0,
            badge: [100; 4],
            moves: &[tackle],
            ability_type: 11,
        };
        let f = dex.fight("TRAINER_RIVAL_OAKS_LAB_BULBASAUR");
        let foes: Vec<Mon> = f.full.iter().map(|m| enemy_mon(&dex, m)).collect();
        let (mut won, mut turns) = (0u32, 0u32);
        for i in 0..400 {
            let mut rng = Rng::new(1, i, 3);
            let o = fight(&dex, &pl, 1.0, &foes, false, &mut rng);
            won += o.won as u32;
            turns += o.turns;
        }
        // Measured on libmgba with Tackle mashed: 5 turns (`human-run/calibration.md`).
        assert!(won > 340, "won {won}/400");
        let mean = turns as f64 / 400.0;
        assert!((3.5..7.5).contains(&mean), "mean turns {mean}");
    }

    #[test]
    fn damage_follows_the_decomp_formula() {
        // ((2*50/5 + 2) * 95 * 100 / 100) / 50 + 2 = 22*95/50 + 2 = 43 (integer arithmetic).
        assert_eq!(base_damage(50, 95, 100, 100), 43);
    }
}
