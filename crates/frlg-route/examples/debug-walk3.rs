//! Reproduce segment 12 (parcel) from the 11-to-viridian checkpoint.

use frlg_route::brock::{self, Leg};
use frlg_route::observe::{Observer, VAR_VIRIDIAN_MART};
use frlg_route::segments::Tuning;
use frlg_route::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("state file argument");
    let rom = frlg_emu::default_rom_path().ok_or("no default ROM")?;
    let sym = frlg_emu::default_sym_path().ok_or("no default sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = Observer::new(syms).map_err(std::io::Error::other)?;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    emu.load_state_file(std::path::Path::new(&state_path))?;
    let state = emu.save_state()?;
    drop(emu);

    let mut rec = Recorder::from_state(&rom, &state)?;
    println!("start: {}", obs.snapshot(rec.emu()));
    let tuning = Tuning {
        turn_hold: 2,
        text_hold: 4,
    };

    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::VIRIDIAN_MART, brock::VIRIDIAN_CITY, (36, 20)),
        frlg_emu::keys::UP,
        1500,
    )?;
    println!("in mart: {}", obs.snapshot(rec.emu()));

    rec.hold_mash_until("the parcel handover", frlg_emu::keys::B, 4, 3000, |emu| {
        obs.var(emu, VAR_VIRIDIAN_MART) == Some(1)
            && !obs.field_controls_locked(emu)
            && obs.player_can_step(emu)
    })?;
    println!("after scene: {}", obs.snapshot(rec.emu()));

    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::VIRIDIAN_CITY, brock::VIRIDIAN_MART, (4, 8)),
        frlg_emu::keys::DOWN,
        1500,
    )?;
    println!("done: {}", obs.snapshot(rec.emu()));
    Ok(())
}
