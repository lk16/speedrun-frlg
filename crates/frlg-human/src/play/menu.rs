//! Menus by cursor position read from RAM: script multichoice lists, yes/no boxes, the start menu.
use frlg_emu::keys;
use frlg_route::Feed;

use super::prims::{press_until, settle, wait_until};
use super::session::{PResult, PlayError, Session};

/// `struct Menu sMenu` of `decompiled/src/menu.c:9-25` (12 bytes; the sym file has a second,
/// unrelated `sMenu`, so it is looked up by name *and* size): `s8 cursorPos` at offset 2.
pub fn menu_cursor(s: &mut Session) -> i8 {
    let a = s.sym_sized("sMenu", 0xC);
    s.hf.rec.emu().read8(a + 2) as i8
}

pub fn task(s: &mut Session, name: &str) -> bool {
    let obs = &s.obs;
    obs.task_active(s.hf.rec.emu(), name)
}

/// Is a script multichoice list taking input (`Task_MultichoiceMenu_HandleInput`,
/// `decompiled/src/script_menu.c:808`)?
pub fn multichoice_up(s: &mut Session) -> bool {
    task(s, "Task_MultichoiceMenu_HandleInput")
}

/// Is a script yes/no box up (`Task_YesNoMenu_HandleInput`, `script_menu.c:887`)?
pub fn yesno_up(s: &mut Session) -> bool {
    task(s, "Task_YesNoMenu_HandleInput")
}

/// Mash A through text until a multichoice list is up.
pub fn text_until_multichoice(s: &mut Session) -> PResult<()> {
    press_until(s, "a multichoice menu", keys::A, 3000, multichoice_up)
}

/// Mash A through text until a yes/no box is up.
pub fn text_until_yesno(s: &mut Session) -> PResult<()> {
    press_until(s, "a yes/no box", keys::A, 3000, yesno_up)
}

/// Move a vertical menu's cursor (`sMenu.cursorPos`) to `want` and press A.
pub fn choose(s: &mut Session, want: i8) -> PResult<()> {
    // The list ignores input for a few frames after it opens (`sDelay`, `tTimer`).
    s.hf.idle(6)?;
    for _ in 0..20 {
        let cur = menu_cursor(s);
        if cur == want {
            s.hf.press(keys::A)?;
            s.hf.idle(2)?;
            return Ok(());
        }
        s.hf.press(if want > cur { keys::DOWN } else { keys::UP })?;
        s.hf.idle(1)?;
    }
    Err(PlayError::Death(format!(
        "soft-lock: menu cursor never reached {want}"
    )))
}

/// Answer the yes/no box that is up: A on YES (the default) or B for NO.
pub fn answer(s: &mut Session, yes: bool) -> PResult<()> {
    s.hf.idle(6)?;
    s.hf.press(if yes { keys::A } else { keys::B })?;
    s.hf.idle(2)?;
    Ok(())
}

/// Open the start menu from the field and wait until it takes input
/// (`Observer::start_menu_taking_input`).
pub fn open_start(s: &mut Session) -> PResult<()> {
    settle(s)?;
    s.hf.press(keys::START)?;
    let obs = s.obs.clone();
    wait_until(s, "the start menu", 120, |s| {
        obs.start_menu_taking_input(s.hf.rec.emu())
    })
}

/// Start menu rows in FRLG order (`decompiled/src/start_menu.c` `sStartMenuActionTable` /
/// `SetUpStartMenu_NormalField`): POKéDEX, POKéMON, BAG, player, SAVE, OPTION, EXIT; the list is
/// built per game state, so the row is looked up by its action in `sStartMenuOrder`.
pub fn start_menu_pick(s: &mut Session, row: u8) -> PResult<()> {
    let obs = s.obs.clone();
    for _ in 0..12 {
        let cur = obs.start_menu_cursor(s.hf.rec.emu());
        if cur == row {
            // Press A until the menu stops taking input (a press on its first frame can be lost).
            for _ in 0..5 {
                s.hf.press(keys::A)?;
                for _ in 0..20 {
                    if !obs.start_menu_taking_input(s.hf.rec.emu()) {
                        return Ok(());
                    }
                    s.hf.step(0)?;
                }
            }
            return Err(PlayError::Death("soft-lock: start menu ignores A".into()));
        }
        s.hf.press(if row > cur { keys::DOWN } else { keys::UP })?;
    }
    Err(PlayError::Death("soft-lock: start menu cursor".into()))
}
