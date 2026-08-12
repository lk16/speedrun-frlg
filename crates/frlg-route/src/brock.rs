//! The defeat-brock continuation: everything after `09-battle-win`.
//!
//! Same contract as `segments.rs`: every segment ends at a cited observable,
//! and every scripted beat cites the script it steps through. The evidence
//! for the story chain (parcel mandatory, tutorial mandatory, Sammy forced,
//! Liam skippable) is `docs/defeat-brock/research/story-gates.md`.
//!
//! Wild encounters are the new ingredient. The semi-naive policy here is:
//! let the path search dodge what it can (a battle-interrupted edge looks
//! blocked, so Dijkstra routes around encounters and trainer sights that
//! have a way around), and when no encounter-free path exists, take the
//! battle on the closest approach and *flee* it (wilds) or *win* it
//! (trainers), then search again. Optimisation of all this -- seed dials,
//! path shaping against the second LCG, not walking Route 1 three times
//! slower than it must -- comes after a full run exists.

use frlg_emu::{keys, Emu};

use crate::nav::{self, Goal};
use crate::observe::{
    Observer, BATTLE_TYPE_TRAINER, B_OUTCOME_RAN, B_OUTCOME_WON, FLAG_DEFEATED_BROCK,
    VAR_OAKS_LAB_SCENE, VAR_VIRIDIAN_MART, VAR_VIRIDIAN_OLD_MAN,
};
use crate::record::{Feed, Recorder, RouteError, Trial};
use crate::segments::{Segment, Starter, Tuning, OAKS_LAB, PALLET_TOWN};

/// `data/maps/map_groups.json` (group_order index, position in group).
pub const ROUTE1: (u8, u8) = (3, 19);
pub const VIRIDIAN_CITY: (u8, u8) = (3, 1);
pub const VIRIDIAN_MART: (u8, u8) = (5, 3);
pub const ROUTE2: (u8, u8) = (3, 20);
pub const FOREST_SOUTH_ENTRANCE: (u8, u8) = (15, 0);
pub const VIRIDIAN_FOREST: (u8, u8) = (1, 0);
pub const FOREST_NORTH_ENTRANCE: (u8, u8) = (15, 3);
pub const PEWTER_CITY: (u8, u8) = (3, 2);
pub const PEWTER_GYM: (u8, u8) = (6, 2);

/// Oak stands at (6,3) facing down (`data/maps/PalletTown_ProfessorOaksLab/
/// map.json`, `OBJ_EVENT_GFX_PROF_OAK`); the delivery talk happens from the
/// tile below him.
const OAK_TALK_TILE: (i16, i16) = (6, 4);

/// The catching-tutorial triggers flank the old man at (20,8)/(22,8)
/// (`data/maps/ViridianCity/map.json:222-238`); the route walks to below the
/// left one and steps up into it.
const TUTORIAL_TRIGGER_APPROACH: (i16, i16) = (20, 9);

/// Brock is at (6,5) facing down (`data/maps/PewterCity_Gym/map.json:18-32`);
/// he is interaction-only, spoken to from the tile below.
const BROCK_TALK_TILE: (i16, i16) = (6, 6);

/// The continuation, in order. Appended to `segments::all` by
/// `Target::DefeatBrock`.
pub fn segments(starter: Starter, tuning: Tuning) -> Vec<Segment> {
    vec![
        exit_lab(tuning),
        to_viridian(tuning),
        parcel(tuning),
        deliver(tuning),
        tutorial(tuning),
        to_forest(tuning),
        forest(tuning),
        to_gym(tuning),
        brock(starter, tuning),
    ]
}

// ---------------------------------------------------------------------------
// Battle handling: flee wilds, win trainers.

/// More frames than any battle here should ever run; hitting it is a loss,
/// not an error.
const BATTLE_FRAME_BUDGET: usize = 20000;

/// Advance until the first action menu of a fresh battle. `gBattleOutcome`
/// is stale from the previous battle until `BattleStartClearSetData` zeroes
/// it (`decompiled/src/battle_main.c:2265`), so nothing before this point may
/// read the outcome. B advances the intro text without selecting anything.
fn to_first_menu(trial: &mut Trial<'_>, obs: &Observer, mash: &[u16]) -> Result<(), RouteError> {
    trial.advance_while("the battle's first action menu", mash, 3000, |emu| {
        obs.battle_choosing_actions(emu)
    })?;
    Ok(())
}

/// One wild battle, fled. Searches small delays at the action menu so the
/// escape roll (`Random()`-fed, `decompiled/src/battle_main.c:4419` region)
/// succeeds first try; a failed attempt costs a whole enemy turn and shows up
/// as a longer battle, so shortest-wins scoring needs no special case.
///
/// RUN is the action menu's bottom-right entry: cursor 0 is FIGHT, DOWN
/// moves to 2 (POKEMON), RIGHT to 3 (RUN) -- driven as mash-until-effect on
/// `gActionSelectionCursor`.
fn flee_wild(rec: &mut Recorder, obs: &Observer, tuning: Tuning) -> Result<(), RouteError> {
    let start = rec.save_state()?;
    let mut mash: Vec<u16> = vec![keys::B; tuning.text_hold.max(1)];
    mash.push(0);

    let run_attempt = |rec: &mut Recorder, delay: usize| -> Result<(Vec<u16>, bool), RouteError> {
        rec.emu().load_state(&start)?;
        let mut trial = Trial::new(rec.emu());
        to_first_menu(&mut trial, obs, &mash)?;
        trial.idle(delay)?;
        trial.advance_while("cursor on POKEMON", &[keys::DOWN, 0], 120, |emu| {
            obs.action_cursor(emu, 0) & 2 != 0
        })?;
        trial.advance_while("cursor on RUN", &[keys::RIGHT, 0], 120, |emu| {
            obs.action_cursor(emu, 0) == 3
        })?;
        // A commits RUN; then advance until the battle ends (ran) or the
        // menu comes back (escape failed -- the attempt is judged by length
        // anyway, so just keep trying inside the same battle).
        let fled = trial.advance_while(
            "the escape",
            &mash_with(keys::A, tuning),
            BATTLE_FRAME_BUDGET,
            |emu| obs.battle_outcome(emu) == B_OUTCOME_RAN,
        );
        match fled {
            Err(RouteError::Timeout { .. }) => Ok((trial.into_inputs(), false)),
            Err(other) => Err(other),
            Ok(_) => Ok((trial.into_inputs(), true)),
        }
    };

    let mut best: Option<Vec<u16>> = None;
    for delay in 0..16 {
        let (inputs, fled) = run_attempt(rec, delay)?;
        if fled && best.as_ref().is_none_or(|b| inputs.len() < b.len()) {
            best = Some(inputs);
        }
    }
    let best = best.ok_or_else(|| RouteError::Timeout {
        what: "any delay to flee the wild battle".into(),
        budget: 16,
        frames: rec.frames(),
    })?;
    rec.emu().load_state(&start)?;
    rec.play(&best)?;
    back_to_field(rec, obs, tuning)
}

fn mash_with(key: u16, tuning: Tuning) -> Vec<u16> {
    let mut mash: Vec<u16> = vec![key; tuning.text_hold.max(1)];
    mash.push(0);
    mash
}

/// Win the battle the recorder is standing at, with the same two-stage
/// delay search as rival-1's `09-battle-win` (stage 1 start delays, stage 2
/// per-turn delays to a fixpoint), plus one new knob: `preferred_move`
/// steers the *first* fight menu to that move id -- the cursor persists for
/// the rest of the battle (`gMoveSelectionCursor`), so one navigation pays
/// for every turn.
fn win_battle(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    preferred_move: Option<u16>,
    label: &str,
) -> Result<(), RouteError> {
    const START_DELAYS: std::ops::Range<usize> = 0..48;
    const TURN_DELAYS: std::ops::Range<usize> = 1..16;
    const MAX_PASSES: usize = 6;

    let start = rec.save_state()?;
    let mash = mash_with(keys::A, tuning);
    let intro_mash = mash_with(keys::B, tuning);

    // `plan[k]` is the idle spent on arriving at the k-th action menu
    // (k = 0 is the first menu, so plan[0] is stage 1's start delay).
    let run_plan =
        |rec: &mut Recorder, plan: &[usize]| -> Result<(Vec<u16>, bool, usize), RouteError> {
            rec.emu().load_state(&start)?;
            let mut trial = Trial::new(rec.emu());
            let mut menu = 0usize;
            let mut move_chosen = preferred_move.is_none();
            to_first_menu(&mut trial, obs, &intro_mash)?;
            trial.idle(plan.first().copied().unwrap_or(0))?;
            let won = loop {
                if !move_chosen {
                    // A selects FIGHT (cursor starts there); wait for the move
                    // menu, then steer the cursor to the wanted slot. Layout is
                    // 2x2, slot = index into gBattleMons[0].moves: bit 1 is the
                    // row (DOWN), bit 0 the column (RIGHT).
                    let entered = trial.advance_while("the move menu", &mash, 600, |emu| {
                        obs.battle_controller_is(emu, 0, "HandleInputChooseMove")
                    });
                    match entered {
                        Err(RouteError::Timeout { .. }) => break false,
                        other => other?,
                    };
                    let mv = preferred_move.unwrap();
                    let slot = obs
                        .battle_mon(trial.core(), 0)
                        .moves
                        .iter()
                        .position(|&m| m == mv);
                    let Some(slot) = slot else {
                        // The mon does not know the move (yet): fall back to
                        // whatever the cursor is on. The caller decided the
                        // route wrongly; the battle search still gets a fair
                        // shot with slot 0.
                        move_chosen = true;
                        continue;
                    };
                    if slot & 2 != 0 {
                        trial.advance_while("cursor row", &[keys::DOWN, 0], 120, |emu| {
                            obs.move_cursor(emu, 0) & 2 != 0
                        })?;
                    }
                    if slot & 1 != 0 {
                        trial.advance_while("cursor column", &[keys::RIGHT, 0], 120, |emu| {
                            obs.move_cursor(emu, 0) & 1 != 0
                        })?;
                    }
                    move_chosen = true;
                }

                // Commit this turn's actions: mash until the selection state
                // exits.
                let to_turn =
                    trial.advance_while("the turn to resolve", &mash, BATTLE_FRAME_BUDGET, |emu| {
                        obs.battle_outcome(emu) != 0 || !obs.battle_choosing_actions(emu)
                    });
                match to_turn {
                    Err(RouteError::Timeout { .. }) => break false,
                    other => other?,
                };
                if obs.battle_outcome(trial.core()) != 0 {
                    break obs.battle_outcome(trial.core()) == B_OUTCOME_WON;
                }
                // To the next turn's menu, or the end.
                let to_menu = trial.advance_while(
                    "the battle menu or the end",
                    &mash,
                    BATTLE_FRAME_BUDGET,
                    |emu| obs.battle_outcome(emu) != 0 || obs.battle_choosing_actions(emu),
                );
                match to_menu {
                    Err(RouteError::Timeout { .. }) => break false,
                    other => other?,
                };
                if obs.battle_outcome(trial.core()) != 0 {
                    break obs.battle_outcome(trial.core()) == B_OUTCOME_WON;
                }
                menu += 1;
                trial.idle(*plan.get(menu).unwrap_or(&0))?;
            };
            Ok((trial.into_inputs(), won, menu + 1))
        };

    // Stage 1: start delay.
    let mut best: Option<(Vec<u16>, Vec<usize>, usize)> = None;
    let mut wins = 0usize;
    for delay in START_DELAYS {
        let (inputs, won, turns) = run_plan(rec, &[delay])?;
        wins += won as usize;
        if won
            && best
                .as_ref()
                .is_none_or(|(seen, _, _)| inputs.len() < seen.len())
        {
            best = Some((inputs, vec![delay], turns));
        }
    }
    let (mut best_inputs, mut plan, mut best_turns) = best.ok_or_else(|| RouteError::Timeout {
        what: format!("any start delay to win {label}"),
        budget: START_DELAYS.end,
        frames: rec.frames(),
    })?;
    eprintln!(
        "      {label} stage 1: {wins}/{} start delays win, delay {} at {} frames",
        START_DELAYS.end,
        plan[0],
        best_inputs.len()
    );

    // Stage 2: per-turn delays, greedy, repeated to a fixpoint.
    for pass in 1..=MAX_PASSES {
        let mut adopted = false;
        let pass_turns = best_turns;
        for turn in 1..=pass_turns {
            for delay in TURN_DELAYS {
                let mut candidate = plan.clone();
                if candidate.len() < turn + 1 {
                    candidate.resize(turn + 1, 0);
                }
                if candidate[turn] == delay {
                    continue;
                }
                candidate[turn] = delay;
                let (inputs, won, turns_seen) = run_plan(rec, &candidate)?;
                if won && inputs.len() < best_inputs.len() {
                    eprintln!(
                        "      {label} stage 2 (pass {pass}): turn {turn} delay {delay} -> {} frames",
                        inputs.len()
                    );
                    best_inputs = inputs;
                    plan = candidate;
                    best_turns = turns_seen;
                    adopted = true;
                }
            }
        }
        if !adopted {
            break;
        }
    }

    eprintln!(
        "      {label}: plan {plan:?}, {} frames, {best_turns} turns",
        best_inputs.len()
    );
    rec.emu().load_state(&start)?;
    rec.play(&best_inputs)?;
    Ok(())
}

/// After a battle's outcome is set, drive back to a controllable overworld:
/// B advances the post-battle text (trainer defeat lines, "got away safely")
/// and the fade, without answering yes to anything.
fn back_to_field(rec: &mut Recorder, obs: &Observer, tuning: Tuning) -> Result<(), RouteError> {
    rec.hold_mash_until(
        "the overworld after the battle",
        keys::B,
        tuning.text_hold,
        3000,
        |emu| obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu),
    )?;
    Ok(())
}

/// Whatever battle just started: flee it if wild, win it if a trainer owns
/// it (`gBattleTypeFlags & BATTLE_TYPE_TRAINER`,
/// `decompiled/include/constants/battle.h:45`).
fn handle_battle(rec: &mut Recorder, obs: &Observer, tuning: Tuning) -> Result<(), RouteError> {
    if obs.battle_type_flags(rec.emu()) & BATTLE_TYPE_TRAINER != 0 {
        win_battle(rec, obs, tuning, None, "trainer en route")?;
        back_to_field(rec, obs, tuning)
    } else {
        flee_wild(rec, obs, tuning)
    }
}

// ---------------------------------------------------------------------------
// Walking that survives battles.

/// Walk to `goal_tile`/`goal_map`, taking forced battles on the chin.
///
/// Each round searches for a clean path (a battle-interrupted edge reads as
/// blocked, so a found path is encounter-free by construction). If the
/// search exhausts, the closest approach is committed and the walk forces
/// one step at a time in the `bias` direction (then its neighbours) until
/// something gives -- usually a battle, which is handled and the loop
/// searches again from the far side.
/// How a `walk_fleeing` leg states its destination.
#[derive(Debug, Clone, Copy)]
enum Leg {
    /// A specific tile.
    Tile((u8, u8), i16, i16),
    /// Any tile of a map, leaving the current map near a known connection
    /// or warp tile -- the directed form every grass crossing uses
    /// (`Goal::AnyOnVia`).
    MapVia((u8, u8), (u8, u8), (i16, i16)),
}

impl Leg {
    fn goal(&self) -> Goal {
        match *self {
            Leg::Tile(map, x, y) => Goal::tile(map, x, y),
            Leg::MapVia(map, via_map, via) => Goal::on_map_via(map, via_map, via),
        }
    }

    fn arrived(&self, obs: &Observer, emu: &mut Emu) -> bool {
        match *self {
            Leg::Tile(map, x, y) => obs.map(emu) == Some(map) && obs.pos(emu) == Some((x, y)),
            Leg::MapVia(map, _, _) => obs.map(emu) == Some(map),
        }
    }
}

fn walk_fleeing(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    leg: Leg,
    bias: u16,
    max_nodes: usize,
) -> Result<(), RouteError> {
    const MAX_ROUNDS: usize = 40;

    for _round in 0..MAX_ROUNDS {
        if leg.arrived(obs, rec.emu()) {
            return Ok(());
        }
        let start = rec.save_state()?;
        let (path, reached) =
            nav::search_best_effort(rec.emu(), obs, &start, leg.goal(), max_nodes)?;
        rec.emu().load_state(&start)?;
        rec.play(&path.inputs)?;
        if reached {
            return Ok(());
        }

        // Force progress: hold the bias direction (then its neighbours) for
        // as many tiles as it gives, stopping at a battle, a map change, or
        // a wall. A battle here is *the point*: fleeing it resets the
        // encounter cooldown (`src/battle_setup.c:205`), which buys 6-7
        // nearly-free grass steps -- often the rest of the belt the search
        // could not cross.
        let mut moved = false;
        let dirs = [
            bias,
            rotate(bias),
            rotate(rotate(rotate(bias))),
            rotate(rotate(bias)),
        ];
        for dir in dirs {
            let here = rec.save_state()?;
            let mut trial = Trial::new(rec.emu());
            let start_place = (obs.map(trial.core()), obs.pos(trial.core()));
            let mut last_place = start_place;
            let mut frames_since_change = 0usize;
            let mut battled = false;
            for _ in 0..1200 {
                trial.step(dir)?;
                if obs.in_battle(trial.core()) {
                    battled = true;
                    break;
                }
                let now = (obs.map(trial.core()), obs.pos(trial.core()));
                if now != last_place {
                    last_place = now;
                    frames_since_change = 0;
                    if now.0 != start_place.0 {
                        break; // crossed into another map: stop and re-plan
                    }
                } else {
                    frames_since_change += 1;
                    // A wall (position changes mid-step come well within
                    // this while walking). Requiring player_can_step here
                    // was the first build's failure: never true mid-hold.
                    if frames_since_change > 64 && last_place != start_place {
                        break;
                    }
                    if frames_since_change > 96 {
                        break;
                    }
                }
            }
            let progressed = battled || last_place != start_place;
            let inputs = trial.into_inputs();
            rec.emu().load_state(&here)?;
            if progressed {
                rec.play(&inputs)?;
                if obs.in_battle(rec.emu()) {
                    handle_battle(rec, obs, tuning)?;
                }
                moved = true;
                break;
            }
        }
        if !moved {
            return Err(RouteError::Timeout {
                what: format!("any progress towards {leg:?}"),
                budget: MAX_ROUNDS,
                frames: rec.frames(),
            });
        }
    }
    Err(RouteError::Timeout {
        what: format!("{leg:?} within the round budget"),
        budget: MAX_ROUNDS,
        frames: rec.frames(),
    })
}

/// UP -> LEFT -> DOWN -> RIGHT -> UP: an arbitrary but fixed neighbour order
/// for the force-step fallback.
fn rotate(dir: u16) -> u16 {
    match dir {
        keys::UP => keys::LEFT,
        keys::LEFT => keys::DOWN,
        keys::DOWN => keys::RIGHT,
        _ => keys::UP,
    }
}

// ---------------------------------------------------------------------------
// The segments.

/// The post-battle lab script plays itself (rival leaves,
/// `..._EventScript_EndRivalBattle`, `data/maps/PalletTown_ProfessorOaksLab/
/// scripts.inc:467-481`); B advances its one msgbox. It ends with the scene
/// var at 4 and control returned; then walk out the door.
fn exit_lab(tuning: Tuning) -> Segment {
    Segment {
        name: "10-exit-lab",
        goal: "back outside in Pallet Town, rival gone (lab scene var 4)".into(),
        run: Box::new(move |rec, obs| {
            rec.hold_mash_until(
                "the rival to leave",
                keys::B,
                tuning.text_hold,
                4000,
                |emu| obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(4) && obs.player_can_step(emu),
            )?;
            nav::walk_to(rec, obs, Goal::on_map(PALLET_TOWN), 4000)?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| {
            obs.map(emu) == Some(PALLET_TOWN) && obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(4)
        }),
    }
}

/// North through Route 1's forced grass (>= 20 land-encounter tiles,
/// `research/story-gates.md`) into Viridian City.
fn to_viridian(tuning: Tuning) -> Segment {
    Segment {
        name: "11-to-viridian",
        goal: "in Viridian City".into(),
        run: Box::new(move |rec, obs| {
            // Connections (research/story-gates.md, map.json cites): Pallet's
            // north exit is x=12/13 row 0; Route 1's top x=10..13 meets
            // Viridian's bottom x=22..25.
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, PALLET_TOWN, (12, 1)),
                keys::UP,
                600,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_CITY, ROUTE1, (12, 1)),
                keys::UP,
                1000,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(VIRIDIAN_CITY)),
    }
}

/// Into the mart, where entering force-plays the parcel scene
/// (`data/maps/ViridianCity_Mart/scripts.inc:15-33`: clerk turns, player is
/// walked to the counter, `giveitem ITEM_OAKS_PARCEL`, mart scene var 1),
/// then back out.
fn parcel(tuning: Tuning) -> Segment {
    Segment {
        name: "12-parcel",
        goal: "Oak's Parcel in the bag, back outside the mart".into(),
        run: Box::new(move |rec, obs| {
            // The mart door warp is at (36,19) (`data/maps/ViridianCity/
            // map.json:194-200`).
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_MART, VIRIDIAN_CITY, (36, 20)),
                keys::UP,
                1500,
            )?;
            rec.hold_mash_until(
                "the parcel handover",
                keys::B,
                tuning.text_hold,
                3000,
                |emu| obs.var(emu, VAR_VIRIDIAN_MART) == Some(1) && obs.player_can_step(emu),
            )?;
            nav::walk_to(rec, obs, Goal::on_map(VIRIDIAN_CITY), 4000)?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| {
            obs.map(emu) == Some(VIRIDIAN_CITY) && obs.var(emu, VAR_VIRIDIAN_MART) == Some(1)
        }),
    }
}

/// South through Route 1 again, into the lab, and talk to Oak: the delivery
/// scene gives the Pokédex and five Poké Balls and ends by setting
/// `VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN = 1` -- the only setter
/// (`data/maps/PalletTown_ProfessorOaksLab/scripts.inc:576,598-684`).
fn deliver(tuning: Tuning) -> Segment {
    Segment {
        name: "13-deliver",
        goal: "Pokédex received, old man armed (his scene var 1)".into(),
        run: Box::new(move |rec, obs| {
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, VIRIDIAN_CITY, (23, 38)),
                keys::DOWN,
                1000,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(PALLET_TOWN, ROUTE1, (12, 38)),
                keys::DOWN,
                1000,
            )?;
            nav::walk_to(rec, obs, Goal::on_map(OAKS_LAB), 6000)?;
            nav::walk_to(
                rec,
                obs,
                Goal::tile(OAKS_LAB, OAK_TALK_TILE.0, OAK_TALK_TILE.1),
                4000,
            )?;
            // Face Oak and talk; the scene has no yes/no prompts, so B/A
            // both only advance. A opens the dialogue.
            rec.hold(keys::UP, 2)?;
            rec.idle(1)?;
            rec.hold_mash_until(
                "the delivery scene",
                keys::A,
                tuning.text_hold,
                8000,
                |emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(1) && obs.player_can_step(emu),
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(1)),
    }
}

/// North once more, then step into the tutorial trigger at (20,8): the old
/// man's catching demo is mandatory and choice-free
/// (`data/maps/ViridianCity/scripts.inc:202-237`); A advances all of it. It
/// ends with his var at 2, which is what opens the road north.
fn tutorial(tuning: Tuning) -> Segment {
    Segment {
        name: "14-tutorial",
        goal: "catching tutorial done (old man var 2), road north open".into(),
        run: Box::new(move |rec, obs| {
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, PALLET_TOWN, (12, 1)),
                keys::UP,
                600,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_CITY, ROUTE1, (12, 1)),
                keys::UP,
                1000,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::Tile(
                    VIRIDIAN_CITY,
                    TUTORIAL_TRIGGER_APPROACH.0,
                    TUTORIAL_TRIGGER_APPROACH.1,
                ),
                keys::UP,
                2000,
            )?;
            // One step up fires the coord event; the demo battle plays
            // itself with A advancing its text.
            rec.hold_mash_until(
                "the catching tutorial",
                keys::A,
                tuning.text_hold,
                12000,
                |emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(2) && obs.player_can_step(emu),
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(2)),
    }
}

/// Viridian -> Route 2 (its grass has a clean bypass) -> the south entrance
/// building -> Viridian Forest.
fn to_forest(tuning: Tuning) -> Segment {
    Segment {
        name: "15-to-forest",
        goal: "inside Viridian Forest".into(),
        run: Box::new(move |rec, obs| {
            // Viridian's north exit x=19..23; the forest south-entrance
            // warps sit at Route 2 (5,51)/(6,51); the entrance building's
            // exit warp is (7,1) (research/story-gates.md).
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE2, VIRIDIAN_CITY, (21, 1)),
                keys::UP,
                1500,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(FOREST_SOUTH_ENTRANCE, ROUTE2, (5, 52)),
                keys::UP,
                1000,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_FOREST, FOREST_SOUTH_ENTRANCE, (7, 2)),
                keys::UP,
                400,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(VIRIDIAN_FOREST)),
    }
}

/// The forest: >= 48 forced grass tiles, four dodgeable trainers the path
/// search will route around (their sight cones read as blocked edges), and
/// Bug Catcher Sammy, whose 4-tile sight line spans the only corridor to
/// the north exit (`research/story-gates.md`) -- his battle is taken and
/// won by the search inside `handle_battle`.
fn forest(tuning: Tuning) -> Segment {
    Segment {
        name: "16-forest",
        goal: "out the forest's north side".into(),
        run: Box::new(move |rec, obs| {
            // Forest exits at (4..6,9) into the north entrance building
            // (research/story-gates.md).
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(FOREST_NORTH_ENTRANCE, VIRIDIAN_FOREST, (5, 10)),
                keys::UP,
                2000,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(FOREST_NORTH_ENTRANCE)),
    }
}

/// Route 2's north half (grass bypassable), Pewter City (no wild header at
/// all), and the gym door at (15,16). The gym-guide triggers live at x=42-43
/// and are never approached (`research/story-gates.md`).
fn to_gym(tuning: Tuning) -> Segment {
    Segment {
        name: "17-to-gym",
        goal: "inside Pewter Gym".into(),
        run: Box::new(move |rec, obs| {
            // North entrance exit warp (7,1); Route 2's top x=8..11 meets
            // Pewter's bottom x=20..23; the gym warp is Pewter (15,16)
            // (research/story-gates.md).
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE2, FOREST_NORTH_ENTRANCE, (7, 2)),
                keys::UP,
                400,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(PEWTER_CITY, ROUTE2, (9, 1)),
                keys::UP,
                1000,
            )?;
            walk_fleeing(
                rec,
                obs,
                tuning,
                Leg::MapVia(PEWTER_GYM, PEWTER_CITY, (15, 17)),
                keys::UP,
                1500,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(PEWTER_GYM)),
    }
}

/// To Brock along the west wall -- Camper Liam's sight line covers (4..7,8)
/// and the search dodges it like any other blocked edge -- then talk, win,
/// and mash through to `FLAG_DEFEATED_BROCK`
/// (`data/maps/PewterCity_Gym/scripts.inc:4-21`).
///
/// The move each starter leads with is the 4x option its learnset has by
/// now (`research/starter-and-brock.md`): Bubble for Squirtle, Vine Whip
/// for Bulbasaur, Ember for Charmander (0.5x, but its best). If the mon has
/// not learned it, the search falls back to whatever the cursor is on.
fn brock(starter: Starter, tuning: Tuning) -> Segment {
    /// `decompiled/include/constants/moves.h`.
    const MOVE_VINE_WHIP: u16 = 22;
    const MOVE_EMBER: u16 = 52;
    const MOVE_BUBBLE: u16 = 145;

    let preferred = match starter {
        Starter::Bulbasaur => MOVE_VINE_WHIP,
        Starter::Squirtle => MOVE_BUBBLE,
        Starter::Charmander => MOVE_EMBER,
    };

    Segment {
        name: "18-brock",
        goal: "FLAG_DEFEATED_BROCK set".into(),
        run: Box::new(move |rec, obs| {
            nav::walk_to(
                rec,
                obs,
                Goal::tile(PEWTER_GYM, BROCK_TALK_TILE.0, BROCK_TALK_TILE.1),
                8000,
            )?;
            rec.hold(keys::UP, 2)?;
            rec.idle(1)?;
            rec.mash_until("the battle to start", keys::A, 2000, |emu| {
                obs.in_battle(emu)
            })?;
            win_battle(rec, obs, tuning, Some(preferred), "brock")?;
            rec.hold_mash_until("the defeat flag", keys::B, tuning.text_hold, 3000, |emu| {
                obs.flag(emu, FLAG_DEFEATED_BROCK) == Some(true)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.flag(emu, FLAG_DEFEATED_BROCK) == Some(true)),
    }
}
