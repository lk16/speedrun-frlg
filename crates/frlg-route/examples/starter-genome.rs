//! Read the committed run's starter as the game computed it: replay the
//! ledger's logs through `07-starter`, then dump `gPlayerParty[0]`'s PID,
//! nature and computed stats, plus the `gRngValue` window around `givemon`
//! so the 4-roll model (`frlg_mon::create::gift_mon`) can be anchored to it.
//!
//! Usage: starter-genome <ledger.json>

use frlg_mon::stats::{NATURE_NAMES, NATURE_STAT_TABLE};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger_path = std::env::args().nth(1).expect("ledger.json path");
    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::default_sym_path().ok_or("sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = frlg_route::Observer::new(syms.clone()).map_err(std::io::Error::other)?;
    let party = syms.get("gPlayerParty").ok_or("gPlayerParty")?.addr;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;

    // `struct Pokemon`, `decompiled/include/pokemon.h:128-141`: box (80
    // bytes, personality at +0), then status, level at 0x54, hp 0x56,
    // maxHP 0x58, attack 0x5A, defense 0x5C, speed 0x5E, spAttack 0x60,
    // spDefense 0x62.
    let mut prev_count = 0u8;
    'outer: for seg in &ledger.segments {
        let log = frlg_emu::InputLog::decode(&std::fs::read(&seg.log)?)?;
        for &keys in &log.frames {
            let rng_before = obs.rng(&mut emu);
            emu.step(keys);
            let count = obs.party_count(&mut emu);
            if count == 1 && prev_count == 0 {
                println!(
                    "givemon frame {}: gRngValue before {rng_before:#010x} after {:#010x}",
                    emu.frame(),
                    obs.rng(&mut emu)
                );
            }
            prev_count = count;
            if seg.name == "07-starter" && &keys == log.frames.last().unwrap() {
                // fallthrough: dump after the segment's last frame below.
            }
        }
        if seg.name == "07-starter" {
            let pid = emu.read32(party);
            let nature = (pid % 25) as usize;
            let sig = NATURE_STAT_TABLE[nature];
            println!(
                "pid {pid:#010x} nature {} ({:?} = +/- on atk/def/spe/spa/spd)",
                NATURE_NAMES[nature], sig
            );
            println!(
                "level {} hp {}/{} atk {} def {} spe {} spa {} spd {}",
                emu.read8(party + 0x54),
                emu.read16(party + 0x56),
                emu.read16(party + 0x58),
                emu.read16(party + 0x5A),
                emu.read16(party + 0x5C),
                emu.read16(party + 0x5E),
                emu.read16(party + 0x60),
                emu.read16(party + 0x62),
            );
            break 'outer;
        }
    }
    Ok(())
}
