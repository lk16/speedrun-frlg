//! The head of the route: power-on to the Route 4 Pokémon Center before Mt. Moon (`mt-moon-pc`),
//! in the plan's segment order (`crates/frlg-human/src/route.rs` prologue: `boot` .. `starter`,
//! `rival-lab`, `exit-lab`, `to-viridian`, `route1-encounters`, `parcel`, `deliver`, `tutorial`,
//! `to-forest`, `route2-encounters`, `forest`, `forest-encounters`, `forest-trainers`, `to-pewter`,
//! `pewter-gym-trainer`, `brock`; then `leg_pewter_cerulean`'s `route3`, `route3-trainers`).
//!
//! Review round 1 (`human-run/PROMPT.md`, items 1-19 of
//! `human-run/video-analysis/1122-lgdescent-human-review.md`) is what this module answers:
//!
//! - **Starter phase** (items 1, 3): a quiet wild-encounter seed picked at the title, then the
//!   starter's genome picked with the plan's manip dials (`manip.rs`, `docs/tricks/starter-manip.md`),
//!   with the human's press errors drawn from the plan's `p_hit`; every miss is a soft reset off the
//!   timer, counted and timed. One-letter rival name when the plan says so.
//! - **Options** (item 2): text Fast, battle scene off, battle style Set, at the first menu that
//!   offers them (the bedroom: the title menu has no OPTION on a fresh cart, `main_menu.c:23-24`).
//! - Potions on the way (item 9), the starter never faints (item 10, a dead branch), no Center
//!   before Brock (item 12), the Pewter mart per the plan (item 13), no Repel on Route 3/4 (item 14),
//!   Route 4 per the plan (items 18, 19).
//!
//! The R3 model's plan keys (`rival_name_len`, `early_potions`, `tm39_sell`, `pewter_balls`,
//! `route4_magikarp`, `mt_moon_heal`; `params.rs`) decide each of those choices, so the controller plays
//! whatever CURRENT says.
use frlg_emu::{keys, SaveState};
use frlg_route::Feed;
use serde_json::json;

use super::names::{id, names};
use super::prims::{settle, wait_until};
use super::session::{PResult, PlayError, Session, Stage};
use super::walk::{go, go_face, go_map, go_to, here, Node, Opts};
use super::FPS;

// ------------------------------------------------------------------------------------------------
// ------------------------------------------------------------------------------------------------
// Small RAM helpers.

/// `gTasks[i].data[k]` of the active task running `func` (`struct Task`,
/// `decompiled/include/task.h`: `func` 0x0, `isActive` 0x4, `data[16]` from 0x8, 40 bytes each).
fn task_data(s: &mut Session, func: &str, k: u32) -> Option<i16> {
    let g = s.sym("gTasks");
    let syms = s.obs.symbols().clone();
    let emu = s.hf.rec.emu();
    (0..16u32).find_map(|i| {
        let base = g + i * 40;
        let active = emu.read8(base + 4) != 0
            && matches!(syms.covering(emu.read32(base)), Some((n, _)) if n == func);
        active.then(|| emu.read16(base + 8 + 2 * k) as i16)
    })
}

fn task(s: &mut Session, func: &str) -> bool {
    s.obs.task_active(s.hf.rec.emu(), func)
}

fn cb2(s: &mut Session, name: &str) -> bool {
    s.obs.callback2_is(s.hf.rec.emu(), name)
}

/// `gRngValue` (`decompiled/src/random.c:5`).
fn rng_value(s: &mut Session) -> u32 {
    s.obs.rng(s.hf.rec.emu())
}

/// The party's first mon: personality and the six IVs (HP, Atk, Def, Spe, SpA, SpD) from the
/// decrypted misc substruct (`struct PokemonSubstruct3`, `decompiled/include/pokemon.h:40-57`: the
/// IV word at offset 4, five bits each in that order), and the nature `personality % 25`
/// (`GetNature`, `decompiled/src/pokemon.c:5020-5023`).
fn starter_genome(s: &mut Session) -> (u32, [u32; 6], u8) {
    let a = s.sym("gPlayerParty");
    let emu = s.hf.rec.emu();
    let raw = emu.read_bytes(a, 100);
    let u32_at = |o: usize| u32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]);
    let pid = u32_at(0);
    let key = pid ^ u32_at(4);
    // Substruct order per personality % 24 (`decompiled/src/pokemon.c:2817-2895`): position of
    // type 3 (misc).
    let pos_misc: [u8; 24] = [
        3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0,
    ];
    let o = 0x20 + pos_misc[(pid % 24) as usize] as usize * 12 + 4;
    let w = u32_at(o) ^ key;
    let ivs = std::array::from_fn(|i| (w >> (5 * i)) & 31);
    (pid, ivs, (pid % 25) as u8)
}

/// `Random()`'s LCG (`decompiled/src/random.c:9-13`: `gRngValue = 1103515245 * x + 24691`, return
/// the top 16 bits).
fn lcg(x: u32) -> u32 {
    x.wrapping_mul(1_103_515_245).wrapping_add(24_691)
}

/// What `CreateBoxMon` draws from `gRngValue` = `x`: `Random32()` = low half first, then two IV
/// words (`decompiled/src/pokemon.c:1778,1836-1852`, `include/random.h:14`).
fn genome_from(x: u32) -> (u32, [u32; 6], u8) {
    let a = lcg(x);
    let b = lcg(a);
    let c = lcg(b);
    let d = lcg(c);
    let pid = (a >> 16) | ((b >> 16) << 16);
    let w1 = c >> 16;
    let w2 = d >> 16;
    let ivs = [
        w1 & 31,
        (w1 >> 5) & 31,
        (w1 >> 10) & 31,
        w2 & 31,
        (w2 >> 5) & 31,
        (w2 >> 10) & 31,
    ];
    (pid, ivs, (pid % 25) as u8)
}

/// Encounters among the first `window` rate tests from wild-RNG state `state` at the map rate
/// `rate` (`decompiled/src/wild_encounter.c:302-332`: `16 * rate` plus the miss buff
/// `buff * 16 / 200`, the buff growing by `rate` per miss and reset on a hit, `:760-784`; LCG
/// `WildEncounterRandom`, `:667-671`). The manip model's quiet tier is defined on 60 tests at the
/// forest's rate 14 (`docs/tricks/starter-manip.md` 3.4, `manip.rs` `quiet_p`), so the controller
/// uses the same definition.
fn wild_hits(state: u32, window: u32, rate: u32) -> u32 {
    let mut r = frlg_rng::WildRng(state);
    let mut buff = 0u32;
    let mut hits = 0;
    for _ in 0..window {
        let v = r.random() as u32;
        let rr = (rate * 16 + buff * 16 / 200).min(1600);
        if v % 1600 < rr {
            hits += 1;
            buff = 0;
        } else {
            buff += rate;
        }
    }
    hits
}

// ------------------------------------------------------------------------------------------------
// In-stage snapshots (the starter phase restarts from power-on without a checkpoint per attempt).

struct Snap {
    state: SaveState,
    rec_len: usize,
    ev_len: usize,
    new_game: Option<u64>,
}

fn snap(s: &mut Session) -> PResult<Snap> {
    Ok(Snap {
        state: s.hf.save_state()?,
        rec_len: s.hf.rec.frames(),
        ev_len: s.log.events.len(),
        new_game: s.log.new_game_frame,
    })
}

fn restore(s: &mut Session, sn: &Snap) -> PResult<()> {
    s.hf.emu()
        .load_state(&sn.state)
        .map_err(|e| e.to_string())?;
    s.hf.rec.truncate(sn.rec_len);
    s.log.events.truncate(sn.ev_len);
    s.log.new_game_frame = sn.new_game;
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// Boot, title, intro, names.

/// `Task_TitleScreenMain`'s `tSceneNum` (data 0) is `TITLESCREENSCENE_RUN` (3) and its `tState`
/// (data 1) is 1, where A or START is read (`decompiled/src/title_screen.c:22-27,443-444,615-653`).
fn title_ready(s: &mut Session) -> bool {
    task_data(s, "Task_TitleScreenMain", 0) == Some(3)
        && task_data(s, "Task_TitleScreenMain", 1) == Some(1)
}

/// Power-on to the title screen taking input: A skips the copyright screen and the intro and the
/// title's animation (`title_screen.c:452-460`), each press late by `U{0..N}`.
fn boot_to_title(s: &mut Session) -> PResult<()> {
    let obs = s.obs.clone();
    let ready = std::cell::Cell::new(false);
    // `human_mash_until` takes an `Emu` predicate; the title task is read through a copy of the
    // observer and the symbol table.
    let g = s.sym("gTasks");
    let syms = obs.symbols().clone();
    s.hf.human_mash_until("the title screen", keys::A, 4000, |e| {
        let r = (0..16u32).any(|i| {
            let base = g + i * 40;
            e.read8(base + 4) != 0
                && matches!(syms.covering(e.read32(base)), Some((n, _)) if n == "Task_TitleScreenMain")
                && e.read16(base + 8) == 3
                && e.read16(base + 10) == 1
        });
        ready.set(r);
        r
    })?;
    if !title_ready(s) {
        return Err(PlayError::Fatal("title screen not ready".into()));
    }
    Ok(())
}

/// Press A once on the title, `d` frames after it became ready, and run to the title exit
/// (`SetTitleScreenScene_Cry`: 90 frames of cry, the fade, then `SeedRngAndSetTrainerId` and
/// `ResetMenuAndMonGlobals` -> `SeedWildEncounterRng(Random())`, `title_screen.c:708-737`,
/// `new_game.c:103`). Returns the wild-RNG state (`sWildEncounterData.rngState`).
fn title_press(s: &mut Session, d: usize) -> PResult<u32> {
    s.hf.idle(d)?;
    s.hf.tap(keys::A)?;
    if task_data(s, "Task_TitleScreenMain", 0) != Some(5) {
        return Err(PlayError::Fatal(format!(
            "title A at +{d} not registered (scene {:?})",
            task_data(s, "Task_TitleScreenMain", 0)
        )));
    }
    for _ in 0..400 {
        if !cb2(s, "CB2_TitleScreenRun") {
            break;
        }
        s.hf.step(0)?;
    }
    Ok(s.obs.wild_data(s.hf.rec.emu()).rng_state)
}

/// Title-frame scan (the human's table, built on the emulator from the title screen he is
/// looking at): the first press offset at or after `from` whose wild seed has at most
/// `max_hits` encounters in the model's quiet window. Returns (offset, wild state, hits).
fn scan_title(
    s: &mut Session,
    at: &Snap,
    from: usize,
    cap: usize,
    window: u32,
    max_hits: u32,
) -> PResult<Option<(usize, u32, u32)>> {
    for d in from..from + cap {
        restore(s, at)?;
        let st = title_press(s, d)?;
        let h = wild_hits(st, window, 14);
        if max_hits == 0 || h <= max_hits {
            restore(s, at)?;
            return Ok(Some((d, st, h)));
        }
    }
    restore(s, at)?;
    Ok(None)
}

/// After the title exit. On a cart with no save (`SAVE_STATUS_EMPTY`) there is no menu: the main
/// menu's task sets `MAIN_MENU_NEWGAME` and executes it at once (`decompiled/src/main_menu.c:
/// 306-318,418-474` -> `StartNewGameScene`). The first frame of `CB2_NewGameScene` is the New Game
/// selection that starts the run timer (`human-run/PROMPT.md`, *The target*; the old prefix used
/// the same point, frame 639). A runner with a save on the cart pays a DOWN + A on the menu more
/// (off the timer).
fn new_game(s: &mut Session) -> PResult<u64> {
    wait_until(s, "New Game", 900, |s| cb2(s, "CB2_NewGameScene"))?;
    let f = s.frame();
    s.log.new_game_frame = Some(f);
    s.log.push(
        f,
        "split",
        "New Game selected: timer starts",
        json!({"frame": f}),
    );
    Ok(f)
}

/// Oak's speech to the player naming screen (the boy/girl menu's default is BOY; A takes it).
fn intro_to_naming(s: &mut Session) -> PResult<()> {
    let obs = s.obs.clone();
    s.hf.human_mash_until("the naming screen", keys::A, 8000, |e| {
        obs.callback2_is(e, "CB2_NamingScreen")
    })?;
    let obs = s.obs.clone();
    wait_until(s, "the naming screen input", 400, |s| {
        obs.naming_screen_accepting_input(s.hf.rec.emu())
    })
}

/// The player's naming screen, already taking input: one letter (A on the cursor's letter),
/// START (cursor to OK, `naming_screen.c:1485`), then the OK press at exactly `n` frames after
/// the screen began taking input. The screen restarted timer 1 on entry and reseeds `gRngValue`
/// from it on exit (`naming_screen.c:427-428,716-723`), so `n` is the naming dial of the manip
/// (`docs/tricks/starter-manip.md` 1.1).
fn player_name(s: &mut Session, n: usize) -> PResult<()> {
    let f0 = s.frame();
    s.hf.press(keys::A)?;
    s.hf.press(keys::START)?;
    let target = f0 + n as u64;
    if s.frame() >= target {
        return Err(PlayError::Fatal(format!(
            "naming OK target {n} too early (letter + START took {} frames)",
            s.frame() - f0
        )));
    }
    while s.frame() < target {
        s.hf.step(0)?;
    }
    s.hf.tap(keys::A)?;
    Ok(())
}

/// `Task_OakSpeech_HandleRivalNameInput` (`decompiled/src/oak_speech.c:1413-1440`): row 0 is NEW
/// NAME, then the presets (`sRivalNameChoices`, `:649-658`; LeafGreen RED first). `len == 1`: NEW
/// NAME, the naming screen opens empty with the cursor on the first letter (`naming_screen.c:494,
/// 2114`): A, START, A. Otherwise the shortest preset (3 letters: RED on LeafGreen row 1, KAZ on
/// FireRed row 3).
fn rival_name(s: &mut Session, len: u64) -> PResult<()> {
    let obs = s.obs.clone();
    s.hf.human_mash_until("the rival name menu", keys::A, 4000, |e| {
        obs.task_active(e, "Task_OakSpeech_HandleRivalNameInput")
    })?;
    s.hf.idle(4)?;
    if len == 1 {
        super::prims::press_until(s, "the rival naming screen", keys::A, 600, |s| {
            cb2(s, "CB2_NamingScreen")
        })?;
        let obs = s.obs.clone();
        wait_until(s, "the rival naming input", 400, |s| {
            obs.naming_screen_accepting_input(s.hf.rec.emu())
        })?;
        s.hf.press(keys::A)?;
        s.hf.press(keys::START)?;
        s.hf.idle(2)?;
        super::prims::press_until(s, "the rival name saved", keys::A, 600, |s| {
            !cb2(s, "CB2_NamingScreen")
        })?;
    } else {
        let lg = s.plan.params.version == "LG";
        if lg {
            s.hf.press(keys::DOWN)?;
        } else {
            s.hf.press(keys::UP)?;
            s.hf.press(keys::UP)?;
        }
        s.hf.idle(2)?;
        super::prims::press_until(s, "the rival preset taken", keys::A, 600, |s| {
            !task(s, "Task_OakSpeech_HandleRivalNameInput")
        })?;
    }
    Ok(())
}

/// The rest of Oak's speech, to the bedroom with control.
fn intro_to_bedroom(s: &mut Session) -> PResult<()> {
    let obs = s.obs.clone();
    s.hf.human_mash_until("the bedroom", keys::A, 8000, |e| {
        obs.callback2_is(e, "CB2_Overworld") && obs.player_can_step(e)
    })?;
    Ok(())
}

/// START -> OPTION in the bedroom, the first menu that has it: text speed FAST (row 0, MID ->
/// FAST), battle scene OFF (row 1), battle style SET (row 2), B saves and leaves
/// (`decompiled/src/option_menu.c`: `MENUITEM_*` rows, `CloseAndSaveOptionMenu`; text costs
/// `sTextSpeedFrameDelays` 4 frames per character at MID and 1 at FAST,
/// `new_menu_helpers.c:27-31`; scene off sets `HITMARKER_NO_ANIMATIONS`, `battle_main.c:2259`).
fn options(s: &mut Session) -> PResult<()> {
    super::menu::open_start(s)?;
    let r = super::items::start_row(s, 5)?;
    super::menu::start_menu_pick(s, r)?;
    let obs = s.obs.clone();
    wait_until(s, "the option menu", 600, |s| {
        obs.option_menu_accepting_input(s.hf.rec.emu())
    })?;
    // (row, wanted value)
    for (row, want) in [(0u16, 2u16), (1, 1), (2, 1)] {
        for _ in 0..12 {
            if obs.option_menu_cursor(s.hf.rec.emu()) == Some(row) {
                break;
            }
            s.hf.press(keys::DOWN)?;
            s.hf.idle(1)?;
        }
        for _ in 0..6 {
            if obs.option_menu_setting(s.hf.rec.emu(), row as u32) == Some(want) {
                break;
            }
            s.hf.press(keys::RIGHT)?;
            s.hf.idle(1)?;
        }
    }
    s.hf.press(keys::B)?;
    super::items::close_to_field(s)?;
    let emu = s.hf.rec.emu();
    let fast = obs.options_text_speed(emu) == Some(frlg_route::observe::TEXT_SPEED_FAST);
    let off = obs.options_battle_scene_off(emu) == Some(true);
    let set = super::battle_ram::battle_style_set(&obs, emu);
    if !(fast && off && set) {
        return Err(PlayError::Fatal(format!(
            "options not saved: fast {fast}, scene off {off}, style set {set}"
        )));
    }
    s.event(
        "decision",
        "options: text FAST, battle scene OFF, battle style SET (first menu that offers them)",
        json!({"text_speed": "fast", "battle_scene": "off", "battle_style": "set"}),
    );
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// The starter phase.

const LAB: &str = "MAP_PALLET_TOWN_PROFESSOR_OAKS_LAB";
const PALLET: &str = "MAP_PALLET_TOWN";
/// The Squirtle ball (`PalletTown_ProfessorOaksLab/map.json` object 6 at (9, 4)); the player
/// stands below it and faces up.
const BALL: (i16, i16) = (9, 4);
/// Frames the human leaves between the naming screen taking input and his OK press (the letter
/// and START fit comfortably: two presses, each late by at most `jitter_n`). A guess for the
/// human's timer target; with one dial (`manip.dials` 1) this frame is fixed.
const NAMING_N0: usize = 24;
/// Title offsets the human's table starts at (half a second after the title is up: a guess).
const TITLE_FROM: usize = 30;
/// The longest ball wait the table covers (frames).
const BALL_JMAX: usize = 15_000;

/// Bedroom -> Pallet -> Oak's trigger at the north exit ((12, 1), `PalletTown/map.json`
/// coord_events, `PalletTown_EventScript_OakTriggerLeft`) -> his scene into the lab -> the
/// starter offer (lab scene var 2, `ChooseStarterScene`) -> stand below the Squirtle ball facing it.
fn bedroom_to_ball(s: &mut Session) -> PResult<()> {
    options(s)?;
    go_map(s, PALLET, &Opts::default())?;
    let g = s.graph();
    let (pallet, lab) = (g.id(PALLET), g.id(LAB));
    // The lab door is not a way in yet (the planner must aim at the trigger, not the door).
    let o = Opts {
        avoid: vec![LAB],
        ..Opts::default()
    };
    go_to(
        s,
        "Oak's trigger",
        &move |n: &Node| n.0 == lab || (n.0 == pallet && n.1 == 12 && n.2 == 1),
        &o,
    )?;
    settle(s)?;
    if here(s).map(|n| n.0) != Some(lab) {
        return Err(PlayError::Fatal(
            "Oak's scene did not take the player to the lab".into(),
        ));
    }
    let scene = names().id("VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB");
    if s.obs.var(s.hf.rec.emu(), scene) != Some(2) {
        return Err(PlayError::Fatal(
            "lab scene var is not 2 after Oak's scene".into(),
        ));
    }
    // Two tiles below the ball, then one step up by hand: the walker's `settle` mashes A whenever
    // the player is not free, and an A facing the ball runs its script at an untimed frame.
    go(s, LAB, BALL.0, BALL.1 + 2, &Opts::default())?;
    let lab_id = s.graph().id(LAB);
    for _ in 0..4 {
        if here(s) == Some((lab_id, BALL.0, BALL.1 + 1)) {
            break;
        }
        wait_until(s, "free below the ball", 120, super::prims::is_free)?;
        s.hf.late()?;
        for _ in 0..24 {
            if here(s) == Some((lab_id, BALL.0, BALL.1 + 1)) {
                break;
            }
            s.hf.step(keys::UP)?;
        }
    }
    wait_until(s, "free facing the ball", 120, super::prims::is_free)?;
    let facing = super::ram::Ram::new(&s.obs).player_facing(s.hf.rec.emu());
    if here(s) != Some((lab_id, BALL.0, BALL.1 + 1)) || facing != 2 {
        return Err(PlayError::Death(format!(
            "soft-lock: not facing the ball ({:?}, facing {facing})",
            here(s)
        )));
    }
    Ok(())
}

/// The ball script from the player facing the Squirtle ball, with the decisive presses timed:
/// A on the ball at the current frame, the YES (a human press), then the A that dismisses "This
/// POKéMON is really energetic!" exactly `d` frames after the ball press; `givemon` follows it
/// (`PalletTown_ProfessorOaksLab/scripts.inc` `ChoseStarter`). Then B through the nickname prompt
/// (NO) and the rival's pick, to lab scene var 3. Returns the dismiss frame.
/// The ball press (at the current frame) through the YES; returns the ball press frame.
fn ball_to_yes(s: &mut Session) -> PResult<u64> {
    wait_until(s, "free facing the ball", 120, super::prims::is_free)?;
    let f0 = s.frame();
    s.hf.tap(keys::A)?;
    // Oak's "Hm! SQUIRTLE is your choice..." page needs an A; the question then prints and opens
    // the yes/no by itself (MSGBOX_YESNO). One press per page and a pause after it: a mashed A on
    // the frame the box opens answers it and runs on into the next text.
    let mut ok = false;
    for _ in 0..12 {
        for _ in 0..50 {
            if super::menu::yesno_up(s) {
                ok = true;
                break;
            }
            s.hf.step(0)?;
        }
        if ok {
            break;
        }
        s.hf.press(keys::A)?;
    }
    if !ok {
        return Err(PlayError::Death("soft-lock: no starter yes/no".into()));
    }
    s.hf.idle(4)?;
    s.hf.press(keys::A)?;
    Ok(f0)
}

fn ball_sequence(s: &mut Session, d: u64) -> PResult<u64> {
    let f0 = ball_to_yes(s)?;
    let target = f0 + d;
    if s.frame() >= target {
        return Err(PlayError::Fatal(format!(
            "ball dismiss target {d} too early ({} frames used)",
            s.frame() - f0
        )));
    }
    while s.frame() < target {
        s.hf.step(0)?;
    }
    s.hf.tap(keys::A)?;
    let obs = s.obs.clone();
    wait_until(s, "the starter in the party", 300, |s| {
        obs.party_count(s.hf.rec.emu()) == 1
    })?;
    let scene = names().id("VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB");
    s.hf.human_mash_until("the rival's pick", keys::B, 4000, |e| {
        obs.var(e, scene) == Some(3) && obs.player_can_step(e)
    })?;
    Ok(target)
}

/// Frames from the ball press to the YES on this branch (the human's own procedure); the dismiss
/// target leaves 90 frames over it for his late presses and the "really energetic" text to print.
fn yes_elapsed(s: &mut Session) -> PResult<u64> {
    let f0 = ball_to_yes(s)?;
    Ok(s.frame() - f0)
}

/// The human's ball table on one branch (facing the ball at `at`): `gRngValue` before each frame
/// of a wait up to [`BALL_JMAX`], the dismiss delay `d`, and the number of LCG steps `k` from the
/// ball press to `CreateBoxMon` (fitted on one press). Returns the smallest wait whose genome
/// meets the plan's tier, checked by playing it.
struct BallTable {
    d: u64,
    k: u32,
    rng: Vec<u32>,
}

fn ball_table(s: &mut Session, at: &Snap) -> PResult<BallTable> {
    restore(s, at)?;
    let d = yes_elapsed(s)? + 90;
    restore(s, at)?;
    let mut rng = Vec::with_capacity(BALL_JMAX + 1);
    rng.push(rng_value(s));
    for _ in 0..BALL_JMAX {
        s.hf.step(0)?;
        rng.push(rng_value(s));
    }
    restore(s, at)?;
    ball_sequence(s, d)?;
    let (pid, _, _) = starter_genome(s);
    let mut x = rng[0];
    let mut k = None;
    for i in 0..6000u32 {
        if genome_from(x).0 == pid {
            k = Some(i);
            break;
        }
        x = lcg(x);
    }
    restore(s, at)?;
    let k = k.ok_or("ball table: the created PID is not on the LCG within 6000 steps")?;
    Ok(BallTable { d, k, rng })
}

impl BallTable {
    fn genome(&self, j: usize) -> (u32, [u32; 6], u8) {
        let mut x = self.rng[j];
        for _ in 0..self.k {
            x = lcg(x);
        }
        genome_from(x)
    }
}

/// One played branch from the title snapshot: title press at `title_d`, New Game, Oak's speech
/// to the player naming screen taking input. Returns (wild state, New Game frame).
fn title_to_naming(s: &mut Session, title: &Snap, title_d: usize, key: u64) -> PResult<(u32, u64)> {
    restore(s, title)?;
    s.hf.rekey(key);
    let wild = title_press(s, title_d)?;
    let ng = new_game(s)?;
    intro_to_naming(s)?;
    Ok((wild, ng))
}

/// From the naming screen: the names (player OK at `naming_n`), the bedroom, the walk to the ball.
fn naming_to_ball(s: &mut Session, naming_n: usize, key: u64, rival_len: u64) -> PResult<()> {
    s.hf.rekey(key);
    player_name(s, naming_n)?;
    rival_name(s, rival_len)?;
    intro_to_bedroom(s)?;
    bedroom_to_ball(s)
}

/// The first ball wait of the table whose starter meets the tier, checked on the emulator (an NPC
/// draw on the press frame would shift the prediction).
fn first_good_wait(
    s: &mut Session,
    at_ball: &Snap,
    table: &BallTable,
    mp: &crate::manip::ManipParams,
    limit: usize,
) -> PResult<Option<usize>> {
    for j in 0..=limit.min(BALL_JMAX) {
        let (_, ivs, nat) = table.genome(j);
        if crate::manip::qualifies(mp, &ivs, nat) {
            restore(s, at_ball)?;
            s.hf.idle(j)?;
            ball_sequence(s, table.d)?;
            let (_, ivs2, nat2) = starter_genome(s);
            if crate::manip::qualifies(mp, &ivs2, nat2) {
                return Ok(Some(j));
            }
        }
    }
    Ok(None)
}

/// A press the human aims at a frame lands on it with the plan's per-press probability
/// (`ManipParams::press_p`, `manip.rs`), otherwise one frame early or late.
fn press_error(s: &mut Session, p: f64) -> i64 {
    let u = s.hf.below(1_000_000) as f64 / 1e6;
    if u < p {
        0
    } else if s.hf.below(2) == 0 {
        -1
    } else {
        1
    }
}

/// Power-on to the starter in the party and the rival's pick (lab scene var 3): the reset loop of
/// `manip.rs` played on the emulator. Each attempt: boot to the title; the human's title table
/// (first quiet seed); his ball table for that title frame and the fixed naming frame (one dial,
/// `manip.dials` 1; the naming frame is a dial too with `dials` 2); then the attempt as his three
/// presses land (each off by one frame with probability `1 - p_hit`). A title press that lands on a
/// seed that is not quiet is reset at once; any other miss shows at the stats screen. Resets and
/// their off-timer time are counted; only the kept attempt is in the log.
fn starter(s: &mut Session) -> PResult<()> {
    let mp = s.plan.params.manip.clone();
    let manip = s.plan.params.starter_model == "manip";
    let p_hit = mp.press_p();
    let rival_len = s.plan.params.rival_name_len as u64;
    let window = mp.quiet_window.min(60);
    let base_key = ((s.stage as u64) << 32) | 0x5000_0000;
    let start = snap(s)?;
    let mut resets = 0u32;
    let mut off_frames = 0u64;
    let mut quiet_resets = 0u32;
    let mut why: Vec<String> = vec![];
    for attempt in 0..400u64 {
        restore(s, &start)?;
        s.hf.rekey(base_key + attempt);
        let errs = [
            press_error(s, p_hit),
            press_error(s, p_hit),
            press_error(s, p_hit),
        ];
        boot_to_title(s)?;
        let title = snap(s)?;
        let Some((d_star, _, hits)) = scan_title(
            s,
            &title,
            TITLE_FROM,
            mp.quiet_wait_cap as usize,
            window,
            if manip { mp.quiet_max_hits } else { 0 },
        )?
        else {
            // No quiet frame before the attract screen: a full reboot, off the timer.
            off_frames += (s.frame() + mp.quiet_wait_cap as u64) + (mp.s_reset * FPS) as u64;
            resets += 1;
            why.push("no quiet title frame".into());
            eprintln!("starter attempt {attempt}: reset, {}", why.last().unwrap());
            continue;
        };
        let key = base_key + 0x100_0000 + attempt;
        let key2 = base_key + 0x200_0000 + attempt;
        // The intended branch: every press on its frame. The naming frame is a dial with
        // `manip.dials` 2 (the cheapest naming idle + ball wait, doc 4.2), fixed with 1.
        title_to_naming(s, &title, d_star, key)?;
        let at_naming = snap(s)?;
        let n_max = if mp.dials >= 2 { 240 } else { 0 };
        let mut best: Option<(usize, usize, u64)> = None; // (naming idle, ball wait, dismiss)
        for dn in 0..=n_max {
            if best.is_some_and(|b| dn >= b.0 + b.1) {
                break;
            }
            restore(s, &at_naming)?;
            naming_to_ball(s, NAMING_N0 + dn, key2, rival_len)?;
            if !manip {
                // No manip: the dismiss is just a human press (150 frames covers the text).
                best = Some((dn, 0, 150));
                break;
            }
            let at_ball = snap(s)?;
            let table = ball_table(s, &at_ball)?;
            let limit = best.map_or(BALL_JMAX, |b| b.0 + b.1 - dn);
            if let Some(j) = first_good_wait(s, &at_ball, &table, &mp, limit)? {
                if best.is_none_or(|b| dn + j < b.0 + b.1) {
                    best = Some((dn, j, table.d));
                }
            }
        }
        let Some((dn_star, j_star, dismiss)) = best else {
            off_frames += s.frame() + (mp.s_reset * FPS) as u64;
            resets += 1;
            why.push("no table wait".into());
            eprintln!("starter attempt {attempt}: reset, {}", why.last().unwrap());
            continue;
        };
        // The attempt as played: the presses land where they land.
        let title_d = (d_star as i64 + errs[0]).max(0) as usize;
        let naming_n = ((NAMING_N0 + dn_star) as i64 + errs[1]).max(0) as usize;
        let j = (j_star as i64 + errs[2]).max(0) as usize;
        let (wild, ng) = title_to_naming(s, &title, title_d, key)?;
        naming_to_ball(s, naming_n, key2, rival_len)?;
        let h_real = wild_hits(wild, window, 14);
        let quiet = !manip || mp.quiet_max_hits == 0 || h_real <= mp.quiet_max_hits;
        if !quiet {
            // The timer program shows the late/early title press (doc 4.2, assumed): reset now.
            off_frames += ng + (mp.s_reset * FPS) as u64;
            resets += 1;
            quiet_resets += 1;
            why.push(format!(
                "title press {:+}: seed not quiet ({h_real} hits)",
                errs[0]
            ));
            eprintln!("starter attempt {attempt}: reset, {}", why.last().unwrap());
            continue;
        }
        s.hf.idle(j)?;
        ball_sequence(s, dismiss)?;
        let (pid, ivs, nat) = starter_genome(s);
        let ok = !manip || crate::manip::qualifies(&mp, &ivs, nat);
        if !ok {
            off_frames += s.frame() + (mp.s_reset * FPS) as u64;
            resets += 1;
            why.push(format!(
                "presses {errs:?}: {} IVs {ivs:?} not in the tier",
                frlg_mon::stats::NATURE_NAMES[nat as usize]
            ));
            eprintln!("starter attempt {attempt}: reset, {}", why.last().unwrap());
            continue;
        }
        let nature = frlg_mon::stats::NATURE_NAMES[nat as usize];
        let off_s = off_frames as f64 / FPS + ng as f64 / FPS;
        s.event(
            "decision",
            format!(
                "starter accepted after {resets} reset(s) (~{off_s:.0}s off-timer): squirtle {nature}, \
                 IVs hp{} atk{} def{} spe{} spa{} spd{}; title +{title_d}f (quiet seed, {h_real} hits \
                 in {window} rolls), ball wait {j}f",
                ivs[0], ivs[1], ivs[2], ivs[3], ivs[4], ivs[5]
            ),
            json!({"resets": resets, "quiet_resets": quiet_resets, "off_timer_s": off_s,
                   "off_timer_model_s_reset": mp.s_reset, "attempt": attempt,
                   "press_errors": errs, "p_hit": p_hit, "title_d": title_d, "title_target": d_star,
                   "table_hits": hits, "wild_hits": h_real, "naming_n": naming_n, "naming_target": NAMING_N0 + dn_star, "dials": mp.dials, "ball_wait": j,
                   "ball_target": j_star, "pid": pid, "nature": nature, "ivs": ivs,
                   "ball_wait_s": j as f64 / FPS, "resets_why": why, "rival_name_len": rival_len}),
        );
        return Ok(());
    }
    Err(PlayError::Fatal(
        "no qualifying starter in 400 attempts".into(),
    ))
}

// ------------------------------------------------------------------------------------------------
// Lab to the Route 4 Center.

const ROUTE1: &str = "MAP_ROUTE1";
const VIRIDIAN: &str = "MAP_VIRIDIAN_CITY";
const FOREST: &str = "MAP_VIRIDIAN_FOREST";
const PEWTER: &str = "MAP_PEWTER_CITY";
const GYM: &str = "MAP_PEWTER_CITY_GYM";

fn flag(s: &mut Session, name: &str) -> bool {
    let f = names().id(name);
    s.obs.flag(s.hf.rec.emu(), f).unwrap_or(false)
}

fn var(s: &mut Session, name: &str) -> u16 {
    let v = names().id(name);
    s.obs.var(s.hf.rec.emu(), v).unwrap_or(0)
}

fn have(s: &mut Session, item: &str) -> u16 {
    super::ram::Ram::new(&s.obs).count(s.hf.rec.emu(), id(item))
}

/// The early Potion pickups the plan takes (`early_potions`: `bedroom_pc`, `route1_clerk`,
/// `forest_hidden`; review item 9).
fn early_potion(s: &Session, name: &str) -> bool {
    s.plan.params.early_potions.iter().any(|p| p == name)
}

/// A starter that faints is a dead branch (review item 10): it takes no exp and forces a Center.
fn starter_alive(s: &mut Session, what: &str) -> PResult<()> {
    let party = super::ram::Ram::new(&s.obs).party(s.hf.rec.emu());
    match party.first() {
        Some(m) if m.hp > 0 => Ok(()),
        _ => Err(PlayError::Death(format!("the starter fainted ({what})"))),
    }
}

/// Before a trainer fight: the simulator's pre-fight heal rule (`sim::entry_heal_floor` in the
/// Torrent phase or for `entry` fights, else `preheal_below` -> 0.85), applied with Potions from
/// the bag on the starter (party slot 0).
fn preheal(s: &mut Session, seg: &str, trainer: &str) -> PResult<()> {
    let dex = s.plan.dex;
    let fight = dex.fight(trainer);
    let foes: Vec<crate::battle::Mon> = fight
        .full
        .iter()
        .map(|m| crate::battle::enemy_mon(dex, m))
        .collect();
    let ram = super::battle_ram::BattleRam::new(s.obs.symbols())?;
    let party = ram.player_party(s.hf.rec.emu());
    let Some(raw) = party.first().cloned() else {
        return Ok(());
    };
    let badges = super::battle_ram::badges(&s.obs, s.hf.rec.emu());
    let Some(me) = super::battle::sim_party_mon(dex, &raw, badges, true, 0) else {
        return Ok(());
    };
    let cfg = s.ctx.tactics.get(seg).cloned().unwrap_or_default();
    let seg_i = s.plan.segs.iter().position(|g| g.name == seg).unwrap_or(0);
    let in_torrent = crate::sim::in_torrent_phase(&s.ctx, seg_i, false);
    let (hp, max) = (raw.hp as i32, raw.max_hp as i32);
    let target = if cfg.entry || in_torrent {
        crate::sim::entry_heal_floor(&s.ctx, cfg.entry, dex, &me, &foes)
    } else if (hp as f64) < s.plan.params.preheal_below * max as f64 && !cfg.torrent {
        (0.85 * max as f64).ceil() as i32
    } else {
        0
    };
    let potions = have(s, "ITEM_POTION");
    let need = if hp < target {
        (target - hp + 19) / 20
    } else {
        0
    };
    s.event(
        "decision",
        format!(
            "before {seg}: starter {hp}/{max} HP, pre-fight floor {target} ({}): {}",
            if cfg.entry || in_torrent {
                "Torrent phase: foes' best hit x torrent_hits + 1"
            } else {
                "preheal_below"
            },
            if need == 0 {
                "no heal".to_string()
            } else {
                format!(
                    "{} Potion(s), {potions} in the bag",
                    need.min(potions as i32)
                )
            }
        ),
        json!({"seg": seg, "trainer": trainer, "hp": hp, "max_hp": max, "floor": target,
               "uses": need, "potions": potions, "torrent": in_torrent}),
    );
    for _ in 0..need.min(potions as i32) {
        super::shop::use_item_on(s, "ITEM_POTION", 0)?;
    }
    Ok(())
}

/// The rival's lab party: `TRAINER_RIVAL_OAKS_LAB_<his starter>` (he takes the one strong against
/// ours, `PalletTown_ProfessorOaksLab/scripts.inc` `RivalPicksStarter`).
fn rival_lab_trainer(s: &Session) -> &'static str {
    match s.plan.params.starter.as_str() {
        "bulbasaur" => "TRAINER_RIVAL_OAKS_LAB_CHARMANDER",
        "charmander" => "TRAINER_RIVAL_OAKS_LAB_SQUIRTLE",
        _ => "TRAINER_RIVAL_OAKS_LAB_BULBASAUR",
    }
}

/// `rival-lab`: walk out; the rival's trigger row (y 8, `PalletTown_ProfessorOaksLab/map.json`
/// coord_events, scene var 3) starts the fight; his exit scene; out the door (`exit-lab`).
fn rival_lab(s: &mut Session) -> PResult<()> {
    let tr = rival_lab_trainer(s);
    preheal(s, "rival-lab", tr)?;
    s.seg = "rival-lab".into();
    go_map(s, PALLET, &Opts::default())?;
    starter_alive(s, "rival 1")?;
    if var(s, "VAR_MAP_SCENE_PALLET_TOWN_PROFESSOR_OAKS_LAB") < 4 {
        return Err(PlayError::Fatal(
            "out of the lab without the rival fight".into(),
        ));
    }
    s.event("split", "rival 1 beaten", json!({"seg": "rival-lab"}));
    Ok(())
}

/// `to-viridian` with the Route 1 clerk's free Potion (`Route1/scripts.inc:4-22`,
/// `FLAG_GOT_POTION_ON_ROUTE_1`; object 1 of `Route1/map.json`, wandering up/down at (6, 28)).
fn to_viridian(s: &mut Session) -> PResult<()> {
    s.seg = "to-viridian".into();
    if early_potion(s, "route1_clerk") {
        super::legs::talk_to(s, ROUTE1, 1)?;
        settle(s)?;
        let got = flag(s, "FLAG_GOT_POTION_ON_ROUTE_1");
        let n = have(s, "ITEM_POTION");
        s.event(
            "decision",
            format!(
                "Route 1 clerk: sample Potion {}",
                if got { "taken" } else { "NOT taken" }
            ),
            json!({"potions": n, "got": got}),
        );
    }
    s.seg = "route1-encounters".into();
    go_map(s, VIRIDIAN, &Opts::default())?;
    starter_alive(s, "Route 1")?;
    s.event("split", "Viridian reached", json!({}));
    Ok(())
}

/// `parcel`: entering the Viridian mart plays the parcel scene (`ViridianCity_Mart/scripts.inc`,
/// `VAR_MAP_SCENE_VIRIDIAN_CITY_MART` 1, `ITEM_OAKS_PARCEL`); then back out.
fn parcel(s: &mut Session) -> PResult<()> {
    s.seg = "parcel".into();
    go_map(s, "MAP_VIRIDIAN_CITY_MART", &Opts::default())?;
    settle(s)?;
    if have(s, "ITEM_OAKS_PARCEL") == 0 {
        return Err(PlayError::Fatal("no parcel after the mart scene".into()));
    }
    go_map(s, VIRIDIAN, &Opts::default())?;
    Ok(())
}

/// `deliver`: back to the lab, talk to Oak (object 4 of the lab): the Pokédex and five Poké Balls
/// (`PalletTown_ProfessorOaksLab/scripts.inc` delivery scene, sets
/// `VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN` 1).
fn deliver(s: &mut Session) -> PResult<()> {
    s.seg = "deliver".into();
    super::legs::talk_to(s, LAB, 4)?;
    settle(s)?;
    if var(s, "VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN") != 1 {
        return Err(PlayError::Fatal("parcel not delivered".into()));
    }
    let balls = super::dex::ball_count(s);
    s.event("split", "Pokedex delivered", json!({"balls": balls}));
    Ok(())
}

/// `tutorial`: north through Viridian; the old man's catching demo fires on the road north
/// ((20, 8) / (22, 8), `ViridianCity/map.json` coord_events, var 1) and ends with his var at 2
/// (`ViridianCity/scripts.inc:234`), which opens the road to Route 2.
fn tutorial(s: &mut Session) -> PResult<()> {
    s.seg = "tutorial".into();
    go_map(s, "MAP_ROUTE2", &Opts::default())?;
    if var(s, "VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN") < 2 {
        return Err(PlayError::Fatal("on Route 2 without the tutorial".into()));
    }
    s.event("split", "tutorial done, road north open", json!({}));
    Ok(())
}

/// `to-forest`, `route2-encounters`, `forest`, `forest-encounters`, `forest-trainers`: Route 2's
/// west side into Viridian Forest, Bug Catcher Sammy (object 5, (7, 22), facing left, sight 4:
/// the forced fight of the plan's `forest-trainers`), the hidden Potion at (3, 22)
/// (`ViridianForest/map.json` bg_events, `FLAG_HIDDEN_ITEM_VIRIDIAN_FOREST_POTION`), and out of the
/// north exit.
fn forest(s: &mut Session) -> PResult<()> {
    s.seg = "route2-encounters".into();
    go_map(s, FOREST, &Opts::default())?;
    s.seg = "forest-encounters".into();
    let fid = s.graph().id(FOREST);
    // Up to the west column below Sammy's line; heal by the plan's rule; then talk to him.
    go_to(
        s,
        "the forest's west column",
        &move |n: &Node| n.0 == fid && n.2 <= 26 && n.2 >= 23 && n.1 <= 6,
        &Opts::default(),
    )?;
    preheal(s, "forest-trainers", "TRAINER_BUG_CATCHER_SAMMY")?;
    s.seg = "forest-trainers".into();
    super::legs::talk_to(s, FOREST, 5)?;
    settle(s)?;
    starter_alive(s, "Sammy")?;
    if early_potion(s, "forest_hidden") && !flag(s, "FLAG_HIDDEN_ITEM_VIRIDIAN_FOREST_POTION") {
        go_face(s, FOREST, 3, 22, &Opts::default())?;
        s.hf.press(keys::A)?;
        settle(s)?;
        let got = flag(s, "FLAG_HIDDEN_ITEM_VIRIDIAN_FOREST_POTION");
        let n = have(s, "ITEM_POTION");
        s.event(
            "decision",
            format!(
                "Forest hidden Potion at (3, 22): {}",
                if got { "found" } else { "NOT found" }
            ),
            json!({"potions": n, "got": got}),
        );
    }
    s.seg = "forest-encounters".into();
    go_map(
        s,
        "MAP_ROUTE2_VIRIDIAN_FOREST_NORTH_ENTRANCE",
        &Opts::default(),
    )?;
    starter_alive(s, "the forest")?;
    Ok(())
}

/// `to-pewter`: Route 2's north half into Pewter and to the gym door (`PewterCity/map.json` warp
/// (15, 16)); no Center on the way (review item 12).
fn to_pewter(s: &mut Session) -> PResult<()> {
    s.seg = "to-pewter".into();
    go_map(s, GYM, &Opts::default())?;
    Ok(())
}

/// `pewter-gym-trainer` (Camper Liam, object 2 of `PewterCity_Gym/map.json`, forced in the WR
/// order, `route.rs`) and `brock` (object 1); no heal before or in the gym other than the plan's
/// own pre-fight rule (review items 11, 12). The badge and TM39 are Brock's
/// (`PewterCity_Gym/scripts.inc`).
fn brock(s: &mut Session) -> PResult<()> {
    if s.plan.params.route_style == "wr" {
        preheal(s, "pewter-gym-trainer", "TRAINER_CAMPER_LIAM")?;
        s.seg = "pewter-gym-trainer".into();
        super::legs::talk_to(s, GYM, 2)?;
        settle(s)?;
        starter_alive(s, "Liam")?;
    }
    preheal(s, "brock", "TRAINER_LEADER_BROCK")?;
    s.seg = "brock".into();
    super::legs::talk_to(s, GYM, 1)?;
    settle(s)?;
    starter_alive(s, "Brock")?;
    if !flag(s, "FLAG_BADGE01_GET") {
        return Err(PlayError::Fatal("no Boulder Badge after Brock".into()));
    }
    super::legs::split(s, "Brock", "brock")
}

/// Repels the plan's `repel_on` stretches need before the next mart (`ball_shop_from`, Cerulean):
/// one per 100 steps of each listed walking segment (`sim::use_repels`, Repel 100 steps,
/// `decompiled/src/data/items.json`).
fn repels_wanted(s: &Session) -> u16 {
    let p = &s.plan.params;
    let from = s
        .plan
        .segs
        .iter()
        .position(|g| g.name == "brock")
        .unwrap_or(0);
    let to = s
        .plan
        .segs
        .iter()
        .position(|g| g.name == p.ball_shop_from)
        .unwrap_or(s.plan.segs.len());
    let tiles: u32 = s.plan.segs[from..to]
        .iter()
        .filter(|g| p.repel_on.contains(&g.name))
        // Only stretches whose path crosses encounter tiles (review item 14: Route 3's path has
        // no grass, `enc` None in `leg_pewter_cerulean`; the plan's `repel_on` still lists it).
        .map(|g| match &g.kind {
            crate::route::Kind::Walk {
                tiles,
                enc: Some(e),
                ..
            } if e.steps > 0 => *tiles,
            _ => 0,
        })
        .sum();
    tiles.div_ceil(100) as u16
}

/// Pewter mart (review item 13): sell TM39 when the plan does (`tm39_sell`, R3 key; the TM is
/// never taught, it sells for 1500: price 3000 in `decompiled/src/data/items.json`, half at
/// `item_menu.c:1933`), buy the Repels for Mt. Moon (none on Route 3/4: no grass on the path, item
/// 14), Potions to the plan's stock for the fights before Cerulean (`fight_tactics` `potions`), and
/// Poké Balls per `pewter_balls` (R3 key, default `early_balls`).
fn pewter_mart(s: &mut Session) -> PResult<()> {
    s.seg = "route3".into();
    let mart = "MAP_PEWTER_CITY_MART";
    let sell = s.plan.params.tm39_sell;
    if sell && have(s, "ITEM_TM39") > 0 {
        let got = super::shop::sell(s, mart, None, &["ITEM_TM39"])?;
        s.event(
            "decision",
            format!("sold TM39 (Rock Tomb, never taught) for {got}"),
            json!({"money": got, "plan_key": "tm39_sell"}),
        );
    }
    let stock = [
        "route3-trainers",
        "mt-moon-miguel",
        "nugget-bridge-trainers",
        "rival-cerulean",
    ]
    .iter()
    .filter_map(|k| s.ctx.tactics.get(*k).map(|c| c.potions).filter(|&p| p >= 0))
    .max()
    .unwrap_or(3) as u16;
    let balls = s.plan.params.pewter_balls as u16;
    let mut want = vec![("ITEM_REPEL", repels_wanted(s)), ("ITEM_POTION", stock)];
    if balls > 0 {
        let have_balls = super::dex::ball_count(s);
        want.push(("ITEM_POKE_BALL", balls + have_balls));
    }
    super::shop::mart_topup(
        s,
        mart,
        None,
        &want,
        "Repels for Mt. Moon, Potions for the fights to Cerulean",
    )?;
    go_map(s, PEWTER, &Opts::default())?;
    Ok(())
}

/// `route3`, `route3-trainers`: the plan's forced Route 3 trainers (`fight_lists`
/// `route3-trainers`: Colton (object 8), Greg (5), James (3) of `Route3/map.json`) in walking
/// order, the others passed (Ben, Robin, ...: review items 16, 17, sight lines are the walker's),
/// then Route 4 into its Pokémon Center, where the run checkpoints as `mt-moon-pc`.
fn route3(s: &mut Session) -> PResult<()> {
    let list: Vec<String> = s
        .plan
        .params
        .fight_lists
        .get("route3-trainers")
        .cloned()
        .unwrap_or_default();
    let objs = [
        ("TRAINER_BUG_CATCHER_COLTON", 8u8),
        ("TRAINER_BUG_CATCHER_GREG", 5),
        ("TRAINER_BUG_CATCHER_JAMES", 3),
    ];
    for (tr, local) in objs {
        if !list.iter().any(|t| t == tr) {
            continue;
        }
        s.seg = "route3".into();
        preheal(s, "route3-trainers", tr)?;
        s.seg = "route3-trainers".into();
        super::legs::talk_to(s, "MAP_ROUTE3", local)?;
        settle(s)?;
        starter_alive(s, tr)?;
    }
    s.seg = "route3".into();
    go_map(s, "MAP_ROUTE4_POKEMON_CENTER_1F", &Opts::default())?;
    starter_alive(s, "Route 3")?;
    route4_center(s)?;
    super::legs::split(s, "Mt. Moon PC", "route3-trainers")
}

/// The Route 4 Center (review items 18, 19): heal per `mt_moon_heal` (R3 key; the model charges
/// no heal here, so default off), the Magikarp salesman per `route4_magikarp` (R3 key, item 19
/// CONFIRMED: default on; 500, `Route4_PokemonCenter_1F/scripts.inc:1,23-53`, object 2; YES to
/// buy, NO to the nickname).
fn magikarp_planned(s: &Session) -> bool {
    s.plan.params.route4_magikarp
        || s.plan.params.specials.iter().any(|sp| {
            sp.after == "route4"
                && sp.kind == "buy"
                && sp.species.iter().any(|x| x == "SPECIES_MAGIKARP")
        })
}

fn route4_center(s: &mut Session) -> PResult<()> {
    let center = "MAP_ROUTE4_POKEMON_CENTER_1F";
    if magikarp_planned(s) && !flag(s, "FLAG_BOUGHT_MAGIKARP") {
        let money = super::ram::Ram::new(&s.obs).money(s.hf.rec.emu());
        if money >= 500 {
            super::legs::talk_to(s, center, 2)?;
            super::menu::text_until_yesno(s)?;
            super::menu::answer(s, true)?;
            super::menu::text_until_yesno(s)?;
            super::menu::answer(s, false)?;
            settle(s)?;
            let got = flag(s, "FLAG_BOUGHT_MAGIKARP");
            s.event(
                "decision",
                format!("Magikarp bought at the Route 4 Center: {got} (${money} before)"),
                json!({"bought": got, "money": money, "plan_key": "route4_magikarp"}),
            );
        } else {
            s.event(
                "warn",
                format!("Magikarp not bought: ${money} < 500"),
                json!({"money": money}),
            );
        }
    }
    if s.plan.params.mt_moon_heal {
        super::legs::center_heal(s, center)?;
    }
    Ok(())
}

fn noop(_s: &mut Session) -> PResult<()> {
    Ok(())
}

pub fn stages() -> Vec<Stage> {
    vec![
        Stage {
            name: "power-on",
            run: noop,
        },
        Stage {
            name: "starter",
            run: starter,
        },
        Stage {
            name: "rival-lab",
            run: rival_lab,
        },
        Stage {
            name: "viridian",
            run: to_viridian,
        },
        Stage {
            name: "parcel",
            run: parcel,
        },
        Stage {
            name: "deliver",
            run: deliver,
        },
        Stage {
            name: "tutorial",
            run: tutorial,
        },
        Stage {
            name: "forest-north",
            run: forest,
        },
        Stage {
            name: "pewter-gym",
            run: to_pewter,
        },
        Stage {
            name: "brock",
            run: brock,
        },
        Stage {
            name: "pewter-mart",
            run: pewter_mart,
        },
        Stage {
            name: "mt-moon-pc",
            run: route3,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_table_matches_the_doc() {
        // doc 3.4: P(hits <= 6 in 60 forest rolls) = 21.8 % over all 65536 seeds.
        let n = (0..65536u32)
            .filter(|&sd| wild_hits(sd, 60, 14) <= 6)
            .count() as f64;
        assert!((n / 65536.0 - 0.218).abs() < 0.01, "{}", n / 65536.0);
    }
}
