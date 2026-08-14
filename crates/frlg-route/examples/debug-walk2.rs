//! Reproduce a `walk_fleeing` leg from a build checkpoint.
//!
//!     cargo run --release -p frlg-route --example debug-walk2 -- \
//!         <state file> [text_hold]

use frlg_route::brock::{self, Leg};
use frlg_route::observe::Observer;
use frlg_route::segments::Tuning;
use frlg_route::Recorder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("state file argument");
    let text_hold: usize = std::env::args().nth(2).map_or(4, |s| s.parse().unwrap());
    let rom = frlg_emu::default_rom_path().ok_or("no default ROM")?;
    let sym = frlg_emu::default_sym_path().ok_or("no default sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = Observer::new(syms).map_err(std::io::Error::other)?;

    // File states carry savedata; convert to an in-memory state a Recorder
    // can resume from.
    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    emu.load_state_file(std::path::Path::new(&state_path))?;
    let state = emu.save_state()?;
    drop(emu);

    let mut rec = Recorder::from_state(&rom, &state)?;
    println!("start: {}", obs.snapshot(rec.emu()));
    let tuning = Tuning {
        turn_hold: 2,
        text_hold,
        seed_delay: 0,
        ball_delay: 0,
    };

    let keys_up = frlg_emu::keys::UP;
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::ROUTE1, (3, 0), (12, 1)),
        keys_up,
        600,
    )?;
    println!("on route 1: {}", obs.snapshot(rec.emu()));
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::VIRIDIAN_CITY, brock::ROUTE1, (12, 1)),
        keys_up,
        1000,
    )?;
    println!("done: {}", obs.snapshot(rec.emu()));
    Ok(())
}
