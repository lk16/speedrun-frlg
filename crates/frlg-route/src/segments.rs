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
use crate::observe::{self, Observer, B_OUTCOME_WON, VAR_OAKS_LAB_SCENE, VAR_STARTER_MON};
use crate::record::{Feed, Recorder, RouteError, Trial};

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

/// Which game the ROM is, read from its header
/// ([`frlg_emu::game_code`]). The two versions are one route with one
/// version-dependent beat: the rival's preset-name menu rows
/// (`sRivalNameChoices`, `decompiled/src/oak_speech.c:649-658` -- GREEN,
/// GARY, KAZ, TORU on FireRed; RED, ASH, KENE, GEKI on LeafGreen).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Version {
    FireRed,
    LeafGreen,
}

impl Version {
    /// From the header's game code: BPRE is FireRed, BPGE is LeafGreen
    /// (`decompiled/config.mk:29-57`).
    pub fn from_game_code(code: [u8; 4]) -> Option<Version> {
        match &code {
            b"BPRE" => Some(Version::FireRed),
            b"BPGE" => Some(Version::LeafGreen),
            _ => None,
        }
    }

    pub fn of_rom(rom: &std::path::Path) -> std::io::Result<Option<Version>> {
        Ok(Self::from_game_code(frlg_emu::game_code(rom)?))
    }
}

/// Knobs whose right value is not a local question.
///
/// Measured the hard way: trimming `turn_hold` from 8 frames to the 1 that
/// still works saved 6 frames in `06-starter` and cost 391 in the battle,
/// because every frame before a battle moves `gRngValue` and the battle is
/// worth two orders of magnitude more than the trim. So these are route-level
/// variants, swept end-to-end by `frlg route tune`, not decisions a segment
/// gets to make for itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Tuning {
    /// Frames of UP held to turn towards the starter's ball without walking.
    pub turn_hold: usize,
    /// Frames A (or B) is *held* per one-frame release in every dialogue
    /// mash. 1 is the plain `[keys, 0]` mash. Longer holds print text faster
    /// -- `RenderText` zeroes the per-character delay on held frames once one
    /// press has landed (`decompiled/src/text.c:639-650`), and both the
    /// intro's boxes and the battle's enable that
    /// (`decompiled/src/oak_speech.c:761-762`,
    /// `decompiled/src/battle_message.c:2778-2785`) -- but register each
    /// menu-advancing press up to `text_hold` frames later.
    #[serde(default = "text_hold_compat")]
    pub text_hold: usize,
}

/// What `text_hold` was before it existed: ledgers written without the field
/// were built by drives that mashed `[keys, 0]`.
fn text_hold_compat() -> usize {
    1
}

impl Default for Tuning {
    fn default() -> Self {
        // turn_hold: the value the 2026-08-12 sweep settled on (10085 frames;
        // 8, the previous winner, scored 10531 and 7 could not win its battle
        // at all); `frlg route tune` re-derives it, and the ledger's recorded
        // value wins over this default whenever one exists.
        Self {
            turn_hold: 2,
            text_hold: 1,
        }
    }
}

impl Tuning {
    /// The variants a tuning sweep tries: the product of both knobs.
    /// `text_hold`'s candidates are the top of the intro-only measurement
    /// (`text_hold_on_the_intro_alone`, 2026-08-12: 1 -> 3699, 2 -> 3361,
    /// 4 -> 3229 frames, with 3, 7 and above all worse) plus 1 as the
    /// baseline; the landscape is phase-alignment, not a curve, so the sweep
    /// re-tries the measured leaders rather than a spread.
    pub fn variants() -> impl Iterator<Item = Tuning> {
        (1..=8).flat_map(|turn_hold| {
            [1usize, 2, 4].into_iter().map(move |text_hold| Tuning {
                turn_hold,
                text_hold,
            })
        })
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
pub fn all(version: Version, starter: Starter, tuning: Tuning) -> Vec<Segment> {
    vec![
        boot(),
        intro_oak(tuning),
        names(version, tuning),
        options(),
        house(),
        to_lab(tuning),
        starter_segment(starter, tuning),
        battle_start(),
        battle_win(tuning),
    ]
}

/// Which TAS this build is for. A target is a segment list and a home
/// directory under `route/<name>/`; rival-1 is a strict prefix of
/// defeat-brock, so the enum picks how far the build runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Rival1,
    DefeatBrock,
}

impl Target {
    pub fn parse(name: &str) -> Option<Target> {
        match name {
            "rival-1" => Some(Target::Rival1),
            "defeat-brock" => Some(Target::DefeatBrock),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Target::Rival1 => "rival-1",
            Target::DefeatBrock => "defeat-brock",
        }
    }

    /// The target's segments, in order.
    pub fn segments(self, version: Version, starter: Starter, tuning: Tuning) -> Vec<Segment> {
        let mut segments = all(version, starter, tuning);
        if self == Target::DefeatBrock {
            segments.extend(crate::brock::segments(starter, tuning));
        }
        segments
    }
}

/// The task that owns both preset-name menus
/// (`decompiled/src/oak_speech.c:1413`). Its name says "rival" but it handles
/// the player's menu too -- `hasPlayerBeenNamed` is what distinguishes them.
const NAME_MENU_TASK: &str = "Task_OakSpeech_HandleRivalNameInput";

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
/// The choice is a menu, so mashing A takes the highlighted entry (BOY);
/// reaching the naming screen at all proves the choice was made. There is no
/// preset menu on the way in -- `Task_OakSpeech_YourNameWhatIsIt` hands
/// straight to the naming screen fade
/// (`decompiled/src/oak_speech.c:1352-1379`); the presets only appear on the
/// re-ask path the route never takes.
fn intro_oak(tuning: Tuning) -> Segment {
    Segment {
        name: "02-intro-oak",
        goal: "the player naming screen is up (gender chosen)".into(),
        run: Box::new(move |rec, obs| {
            rec.hold_mash_until(
                "the naming screen",
                keys::A,
                tuning.text_hold,
                4000,
                |emu| obs.callback2_is(emu, "CB2_NamingScreen"),
            )?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.callback2_is(emu, "CB2_NamingScreen")),
    }
}

/// Both names, then the rest of the intro, ending in the bedroom.
///
/// The mashed version typed seven letters per name and cost 1450 frames; this
/// one gets a one-character player name and a three-character rival name, and
/// the difference is billed on every later message box that prints either --
/// a printed character costs `sTextSpeedFrameDelays[speed]` frames
/// (`decompiled/src/new_menu_helpers.c:27`).
///
/// The two names go through different machinery:
///
/// - **The player always gets the naming screen** --
///   `Task_OakSpeech_YourNameWhatIsIt` fades straight into it
///   (`decompiled/src/oak_speech.c:1352-1379`); the preset menu only exists
///   on the say-NO re-ask path. So: one letter, then START, which is a
///   shortcut to the OK button (`HandleKeyboardEvent`,
///   `decompiled/src/naming_screen.c:1485`), then A. A one-character name is
///   accepted -- `SaveInputText` (`naming_screen.c:1851`) copies anything
///   with a non-space character in it.
/// - **The rival's menu is real and its rows are literal**: NEW NAME on top,
///   then `sRivalNameChoices` -- GREEN, GARY, KAZ, TORU on FireRed; RED,
///   ASH, KENE, GEKI on LeafGreen (`decompiled/src/oak_speech.c:649-658`,
///   shown by `PrintNameChoiceOptions`, `:2117`). Row `n` maps to
///   `sRivalNameChoices[n - 1]` (`Task_OakSpeech_HandleRivalNameInput` ->
///   `GetDefaultName(_, input - 1)`, `oak_speech.c:1431`), and the menu
///   wraps (`Menu_MoveCursor`, `decompiled/src/menu.c:306`). The shortest
///   name is 3 characters on both versions, but on a different row: KAZ is
///   row 3 (two UPs, wrapping), RED is row 1 (one DOWN).
///
/// Both names land on a confirm box whose YES/NO menu starts on YES, so the
/// A mash between the beats answers everything correctly.
fn names(version: Version, tuning: Tuning) -> Segment {
    Segment {
        name: "03-names",
        goal: "in the bedroom, 1-char player name and 3-char rival name".into(),
        run: Box::new(move |rec, obs| {
            // The naming screen drops input until its fade-in is done.
            rec.wait_until("the naming screen to take input", 300, |emu| {
                obs.naming_screen_accepting_input(emu)
            })?;
            rec.tap(keys::A)?; // one letter, wherever the cursor starts
            rec.tap(keys::START)?; // cursor to OK
            rec.tap(keys::A)?; // OK: save the name, leave

            // Through the confirm box and the rival intro, to the rival's
            // preset menu.
            rec.hold_mash_until(
                "the rival name menu",
                keys::A,
                tuning.text_hold,
                3000,
                |emu| obs.task_active(emu, NAME_MENU_TASK),
            )?;

            // The shortest preset, then take it.
            match version {
                Version::FireRed => {
                    // KAZ: wrap upwards to row 3.
                    rec.tap(keys::UP)?;
                    rec.tap(keys::UP)?;
                }
                Version::LeafGreen => {
                    // RED: row 1.
                    rec.tap(keys::DOWN)?;
                }
            }
            rec.tap(keys::A)?;

            rec.hold_mash_until("the overworld", keys::A, tuning.text_hold, 6000, |emu| {
                obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| {
            obs.callback2_is(emu, "CB2_Overworld")
                && obs.map(emu) == Some(PLAYERS_HOUSE_2F)
                && obs.player_can_step(emu)
                && obs.player_name_len(emu) == Some(1)
                && obs.rival_name_len(emu) == Some(3)
        }),
    }
}

/// START -> OPTION in the bedroom: text speed FAST, battle animations off.
///
/// Both settings live behind the same detour, so they are priced together.
/// What they buy, cited: a printed character costs
/// `sTextSpeedFrameDelays[optionsTextSpeed]` frames -- 4 at the default MID, 1
/// at FAST (`decompiled/src/new_menu_helpers.c:27-32`) -- on every message box
/// from here to the win; and `BattleStartClearSetData` only sets
/// `HITMARKER_NO_ANIMATIONS` when `gSaveBlock2Ptr->optionsBattleSceneOff` is
/// set (`decompiled/src/battle_main.c:2259`), which skips every attack
/// animation in the rival fight.
///
/// The menu order without dex or party is BAG, PLAYER, SAVE, OPTION, EXIT
/// (`SetUpStartMenu_NormalField`, `decompiled/src/start_menu.c:213`); the
/// cursor starts on top and wraps, so two UPs reach OPTION. In the option
/// menu the cursor starts on TEXT SPEED; RIGHT moves MID -> FAST, DOWN lands
/// on BATTLE SCENE, RIGHT moves ON -> OFF, and A saves and leaves
/// (`OptionMenu_ProcessInput` returns 1 -> fade -> `CloseAndSaveOptionMenu`,
/// `decompiled/src/option_menu.c:508`). Leaving re-opens the start menu --
/// `gMain.savedCallback` is `CB2_ReturnToFieldWithOpenMenu`
/// (`StartMenuOptionCallback`, `decompiled/src/start_menu.c:531`) -- so B
/// closes it.
fn options() -> Segment {
    Segment {
        name: "04-options",
        goal: "text speed FAST and battle animations off, back in the bedroom".into(),
        run: Box::new(|rec, obs| {
            // Every press in here is a mash-until-effect, not a tap. Two
            // things eat single-frame taps: the field swallows input for ~20
            // frames after the walk-in transition (`Task_ExitNonDoor` still
            // running when `player_can_step` first goes true), and the start
            // menu lags -- measured on this core, `gMain.newKeys` goes stale
            // for runs of 2-3 frames while it is up, so a 1-frame press can
            // fall on a frame whose input is never read. Mashing until the
            // *effect* is visible cannot overshoot: the mash stops on the
            // frame the effect lands, and the next registrable edge is
            // frames away.
            rec.mash_until("the start menu", keys::START, 300, |emu| {
                obs.start_menu_taking_input(emu)
            })?;
            // Two rows up, wrapping: BAG -> EXIT -> OPTION.
            rec.mash_until("the cursor on OPTION", keys::UP, 120, |emu| {
                obs.start_menu_cursor(emu) == 3
            })?;
            rec.mash_until("the option menu", keys::A, 300, |emu| {
                obs.option_menu_accepting_input(emu)
            })?;
            // TEXT SPEED: MID -> FAST. RIGHT wraps forward through 3 values,
            // and the mash stops the frame the working value reads FAST.
            rec.mash_until("text speed FAST", keys::RIGHT, 120, |emu| {
                obs.option_menu_setting(emu, observe::MENUITEM_TEXTSPEED)
                    == Some(observe::TEXT_SPEED_FAST)
            })?;
            rec.mash_until("the cursor on BATTLE SCENE", keys::DOWN, 120, |emu| {
                obs.option_menu_cursor(emu) == Some(observe::MENUITEM_BATTLESCENE as u16)
            })?;
            rec.mash_until("battle scene OFF", keys::RIGHT, 120, |emu| {
                obs.option_menu_setting(emu, observe::MENUITEM_BATTLESCENE) == Some(1)
            })?;
            // A saves and leaves; the extra presses land during the fade,
            // which `Task_OptionMenu` ignores. Back out of the re-opened
            // start menu with B, whose registered press destroys the task.
            rec.mash_until("the start menu back", keys::A, 300, |emu| {
                obs.start_menu_taking_input(emu)
            })?;
            rec.mash_until("the start menu closed", keys::B, 120, |emu| {
                !obs.task_active(emu, "Task_StartMenuHandleInput")
            })?;
            rec.wait_until("the overworld", 120, |emu| {
                obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu)
            })?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| {
            obs.options_text_speed(emu) == Some(observe::TEXT_SPEED_FAST)
                && obs.options_battle_scene_off(emu) == Some(true)
                && obs.map(emu) == Some(PLAYERS_HOUSE_2F)
                && obs.player_can_step(emu)
        }),
    }
}

/// Bedroom -> ground floor -> out the front door.
fn house() -> Segment {
    Segment {
        name: "05-house",
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
fn to_lab(tuning: Tuning) -> Segment {
    Segment {
        name: "06-to-lab",
        goal: "inside Oak's lab, after his interruption".into(),
        run: Box::new(move |rec, obs| {
            nav::walk_to(
                rec,
                obs,
                Goal::tile(PALLET_TOWN, OAK_TRIGGER.0, OAK_TRIGGER.1),
                6000,
            )?;
            // The scene talks, walks the player south and warps. A advances
            // its one msgbox; the rest is on a timer.
            rec.hold_mash_until(
                "the warp into the lab",
                keys::A,
                tuning.text_hold,
                3000,
                |emu| obs.map(emu) == Some(OAKS_LAB),
            )?;
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
fn starter_segment(starter: Starter, tuning: Tuning) -> Segment {
    Segment {
        name: "07-starter",
        goal: format!("{} in the party, rival has his", starter.name()),
        run: Box::new(move |rec, obs| {
            // Entering the lab runs ChooseStarterScene off the on-frame table:
            // Oak walks the player up the room and offers the three balls. It
            // ends with `releaseall`, i.e. the player can move again, and with
            // the scene var at 2.
            rec.hold_mash_until("Oak's offer", keys::A, tuning.text_hold, 4000, |emu| {
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

            // Turn towards the ball. One frame of UP is enough to turn without
            // walking, but the right number is not a local question -- see
            // `Tuning`.
            rec.hold(keys::UP, tuning.turn_hold)?;
            rec.idle(1)?;

            // A opens the ball's script and answers YES to "so, you want it?".
            // Stop the moment the mon is in the party: the next prompt is the
            // nickname one, and A would say yes to that too.
            rec.hold_mash_until(
                "the starter in the party",
                keys::A,
                tuning.text_hold,
                1200,
                |emu| obs.party_count(emu) == 1,
            )?;
            // B: no nickname, and it advances everything else up to the rival
            // taking his ball.
            rec.hold_mash_until(
                "the rival to take his",
                keys::B,
                tuning.text_hold,
                4000,
                |emu| obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(3) && obs.player_can_step(emu),
            )?;
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
        name: "08-battle-start",
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

/// Fight, and manipulate the fight.
///
/// A takes FIGHT and then the first move and advances every message, so a
/// battle is one mash *plus a delay plan*. Whether it wins -- and how long it
/// runs -- is decided by the RNG stream: both mons are level 5 with no
/// type-effective moves, criticals are suppressed only until the tutorial has
/// said its piece (`decompiled/src/battle_script_commands.c:1199`), and what
/// is left is the 85-100% damage roll (`:1558`) and accuracy (`:1093`), all
/// off `gRngValue` (`decompiled/src/random.c`), which advances once per frame
/// (`decompiled/src/main.c:412`). Idle frames are therefore the manipulation
/// primitive, and there is one useful place to spend them besides the start:
/// each turn's action selection, whose state the battle re-enters once per
/// turn (`BattleTurnPassed`, `decompiled/src/battle_main.c:2998`).
///
/// The search has two stages, both scored on total frames of the whole
/// battle, wins only:
///
/// 1. **Start delay, 0..64.** Delaying the first press by one frame flips
///    win to loss and back (`docs/rival-1/route.md`), and winning battles on
///    adjacent delays differ by hundreds of frames -- while the widest delay
///    costs 63. Sampling widely is cheap expected profit.
/// 2. **Per-turn delays, greedy, repeated to a fixpoint.** For each turn of
///    the current best battle, try idling 1..16 frames at that turn's
///    selection state, replaying the rest of the battle in full each time --
///    a shorter *battle* is the only accepted improvement, never a shorter
///    turn, which is the same measure-through-the-fight rule the `turn_hold`
///    sweep taught. Adopting an improvement re-rolls everything after it, so
///    later turns are searched on the improved stream -- and *earlier* turns
///    then deserve another look on the new stream too, which is why the pass
///    repeats until one full pass adopts nothing (bounded, in case the
///    landscape cycles). What still never moves: the start delay after stage
///    1, and what is pressed -- move choice is untouched.
fn battle_win(tuning: Tuning) -> Segment {
    const START_DELAYS: std::ops::Range<usize> = 0..64;
    const TURN_DELAYS: std::ops::Range<usize> = 1..16;
    /// More than any winning battle here has ever used; a plan that runs
    /// past it loses rather than aborting the build.
    const FRAME_BUDGET: usize = 20000;

    Segment {
        name: "09-battle-win",
        goal: "gBattleOutcome == B_OUTCOME_WON".into(),
        run: Box::new(move |rec, obs| {
            let start = rec.save_state()?;

            // The battle's own boxes take the held-A speed-up too
            // (`decompiled/src/battle_message.c:2778-2785` sets
            // `canABSpeedUpPrint` for B_WIN_MSG and the tutorial window), so
            // the drive mashes at the route's text_hold duty cycle.
            let mut mash: Vec<u16> = vec![keys::A; tuning.text_hold.max(1)];
            mash.push(0);

            // One battle under a delay plan: plan[0] idle frames before any
            // input, plan[k] idle frames on arriving at the k-th turn's
            // action selection. Timeouts are losses, not errors. Returns the
            // masks fed, whether it won, and how many turns it saw.
            let run_plan = |rec: &mut Recorder,
                            plan: &[usize]|
             -> Result<(Vec<u16>, bool, usize), RouteError> {
                rec.emu().load_state(&start)?;
                let mut trial = Trial::new(rec.emu());
                let mut turns = 0usize;
                trial.idle(plan.first().copied().unwrap_or(0))?;
                let won = loop {
                    // To this turn's selection state, or the end.
                    let to_menu = trial.advance_while(
                        "the battle menu or the end",
                        &mash,
                        FRAME_BUDGET,
                        |emu| obs.battle_outcome(emu) != 0 || obs.battle_choosing_actions(emu),
                    );
                    match to_menu {
                        Err(RouteError::Timeout { .. }) => break false,
                        other => other?,
                    };
                    if obs.battle_outcome(trial.core()) != 0 {
                        break obs.battle_outcome(trial.core()) == B_OUTCOME_WON;
                    }
                    turns += 1;
                    trial.idle(*plan.get(turns).unwrap_or(&0))?;
                    // Commit this turn's actions: mash until the state exits.
                    let to_turn =
                        trial.advance_while("the turn to resolve", &mash, FRAME_BUDGET, |emu| {
                            obs.battle_outcome(emu) != 0 || !obs.battle_choosing_actions(emu)
                        });
                    match to_turn {
                        Err(RouteError::Timeout { .. }) => break false,
                        other => other?,
                    };
                    if obs.battle_outcome(trial.core()) != 0 {
                        break obs.battle_outcome(trial.core()) == B_OUTCOME_WON;
                    }
                };
                Ok((trial.into_inputs(), won, turns))
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
            let (mut best_inputs, mut plan, turns) = best.ok_or_else(|| RouteError::Timeout {
                what: "any start delay to win the battle".to_string(),
                budget: START_DELAYS.end,
                frames: rec.frames(),
            })?;
            eprintln!(
                "      battle stage 1: {wins}/{} start delays win, delay {} at {} frames",
                START_DELAYS.end,
                plan[0],
                best_inputs.len()
            );

            // Stage 2: per-turn delays, greedy on the winning stream,
            // repeated until a pass adopts nothing. An adoption changes the
            // stream for every later turn *and* invalidates what the pass
            // already settled for earlier ones, so a single pass is only the
            // first approximation. The adopted plan's turn count can move as
            // the battle shortens; each pass iterates the current best
            // battle's count, and stale indices past the end simply change
            // nothing and cannot win. MAX_PASSES bounds a pathological
            // adopt-one-frame-forever landscape, far above anything observed.
            const MAX_PASSES: usize = 8;
            let mut best_turns = turns;
            for pass in 1..=MAX_PASSES {
                let mut adopted = false;
                // The pass walks the turn count the battle had when the pass
                // started; an adoption mid-pass updates `best_turns` for the
                // *next* pass.
                let pass_turns = best_turns;
                for turn in 1..=pass_turns {
                    for delay in TURN_DELAYS {
                        let mut candidate = plan.clone();
                        if candidate.len() < turn + 1 {
                            candidate.resize(turn + 1, 0);
                        }
                        if candidate[turn] == delay {
                            continue; // the current best, already measured
                        }
                        candidate[turn] = delay;
                        let (inputs, won, turns_seen) = run_plan(rec, &candidate)?;
                        if won && inputs.len() < best_inputs.len() {
                            eprintln!(
                                "      battle stage 2 (pass {pass}): turn {turn} delay {delay} -> {} frames",
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
                "      battle: plan {plan:?}, {} frames, {best_turns} turns",
                best_inputs.len()
            );
            rec.emu().load_state(&start)?;
            rec.play(&best_inputs)?;
            Ok(())
        }),
        reached: Box::new(|obs, emu| obs.battle_outcome(emu) == B_OUTCOME_WON),
    }
}
