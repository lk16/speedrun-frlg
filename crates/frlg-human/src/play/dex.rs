//! S7, the dex fill: the plan's `hunts`, `specials` and `evolutions` played in the game, with every
//! decision taken by the simulator's own rules (`crate::sim`):
//!
//! | choice | simulator code |
//! | --- | --- |
//! | catch or run on a wild encounter | [`sim::wants_catch`] with [`sim::hunt_skip`]; owned = Pokédex flags in RAM |
//! | leave the site | [`sim::site_done`], [`sim::hunt_stop_new`] (`stop_after`, `safari_stop`), the step budget |
//! | Safari throw order | [`sim::SafariMon::next_act`] (bait / rocks / balls) |
//! | ball, weaken, throws | `play::battle::fight` with `WildPlan::Catch` ([`sim::ball_tier`], [`sim::bag_ball`], ...) |
//! | which mon evolves | [`sim::evo_source`] |
//!
//! # API (for the route stages: `legs.rs` calls [`dex_hook`] after a model segment)
//!
//! ```ignore
//! use frlg_human::play::dex;
//! // Everything the plan anchors after model segment `seg`, in the model's order
//! // (`route.rs`: evolutions, then specials, then hunts, each in plan order).
//! dex::dex_hook(s, "safari-walk")?;
//! // One hunt (walks to the site with play/walk.rs first).
//! let rep = dex::hunt(s, &plan.params.hunts[0])?;
//! // One special (gift, in-game trade, purchase) / one evolution.
//! dex::special(s, &plan.params.specials[3])?;
//! dex::evolve(s, &plan.params.evolutions[4])?;
//! // Progress toward the National Dex: Oak's check is caught >= 60 (Kanto + the rest:
//! // `decompiled/data/maps/PalletTown/scripts.inc:53-57`, `GetPokedexCount` -> VAR_0x8006).
//! let n = dex::caught_count(s);
//! ```
//!
//! Encounters met while a hunt runs go through [`wild_plan`] (called by the field's battle entry,
//! `fallback_battle::battle`), so a catch can also happen on the walk to the site.
//!
//! # Logged
//!
//! `decision` per hunt start (needed species at the site, budget, cap) and end (steps, encounters,
//! catches, frames per step against the model's), per wild encounter (catch or run, with the catch
//! rate, owned/skip reason and needed species left); `catch` per Safari throw (factors, balls left,
//! catch chance); `model-vs-actual` where the game leaves the model (a rod other than the plan's,
//! the fishing bite needing no input, a special the controller cannot do yet).
use frlg_emu::keys;
use frlg_route::Feed;
use serde_json::json;

use super::battle::{BattleEnd, WildPlan};
use super::menu;
use super::names::{id, names};
use super::prims::{is_free, settle, wait_until};
use super::ram::{read_mon, Ram};
use super::session::{PResult, PlayError, Session};
use super::walk::{face, go, go_map, here, Opts};
use crate::data::Area;
use crate::params::{Evolve, Hunt, Special};
use crate::sim::{self, SafariAct, SafariMon};

/// The hunt in progress, read by [`wild_plan`] on every wild encounter.
#[derive(Clone, Debug, Default)]
pub struct HuntRun {
    pub site: String,
    /// Dex species indices the human leaves alone (`sim::hunt_skip`).
    pub skip: Vec<usize>,
    pub safari: bool,
    pub encounters: u32,
    /// Species caught on this hunt (constant names).
    pub caught: Vec<String>,
}

/// Dex-fill state carried by the session (`Session::dex`).
#[derive(Clone, Debug, Default)]
pub struct DexState {
    pub hunt: Option<HuntRun>,
    /// New species caught in the Safari Zone on this visit (`safari_stop`).
    pub safari_new: u32,
    /// The plan's `safari_stop` was reached: the zone is left for good (`sim::hunt`).
    pub safari_done: bool,
}

/// What one hunt did.
#[derive(Clone, Debug, Default)]
pub struct HuntReport {
    pub site: String,
    /// Steps paced (land, water) or casts (fishing).
    pub steps: u32,
    pub encounters: u32,
    pub caught: Vec<String>,
    pub frames: u64,
    /// Of `frames`, the walk to the site (to the first hunting tile).
    pub walk_frames: u64,
    /// Why it ended: `complete`, `stop_after`, `budget`, `safari steps`, `safari balls`, `skipped`.
    pub why: String,
}

// ----------------------------------------------------------------------------------------------
// RAM: the Pokédex, the wild mon, the Safari Zone.

/// `struct Pokedex` at `gSaveBlock2Ptr + 0x18` (`decompiled/include/global.h:193-204,344`):
/// `owned[]` at 0x10, `seen[]` at 0x44, bit `n - 1` for national number `n`
/// (`DexScreen_GetSetPokedexFlag`, `decompiled/src/pokedex_screen.c:2222-2268`).
pub fn owned(s: &mut Session, national: u32) -> bool {
    if national == 0 {
        return false;
    }
    let emu = s.hf.rec.emu();
    let Some(sb2) = s.obs.save_block2(emu) else {
        return false;
    };
    let i = national - 1;
    let o = emu.read8(sb2 + 0x18 + 0x10 + i / 8);
    let n = emu.read8(sb2 + 0x18 + 0x44 + i / 8);
    let m = 1u8 << (i % 8);
    o & m != 0 && n & m != 0
}

/// All owned national numbers 1..=386 at once (one read per byte).
pub fn owned_set(s: &mut Session) -> Vec<bool> {
    let emu = s.hf.rec.emu();
    let mut v = vec![false; 387];
    let Some(sb2) = s.obs.save_block2(emu) else {
        return v;
    };
    let own = emu.read_bytes(sb2 + 0x28, 49);
    let seen = emu.read_bytes(sb2 + 0x5C, 49);
    for (n, slot) in v.iter_mut().enumerate().skip(1) {
        let i = n - 1;
        *slot = (own[i / 8] & seen[i / 8]) & (1 << (i % 8)) != 0;
    }
    v
}

/// Species caught (owned flags), the number Oak's National Dex check compares with 60
/// (`decompiled/data/maps/PalletTown/scripts.inc:53-57`: `GetPokedexCount`, `VAR_0x8006`, the
/// caught count; `decompiled/src/field_specials.c` `GetPokedexCount`).
pub fn caught_count(s: &mut Session) -> u32 {
    owned_set(s).iter().filter(|&&b| b).count() as u32
}

/// The dex species index and national number of a game species id.
fn dex_species(s: &Session, game_id: u16) -> Option<(usize, u32)> {
    let name = names().species(game_id).to_string();
    let dex = s.plan.dex;
    dex.species
        .iter()
        .position(|sp| sp.name == name)
        .map(|i| (i, dex.species[i].dex))
}

/// The wild mon: `gEnemyParty[0]` (species and level; set before the battle starts,
/// `CreateWildMon`, `decompiled/src/wild_encounter.c`).
fn wild_mon(s: &mut Session) -> (u16, u8) {
    let a = s.sym("gEnemyParty");
    let m = read_mon(s.hf.rec.emu(), a);
    (m.species, m.level)
}

/// `gNumSafariBalls`, `gSafariZoneStepCounter` (`decompiled/src/safari_zone.c:9-10`).
pub fn safari_counters(s: &mut Session) -> (u8, u16) {
    let b = s.sym("gNumSafariBalls");
    let c = s.sym("gSafariZoneStepCounter");
    let emu = s.hf.rec.emu();
    (emu.read8(b), emu.read16(c))
}

/// `gBattleStruct->safariRockThrowCounter, safariBaitThrowCounter, safariEscapeFactor,
/// safariCatchFactor` at 0x79-0x7C of `struct BattleStruct` (`decompiled/include/battle.h:372-408`:
/// the fields before them end with `runTries` 0x6C and `caughtMonNick[11]` 0x6D-0x77, `field_78`).
fn safari_factors(s: &mut Session) -> [u8; 4] {
    let p = s.sym("gBattleStruct");
    let emu = s.hf.rec.emu();
    let bs = emu.read32(p);
    std::array::from_fn(|i| emu.read8(bs + 0x79 + i as u32))
}

fn short(name: &str) -> &str {
    name.trim_start_matches("SPECIES_")
        .trim_start_matches("ITEM_")
}

// ----------------------------------------------------------------------------------------------
// Wild encounters.

/// The site's species the human still needs: not owned, not skipped (`sim::wants_catch`).
fn needed_at(s: &mut Session, slots: &[crate::data::Slot], skip: &[usize]) -> Vec<String> {
    let own = owned_set(s);
    let mut v: Vec<String> = vec![];
    for sl in slots {
        let sp = &s.plan.dex.species[sl.species];
        if sim::wants_catch(&s.ctx, sl.species, own[sp.dex as usize], skip, true)
            && !v.contains(&sp.name)
        {
            v.push(sp.name.clone());
        }
    }
    v
}

/// What to do with the wild mon on screen. Outside a hunt: the stage's own plan (`default`). In a
/// hunt: catch exactly when [`sim::wants_catch`] says so for this species (new, not skipped).
/// Logged as a `decision` with the numbers behind it.
pub fn wild_plan(s: &mut Session, default: WildPlan) -> WildPlan {
    let Some(h) = s.dex.hunt.clone() else {
        return pass_plan(s, default);
    };
    let (game_id, level) = wild_mon(s);
    let Some((idx, national)) = dex_species(s, game_id) else {
        return default;
    };
    let is_owned = owned(s, national);
    let want = sim::wants_catch(&s.ctx, idx, is_owned, &h.skip, true);
    let sp = &s.plan.dex.species[idx];
    let (name, rate) = (sp.name.clone(), sp.catch_rate);
    let why = if national == 0 {
        "not in the Kanto dex"
    } else if is_owned {
        "already owned"
    } else if h.skip.contains(&idx) {
        "on the plan's skip list"
    } else {
        "new"
    };
    if let Some(r) = s.dex.hunt.as_mut() {
        r.encounters += 1;
    }
    let caught_n = caught_count(s);
    s.event(
        "decision",
        format!(
            "wild {} Lv{level} at {}: {} ({why}, catch rate {rate})",
            short(&name),
            short(&h.site).trim_start_matches("MAP_"),
            if want { "catch" } else { "run" }
        ),
        json!({"species": name, "level": level, "catch": want, "reason": why, "catch_rate": rate,
               "owned_total": caught_n, "site": h.site}),
    );
    if want {
        WildPlan::Catch
    } else {
        WildPlan::Flee
    }
}

/// Balls in the bag the human throws at a passing mon (the Master Ball is kept for Mewtwo).
pub fn ball_count(s: &mut Session) -> u16 {
    let master = id("ITEM_MASTER_BALL");
    Ram::new(&s.obs)
        .pocket(s.hf.rec.emu(), 2)
        .iter()
        .filter(|&&(it, _)| it != master)
        .map(|&(_, n)| n)
        .sum()
}

/// A passing encounter outside a hunt (the stage's own plan is `Flee`): the sim's rule for
/// encounters on the way (`sim::run_encounters`: `want_catch` = `catch_on_pass` and the species'
/// catch rate at least `pass_min_catch`, never on the plan's `pass_off` segments), through the
/// same [`sim::wants_catch`] as a hunt, with the Pokédex flags from RAM. The controller adds the
/// one thing the sim abstracts: a ball in the bag (Safari Balls in the Safari Zone).
fn pass_plan(s: &mut Session, default: WildPlan) -> WildPlan {
    if default != WildPlan::Flee {
        return default;
    }
    let (game_id, level) = wild_mon(s);
    let Some((idx, national)) = dex_species(s, game_id) else {
        return default;
    };
    let p = &s.plan.params;
    let sp = &s.plan.dex.species[idx];
    let (name, rate) = (sp.name.clone(), sp.catch_rate);
    let allowed = p.catch_on_pass && rate >= p.pass_min_catch && !p.pass_off.contains(&s.seg);
    let is_owned = owned(s, national);
    if !sim::wants_catch(&s.ctx, idx, is_owned, &[], allowed) {
        return default;
    }
    let flags = s.obs.battle_type_flags(s.hf.rec.emu());
    let safari = flags & (1 << 7) != 0;
    let balls = if safari {
        safari_counters(s).0 as u16
    } else {
        ball_count(s)
    };
    let caught_n = caught_count(s);
    if balls == 0 {
        s.event(
            "model-vs-actual",
            format!(
                "passing {} Lv{level} (new, catch rate {rate}): no ball in the bag, fled",
                short(&name)
            ),
            json!({"species": name, "level": level, "catch_rate": rate, "seg": s.seg}),
        );
        return default;
    }
    s.event(
        "decision",
        format!(
            "passing {} Lv{level} on {}: catch (new, catch rate {rate} >= pass_min_catch {}; {balls} balls)",
            short(&name),
            s.seg,
            p.pass_min_catch
        ),
        json!({"species": name, "level": level, "catch": true, "reason": "pass", "catch_rate": rate,
               "owned_total": caught_n, "balls": balls, "seg": s.seg}),
    );
    WildPlan::Catch
}

/// After a wild battle of a hunt: count the catch.
pub fn after_wild(s: &mut Session, end: BattleEnd, species: u16) {
    if end != BattleEnd::Caught {
        return;
    }
    let name = names().species(species).to_string();
    let safari = match s.dex.hunt.as_mut() {
        Some(h) => {
            h.caught.push(name.clone());
            h.safari
        }
        None => false,
    };
    if safari {
        s.dex.safari_new += 1;
    }
    let n = caught_count(s);
    s.event(
        "catch",
        format!("caught {}: {n} species owned", short(&name)),
        json!({"species": name, "owned_total": n, "to_60": 60i64 - n as i64}),
    );
}

/// The field's battle entry for a wild battle: decide catch/run ([`wild_plan`]), play it (the
/// Safari driver below or `play::battle::fight`), count the catch. Returns `None` when this is not
/// a wild battle (the caller plays it as before).
pub fn wild_battle(s: &mut Session) -> Option<PResult<()>> {
    let flags = s.obs.battle_type_flags(s.hf.rec.emu());
    if flags & frlg_route::observe::BATTLE_TYPE_TRAINER != 0 {
        return None;
    }
    let (species, _) = wild_mon(s);
    let mut plan = wild_plan(s, s.wild_plan);
    // Arena Trap (Diglett, Dugtrio in Kanto) blocks RUN for a grounded player mon
    // (`decompiled/src/battle_main.c:3031,3203`): "Can't escape!" every turn, and the flee loop
    // spun forever in the v3 fill (Diglett's Cave B1F). The human knocks it out instead.
    if plan == WildPlan::Flee {
        let name = names().species(species);
        if name == "SPECIES_DIGLETT" || name == "SPECIES_DUGTRIO" {
            s.event(
                "decision",
                format!("wild {}: Arena Trap, cannot run: fighting", short(name)),
                json!({"species": name}),
            );
            plan = WildPlan::Fight;
        }
    }
    let safari = flags & (1 << 7) != 0; // BATTLE_TYPE_SAFARI, `include/constants/battle.h:52`
    if safari {
        return Some(safari_battle(s, plan == WildPlan::Catch));
    }
    let cfg = super::battle::BattleCfg {
        wild: plan,
        ..super::battle::BattleCfg::new(&s.ctx, &s.seg)
    };
    let rep = match super::battle::fight(&mut s.hf, &s.obs, &mut s.log, &cfg) {
        Ok(r) => r,
        Err(e) => return Some(Err(e.into())),
    };
    if rep.end == BattleEnd::Lost {
        return Some(Err(PlayError::Death(format!(
            "blacked out in a wild battle ({})",
            s.seg
        ))));
    }
    after_wild(s, rep.end, species);
    Some(Ok(()))
}

/// The Safari action menu takes input: `gBattlerControllerFuncs[0]` is the Safari controller's
/// `HandleInputChooseAction` (`decompiled/src/battle_controller_safari.c:162`; the sym file has
/// four functions of that name, the Safari one is 0x184 bytes).
fn safari_menu(s: &mut Session) -> bool {
    let a = s.sym("gBattlerControllerFuncs");
    let want = s.sym_sized("HandleInputChooseAction", 0x184);
    s.hf.rec.emu().read32(a) & !1 == want
}

/// Put the Safari menu cursor (`gActionSelectionCursor[0]`, 2 x 2: bit 0 column, bit 1 row) on
/// `want` and press A until the menu lets go.
fn safari_pick(s: &mut Session, want: u8) -> PResult<()> {
    for _ in 0..6 {
        let cur = s.obs.action_cursor(s.hf.rec.emu(), 0);
        if cur == want {
            break;
        }
        let k = if (cur & 1) != (want & 1) {
            if want & 1 != 0 {
                keys::RIGHT
            } else {
                keys::LEFT
            }
        } else if want & 2 != 0 {
            keys::DOWN
        } else {
            keys::UP
        };
        s.hf.press(k)?;
        s.hf.idle(1)?;
    }
    super::prims::press_until(s, "the Safari choice", keys::A, 120, |s| !safari_menu(s))
}

/// A Safari Zone battle (`catch`: the plan wants it, else RUN): the throw order of [`sim::SafariMon::next_act`] on
/// the BALL / BAIT / ROCK / RUN menu (`gActionSelectionCursor`, 2 x 2,
/// `decompiled/src/battle_controller_safari.c:162-220`), text advanced with B (B answers NO to
/// the nickname prompt). Each throw is logged with the real factors from RAM.
pub fn safari_battle(s: &mut Session, catch: bool) -> PResult<()> {
    let (species, level) = wild_mon(s);
    let Some((idx, _)) = dex_species(s, species) else {
        return Err(PlayError::Fatal(format!(
            "species {species} not in the dex"
        )));
    };
    let sp = &s.plan.dex.species[idx];
    let mut m = SafariMon::new(&s.ctx, sp);
    let name = sp.name.clone();
    let start = s.hf.fed();
    let mut throws = 0u32;
    let mut reacted = false;
    loop {
        if s.hf.fed() - start > 40_000 {
            let a = s.sym("gBattlerControllerFuncs");
            let f = s.hf.rec.emu().read32(a);
            return Err(PlayError::Death(format!(
                "soft-lock: Safari battle (controller {})",
                s.obs.symbols().describe(f)
            )));
        }
        let emu = s.hf.rec.emu();
        if !s.obs.in_battle(emu) && s.obs.callback2_is(emu, "CB2_Overworld") {
            break;
        }
        if safari_menu(s) {
            if !reacted {
                // The human sees what came out before the first choice.
                s.hf.react()?;
                reacted = true;
            }
            if !catch {
                // RUN: always succeeds in the Safari (`HandleAction_SafariZoneRun`,
                // `decompiled/src/battle_main.c:4414`).
                safari_pick(s, 3)?;
                continue;
            }
            let act = m.next_act();
            let want: u8 = match act {
                SafariAct::Ball => 0,
                SafariAct::Bait => 1,
                SafariAct::Rock => 2,
            };
            let [rock, bait, esc, factor] = safari_factors(s);
            let (balls, steps) = safari_counters(s);
            throws += 1;
            let chance = sim::catch_prob(sim::safari_ball_odds(factor as u32));
            s.event(
                "catch",
                format!(
                    "Safari {:?} #{throws} at {} Lv{level}: catch factor {factor}, flee factor {esc}, ball chance {chance:.2}, {balls} balls left",
                    act,
                    short(&name)
                ),
                json!({"act": format!("{act:?}"), "throw": throws, "species": name, "catch_factor": factor,
                       "escape_factor": esc, "rock_counter": rock, "bait_counter": bait,
                       "chance": chance, "balls": balls, "steps_left": steps,
                       "model_factor": m.factor, "model_escape": m.esc}),
            );
            if throws == 1 && (factor as u32 != m.base || esc as u32 != m.esc) {
                s.event(
                    "model-vs-actual",
                    format!(
                        "Safari factors in RAM ({factor}, {esc}) differ from the model's ({}, {})",
                        m.base, m.esc
                    ),
                    json!({"ram": [factor, esc], "model": [m.base, m.esc]}),
                );
            }
            safari_pick(s, want)?;
            continue;
        }
        // Text, animations, the nickname prompt: B, late by U{0..N}.
        s.hf.late()?;
        if safari_menu(s) || {
            let emu = s.hf.rec.emu();
            !s.obs.in_battle(emu) && s.obs.callback2_is(emu, "CB2_Overworld")
        } {
            continue;
        }
        s.hf.step(keys::B)?;
        s.hf.step(0)?;
    }
    let outcome = s.obs.battle_outcome(s.hf.rec.emu());
    let end = match outcome {
        7 => BattleEnd::Caught,
        4 => BattleEnd::Ran,
        o => BattleEnd::Other(o),
    };
    let (balls, steps) = safari_counters(s);
    s.event(
        "battle",
        format!(
            "Safari battle over: {} after {throws} throws ({balls} balls, {steps} steps left)",
            match outcome {
                7 => "caught".to_string(),
                6 => "it fled".to_string(),
                8 => "out of balls".to_string(),
                o => format!("outcome {o}"),
            }
        ),
        json!({"outcome": outcome, "throws": throws, "balls": balls, "steps": steps,
               "frames": s.hf.fed() - start}),
    );
    after_wild(s, end, species);
    settle(s)
}

// ----------------------------------------------------------------------------------------------
// Hunts.

/// Fishing rods in the bag, best first (`ITEM_SUPER_ROD`, `ITEM_GOOD_ROD`, `ITEM_OLD_ROD`), with the
/// plan's word for each (`Params::rod`, `Dex::area`).
fn best_rod(s: &mut Session) -> Option<(&'static str, &'static str)> {
    let ram = Ram::new(&s.obs);
    [
        ("ITEM_SUPER_ROD", "super"),
        ("ITEM_GOOD_ROD", "good"),
        ("ITEM_OLD_ROD", "old"),
    ]
    .into_iter()
    .find(|(it, _)| ram.count(s.hf.rec.emu(), id(it)) > 0)
}

/// Water behaviors (`walk.rs`'s surfable set, `metatile_behaviors.h` 0x10-0x22).
fn is_water(b: u16) -> bool {
    [0x10, 0x11, 0x12, 0x13, 0x15, 0x19, 0x1A, 0x1B, 0x22].contains(&b)
}

/// Candidate pacing pairs on `map`: two neighbouring walkable tiles that both roll land encounters
/// (`Tile::land`, the metatile's encounter type) -- or water for a water hunt -- nearest to
/// `from` first, away from people.
fn pace_pairs(s: &Session, map: &str, water: bool, from: (i16, i16)) -> Vec<[(i16, i16); 2]> {
    let g = s.graph();
    let m = &g.maps[&g.id(map)];
    let busy: Vec<(i16, i16)> = m.objects.iter().map(|o| (o.x, o.y)).collect();
    let ok = |x: i16, y: i16| {
        m.tile(x, y).is_some_and(|t| {
            if water {
                is_water(t.behavior)
            } else {
                t.land && t.collision == 0 && t.behavior < 0x38
            }
        }) && !busy
            .iter()
            .any(|&(ox, oy)| (ox - x).abs() + (oy - y).abs() <= 1)
    };
    // Only tiles connected to where the player stands on this map (a flood fill over the
    // layout's passable tiles, water included when surfing). An unreachable pair made every
    // `go` a whole-world search: the v3 fill spent 20+ CPU minutes in Diglett's Cave B1F.
    let mut reach = vec![false; (m.w as usize) * (m.h as usize)];
    let pass = |x: i16, y: i16| {
        m.tile(x, y)
            .is_some_and(|t| t.collision == 0 && (water || !is_water(t.behavior)))
    };
    let mut stack = vec![from];
    while let Some((x, y)) = stack.pop() {
        if x < 0 || y < 0 || x >= m.w || y >= m.h {
            continue;
        }
        let i = (x as usize) + (y as usize) * (m.w as usize);
        if reach[i] || !(pass(x, y) || (x, y) == from) {
            continue;
        }
        reach[i] = true;
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            stack.push((x + dx, y + dy));
        }
    }
    // A start the layout itself calls blocked (a connection edge, Route 23 (11, 0) from the
    // plateau) floods nothing: then every pair stays a candidate, as before.
    let flooded = reach.iter().filter(|&&b| b).count() >= 20;
    let reachable = |x: i16, y: i16| {
        !flooded
            || x >= 0
                && y >= 0
                && x < m.w
                && y < m.h
                && reach[(x as usize) + (y as usize) * (m.w as usize)]
    };
    let mut v = vec![];
    for y in 0..m.h {
        for x in 0..m.w {
            if !ok(x, y) || !reachable(x, y) {
                continue;
            }
            for (dx, dy) in [(1, 0), (0, 1)] {
                if ok(x + dx, y + dy) && reachable(x + dx, y + dy) {
                    v.push([(x, y), (x + dx, y + dy)]);
                }
            }
        }
    }
    v.sort_by_key(|p| (p[0].0 - from.0).abs() + (p[0].1 - from.1).abs());
    v
}

/// A fishing spot on `map`: a walkable tile next to water, and the direction facing it.
fn fish_spots(s: &Session, map: &str, from: (i16, i16)) -> Vec<(i16, i16, u16)> {
    let g = s.graph();
    let m = &g.maps[&g.id(map)];
    let mut v = vec![];
    for y in 0..m.h {
        for x in 0..m.w {
            let Some(t) = m.tile(x, y) else { continue };
            if t.collision != 0 || is_water(t.behavior) {
                continue;
            }
            for (dx, dy, k) in [
                (0, -1, keys::UP),
                (0, 1, keys::DOWN),
                (-1, 0, keys::LEFT),
                (1, 0, keys::RIGHT),
            ] {
                if m.tile(x + dx, y + dy).is_some_and(|w| is_water(w.behavior)) {
                    v.push((x, y, k));
                    break;
                }
            }
        }
    }
    v.sort_by_key(|&(x, y, _)| (x - from.0).abs() + (y - from.1).abs());
    v
}

/// Walk to the first reachable of `cands` (planner refusals are tried down the list).
fn go_first(s: &mut Session, map: &str, cands: &[(i16, i16)], opts: &Opts) -> PResult<usize> {
    let mut last = None;
    for (i, &(x, y)) in cands.iter().enumerate().take(8) {
        match go(s, map, x, y, opts) {
            Ok(()) => return Ok(i),
            Err(PlayError::Fatal(e)) if e.contains("no path") => last = Some(e),
            Err(PlayError::Death(e)) if e.contains("goal tile bumped") => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(PlayError::Fatal(format!(
        "no reachable hunting spot on {map}: {}",
        last.unwrap_or_default()
    )))
}

/// Fish once with the registered rod: SELECT, then watch. FRLG has no reel input: a bite goes
/// straight to "on the hook" (`Fishing7` skips to `FISHING_ON_HOOK`,
/// `decompiled/src/field_player_avatar.c:1798-1802`), so the human only waits for the dots
/// (4-10 x 20 frames, `Fishing4`/`Fishing5`) and the 50% bite (`Fishing6`, `Random() & 1`).
fn cast(s: &mut Session) -> PResult<()> {
    settle(s)?;
    s.hf.press(keys::SELECT)?;
    wait_until(s, "the rod cast", 120, |s| menu::task(s, "Task_Fishing"))?;
    // The dots take no input; the result text ("on the hook!" / "Not even a nibble...") waits
    // for A. A is mashed throughout (`Fishing5` ignores it).
    super::prims::press_until(s, "the fishing result", keys::A, 1200, |s| {
        !menu::task(s, "Task_Fishing")
    })?;
    // A bite starts the battle (`FishingWildEncounter`); otherwise "Not even a nibble...".
    settle(s)
}

/// Register `item` to SELECT (key items context menu row 1 = REGISTER, as `items::register_bike`).
fn register(s: &mut Session, item: &str) -> PResult<()> {
    let it = id(item);
    if super::items::registered_item(s) == it {
        return Ok(());
    }
    super::items::open_bag(s)?;
    super::items::bag_use(s, it, 1)?;
    s.hf.idle(10)?;
    super::items::close_to_field(s)?;
    s.event(
        "decision",
        format!("registered {} to SELECT", short(item)),
        json!({"item": item}),
    );
    Ok(())
}

/// Play one hunt of the plan: walk to the site, pace (land/water) or fish, catch what
/// [`wild_plan`] says, and leave on the first of: the site complete ([`sim::site_done`]), the visit's
/// cap ([`sim::hunt_stop_new`]), the step budget (`steps`, casts for fishing), the Safari's steps or
/// balls running out.
pub fn hunt(s: &mut Session, h: &Hunt) -> PResult<HuntReport> {
    let t0 = s.frame();
    let area = sim::hunt_area(h);
    let skip = sim::hunt_skip(&s.ctx, h);
    let safari = sim::is_safari_hunt(&s.ctx, h);
    let mut rep = HuntReport {
        site: h.site.clone(),
        ..Default::default()
    };
    if safari && h.on_route && s.dex.safari_done {
        rep.why = "skipped".into();
        s.event(
            "decision",
            format!(
                "hunt {}: skipped, the Safari stop ({}) was reached",
                h.site, s.plan.params.safari_stop
            ),
            json!({"site": h.site}),
        );
        return Ok(rep);
    }
    let stop_new = sim::hunt_stop_new(&s.ctx, h, s.dex.safari_new);
    let lg = s.plan.params.version == "LG";
    let rod = if area == Area::Fish {
        let Some((item, word)) = best_rod(s) else {
            s.event(
                "model-vs-actual",
                format!(
                    "fishing hunt at {} without a rod in the bag: skipped",
                    h.site
                ),
                json!({"site": h.site}),
            );
            rep.why = "no rod".into();
            return Ok(rep);
        };
        if word != s.plan.params.rod {
            s.event(
                "model-vs-actual",
                format!(
                    "fishing with the {} rod; the plan's tables use rod '{}'",
                    word, s.plan.params.rod
                ),
                json!({"rod": item, "plan_rod": s.plan.params.rod}),
            );
        }
        Some((item, word))
    } else {
        None
    };
    let rod_word = rod.map_or(s.plan.params.rod.clone(), |r| r.1.to_string());
    let Some((rate, slots, _w)) = s.plan.dex.area(&h.site, area, lg, &rod_word) else {
        return Err(PlayError::Fatal(format!(
            "no {:?} encounters at {}",
            area, h.site
        )));
    };
    let need = needed_at(s, &slots, &skip);
    let owned0 = caught_count(s);
    s.event(
        "decision",
        format!(
            "hunt at {} ({}): need {} [{}], budget {} {}, stop after {}",
            h.site.trim_start_matches("MAP_"),
            h.area,
            need.len(),
            need.iter().map(|n| short(n)).collect::<Vec<_>>().join(", "),
            h.steps,
            if area == Area::Fish { "casts" } else { "steps" },
            if stop_new == 0 {
                "the site".to_string()
            } else {
                format!("{stop_new} new")
            }
        ),
        json!({"site": h.site, "area": h.area, "needed": need, "steps": h.steps, "stop_new": stop_new,
               "rate": rate, "owned_total": owned0, "safari_new": s.dex.safari_new}),
    );
    if need.is_empty() {
        rep.why = "complete".into();
        return Ok(rep);
    }
    // No ball to throw (the Master Ball is kept): the hunt would weaken and flee from every new
    // species (seen: Bond Bridge, 9 encounters, 166 s, nothing caught). The human skips it.
    if !safari {
        let master = id("ITEM_MASTER_BALL");
        let balls: u16 = Ram::new(&s.obs)
            .pocket(s.hf.rec.emu(), 2)
            .iter()
            .filter(|&&(it, _)| it != master)
            .map(|&(_, n)| n)
            .sum();
        if balls == 0 {
            s.event(
                "model-vs-actual",
                format!(
                    "hunt at {}: skipped, no ball in the bag (the plan buys none before it)",
                    h.site.trim_start_matches("MAP_")
                ),
                json!({"site": h.site, "needed": need}),
            );
            rep.why = "no balls".into();
            return Ok(rep);
        }
    }
    s.dex.hunt = Some(HuntRun {
        site: h.site.clone(),
        skip: skip.clone(),
        safari,
        ..Default::default()
    });
    let res = hunt_loop(s, h, area, rod, &slots, &skip, stop_new, &mut rep);
    let run = s.dex.hunt.take().unwrap_or_default();
    res?;
    rep.encounters = run.encounters;
    rep.caught = run.caught;
    rep.frames = s.frame() - t0;
    if safari && s.plan.params.safari_stop > 0 && s.dex.safari_new >= s.plan.params.safari_stop {
        s.dex.safari_done = true;
    }
    // The model: `per_step` frames per step (run speed x path overhead) or 180 per cast (a guess),
    // plus the encounters; compare the whole hunt per step.
    let per = if rep.steps > 0 {
        (rep.frames - rep.walk_frames) as f64 / rep.steps as f64
    } else {
        0.0
    };
    let model_per = if area == Area::Land {
        crate::route::Era::Run.frames_per_tile() * s.plan.params.path_overhead
    } else {
        180.0
    };
    let left = needed_at(s, &slots, &skip);
    let owned_now = caught_count(s);
    s.event(
        "decision",
        format!(
            "hunt at {} done ({}): {} {}, {} encounters, caught [{}], {} frames ({} walking there, then {per:.0}/{} incl. battles; model {model_per:.0} + encounters), still needed [{}]",
            h.site.trim_start_matches("MAP_"),
            rep.why,
            rep.steps,
            if area == Area::Fish { "casts" } else { "steps" },
            rep.encounters,
            rep.caught.iter().map(|n| short(n)).collect::<Vec<_>>().join(", "),
            rep.frames,
            rep.walk_frames,
            if area == Area::Fish { "cast" } else { "step" },
            left.iter().map(|n| short(n)).collect::<Vec<_>>().join(", ")
        ),
        json!({"site": h.site, "why": rep.why, "steps": rep.steps, "encounters": rep.encounters,
               "caught": rep.caught, "frames": rep.frames, "walk_frames": rep.walk_frames,
               "frames_per_step": per,
               "model_walk_per_step": model_per, "still_needed": left,
               "owned_total": owned_now, "safari_new": s.dex.safari_new}),
    );
    Ok(rep)
}

#[allow(clippy::too_many_arguments)]
fn hunt_loop(
    s: &mut Session,
    h: &Hunt,
    area: Area,
    rod: Option<(&str, &str)>,
    slots: &[crate::data::Slot],
    skip: &[usize],
    stop_new: u32,
    rep: &mut HuntReport,
) -> PResult<()> {
    let t0 = s.frame();
    let water = area == Area::Water;
    let opts = Opts {
        // Surf on the way when a party mon knows it (Route 23's grass is past water).
        surf: water || super::items::slot_with_move(s, "MOVE_SURF").is_some(),
        cut: true,
        ..Opts::default()
    };
    go_map(s, &h.site, &opts)?;
    let site = s.graph().id(&h.site);
    let me = here(s).map_or((0, 0), |n| (n.1, n.2));
    // Where the human hunts: two neighbouring grass (or water) tiles, or a spot facing water.
    let mut pair = [(0, 0); 2];
    if area == Area::Fish {
        let spots = fish_spots(s, &h.site, me);
        let cands: Vec<(i16, i16)> = spots.iter().map(|&(x, y, _)| (x, y)).collect();
        let i = go_first(s, &h.site, &cands, &opts)?;
        face(s, spots[i].2)?;
        register(s, rod.unwrap().0)?;
        // Registering opened the bag: face the water again.
        face(s, spots[i].2)?;
    } else {
        let pairs = pace_pairs(s, &h.site, water, me);
        let cands: Vec<(i16, i16)> = pairs.iter().map(|p| p[0]).collect();
        let i = go_first(s, &h.site, &cands, &opts)?;
        pair = pairs[i];
    }
    rep.walk_frames = s.frame() - t0;
    let caught0 = s.dex.hunt.as_ref().map_or(0, |r| r.caught.len()) as u32;
    loop {
        let own = owned_set(s);
        if sim::site_done(&s.ctx, slots, &|d| own[d as usize], skip) {
            rep.why = "complete".into();
            break;
        }
        let new = s.dex.hunt.as_ref().map_or(0, |r| r.caught.len()) as u32 - caught0;
        if stop_new > 0 && new >= stop_new {
            rep.why = "stop_after".into();
            break;
        }
        if rep.steps >= h.steps {
            rep.why = "budget".into();
            break;
        }
        if s.dex.hunt.as_ref().is_some_and(|r| r.safari) {
            let (balls, steps) = safari_counters(s);
            let inside =
                here(s).is_some_and(|n| s.graph().maps[&n.0].id.starts_with("MAP_SAFARI_ZONE"));
            if balls == 0 || !inside {
                rep.why = "safari balls".into();
                break;
            }
            if steps == 0 {
                rep.why = "safari steps".into();
                break;
            }
        }
        if here(s).map(|n| n.0) != Some(site) {
            // A whiteout, the Safari's "time's up", or a script moved the player.
            rep.why = "left the site".into();
            break;
        }
        if area == Area::Fish {
            cast(s)?;
        } else {
            // A pacing tile the walker bumped into (the layout calls it walkable, the game does
            // not: Victory Road 2F (33, 17), 57 bumps in one hunt of the 11-22 run): pace
            // somewhere else.
            let bumped = |s: &Session, t: (i16, i16)| s.bumped.contains_key(&(site, t.0, t.1));
            if bumped(s, pair[0]) || bumped(s, pair[1]) {
                let me = here(s).map_or((0, 0), |n| (n.1, n.2));
                let pairs: Vec<[(i16, i16); 2]> = pace_pairs(s, &h.site, water, me)
                    .into_iter()
                    .filter(|p| !bumped(s, p[0]) && !bumped(s, p[1]))
                    .collect();
                let cands: Vec<(i16, i16)> = pairs.iter().map(|p| p[0]).collect();
                let i = go_first(s, &h.site, &cands, &opts)?;
                s.event(
                    "warn",
                    format!(
                        "hunt: pacing pair {:?} bumped; pacing {:?} instead",
                        pair, pairs[i]
                    ),
                    json!({"site": h.site}),
                );
                pair = pairs[i];
            }
            let cur = here(s).map(|n| (n.1, n.2));
            let target = if cur == Some(pair[0]) {
                pair[1]
            } else {
                pair[0]
            };
            match go(s, &h.site, target.0, target.1, &opts) {
                Err(PlayError::Death(m)) if m.contains("goal tile bumped") => continue,
                r => r?,
            }
        }
        // The plan's `steps` are the sim's encounter rolls (one per step). Pacing a pair of tiles
        // rolls twice per tile, at the turn and at the step (`checkStandardWildEncounter` runs at
        // every tile centre, turns included: `decompiled/src/field_control_avatar.c:138-141`).
        rep.steps += if area == Area::Fish || rep.steps == 0 {
            1
        } else {
            2
        };
    }
    Ok(())
}

// ----------------------------------------------------------------------------------------------
// Specials and evolutions.

/// Where each special of the plan happens: species -> (map, the object's script, kind). Scripts:
/// `decompiled/data/maps/<map>/scripts.inc` (gifts `givemon`, purchases, in-game trades
/// `EventScript_ChooseMonForInGameTrade` + `DoInGameTrade`, `INGAME_TRADE_*` of
/// `include/constants/trade.h`).
const SPECIALS: &[(&str, &str, &str)] = &[
    (
        "SPECIES_EEVEE",
        "MAP_CELADON_CITY_CONDOMINIUMS_ROOF_ROOM",
        "CeladonCity_Condominiums_RoofRoom_EventScript_EeveeBall",
    ),
    (
        "SPECIES_LAPRAS",
        "MAP_SILPH_CO_7F",
        "SilphCo_7F_EventScript_LaprasGuy",
    ),
    (
        "SPECIES_HITMONLEE",
        "MAP_SAFFRON_CITY_DOJO",
        "SaffronCity_Dojo_EventScript_HitmonleeBall",
    ),
    (
        "SPECIES_MAGIKARP",
        "MAP_ROUTE4_POKEMON_CENTER_1F",
        "Route4_PokemonCenter_1F_EventScript_MagikarpSalesman",
    ),
    (
        "SPECIES_MR_MIME",
        "MAP_ROUTE2_HOUSE",
        "Route2_House_EventScript_Reyley",
    ),
    (
        "SPECIES_ELECTRODE",
        "MAP_CINNABAR_ISLAND_POKEMON_LAB_LOUNGE",
        "CinnabarIsland_PokemonLab_Lounge_EventScript_Clifton",
    ),
    (
        "SPECIES_TANGELA",
        "MAP_CINNABAR_ISLAND_POKEMON_LAB_LOUNGE",
        "CinnabarIsland_PokemonLab_Lounge_EventScript_Norma",
    ),
    (
        "SPECIES_NIDORAN_M",
        "MAP_UNDERGROUND_PATH_NORTH_ENTRANCE",
        "UndergroundPath_NorthEntrance_EventScript_Saige",
    ),
    (
        "SPECIES_FARFETCHD",
        "MAP_VERMILION_CITY_HOUSE2",
        "VermilionCity_House2_EventScript_Elyssa",
    ),
    // `INGAME_TRADE_NIDORINOA` (LG Nidorino, FR Nidorina), `Route11_EastEntrance_2F/scripts.inc:22-37`.
    (
        "SPECIES_NIDORINO",
        "MAP_ROUTE11_EAST_ENTRANCE_2F",
        "Route11_EastEntrance_2F_EventScript_Turner",
    ),
    (
        "SPECIES_NIDORINA",
        "MAP_ROUTE11_EAST_ENTRANCE_2F",
        "Route11_EastEntrance_2F_EventScript_Turner",
    ),
    // `INGAME_TRADE_LICKITUNG`, `Route18_EastEntrance_2F/scripts.inc:12-27`.
    (
        "SPECIES_LICKITUNG",
        "MAP_ROUTE18_EAST_ENTRANCE_2F",
        "Route18_EastEntrance_2F_EventScript_Haden",
    ),
    // `INGAME_TRADE_JYNX`, `CeruleanCity_House3/scripts.inc:11`.
    (
        "SPECIES_JYNX",
        "MAP_CERULEAN_CITY_HOUSE3",
        "CeruleanCity_House3_EventScript_Dontae",
    ),
];

/// The local id of the object on `map` running `script` (`walk::ObjInfo::local_id`).
fn object_with_script(s: &Session, map: &str, script: &str) -> Option<u8> {
    let g = s.graph();
    g.maps[&g.id(map)]
        .objects
        .iter()
        .find(|o| o.script == script)
        .map(|o| o.local_id)
}

/// Party slot holding `species` (game id).
pub fn party_slot(s: &mut Session, species: u16) -> Option<usize> {
    Ram::new(&s.obs)
        .party(s.hf.rec.emu())
        .iter()
        .position(|m| m.species == species)
}

/// Run an NPC's script to its end, answering its yes/no boxes the human way: YES while the mon
/// has not been received yet (take it, buy it, trade), NO once it is owned (the nickname
/// prompt); a party menu (the trade's "which mon?") gets `trade_slot`.
pub(super) fn run_npc(s: &mut Session, got: u32, trade_slot: Option<usize>) -> PResult<()> {
    let start = s.hf.fed();
    loop {
        if s.hf.fed() - start > 20_000 {
            return Err(PlayError::Death("soft-lock: NPC script".into()));
        }
        if is_free(s) {
            return settle(s);
        }
        if menu::yesno_up(s) {
            let have = owned(s, got);
            menu::answer(s, !have)?;
            continue;
        }
        if menu::task(s, "Task_HandleChooseMonInput") {
            let slot = trade_slot.unwrap_or(0) as i8;
            super::items::party_choose(s, slot)?;
            continue;
        }
        if s.obs.callback2_is(s.hf.rec.emu(), "CB2_NamingScreen") {
            s.hf.press(keys::START)?;
            s.hf.press(keys::A)?;
            continue;
        }
        s.hf.late()?;
        if is_free(s) || menu::yesno_up(s) {
            continue;
        }
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
}

/// Play one of the plan's `specials` (gift, in-game trade, purchase). Skipped (logged) when every
/// species is already owned, when a trade's `requires` is not in the party, or when the
/// controller has no script for it yet (static encounters, Game Corner prizes, fossils). Walks to
/// the NPC with play/walk.rs. Returns whether a new species was obtained.
pub fn special(s: &mut Session, sp: &Special) -> PResult<bool> {
    let n = names();
    let targets: Vec<(String, u32)> = sp
        .species
        .iter()
        .filter_map(|name| {
            let i = s.plan.dex.species.iter().position(|x| &x.name == name)?;
            Some((name.clone(), s.plan.dex.species[i].dex))
        })
        .collect();
    let own = owned_set(s);
    if !targets.is_empty() && targets.iter().all(|(_, d)| own[*d as usize]) {
        s.event(
            "decision",
            format!("special {} {:?}: already owned", sp.kind, sp.species),
            json!({"kind": sp.kind, "species": sp.species}),
        );
        return Ok(false);
    }
    let entry = sp
        .species
        .first()
        .and_then(|first| SPECIALS.iter().find(|e| e.0 == first));
    let Some(&(species, map, script)) = entry else {
        s.event(
            "model-vs-actual",
            format!(
                "special {} {:?} after {}: no controller script yet (skipped)",
                sp.kind, sp.species, sp.after
            ),
            json!({"kind": sp.kind, "species": sp.species, "after": sp.after}),
        );
        return Ok(false);
    };
    let got = targets.first().map_or(0, |t| t.1);
    let mut trade_slot = None;
    if sp.kind == "trade" {
        let req = sp.requires.first().map(|r| id(r));
        let slot = req.and_then(|r| party_slot(s, r));
        if slot.is_none() {
            s.event(
                "model-vs-actual",
                format!(
                    "trade for {species}: {:?} is not in the party (withdraw it first with shop::pc)",
                    sp.requires
                ),
                json!({"species": species, "requires": sp.requires}),
            );
            return Ok(false);
        }
        trade_slot = slot;
    }
    let Some(local) = object_with_script(s, map, script) else {
        return Err(PlayError::Fatal(format!("no object {script} on {map}")));
    };
    let money0 = Ram::new(&s.obs).money(s.hf.rec.emu());
    super::legs::talk_to(s, map, local)?;
    run_npc(s, got, trade_slot)?;
    let ok = owned(s, got);
    let money = Ram::new(&s.obs).money(s.hf.rec.emu());
    let owned_now = caught_count(s);
    s.event(
        if ok { "catch" } else { "model-vs-actual" },
        format!(
            "{} {}: {} ({} species owned)",
            sp.kind,
            short(species),
            if ok { "obtained" } else { "NOT obtained" },
            owned_now
        ),
        json!({"kind": sp.kind, "species": species, "obtained": ok, "money_spent": money0.saturating_sub(money),
               "gave": trade_slot, "owned_total": owned_now}),
    );
    let _ = n;
    Ok(ok)
}

/// Play one of the plan's `evolutions`: find the pre-evolution in the party ([`sim::evo_source`]),
/// and use the stone (`via: stone`) or Rare Candies (`via: candy`, one at a time until it evolves)
/// on it from the bag ([`super::shop::use_item_on`]). `via: level` is only checked (the level-up
/// happens in battle). Skipped (logged) when the target is owned, the pre-evolution is not in the
/// party, or the stone / candy is not in the bag (`buy: true` stones are bought with
/// [`super::shop::buy`] by the route, at the Celadon store's 4F). Returns whether it evolved.
pub fn evolve(s: &mut Session, evo: &Evolve) -> PResult<bool> {
    let n = names();
    let Some(ti) = s
        .plan
        .dex
        .species
        .iter()
        .position(|x| x.name == evo.species)
    else {
        return Err(PlayError::Fatal(format!("{} not in the dex", evo.species)));
    };
    let tdex = s.plan.dex.species[ti].dex;
    if owned(s, tdex) {
        s.event(
            "decision",
            format!("evolution to {}: already owned", short(&evo.species)),
            json!({"species": evo.species}),
        );
        return Ok(false);
    }
    let party = Ram::new(&s.obs).party(s.hf.rec.emu());
    let party_dex: Vec<u32> = party
        .iter()
        .map(|m| dex_species(s, m.species).map_or(0, |d| d.1))
        .collect();
    let src = sim::evo_source(&s.ctx, evo, &|d| party_dex.contains(&d));
    let Some((pre, param)) = src else {
        s.event(
            "model-vs-actual",
            format!(
                "evolution to {} ({}): no pre-evolution in the party",
                short(&evo.species),
                evo.via
            ),
            json!({"species": evo.species, "via": evo.via}),
        );
        return Ok(false);
    };
    let slot = party_dex.iter().position(|&d| d == pre).unwrap();
    let pre_name = n.species(party[slot].species).to_string();
    let target_id = id(&evo.species);
    match evo.via.as_str() {
        "stone" => {
            let item = sim::evo_item(evo).unwrap_or(param.clone());
            let have = Ram::new(&s.obs).count(s.hf.rec.emu(), id(&item));
            if have == 0 {
                s.event(
                    "model-vs-actual",
                    format!(
                        "evolution {} -> {}: no {} in the bag{}",
                        short(&pre_name),
                        short(&evo.species),
                        short(&item),
                        if evo.buy {
                            " (the plan buys it: shop::buy at the Celadon store 4F first)"
                        } else {
                            ""
                        }
                    ),
                    json!({"species": evo.species, "item": item, "buy": evo.buy}),
                );
                return Ok(false);
            }
            s.event(
                "decision",
                format!(
                    "evolve {} (slot {slot}) with a {} into {}",
                    short(&pre_name),
                    short(&item),
                    short(&evo.species)
                ),
                json!({"species": evo.species, "from": pre_name, "item": item, "slot": slot}),
            );
            super::shop::use_item_on(s, &item, slot)?;
        }
        "candy" => {
            let at: u32 = param.parse().unwrap_or(0);
            let lv = party[slot].level as u32;
            let need = if evo.candies > 0 {
                evo.candies
            } else {
                at.saturating_sub(lv).max(1)
            };
            let have = Ram::new(&s.obs).count(s.hf.rec.emu(), id("ITEM_RARE_CANDY")) as u32;
            s.event(
                "decision",
                format!(
                    "evolve {} Lv{lv} with Rare Candies into {} (Lv{at}: {need} candies, {have} in the bag)",
                    short(&pre_name),
                    short(&evo.species)
                ),
                json!({"species": evo.species, "from": pre_name, "need": need, "have": have, "level": lv}),
            );
            if have < need {
                s.event(
                    "model-vs-actual",
                    format!("not enough Rare Candies ({have} < {need})"),
                    json!({"have": have, "need": need}),
                );
                return Ok(false);
            }
            for _ in 0..need {
                let now = super::shop::use_item_on(s, "ITEM_RARE_CANDY", slot)?;
                if now == target_id {
                    break;
                }
            }
        }
        _ => {
            s.event(
                "model-vs-actual",
                format!(
                    "level evolution {} -> {} (Lv{param}) not owned: it happens in battle, the controller does not force it",
                    short(&pre_name),
                    short(&evo.species)
                ),
                json!({"species": evo.species, "from": pre_name, "level": param}),
            );
            return Ok(false);
        }
    }
    let ok = owned(s, tdex);
    let owned_now = caught_count(s);
    s.event(
        if ok { "catch" } else { "model-vs-actual" },
        format!(
            "{} -> {}: {} ({} species owned)",
            short(&pre_name),
            short(&evo.species),
            if ok { "evolved" } else { "did NOT evolve" },
            owned_now
        ),
        json!({"species": evo.species, "evolved": ok, "owned_total": owned_now}),
    );
    Ok(ok)
}

/// Everything the plan anchors after model segment `seg`, in the model's order (`route.rs`
/// inserts each kind at `after + 1` in reverse, so after the anchor come the evolutions, then the
/// specials, then the hunts, each in plan order).
pub fn dex_hook(s: &mut Session, seg: &str) -> PResult<()> {
    let p = &s.plan.params;
    let evos: Vec<Evolve> = p
        .evolutions
        .iter()
        .filter(|e| e.after == seg)
        .cloned()
        .collect();
    let specials: Vec<Special> = p
        .specials
        .iter()
        .filter(|e| e.after == seg)
        .cloned()
        .collect();
    let hunts: Vec<Hunt> = p.hunts.iter().filter(|e| e.after == seg).cloned().collect();
    for e in &evos {
        evolve(s, e)?;
    }
    for sp in &specials {
        special(s, sp)?;
    }
    for h in &hunts {
        hunt(s, h)?;
    }
    Ok(())
}
