//! The route, as segments.
//!
//! Each segment drives the game from the previous segment's end state to an
//! observable of its own, and is judged by that observable rather than by a
//! frame count -- a segment that "worked" but left the game somewhere else is
//! a failure the ledger has to be able to see.
//!
//! Map ids are `(group, number)` indices into
//! `decompiled/data/maps/map_groups.json`; tile coordinates are the ones in
//! each map's `map.json`, which is the same space as
//! `gSaveBlock1Ptr->pos`. Every scripted beat below cites the script it is
//! stepping through.

use frlg_emu::{keys, Emu};

use crate::nav::{self, Goal};
use crate::observe::{Observer, B_OUTCOME_WON, VAR_OAKS_LAB_SCENE, VAR_STARTER_MON};
use crate::record::{Recorder, RouteError};

/// `data/maps/map_groups.json`, `group_order` index and position in the group.
pub const PLAYERS_HOUSE_2F: (u8, u8) = (4, 1);
pub const PALLET_TOWN: (u8, u8) = (3, 0);
pub const OAKS_LAB: (u8, u8) = (4, 3);

/// `data/maps/PalletTown/map.json`: the `coord_events` that run
/// `PalletTown_EventScript_OakTriggerLeft` -- Oak's "don't go out" scene, which
/// ends by warping the player into the lab.
const OAK_TRIGGER: (i16, i16) = (12, 1);

/// `data/maps/PalletTown_ProfessorOaksLab/map.json`: the three
/// `OBJ_EVENT_GFX_ITEM_BALL` object events, and the `coord_events` row that
/// fires `..._EventScript_RivalBattleTrigger*` once the scene var is 3.
const BALL_ROW_Y: i16 = 4;
const BATTLE_TRIGGER: (i16, i16) = (6, 8);

/// Which ball to take. The numbering is `VAR_STARTER_MON`'s
/// (`decompiled/include/constants/vars.h:98`), and the rival always takes the
/// one that beats it -- see the `RIVAL_STARTER_SPECIES` each ball script sets
/// in `data/maps/PalletTown_ProfessorOaksLab/scripts.inc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Starter {
    Bulbasaur,
    Squirtle,
    Charmander,
}

impl Starter {
    pub const ALL: [Starter; 3] = [Starter::Bulbasaur, Starter::Squirtle, Starter::Charmander];

    pub fn name(self) -> &'static str {
        match self {
            Starter::Bulbasaur => "bulbasaur",
            Starter::Squirtle => "squirtle",
            Starter::Charmander => "charmander",
        }
    }

    /// `decompiled/include/constants/species.h`.
    pub fn species(self) -> u16 {
        match self {
            Starter::Bulbasaur => 1,
            Starter::Squirtle => 7,
            Starter::Charmander => 4,
        }
    }

    /// What `VAR_STARTER_MON` holds after the choice.
    pub fn var_value(self) -> u16 {
        match self {
            Starter::Bulbasaur => 0,
            Starter::Squirtle => 1,
            Starter::Charmander => 2,
        }
    }

    /// The x of its ball on the lab's table.
    fn ball_x(self) -> i16 {
        match self {
            Starter::Bulbasaur => 8,
            Starter::Squirtle => 9,
            Starter::Charmander => 10,
        }
    }

    /// What the rival ends up with: the type that beats yours.
    pub fn rival_species(self) -> u16 {
        match self {
            Starter::Bulbasaur => Starter::Charmander.species(),
            Starter::Squirtle => Starter::Bulbasaur.species(),
            Starter::Charmander => Starter::Squirtle.species(),
        }
    }
}

/// Drives the game through a segment, recording every frame it advances.
pub type Drive = Box<dyn Fn(&mut Recorder, &Observer) -> Result<(), RouteError>>;
/// Answers "is the game where the segment says it should be?".
pub type Check = Box<dyn Fn(&Observer, &mut Emu) -> bool>;

/// One step of the route: how to get there, and how to tell that you did.
pub struct Segment {
    pub name: &'static str,
    /// What the segment is for, in one line, for the ledger and the log.
    pub goal: String,
    /// Drives the game. Must leave it satisfying `reached`.
    pub run: Drive,
    /// The observable. Checked by the builder *and* by the verifier, which is
    /// the only reason a replayed log can be trusted.
    pub reached: Check,
}

/// The whole route, in order.
pub fn all(starter: Starter) -> Vec<Segment> {
    vec![
        boot(),
        intro_oak(),
        names(),
        house(),
        to_lab(),
        starter_segment(starter),
        battle_start(),
        battle_win(),
    ]
}

/// Reset -> title -> main menu -> NEW GAME.
///
/// A is the whole segment: it dismisses the copyright screen, skips the intro
/// movie, starts the title screen and picks the first main-menu entry, which
/// with no save file is NEW GAME (`CB2_MainMenu`, `src/main_menu.c`).
fn boot() -> Segment {
    Segment {
        name: "01-boot",
        goal: "NEW GAME selected (CB2_NewGameScene running)".into(),
        run: Box::new(|rec, obs| {
            rec.mash_until("NEW GAME", keys::A, 2000, |emu| {
                obs.callback2_is(emu, "CB2_NewGameScene")
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.callback2_is(emu, "CB2_NewGameScene")),
    }
}

/// Oak's speech, up to and including the boy/girl choice.
///
/// The choice is a menu, so mashing A takes the highlighted entry; reaching the
/// naming screen at all proves the choice was made, since `CB2_LoadNamingScreen`
/// is what the gender menu hands over to.
fn intro_oak() -> Segment {
    Segment {
        name: "02-intro-oak",
        goal: "the player naming screen is up (gender chosen)".into(),
        run: Box::new(|rec, obs| {
            rec.mash_until("the naming screen", keys::A, 4000, |emu| {
                obs.callback2_is(emu, "CB2_NamingScreen")
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.callback2_is(emu, "CB2_NamingScreen")),
    }
}

/// Both names, then the rest of the intro, ending in the bedroom.
///
/// Mashing A on the naming screen enters whatever letter the cursor starts on
/// until the name is full and the screen accepts it. That is almost certainly
/// not optimal -- picking a preset name is the obvious alternative -- and it is
/// the first thing the optimisation pass should measure.
fn names() -> Segment {
    Segment {
        name: "03-names",
        goal: "standing in the bedroom, both names entered".into(),
        run: Box::new(|rec, obs| {
            rec.mash_until("the overworld", keys::A, 6000, |emu| {
                obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| {
            obs.callback2_is(emu, "CB2_Overworld")
                && obs.map(emu) == Some(PLAYERS_HOUSE_2F)
                && obs.player_can_step(emu)
        }),
    }
}

/// Bedroom -> ground floor -> out the front door.
fn house() -> Segment {
    Segment {
        name: "04-house",
        goal: "outside, in Pallet Town".into(),
        run: Box::new(|rec, obs| {
            nav::walk_to(rec, obs, Goal::on_map(PALLET_TOWN), 4000)?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(PALLET_TOWN)),
    }
}

/// North to the route exit, where Oak stops the player and walks them to his
/// lab (`PalletTown_EventScript_OakTrigger`, which ends in
/// `warp MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB`).
fn to_lab() -> Segment {
    Segment {
        name: "05-to-lab",
        goal: "inside Oak's lab, after his interruption".into(),
        run: Box::new(|rec, obs| {
            nav::walk_to(
                rec,
                obs,
                Goal::tile(PALLET_TOWN, OAK_TRIGGER.0, OAK_TRIGGER.1),
                6000,
            )?;
            // The scene talks, walks the player south and warps. A advances
            // its one msgbox; the rest is on a timer.
            rec.mash_until("the warp into the lab", keys::A, 3000, |emu| {
                obs.map(emu) == Some(OAKS_LAB)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.map(emu) == Some(OAKS_LAB)),
    }
}

/// Oak's offer, then take a ball.
///
/// Two prompts have to be answered differently, which is why this is not a
/// mash: `..._EventScript_ConfirmStarterChoice` asks YES/NO to the starter (A =
/// yes), and `EventScript_ChoseStarter` then asks YES/NO to a nickname, where
/// yes costs a whole naming screen. B answers no to that one -- and B also
/// advances the text either side of it, so the tail of this segment is a B
/// mash rather than an A mash.
///
/// It ends with the rival taking his, because the battle trigger is inert until
/// `VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB` reaches 3.
fn starter_segment(starter: Starter) -> Segment {
    Segment {
        name: "06-starter",
        goal: format!("{} in the party, rival has his", starter.name()),
        run: Box::new(move |rec, obs| {
            // Entering the lab runs ChooseStarterScene off the on-frame table:
            // Oak walks the player up the room and offers the three balls. It
            // ends with `releaseall`, i.e. the player can move again, and with
            // the scene var at 2.
            rec.mash_until("Oak's offer", keys::A, 4000, |emu| {
                obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(2) && obs.player_can_step(emu)
            })?;

            // Stand below the ball and face it. The ball is an object event, so
            // pressing up against it turns the player without moving them.
            nav::walk_to(
                rec,
                obs,
                Goal::tile(OAKS_LAB, starter.ball_x(), BALL_ROW_Y + 1),
                4000,
            )?;
            rec.wait_until("the player to settle", 240, |emu| obs.player_can_step(emu))?;
            rec.hold(keys::UP, 8)?;
            rec.idle(1)?;

            // A opens the ball's script and answers YES to "so, you want it?".
            // Stop the moment the mon is in the party -- the next prompt is the
            // nickname one, and A would say yes.
            rec.mash_until("the starter in the party", keys::A, 1200, |emu| {
                obs.party_count(emu) == 1
            })?;
            // B: no nickname, and it advances everything else up to the rival
            // taking his ball.
            rec.mash_until("the rival to take his", keys::B, 4000, |emu| {
                obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(3) && obs.player_can_step(emu)
            })?;
            Ok(())
        }),
        reached: Box::new(move |obs, emu| {
            obs.party_count(emu) == 1
                && obs.var(emu, VAR_STARTER_MON) == Some(starter.var_value())
                && obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(3)
        }),
    }
}

/// Walk onto the trigger row; the rival turns round and the battle starts.
fn battle_start() -> Segment {
    Segment {
        name: "07-battle-start",
        goal: "the rival battle has started".into(),
        run: Box::new(|rec, obs| {
            nav::walk_to(
                rec,
                obs,
                Goal::tile(OAKS_LAB, BATTLE_TRIGGER.0, BATTLE_TRIGGER.1),
                4000,
            )?;
            rec.mash_until("the battle to start", keys::A, 3000, |emu| {
                obs.in_battle(emu)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.in_battle(emu)),
    }
}

/// Fight. A takes FIGHT and then the first move, and advances every message in
/// between, so the whole battle is one mash until `gBattleOutcome` is set.
///
/// Whether that wins is not a given -- both mons are level 5 with no
/// type-effective moves, so it comes down to damage rolls and criticals, which
/// is exactly what the optimisation pass has to look at.
fn battle_win() -> Segment {
    Segment {
        name: "08-battle-win",
        goal: "gBattleOutcome == B_OUTCOME_WON".into(),
        run: Box::new(|rec, obs| {
            rec.mash_until("the battle to end", keys::A, 20000, |emu| {
                obs.battle_outcome(emu) != 0
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.battle_outcome(emu) == B_OUTCOME_WON),
    }
}
