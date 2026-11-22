//! A minimal battle driver, a stand-in until the sim-policy bridge (`play/battle.rs`, written
//! elsewhere) is merged. Its single entry point [`battle`] is the one call site the field code
//! uses, so swapping in the real driver is a one-line change.
//!
//! Policy (a human's crude default, *not* the plan's policy -- every fight it plays is logged with
//! `"policy": "fallback"`): wild battles run; trainer battles use the lead's damaging move with the
//! best `power x STAB x type effectiveness x accuracy` that has PP. A fainted lead ends the branch
//! (no switching here), and so does a blackout. Learn-move prompts are declined (`keep_moves`).
use frlg_emu::keys;
use frlg_route::observe::{BATTLE_TYPE_TRAINER, B_OUTCOME_LOST};
use frlg_route::Feed;
use serde_json::json;

use super::names::names;
use super::prims::battle_opcode;
use super::session::{PResult, PlayError, Session};

const BATTLE_BUDGET: usize = 40_000;

/// `struct BattlePokemon` (`decompiled/include/pokemon.h:170-200`): `type1`/`type2` at 0x21/0x22,
/// `pp[4]` at 0x24, size 0x58.
const BMON_TYPES: u32 = 0x21;
const BMON_PP: u32 = 0x24;
const BMON_SIZE: u32 = 0x58;

fn bmon_addr(s: &Session, i: u32) -> u32 {
    s.obs.symbols().get("gBattleMons").unwrap().addr + i * BMON_SIZE
}

/// Pick the lead's move: (slot, score). `None` when nothing damaging has PP.
fn pick_move(s: &mut Session) -> Option<(usize, f64)> {
    let dex = s.plan.dex;
    let me = s.obs.battle_mon(s.hf.rec.emu(), 0);
    let a0 = bmon_addr(s, 0);
    let a1 = bmon_addr(s, 1);
    let emu = s.hf.rec.emu();
    let pp: [u8; 4] = std::array::from_fn(|i| emu.read8(a0 + BMON_PP + i as u32));
    let my_t = (emu.read8(a0 + BMON_TYPES), emu.read8(a0 + BMON_TYPES + 1));
    let foe_t = (emu.read8(a1 + BMON_TYPES), emu.read8(a1 + BMON_TYPES + 1));
    let n = names();
    let mut best: Option<(usize, f64)> = None;
    #[allow(clippy::needless_range_loop)]
    for slot in 0..4 {
        let mv = me.moves[slot];
        if mv == 0 || pp[slot] == 0 {
            continue;
        }
        let name = n.mv(mv);
        let Some(&id) = dex_move(dex, name) else {
            continue;
        };
        let m = &dex.moves[id];
        if m.power <= 1 {
            continue;
        }
        let stab = if m.mtype == my_t.0 || m.mtype == my_t.1 {
            1.5
        } else {
            1.0
        };
        let acc = if m.acc == 0 { 100.0 } else { m.acc as f64 };
        let score = m.power as f64 * stab * dex.eff(m.mtype, foe_t) * acc / 100.0;
        if best.is_none_or(|b| score > b.1) {
            best = Some((slot, score));
        }
    }
    best
}

fn dex_move<'a>(dex: &'a crate::data::Dex, name: &str) -> Option<&'a usize> {
    dex.move_index(name)
}

/// Move a 2x2 menu cursor from `cur` to `want` (0 top-left, 1 top-right, 2 bottom-left,
/// 3 bottom-right), one human press per step.
fn cursor_2x2(s: &mut Session, cur: u8, want: u8) -> PResult<()> {
    let (cx, cy) = (cur % 2, cur / 2);
    let (wx, wy) = (want % 2, want / 2);
    if cx != wx {
        s.hf.press(if wx > cx { keys::RIGHT } else { keys::LEFT })?;
    }
    if cy != wy {
        s.hf.press(if wy > cy { keys::DOWN } else { keys::UP })?;
    }
    Ok(())
}

/// Play the battle that is on screen to its end and back to the field.
pub fn battle(s: &mut Session) -> PResult<()> {
    let start = s.hf.fed();
    let emu = s.hf.rec.emu();
    let trainer = s.obs.battle_type_flags(emu) & BATTLE_TYPE_TRAINER != 0;
    let foe = s.obs.battle_mon(emu, 1);
    let n = names();
    s.hf.react()?; // the human sees what the battle is
    s.event(
        if trainer { "battle" } else { "encounter" },
        format!(
            "{} {} L{}",
            if trainer {
                "trainer battle vs"
            } else {
                "wild"
            },
            n.species(foe.species).trim_start_matches("SPECIES_"),
            foe.level
        ),
        json!({"trainer": trainer, "species": n.species(foe.species), "level": foe.level, "policy": "fallback"}),
    );
    let mut turns = 0u32;
    let mut last_foe = foe.species;
    loop {
        if s.hf.fed() - start > BATTLE_BUDGET {
            return Err(PlayError::Death(format!(
                "soft-lock: battle not over after {BATTLE_BUDGET} frames"
            )));
        }
        let emu = s.hf.rec.emu();
        if !s.obs.in_battle(emu) {
            break;
        }
        let me = s.obs.battle_mon(emu, 0);
        if s.obs.battle_choosing_actions(emu) && me.hp == 0 {
            return Err(PlayError::Death(format!(
                "lead {} fainted (fallback battle driver does not switch)",
                n.species(me.species)
            )));
        }
        if s.obs.callback2_is(emu, "CB2_UpdatePartyMenu") {
            if me.hp == 0 {
                return Err(PlayError::Death(
                    "party menu in battle: lead fainted (fallback driver does not switch)".into(),
                ));
            }
            // An optional party screen (a yes to the SHIFT prompt slipped through): back out.
            s.hf.press(keys::B)?;
            s.hf.idle(4)?;
            continue;
        }
        let foe = s.obs.battle_mon(emu, 1);
        if foe.species != last_foe && foe.species != 0 {
            last_foe = foe.species;
        }
        if s.obs
            .battle_controller_is(emu, 0, "HandleInputChooseAction")
        {
            let cur = s.obs.action_cursor(emu, 0);
            let want = if trainer { 0 } else { 3 };
            if cur != want {
                cursor_2x2(s, cur, want)?;
                continue;
            }
            s.hf.press(keys::A)?;
            s.hf.idle(2)?;
            continue;
        }
        if s.obs.battle_controller_is(emu, 0, "HandleInputChooseMove") {
            if !trainer {
                s.hf.press(keys::B)?;
                continue;
            }
            let Some((slot, score)) = pick_move(s) else {
                return Err(PlayError::Death(
                    "no damaging move with PP left (fallback driver)".into(),
                ));
            };
            let cur = s.obs.move_cursor(s.hf.rec.emu(), 0);
            if cur as usize != slot {
                cursor_2x2(s, cur, slot as u8)?;
                continue;
            }
            turns += 1;
            let mv = s.obs.battle_mon(s.hf.rec.emu(), 0).moves[slot];
            s.event(
                "battle",
                format!(
                    "turn {turns}: {} (fallback: best power x type, score {score:.0})",
                    n.mv(mv).trim_start_matches("MOVE_")
                ),
                json!({"turn": turns, "slot": slot, "move": n.mv(mv), "my_hp": me.hp,
                       "enemy": n.species(foe.species), "enemy_hp": foe.hp, "policy": "fallback"}),
            );
            s.hf.press(keys::A)?;
            s.hf.idle(2)?;
            continue;
        }
        match battle_opcode(s) {
            // "Delete a move to make room?" -> no.
            0x5A => {
                s.hf.press(keys::B)?;
                s.hf.idle(1)?;
                continue;
            }
            // `yesnobox` 0x67 (`battle_script_commands.c:417`): with the SHIFT battle style
            // (`new_game.c:65`, never changed in this run) the game asks whether to switch when
            // the trainer sends the next mon (`BattleScript_HandleFaintedMon`,
            // `data/battle_scripts_1.s:2842-2846`) -> no.
            0x67 => {
                s.hf.press(keys::B)?;
                s.hf.idle(1)?;
                continue;
            }
            // "Stop learning the move?" -> yes.
            0x5B => {
                s.hf.press(keys::A)?;
                s.hf.idle(1)?;
                continue;
            }
            _ => {}
        }
        // Text: one human A press.
        s.hf.late()?;
        if matches!(battle_opcode(s), 0x5A | 0x5B | 0x67) {
            continue;
        }
        let emu = s.hf.rec.emu();
        if !s.obs.in_battle(emu)
            || s.obs
                .battle_controller_is(emu, 0, "HandleInputChooseAction")
            || s.obs.battle_controller_is(emu, 0, "HandleInputChooseMove")
        {
            continue;
        }
        s.hf.step(keys::A)?;
        s.hf.step(0)?;
    }
    let outcome = s.obs.battle_outcome(s.hf.rec.emu());
    s.event(
        if trainer { "battle" } else { "encounter" },
        format!(
            "battle over: outcome {outcome} after {turns} turns ({:.1}s)",
            (s.hf.fed() - start) as f64 / super::FPS
        ),
        json!({"outcome": outcome, "turns": turns, "frames": s.hf.fed() - start, "policy": "fallback"}),
    );
    if outcome == B_OUTCOME_LOST {
        return Err(PlayError::Death("blacked out".into()));
    }
    Ok(())
}
