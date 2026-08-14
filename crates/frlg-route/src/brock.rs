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

use std::cell::RefCell;
use std::collections::HashSet;

use frlg_emu::{keys, Emu};

use crate::nav::{self, Goal};
use crate::observe::{
    Observer, BATTLE_TYPE_TRAINER, B_OUTCOME_RAN, B_OUTCOME_WON, FLAG_DEFEATED_BROCK,
    VAR_OAKS_LAB_SCENE, VAR_VIRIDIAN_MART, VAR_VIRIDIAN_OLD_MAN,
};
use crate::plan::{self, PlanRequest, PlanStep, StepKind};
use crate::record::{Feed, Recorder, RouteError, Trial};
use crate::segments::{Segment, Starter, Tuning, Version, OAKS_LAB, PALLET_TOWN};
use crate::world::World;

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
pub const PEWTER_POKECENTER: (u8, u8) = (6, 5);

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
        to_pewter(tuning),
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

/// The direction that undoes `dir`.
fn nav_opposite(dir: u16) -> u16 {
    match dir {
        keys::UP => keys::DOWN,
        keys::DOWN => keys::UP,
        keys::LEFT => keys::RIGHT,
        keys::RIGHT => keys::LEFT,
        _ => 0,
    }
}

/// A scripted scene is over only when the script lock is released *and* the
/// avatar is controllable -- `player_can_step` alone reads true mid-scene
/// between forced moves (`Observer::field_controls_locked`).
fn scene_over(obs: &Observer, emu: &mut Emu) -> bool {
    !obs.field_controls_locked(emu) && obs.player_can_step(emu)
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
pub fn win_battle(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    preferred_move: Option<u16>,
    label: &str,
    start_delays: usize,
) -> Result<(), RouteError> {
    let start_delays = 0..start_delays;
    const TURN_DELAYS: std::ops::Range<usize> = 1..16;
    const MAX_PASSES: usize = 6;

    let start = rec.save_state()?;
    let mash = mash_with(keys::A, tuning);
    let intro_mash = mash_with(keys::B, tuning);

    // A checkpoint at a menu arrival: the emulator state and the inputs that
    // got there from the battle's start, both taken *before* the plan's idle
    // for that menu is spent. A candidate plan that differs from the current
    // best only from menu k onwards resumes at checkpoint k instead of
    // replaying the intro and the first k turns -- same trajectory, a
    // fraction of the emulation. (Verified against the replay-from-start
    // implementation on the committed Rick and Sammy fights: identical
    // plans, identical frame counts.)
    struct Menu {
        state: frlg_emu::SaveState,
        prefix: Vec<u16>,
    }
    /// What one candidate run establishes: the whole battle's inputs, whether
    /// it won, how many menus the battle visited, and checkpoints for the
    /// menus after the resume point.
    type PlanOutcome = (Vec<u16>, bool, usize, Vec<Menu>);

    // Drive to the first action menu once; every candidate resumes from it.
    let menu0 = {
        let mut trial = Trial::new(rec.emu());
        to_first_menu(&mut trial, obs, &intro_mash)?;
        let prefix = trial.into_inputs();
        Menu {
            state: rec.save_state()?,
            prefix,
        }
    };

    // `plan[k]` is the idle spent on arriving at the k-th action menu
    // (k = 0 is the first menu, so plan[0] is stage 1's start delay).
    // Run from menu `k` to the battle's end under `plan`, collecting a
    // checkpoint at every later menu. Returns the whole battle's inputs
    // (checkpoint prefix included), whether it won, how many menus the
    // battle visited in total, and the checkpoints for menus k+1 onwards.
    let continue_plan = |rec: &mut Recorder,
                         from: &Menu,
                         k: usize,
                         plan: &[usize]|
     -> Result<PlanOutcome, RouteError> {
        rec.emu().load_state(&from.state)?;
        let mut trial = Trial::new(rec.emu());
        let mut menu = k;
        // Move steering happens on the first menu only; the cursor then
        // persists for the whole battle (`gMoveSelectionCursor`), so a
        // resume at k > 0 never needs to steer.
        let mut move_chosen = preferred_move.is_none() || k > 0;
        // (state, inputs fed since the resume) per menu arrival.
        let mut later: Vec<(frlg_emu::SaveState, usize)> = Vec::new();
        trial.idle(plan.get(k).copied().unwrap_or(0))?;
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
            later.push((trial.core().save_state()?, trial.fed()));
            trial.idle(*plan.get(menu).unwrap_or(&0))?;
        };
        let suffix = trial.into_inputs();
        let mut inputs = from.prefix.clone();
        inputs.extend_from_slice(&suffix);
        let menus = later
            .into_iter()
            .map(|(state, len)| {
                let mut prefix = from.prefix.clone();
                prefix.extend_from_slice(&suffix[..len]);
                Menu { state, prefix }
            })
            .collect();
        Ok((inputs, won, menu + 1, menus))
    };

    // Stage 1: start delay. The best winner's outcome, plus the plan that
    // produced it.
    let mut best: Option<(PlanOutcome, Vec<usize>)> = None;
    let mut wins = 0usize;
    for delay in start_delays.clone() {
        let (inputs, won, turns, menus) = continue_plan(rec, &menu0, 0, &[delay])?;
        wins += won as usize;
        if won
            && best
                .as_ref()
                .is_none_or(|((seen, ..), _)| inputs.len() < seen.len())
        {
            best = Some(((inputs, won, turns, menus), vec![delay]));
        }
    }
    let ((mut best_inputs, _, mut best_turns, best_menus), mut plan) =
        best.ok_or_else(|| RouteError::Timeout {
            what: format!("any start delay to win {label}"),
            budget: start_delays.end,
            frames: rec.frames(),
        })?;
    eprintln!(
        "      {label} stage 1: {wins}/{} start delays win, delay {} at {} frames",
        start_delays.end,
        plan[0],
        best_inputs.len()
    );

    // The checkpoints along the current best trajectory: menus[k] is the
    // arrival at menu k. On adoption, everything up to and including the
    // changed turn's arrival is unchanged (the candidate differed only in
    // the idle spent *after* arriving there), so only the tail is replaced.
    let mut menus: Vec<Menu> = vec![menu0];
    menus.extend(best_menus);

    // Stage 2: per-turn delays, greedy, repeated to a fixpoint.
    for pass in 1..=MAX_PASSES {
        let mut adopted = false;
        for turn in 1..menus.len() {
            if turn >= menus.len() {
                break; // an adoption shrank the battle below this turn
            }
            for delay in TURN_DELAYS {
                if plan.get(turn) == Some(&delay) {
                    continue;
                }
                let mut candidate = plan.clone();
                if candidate.len() < turn + 1 {
                    candidate.resize(turn + 1, 0);
                }
                candidate[turn] = delay;
                let (inputs, won, turns_seen, new_menus) =
                    continue_plan(rec, &menus[turn], turn, &candidate)?;
                if won && inputs.len() < best_inputs.len() {
                    eprintln!(
                        "      {label} stage 2 (pass {pass}): turn {turn} delay {delay} -> {} frames",
                        inputs.len()
                    );
                    best_inputs = inputs;
                    plan = candidate;
                    best_turns = turns_seen;
                    menus.truncate(turn + 1);
                    menus.extend(new_menus);
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
        |emu| obs.callback2_is(emu, "CB2_Overworld") && scene_over(obs, emu),
    )?;
    Ok(())
}

/// Whatever battle just started: flee it if wild, win it if a trainer owns
/// it (`gBattleTypeFlags & BATTLE_TYPE_TRAINER`,
/// `decompiled/include/constants/battle.h:45`).
fn handle_battle(rec: &mut Recorder, obs: &Observer, tuning: Tuning) -> Result<(), RouteError> {
    if obs.battle_type_flags(rec.emu()) & BATTLE_TYPE_TRAINER != 0 {
        // 192 start delays, not 48: on the committed Rick fight the optimum
        // sat at delay 111 (3894 frames vs 4059 at width 48; fight-lab,
        // 2026-08-13), and the checkpointed search makes the extra width
        // cheap. Sammy's optimum (delay 12) was inside 48, so the width
        // only ever helps.
        win_battle(rec, obs, tuning, None, "trainer en route", 192)?;
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
pub enum Leg {
    /// A specific tile.
    Tile((u8, u8), i16, i16),
    /// Any tile of a map, leaving the current map near a known connection
    /// or warp tile -- the directed form every grass crossing uses
    /// (`Goal::AnyOnVia`).
    MapVia((u8, u8), (u8, u8), (i16, i16)),
    /// Within `tol` tiles of a waypoint, greedily (`Goal::NearVia`) -- for
    /// steering through a specific corridor in encounter country.
    Near((u8, u8), i16, i16, u16),
}

impl Leg {
    fn goal(&self) -> Goal {
        match *self {
            Leg::Tile(map, x, y) => Goal::tile(map, x, y),
            Leg::MapVia(map, via_map, via) => Goal::on_map_via(map, via_map, via),
            Leg::Near(map, x, y, tol) => Goal::near_via(map, x, y, tol),
        }
    }

    fn arrived(&self, obs: &Observer, emu: &mut Emu) -> bool {
        match *self {
            Leg::Tile(map, x, y) => obs.map(emu) == Some(map) && obs.pos(emu) == Some((x, y)),
            Leg::MapVia(map, _, _) => obs.map(emu) == Some(map),
            Leg::Near(map, x, y, tol) => {
                obs.map(emu) == Some(map)
                    && obs
                        .pos(emu)
                        .is_some_and(|(px, py)| px.abs_diff(x) + py.abs_diff(y) <= tol)
            }
        }
    }

    /// The tile the leg is steering towards, for ordering forced steps.
    fn hint(&self) -> (i16, i16) {
        match *self {
            Leg::Tile(_, x, y) => (x, y),
            Leg::MapVia(_, _, via) => via,
            Leg::Near(_, x, y, _) => (x, y),
        }
    }
}

pub fn walk_fleeing(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    leg: Leg,
    bias: u16,
    max_nodes: usize,
) -> Result<(), RouteError> {
    const MAX_ROUNDS: usize = 40;
    let mut stagnant = 0usize;

    for round in 0..MAX_ROUNDS {
        // A trainer's sight line fires on its own: the script locks field
        // controls and waits on its intro text (measured: Bug Catcher Rick
        // at (42..46,45) freezing the walk at waypoint (41,44)). Drive any
        // such ambush to its battle and win it before planning further.
        if obs.field_controls_locked(rec.emu()) && !obs.in_battle(rec.emu()) {
            rec.hold_mash_until(
                "the ambush to become a battle",
                keys::A,
                tuning.text_hold,
                2400,
                |emu| obs.in_battle(emu) || !obs.field_controls_locked(emu),
            )?;
            if obs.in_battle(rec.emu()) {
                handle_battle(rec, obs, tuning)?;
            }
        }
        if leg.arrived(obs, rec.emu()) {
            return Ok(());
        }
        // Escalate: a round that needed the force-step fallback left the
        // search stuck on something (usually an encounter belt every
        // explored index hits) -- later rounds search harder rather than
        // repeating the same exhaustion.
        // Cap the escalation low: a 32k-node search in the forest ran for
        // most of an hour and answered nothing the 8k one had not.
        let budget = max_nodes.saturating_mul(1 << round.min(2));
        let start = rec.save_state()?;
        let (path, reached) = nav::search_best_effort(rec.emu(), obs, &start, leg.goal(), budget)?;
        rec.emu().load_state(&start)?;
        rec.play(&path.inputs)?;
        eprintln!(
            "      walk {leg:?} round {round}: search {} ({} frames), now {:?} {:?}",
            if reached { "reached" } else { "best-effort" },
            path.frames,
            obs.map(rec.emu()),
            obs.pos(rec.emu()),
        );
        if reached {
            return Ok(());
        }

        // Force progress: try directions ordered by how much closer their
        // first tile lands to the leg's hint, a few tiles each, stopping at
        // a battle, a map change, or a wall. A battle here is *the point*:
        // fleeing it resets the encounter cooldown
        // (`src/battle_setup.c:205`), which buys 6-7 nearly-free grass
        // steps -- often the rest of the belt the search could not cross.
        // The walk is capped at a few tiles so a sideways escape cannot run
        // to the far wall and oscillate (measured: the first Route 1 build
        // ping-ponged (12,17) <-> (2,17) for 40 rounds).
        const FORCE_TILES: usize = 6;
        // After a fled battle the cooldown is fresh -- the very next steps
        // are nearly encounter-free -- but a search's committed approach
        // path re-walks grass and burns it before arriving. So once forcing
        // starts, *keep* forcing across battles within the round (measured:
        // searching between battles oscillated at (11,12) for ten rounds),
        // and only hand back to the search after real tile progress.
        const FORCE_BATTLES: usize = 4;
        let mut moved = false;
        let (hx, hy) = leg.hint();
        for _push in 0..FORCE_BATTLES {
            let here_pos = obs.pos(rec.emu()).unwrap_or((0, 0));
            let mut dirs = [
                (keys::UP, (0i16, -1i16)),
                (keys::LEFT, (-1i16, 0i16)),
                (keys::RIGHT, (1, 0)),
                (keys::DOWN, (0, 1)),
            ];
            dirs.sort_by_key(|(dir, (dx, dy))| {
                let d = (here_pos.0 + dx).abs_diff(hx) as usize
                    + (here_pos.1 + dy).abs_diff(hy) as usize;
                // Backward loses every tie -- it undoes the search's
                // approach (measured: (12,17) -> forced DOWN -> (12,21),
                // forever).
                (d, (*dir == nav_opposite(bias)) as usize)
            });
            let mut pushed = false;
            for (dir, _) in dirs {
                let here = rec.save_state()?;
                let mut trial = Trial::new(rec.emu());
                let start_place = (obs.map(trial.core()), obs.pos(trial.core()));
                let mut last_place = start_place;
                let mut frames_since_change = 0usize;
                let mut battled = false;
                let mut tiles = 0usize;
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
                        tiles += 1;
                        if now.0 != start_place.0 || tiles >= FORCE_TILES {
                            break; // another map, or far enough: re-plan
                        }
                    } else {
                        frames_since_change += 1;
                        // A wall (position changes mid-step come well within
                        // this while walking). Requiring player_can_step
                        // here was the first build's failure: never true
                        // mid-hold.
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
                    let fought = obs.in_battle(rec.emu());
                    if fought {
                        handle_battle(rec, obs, tuning)?;
                    }
                    eprintln!(
                        "      walk {leg:?} round {round}: forced dir {dir:#06x}{}, now {:?} {:?}",
                        if fought { " into a battle" } else { "" },
                        obs.map(rec.emu()),
                        obs.pos(rec.emu()),
                    );
                    moved = true;
                    pushed = fought; // a clean multi-tile walk: back to the search
                    break;
                }
            }
            if !pushed || leg.arrived(obs, rec.emu()) {
                break;
            }
        }
        if !moved {
            // Boxed in -- usually a wandering NPC standing on the only open
            // tile (measured: the mart counter after the parcel scene).
            // NPCs move on their own schedule; wait a beat and re-plan.
            stagnant += 1;
            if stagnant > 8 {
                return Err(RouteError::Timeout {
                    what: format!("any progress towards {leg:?}"),
                    budget: MAX_ROUNDS,
                    frames: rec.frames(),
                });
            }
            rec.idle(64)?;
        } else {
            stagnant = 0;
        }
    }
    Err(RouteError::Timeout {
        what: format!("{leg:?} within the round budget"),
        budget: MAX_ROUNDS,
        frames: rec.frames(),
    })
}

// ---------------------------------------------------------------------------
// Model-driven walking: plan on the decoded map (`plan.rs`), execute in the
// emulator, steer the frame-timed gates with 1-frame delays, replan when
// reality disagrees. `walk_fleeing` stays as the fallback for anything the
// static model cannot see.

thread_local! {
    /// One decoded-world cache per build thread. `None` after a failed load
    /// -- the build then runs entirely on the `walk_fleeing` fallback.
    static WORLD: RefCell<Option<World>> = RefCell::new(match World::load() {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("      no decoded world ({e}); walks fall back to emulator search");
            None
        }
    });
}

/// The land encounter table for a map, by ROM version
/// (`frlg_mon::wild`, cited there to `wild_encounters.json`).
fn wild_table(map: (u8, u8), version: Version) -> Option<&'static frlg_mon::wild::MapWild> {
    use frlg_mon::wild;
    match map {
        ROUTE1 => Some(&wild::ROUTE1),
        ROUTE2 => Some(&wild::ROUTE2),
        VIRIDIAN_FOREST => Some(match version {
            Version::FireRed => &wild::VIRIDIAN_FOREST_FR,
            Version::LeafGreen => &wild::VIRIDIAN_FOREST_LG,
        }),
        _ => None,
    }
}

fn version_of(emu: &Emu) -> Version {
    let code = emu.game_code();
    let bytes: [u8; 4] = code.as_bytes().try_into().unwrap_or(*b"BPRE");
    Version::from_game_code(bytes).unwrap_or(Version::FireRed)
}

/// What one attempted step did.
enum StepTry {
    /// Arrived on a tile; `consumed` is whether the wild rate test advanced.
    Moved { consumed: bool },
    /// A battle owns the screen (wild encounter or sight-line trainer).
    Battle,
    /// A script grabbed the player without a battle (ambush intro, coord
    /// event).
    Script,
    /// Nothing happened within the budget -- an NPC or wall in the way.
    Stuck,
}

/// Feed `dir` (after `delay` idle frames) until the player lands on a new
/// tile; settle across a warp fade. The inputs are committed to the recorder
/// only by the caller deciding to keep them.
fn attempt_step(
    trial: &mut Trial<'_>,
    obs: &Observer,
    dir: u16,
    delay: usize,
) -> Result<StepTry, RouteError> {
    trial.idle(delay)?;
    let start_pos = obs.pos(trial.core());
    let start_map = obs.map(trial.core());
    let wild_before = obs.wild_data(trial.core()).rng_state;
    for _ in 0..96 {
        trial.step(dir)?;
        if obs.in_battle(trial.core()) {
            return Ok(StepTry::Battle);
        }
        if obs.prevent_step(trial.core()) && obs.field_controls_locked(trial.core()) {
            return Ok(StepTry::Script);
        }
        let now_map = obs.map(trial.core());
        if obs.pos(trial.core()) != start_pos || now_map != start_map {
            if now_map != start_map {
                // A warp: hold nothing through the fade.
                for _ in 0..300 {
                    if obs.player_can_step(trial.core()) || obs.in_battle(trial.core()) {
                        break;
                    }
                    trial.step(0)?;
                }
                if obs.in_battle(trial.core()) {
                    return Ok(StepTry::Battle);
                }
            }
            let consumed = obs.wild_data(trial.core()).rng_state != wild_before;
            return Ok(StepTry::Moved { consumed });
        }
    }
    Ok(StepTry::Stuck)
}

/// Whether a step's observed rate-test consumption matches the plan's
/// expectation. Steps the plan calls `Free`/`Jump` never consume; `Cooldown`
/// and `SkipBoundary` must not; `Consume` must.
fn consumption_matches(kind: StepKind, consumed: bool) -> bool {
    match kind {
        StepKind::Consume { .. } => consumed,
        _ => !consumed,
    }
}

/// The delays tried when a step's first attempt does the wrong thing: each
/// idle frame re-rolls the `gRngValue` gates (5% cooldown gate, 60%
/// behavior roll) that decide everything the rate-test index does not.
const STEER_DELAYS: [usize; 6] = [1, 2, 3, 4, 6, 9];

/// Walk a leg by planning against the model. Returns `Ok(true)` on arrival,
/// `Ok(false)` when planning is impossible (no world, no targets, no path)
/// and the caller should fall back to `walk_fleeing`.
pub fn walk_planned(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    leg: Leg,
    bias: u16,
) -> Result<bool, RouteError> {
    const MAX_REPLANS: usize = 60;
    let version = version_of(rec.emu());
    let mut blocked: HashSet<((i16, i16), (i16, i16))> = HashSet::new();
    let mut stuck_streak = 0usize;

    for round in 0..MAX_REPLANS {
        // Scenes and ambushes first, exactly as `walk_fleeing` does.
        if obs.field_controls_locked(rec.emu()) && !obs.in_battle(rec.emu()) {
            rec.hold_mash_until(
                "the ambush to become a battle",
                keys::A,
                tuning.text_hold,
                2400,
                |emu| obs.in_battle(emu) || !obs.field_controls_locked(emu),
            )?;
            if obs.in_battle(rec.emu()) {
                handle_battle(rec, obs, tuning)?;
            }
        }
        if leg.arrived(obs, rec.emu()) {
            return Ok(true);
        }
        rec.wait_until("the player to settle", 240, |emu| obs.player_can_step(emu))?;
        if leg.arrived(obs, rec.emu()) {
            return Ok(true);
        }

        let Some(map) = obs.map(rec.emu()) else {
            return Ok(false);
        };
        let Some(pos) = obs.pos(rec.emu()) else {
            return Ok(false);
        };
        let wild_data = obs.wild_data(rec.emu());
        let wild = wild_table(map, version);

        // Plan on the current map. Everything that needs the borrowed map
        // happens inside; the plan comes out owned.
        let planned: Option<(Vec<PlanStep>, u32, bool)> = WORLD.with(|w| {
            let mut w = w.borrow_mut();
            let world = w.as_mut()?;
            let data = match world.map(map) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("      no decode for map {map:?} ({e})");
                    return None;
                }
            };
            let (targets, crossing) = leg_targets(&leg, data, map, bias)?;
            let req = PlanRequest {
                map: data,
                wild,
                start: pos,
                wild_data,
                targets,
                blocked: blocked.clone(),
            };
            plan::plan(&req).map(|(steps, cost)| (steps, cost, crossing))
        });
        let Some((steps, cost, needs_crossing)) = planned else {
            return Ok(false);
        };
        if round == 0 {
            eprintln!(
                "      walk {leg:?}: planned {} steps (model cost {cost})",
                steps.len()
            );
        }

        let mut diverged = false;
        for step in &steps {
            if leg.arrived(obs, rec.emu()) {
                return Ok(true);
            }
            let before = rec.save_state()?;
            let mut outcome = None;
            // First attempt with no delay, then steer.
            for (try_no, delay) in std::iter::once(0)
                .chain(STEER_DELAYS.iter().copied())
                .enumerate()
            {
                rec.emu().load_state(&before)?;
                let mut trial = Trial::new(rec.emu());
                let result = attempt_step(&mut trial, obs, step.dir, delay)?;
                let inputs = trial.into_inputs();
                match result {
                    StepTry::Moved { consumed } => {
                        let planned_battle =
                            matches!(step.kind, StepKind::Consume { fated_pass: true, .. });
                        if consumption_matches(step.kind, consumed) && !planned_battle {
                            outcome = Some((inputs, StepTry::Moved { consumed }));
                            break;
                        }
                        // Wrong branch of a gate: steer with one more delay
                        // frame. If steering runs out, accept and replan --
                        // the model realigns from RAM.
                        if try_no == STEER_DELAYS.len() {
                            outcome = Some((inputs, StepTry::Moved { consumed }));
                            diverged = true;
                        }
                    }
                    StepTry::Battle => {
                        let planned_battle =
                            matches!(step.kind, StepKind::Consume { fated_pass: true, .. })
                                || step_into_cone(step);
                        if planned_battle || try_no == STEER_DELAYS.len() {
                            outcome = Some((inputs, StepTry::Battle));
                            break;
                        }
                    }
                    StepTry::Script => {
                        outcome = Some((inputs, StepTry::Script));
                        break;
                    }
                    StepTry::Stuck => {
                        // Delays do not move NPCs much; bail to the stuck
                        // path quickly.
                        if try_no >= 1 {
                            outcome = Some((inputs, StepTry::Stuck));
                            break;
                        }
                    }
                }
                if outcome.is_some() {
                    break;
                }
            }

            let (inputs, result) = outcome.expect("the retry loop always resolves");
            match result {
                StepTry::Moved { .. } => {
                    rec.emu().load_state(&before)?;
                    rec.play(&inputs)?;
                    stuck_streak = 0;
                    if diverged {
                        break; // replan from the realigned state
                    }
                }
                StepTry::Battle => {
                    rec.emu().load_state(&before)?;
                    rec.play(&inputs)?;
                    handle_battle(rec, obs, tuning)?;
                    stuck_streak = 0;
                    diverged = true;
                    break; // modifiers reset; replan
                }
                StepTry::Script => {
                    rec.emu().load_state(&before)?;
                    rec.play(&inputs)?;
                    // The round preamble drives the script/ambush.
                    stuck_streak = 0;
                    diverged = true;
                    break;
                }
                StepTry::Stuck => {
                    rec.emu().load_state(&before)?;
                    let from = obs.pos(rec.emu()).unwrap_or((0, 0));
                    blocked.insert((from, step.to));
                    stuck_streak += 1;
                    if stuck_streak >= 3 {
                        // An NPC camping the corridor: wait a beat and give
                        // the tile another chance.
                        rec.idle(48)?;
                        blocked.clear();
                        stuck_streak = 0;
                    }
                    diverged = true;
                    break;
                }
            }
        }

        if !diverged {
            // The plan ran to its last tile. A connection crossing still
            // needs the actual off-map step.
            if needs_crossing && !leg.arrived(obs, rec.emu()) {
                let start_map = obs.map(rec.emu());
                let crossed = rec.advance_while("the map connection", &[bias], 120, |emu| {
                    obs.map(emu) != start_map
                });
                if crossed.is_err() {
                    return Ok(false);
                }
                rec.wait_until("the player to settle", 300, |emu| {
                    obs.player_can_step(emu) || obs.in_battle(emu)
                })?;
                if obs.in_battle(rec.emu()) {
                    handle_battle(rec, obs, tuning)?;
                }
            }
            if leg.arrived(obs, rec.emu()) {
                return Ok(true);
            }
            // Ran the whole plan and did not arrive (a Near/Tile goal the
            // model mis-placed): replan once more; the round loop caps it.
        }
    }
    Err(RouteError::Timeout {
        what: format!("{leg:?} within the replan budget"),
        budget: MAX_REPLANS,
        frames: rec.frames(),
    })
}

/// Whether a planned step knowingly enters a trainer sight cone -- the
/// planner prices cones rather than forbidding them, so a battle on such a
/// step is the plan working, not failing. Detected by cost bookkeeping being
/// unavailable per step: the executor simply treats any battle on the last
/// steer attempt as accepted, so this only matters for skipping the steering
/// retries on genuinely planned fights. Conservative `false` keeps the
/// retries; they cost ~100 emulated frames.
fn step_into_cone(_step: &PlanStep) -> bool {
    false
}

/// The tiles a leg wants, on the decoded map, plus whether reaching them
/// still leaves a connection crossing to do.
fn leg_targets(
    leg: &Leg,
    data: &crate::world::MapData,
    current_map: (u8, u8),
    bias: u16,
) -> Option<(Vec<(i16, i16)>, bool)> {
    match *leg {
        Leg::Tile(map, x, y) => (map == current_map).then(|| (vec![(x, y)], false)),
        Leg::Near(map, x, y, tol) => (map == current_map).then(|| {
            let mut tiles = Vec::new();
            let t = tol as i16;
            for dx in -t..=t {
                for dy in -t..=t {
                    if dx.abs() + dy.abs() > t {
                        continue;
                    }
                    let (px, py) = (x + dx, y + dy);
                    if data.tile(px, py).is_some_and(|t| t.collision == 0) {
                        tiles.push((px, py));
                    }
                }
            }
            (tiles, false)
        }),
        Leg::MapVia(_, via_map, (vx, vy)) => {
            if via_map != current_map {
                return None;
            }
            // A warp near the via tile means "walk onto the warp"; otherwise
            // it is a map connection and the plan ends on the edge, one held
            // step short of the next map.
            let warps: Vec<(i16, i16)> = data
                .warps
                .iter()
                .filter(|w| (w.x - vx).abs() + (w.y - vy).abs() <= 4)
                .map(|w| (w.x, w.y))
                .collect();
            if !warps.is_empty() {
                return Some((warps, false));
            }
            let (w, h) = (data.width as i16, data.height as i16);
            let edge: Vec<(i16, i16)> = match bias {
                b if b == frlg_emu::keys::UP => (0..w).map(|x| (x, 0)).collect(),
                b if b == frlg_emu::keys::DOWN => (0..w).map(|x| (x, h - 1)).collect(),
                b if b == frlg_emu::keys::LEFT => (0..h).map(|y| (0, y)).collect(),
                _ => (0..h).map(|y| (w - 1, y)).collect(),
            };
            let near: Vec<(i16, i16)> = edge
                .into_iter()
                .filter(|&(x, y)| {
                    (x - vx).abs() + (y - vy).abs() <= 8
                        && data.tile(x, y).is_some_and(|t| t.collision == 0)
                })
                .collect();
            (!near.is_empty()).then_some((near, true))
        }
    }
}

/// Plan-first walking with the emulator search as the fallback. The planner
/// covers everything the static model can see; `walk_fleeing` remains for
/// what it cannot (a mis-decoded tile, a scripted obstacle).
pub fn walk_smart(
    rec: &mut Recorder,
    obs: &Observer,
    tuning: Tuning,
    leg: Leg,
    bias: u16,
    max_nodes: usize,
) -> Result<(), RouteError> {
    match walk_planned(rec, obs, tuning, leg, bias) {
        Ok(true) => return Ok(()),
        Ok(false) => eprintln!("      walk {leg:?}: no plan; emulator search takes over"),
        Err(e) => eprintln!("      walk {leg:?}: planned walk gave up ({e}); emulator search takes over"),
    }
    walk_fleeing(rec, obs, tuning, leg, bias, max_nodes)
}

// ---------------------------------------------------------------------------
// The segments.

/// The post-battle lab script plays itself (rival leaves,
/// `..._EventScript_EndRivalBattle`, `data/maps/PalletTown_ProfessorOaksLab/
/// scripts.inc:467-481`); B advances its one msgbox. It ends with the scene
/// var at 4 and control returned; then walk out the door.
fn exit_lab(tuning: Tuning) -> Segment {
    Segment {
        name: "exit-lab",
        goal: "back outside in Pallet Town, rival gone (lab scene var 4)".into(),
        run: Box::new(move |rec, obs| {
            rec.hold_mash_until(
                "the rival to leave",
                keys::B,
                tuning.text_hold,
                4000,
                |emu| obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(4) && scene_over(obs, emu),
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(PALLET_TOWN, OAKS_LAB, (6, 12)),
                keys::DOWN,
                1500,
            )?;
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
        name: "to-viridian",
        goal: "in Viridian City".into(),
        run: Box::new(move |rec, obs| {
            // Connections (research/story-gates.md, map.json cites): Pallet's
            // north exit is x=12/13 row 0; Route 1's top x=10..13 meets
            // Viridian's bottom x=22..25.
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, PALLET_TOWN, (12, 1)),
                keys::UP,
                600,
            )?;
            walk_smart(
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
        name: "parcel",
        goal: "Oak's Parcel in the bag, back outside the mart".into(),
        run: Box::new(move |rec, obs| {
            // The mart door warp is at (36,19) (`data/maps/ViridianCity/
            // map.json:194-200`).
            walk_smart(
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
                |emu| obs.var(emu, VAR_VIRIDIAN_MART) == Some(1) && scene_over(obs, emu),
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_CITY, VIRIDIAN_MART, (4, 8)),
                keys::DOWN,
                1500,
            )?;
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
        name: "deliver",
        goal: "Pokédex received, old man armed (his scene var 1)".into(),
        run: Box::new(move |rec, obs| {
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, VIRIDIAN_CITY, (23, 38)),
                keys::DOWN,
                1000,
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(PALLET_TOWN, ROUTE1, (12, 38)),
                keys::DOWN,
                1000,
            )?;
            // The lab door in Pallet is at (16,5)-ish; entering is a warp
            // like any other.
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(OAKS_LAB, PALLET_TOWN, (16, 6)),
                keys::UP,
                2000,
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::Tile(OAKS_LAB, OAK_TALK_TILE.0, OAK_TALK_TILE.1),
                keys::UP,
                2000,
            )?;
            // Face Oak and talk; the scene has no yes/no prompts, so B/A
            // both only advance. A opens the dialogue. Settle first -- a
            // turn pressed mid-step is swallowed.
            rec.wait_until("the player to settle", 240, |emu| obs.player_can_step(emu))?;
            rec.hold(keys::UP, 2)?;
            rec.idle(6)?;
            rec.hold_mash_until(
                "the delivery scene",
                keys::A,
                tuning.text_hold,
                8000,
                |emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(1) && scene_over(obs, emu),
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
        name: "tutorial",
        goal: "catching tutorial done (old man var 2), road north open".into(),
        run: Box::new(move |rec, obs| {
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE1, PALLET_TOWN, (12, 1)),
                keys::UP,
                600,
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(VIRIDIAN_CITY, ROUTE1, (12, 1)),
                keys::UP,
                1000,
            )?;
            walk_smart(
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
            // Step up into the trigger tile; the coord event grabs the
            // player the moment the step lands.
            rec.advance_while("the tutorial trigger to fire", &[keys::UP], 240, |emu| {
                obs.field_controls_locked(emu)
            })?;
            // The demo battle plays itself with A advancing its text.
            rec.hold_mash_until(
                "the catching tutorial",
                keys::A,
                tuning.text_hold,
                12000,
                |emu| obs.var(emu, VAR_VIRIDIAN_OLD_MAN) == Some(2) && scene_over(obs, emu),
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
        name: "to-forest",
        goal: "inside Viridian Forest".into(),
        run: Box::new(move |rec, obs| {
            // Viridian's north exit x=19..23; the forest south-entrance
            // warps sit at Route 2 (5,51)/(6,51); the entrance building's
            // exit warp is (7,1) (research/story-gates.md).
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE2, VIRIDIAN_CITY, (21, 1)),
                keys::UP,
                1500,
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(FOREST_SOUTH_ENTRANCE, ROUTE2, (5, 52)),
                keys::UP,
                1000,
            )?;
            walk_smart(
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
        name: "forest",
        goal: "out the forest's north side".into(),
        run: Box::new(move |rec, obs| {
            // One planned leg: the A* in `plan.rs` shapes the whole
            // crossing against the decoded layout and the fated rate-test
            // stream, Sammy's cone priced as the only forced fight
            // (`research/story-gates.md`). The waypoint chain below is the
            // pre-planner fallback, kept because the forest is the one map
            // where a fallback that re-derives the maze from scratch
            // costs hours.
            match walk_planned(
                rec,
                obs,
                tuning,
                Leg::MapVia(FOREST_NORTH_ENTRANCE, VIRIDIAN_FOREST, (5, 10)),
                keys::UP,
            ) {
                Ok(true) => return Ok(()),
                Ok(false) => eprintln!("      forest: no plan; waypoint fallback"),
                Err(e) => eprintln!("      forest: planned walk gave up ({e}); waypoint fallback"),
            }
            // The forest is a maze of walled grass columns; the decoded
            // layout is committed as `research/forest-map.txt` (formats:
            // `include/global.fieldmap.h:4-11`, `src/fieldmap.c:61-83`).
            // The canonical path from the entrance (29,62): north up the
            // x=42..44 clearing, the east grass column x=39..43 to the
            // open north-east, west along the rows 15..17 grass corridor,
            // down column 4 into the middle clearing, up column 3, west
            // along the top corridor, down column 2, west at row 27, and
            // up column 1 -- whose row 22 is Bug Catcher Sammy's sight
            // line, the genuinely forced fight -- to the exit pocket at
            // (4..6,9..11). The row 39..42 block is a dead end (sealed at
            // rows 36..38); six hours of round-robin there taught us to
            // decode the map instead of poking it.
            let waypoints: [((i16, i16), u16, u16); 12] = [
                ((41, 44), 2, keys::UP),
                ((41, 30), 2, keys::UP),
                ((45, 17), 2, keys::UP),
                ((29, 16), 2, keys::LEFT),
                ((29, 23), 2, keys::DOWN),
                ((21, 23), 2, keys::LEFT),
                ((21, 12), 1, keys::UP),
                ((13, 11), 1, keys::LEFT),
                ((13, 26), 1, keys::DOWN),
                ((9, 27), 1, keys::LEFT),
                ((5, 24), 1, keys::UP),
                ((5, 18), 1, keys::UP),
            ];
            for ((x, y), tol, bias) in waypoints {
                walk_smart(
                    rec,
                    obs,
                    tuning,
                    Leg::Near(VIRIDIAN_FOREST, x, y, tol),
                    bias,
                    1200,
                )?;
            }
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(FOREST_NORTH_ENTRANCE, VIRIDIAN_FOREST, (5, 10)),
                keys::UP,
                1500,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(FOREST_NORTH_ENTRANCE)),
    }
}

/// Route 2's north half (grass bypassable) into Pewter City (no wild header
/// at all). North entrance exit warp (7,1); Route 2's top x=8..11 meets
/// Pewter's bottom x=20..23 (research/story-gates.md).
///
/// This used to be `heal-pewter`, which detoured through the Pokémon Center:
/// the semi-naive run arrived from the forest's two fights at 6/28 HP and
/// lost all 192 Brock start delays unhealed. The seed-38 run arrives at
/// 20/23 (Rick dodged, one fight taken), the no-heal probe beat Brock from
/// there (55/192 start delays win), and the 1794-frame detour was deleted --
/// `git log route/defeat-brock` has the healed variant if a future stream
/// arrives low again.
fn to_pewter(tuning: Tuning) -> Segment {
    Segment {
        name: "to-pewter",
        goal: "in Pewter City".into(),
        run: Box::new(move |rec, obs| {
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(ROUTE2, FOREST_NORTH_ENTRANCE, (7, 2)),
                keys::UP,
                400,
            )?;
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::MapVia(PEWTER_CITY, ROUTE2, (9, 1)),
                keys::UP,
                1000,
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(PEWTER_CITY)),
    }
}

/// The gym door at Pewter (15,16); the gym-guide triggers live at x=42-43
/// and are never approached (`research/story-gates.md`).
fn to_gym(tuning: Tuning) -> Segment {
    Segment {
        name: "to-gym",
        goal: "inside Pewter Gym".into(),
        run: Box::new(move |rec, obs| {
            walk_smart(
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
        name: "brock",
        goal: "FLAG_DEFEATED_BROCK set".into(),
        run: Box::new(move |rec, obs| {
            walk_smart(
                rec,
                obs,
                tuning,
                Leg::Tile(PEWTER_GYM, BROCK_TALK_TILE.0, BROCK_TALK_TILE.1),
                keys::UP,
                3000,
            )?;
            // Settle out of the arrival step first: a turn pressed into a
            // running step animation is swallowed, and the A-mash then
            // interrogates whatever the walk happened to face (measured:
            // 2000 frames of A at the empty tile east of (6,6)).
            rec.wait_until("the player to settle", 240, |emu| obs.player_can_step(emu))?;
            rec.hold(keys::UP, 2)?;
            rec.idle(6)?;
            rec.mash_until("the battle to start", keys::A, 3000, |emu| {
                obs.in_battle(emu)
            })?;
            // 2/48 delays won on one stream and 0/48 on its sibling: this
            // fight is knife-edge at the semi-naive level, so it samples
            // wide. 192 delays cover 384 stream steps.
            win_battle(rec, obs, tuning, Some(preferred), "brock", 192)?;
            rec.hold_mash_until("the defeat flag", keys::B, tuning.text_hold, 3000, |emu| {
                obs.flag(emu, FLAG_DEFEATED_BROCK) == Some(true)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.flag(emu, FLAG_DEFEATED_BROCK) == Some(true)),
    }
}
