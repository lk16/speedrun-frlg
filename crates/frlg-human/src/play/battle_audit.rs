//! `frlg-human play-battles --plan <plan.md> <dir>`: replay `<dir>/run.ilog` from boot and print one line
//! per battle: start frame, trainer id (`gTrainerBattleOpponent_A`, 0 = wild; names in
//! `decompiled/include/constants/opponents.h`), the first enemy species and level, and the lead's exp
//! before and after. A diagnostic for the exp-economy audit (the played run's fights vs the plan's).
use std::path::Path;

use frlg_route::{Observer, Recorder};

use super::battle_ram::BattleRam;
use super::names::names;
use super::ram::Ram;

pub fn run(rom: &Path, sym: &Path, dir: &Path) -> Result<(), String> {
    let ilog = super::session::read_ilog(&dir.join("run.ilog"))?;
    let syms = frlg_emu::SymbolTable::load(sym).map_err(|e| e.to_string())?;
    let br = BattleRam::new(&syms)?;
    let flags_addr = syms
        .get("gBattleTypeFlags")
        .ok_or("gBattleTypeFlags is not in the symbol table")?
        .addr;
    let obs = Observer::new(syms).map_err(|e| e.to_string())?;
    let ram = Ram::new(&obs);
    let mut rec = Recorder::from_reset(rom).map_err(|e| e.to_string())?;
    let emu = rec.emu();
    let n = names();
    let mut in_b = false;
    let mut start = (0usize, 0u16, String::new(), 0u32, 0u32);
    for (f, &k) in ilog.frames.iter().enumerate() {
        emu.step(k);
        // Every 8th frame is enough: a battle lasts hundreds of frames.
        if f % 8 != 0 {
            continue;
        }
        let b = obs.in_battle(emu);
        if b && !in_b {
            // Let the enemy party load before reading it.
            let party = ram.party(emu);
            let exp = party.first().map_or(0, |m| m.exp);
            let lvl = party.first().map_or(0, |m| m.level as u32);
            // `BATTLE_TYPE_TRAINER` (`include/constants/battle.h:50`); 0 marks a wild battle.
            let trainer = emu.read32(flags_addr) & (1 << 3) != 0;
            let id = if trainer { br.trainer_id(emu) } else { 0 };
            start = (f, id, String::new(), exp, lvl);
        }
        if b && start.2.is_empty() && f >= start.0 + 400 {
            let e = br.enemy_party(emu);
            start.2 = e
                .iter()
                .map(|m| format!("{}L{}", n.species(m.species), m.level))
                .collect::<Vec<_>>()
                .join(",");
        }
        if !b && in_b {
            let party = ram.party(emu);
            let exp = party.first().map_or(0, |m| m.exp);
            println!(
                "{} {} trainer {} lead L{} exp {} -> {} (+{}) foes {}",
                start.0,
                f,
                start.1,
                start.4,
                start.3,
                exp,
                exp as i64 - start.3 as i64,
                start.2
            );
        }
        in_b = b;
    }
    Ok(())
}
