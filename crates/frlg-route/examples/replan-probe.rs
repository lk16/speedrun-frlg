//! Re-plan one leg from the middle of the committed run: replay the
//! ledger's logs to an absolute frame, read the real position and
//! `sWildEncounterData` from RAM, and ask the planner for the best path to
//! a target tile -- at the honest encounter cost and with encounters
//! forbidden. Answers "was this mid-run dogleg deliberate?" against the
//! model instead of a viewer's intuition.
//!
//! Usage: replan-probe <ledger.json> <frame> <tx> <ty>

use frlg_route::observe::Observer;
use frlg_route::plan::{self, PlanRequest};
use frlg_route::world::World;
use std::collections::HashSet;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let ledger_path = args.next().expect("ledger.json path");
    let frame: u32 = args.next().expect("frame").parse()?;
    let tx: i16 = args.next().expect("tx").parse()?;
    let ty: i16 = args.next().expect("ty").parse()?;

    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let obs = Observer::new(frlg_emu::SymbolTable::load(&sym)?).map_err(std::io::Error::other)?;
    let mut world = World::load()?;
    let version = frlg_route::segments::Version::of_rom(&rom)
        .ok()
        .flatten()
        .unwrap_or(frlg_route::segments::Version::FireRed);

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    let mut n = 0u32;
    'outer: for seg in &ledger.segments {
        let bytes = std::fs::read(&seg.log)?;
        let log = frlg_emu::InputLog::decode(&bytes)?;
        for &keys in &log.frames {
            emu.step(keys);
            n += 1;
            if n >= frame {
                break 'outer;
            }
        }
    }

    let map = obs.map(&mut emu).ok_or("no map")?;
    let pos = obs.pos(&mut emu).ok_or("no pos")?;
    let wd = obs.wild_data(&mut emu);
    println!("f{n}: map {map:?} pos {pos:?} wild {wd:?}");

    let table = frlg_route::brock::wild_table(map, version);
    let data = world.map(map)?;
    for (label, cost) in [
        ("honest", plan::ENCOUNTER_COST),
        ("clean-only", plan::FORBID_ENCOUNTERS),
    ] {
        let req = PlanRequest {
            map: data,
            wild: table,
            start: pos,
            wild_data: wd,
            targets: vec![(tx, ty)],
            blocked: HashSet::new(),
            encounter_cost: cost,
            test_bias: std::env::var("TEST_BIAS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        };
        match plan::plan(&req) {
            None => println!("plan[{label}]: no path"),
            Some((steps, c)) => {
                println!("plan[{label}]: {} steps, model cost {c}", steps.len());
                for s in &steps {
                    println!("  {:?} {:?}", s.to, s.kind);
                }
            }
        }
    }
    Ok(())
}
