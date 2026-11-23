//! Primitives every leg uses: is the player free on the field, advance text and scripts the way a
//! human does (mash A, each press late by `U{0..N}`), and hand battles to the battle driver.
use frlg_emu::keys;
use frlg_route::Feed;

use super::session::{PResult, PlayError, Session};

/// Frames a script/text stretch may take before it counts as a soft-lock.
pub const SCRIPT_BUDGET: usize = 20_000;

/// The player stands on the field with control: `CB2_Overworld` running, the avatar settled and
/// not held by a script (`Observer::player_can_step`), field controls not locked by a script
/// (`sLockFieldControls`, `decompiled/src/script.c:34,199-209`), no battle.
pub fn is_free(s: &mut Session) -> bool {
    let obs = &s.obs;
    let emu = s.hf.rec.emu();
    if obs.in_battle(emu)
        || !obs.callback2_is(emu, "CB2_Overworld")
        || obs.field_controls_locked(emu)
    {
        return false;
    }
    if obs.player_can_step(emu) {
        return true;
    }
    // On Cycling Road the slope keeps the bike rolling south (`FLAG_SYS_ON_CYCLING_ROAD`,
    // `decompiled/data/maps/Route16/scripts.inc` `OnTransitionCyclingRoad`), so the avatar is
    // never settled; with no script holding it, the player is in control.
    let cyc = super::names::names()
        .get("FLAG_SYS_ON_CYCLING_ROAD")
        .and_then(|f| obs.flag(emu, f))
        .unwrap_or(false);
    cyc && obs.player_avatar_flags(emu) != 0 && !obs.prevent_step(emu)
}

/// Is a yes/no prompt of the battle's learn-move script waiting for input? `gBattlescriptCurrInstr`
/// points at the opcode being run: `yesnoboxlearnmove` 0x5A and `yesnoboxstoplearningmove` 0x5B
/// (`decompiled/src/battle_script_commands.c:404-405`).
pub fn battle_opcode(s: &mut Session) -> u8 {
    let a = s
        .obs
        .symbols()
        .get("gBattlescriptCurrInstr")
        .expect("gBattlescriptCurrInstr")
        .addr;
    let emu = s.hf.rec.emu();
    let p = emu.read32(a);
    if (0x0800_0000..0x0A00_0000).contains(&p) {
        emu.read8(p)
    } else {
        0
    }
}

/// The naming screen is up (`CB2_NamingScreen`, `decompiled/src/naming_screen.c`): a gift or catch
/// asked for a nickname and the human said yes. Confirm the default name: START jumps to OK, A
/// accepts.
fn naming_screen(s: &mut Session) -> bool {
    let obs = &s.obs;
    let emu = s.hf.rec.emu();
    obs.callback2_is(emu, "CB2_NamingScreen")
}

/// Advance whatever holds the player -- text, a script, a fade, a battle -- until the player is
/// free on the field. Text is mashed with A (late by `U{0..N}` per press); battles go to the battle
/// driver; a nickname prompt that got a yes is confirmed with the default name.
pub fn settle(s: &mut Session) -> PResult<()> {
    let mut start = s.hf.fed();
    let mut free_run = 0;
    loop {
        if s.hf.fed() - start > SCRIPT_BUDGET {
            return Err(PlayError::Death(format!(
                "soft-lock: not free after {SCRIPT_BUDGET} frames ({})",
                s.obs.snapshot(s.hf.rec.emu())
            )));
        }
        if s.obs.in_battle(s.hf.rec.emu()) {
            super::fallback_battle::battle(s)?;
            // A League battle alone can run past the script budget (two dev branches "soft-locked"
            // in Lorelei-2 after a 20 000-frame fight): the budget restarts after each battle.
            start = s.hf.fed();
            continue;
        }
        if in_hall_of_fame(s) {
            // The run's first Hall of Fame: the caller takes the split here.
            return Ok(());
        }
        if is_free(s) {
            // Free for a few frames in a row: scripts can release and re-lock between commands.
            free_run += 1;
            if free_run >= 3 {
                return Ok(());
            }
            s.hf.step(0)?;
            continue;
        }
        free_run = 0;
        if naming_screen(s) {
            s.hf.press(keys::START)?;
            s.hf.press(keys::A)?;
            continue;
        }
        // Leaving the bag or the party menu reopens the start menu
        // (`Task_StartMenuHandleInput`, `decompiled/src/start_menu.c`): close it with B, never A.
        if s.obs
            .task_active(s.hf.rec.emu(), "Task_StartMenuHandleInput")
            || s.obs.task_active(s.hf.rec.emu(), "Task_ShopMenu")
        {
            s.hf.press(keys::B)?;
            s.hf.idle(2)?;
            continue;
        }
        // A party menu or bag left open (a teach that already happened): back out with B; an A
        // there would pick the mon or the item again ("already knows" loops).
        // v4: also a mart's buy list (`CB2_BuyMenu`, `decompiled/src/shop.c`) a settle walked
        // into (a clerk talked to by a mashed A): every A buys the first row (Fuchsia: 25 Ultra
        // Balls, then "You don't have enough money" until the 20 000-frame budget ran out).
        if s.obs.callback2_is(s.hf.rec.emu(), "CB2_UpdatePartyMenu")
            || s.obs.callback2_is(s.hf.rec.emu(), "CB2_BagMenuRun")
            || s.obs.callback2_is(s.hf.rec.emu(), "CB2_BuyMenu")
        {
            s.hf.late()?;
            s.hf.step(keys::B)?;
            s.hf.idle(8)?;
            continue;
        }
        // One human press of A, then a release frame.
        s.hf.late()?;
        if s.obs.in_battle(s.hf.rec.emu()) || is_free(s) {
            continue;
        }
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
}

/// The Hall of Fame screen or the credits after it (`CB2_DoHallOfFameScreen`, then `CB2_HofIdle`,
/// `decompiled/src/hall_of_fame.c:352,361`; `CB2_Credits`).
pub fn in_hall_of_fame(s: &mut Session) -> bool {
    let emu = s.hf.rec.emu();
    ["CB2_DoHallOfFameScreen", "CB2_HofIdle", "CB2_Credits"]
        .iter()
        .any(|c| s.obs.callback2_is(emu, c))
}

/// Press `key` (late by `U{0..N}`) until `pred` holds, settling nothing else.
pub fn press_until(
    s: &mut Session,
    what: &str,
    key: u16,
    budget: usize,
    mut pred: impl FnMut(&mut Session) -> bool,
) -> PResult<()> {
    let start = s.hf.fed();
    loop {
        if pred(s) {
            return Ok(());
        }
        if s.hf.fed() - start > budget {
            return Err(PlayError::Death(format!(
                "soft-lock: {what} not reached in {budget} frames"
            )));
        }
        s.hf.late()?;
        if pred(s) {
            return Ok(());
        }
        s.hf.step(key)?;
        s.hf.step(0)?;
    }
}

/// Idle until `pred` holds (a human waiting for a fade or an animation).
pub fn wait_until(
    s: &mut Session,
    what: &str,
    budget: usize,
    mut pred: impl FnMut(&mut Session) -> bool,
) -> PResult<()> {
    for _ in 0..budget {
        if pred(s) {
            return Ok(());
        }
        s.hf.step(0)?;
    }
    if pred(s) {
        return Ok(());
    }
    Err(PlayError::Death(format!(
        "soft-lock: waited {budget} frames for {what}"
    )))
}

/// `gPaletteFade.active` (`struct PaletteFadeControl`, `decompiled/include/palette.h:37-56`: after
/// `u32 multipurpose1`, the u8/u16 bitfields put `active` in bit 15 of the u16 at offset 6). Menus
/// ignore input while a fade runs.
pub fn fade_active(s: &mut Session) -> bool {
    let a = s.sym("gPaletteFade");
    s.hf.rec.emu().read8(a + 7) & 0x80 != 0
}
