//! Exploration aid: mash through the intro, then let the search walk.
//!
//! `cargo run --release --example walk`

use frlg_emu::{keys, SymbolTable};
use frlg_route::nav::{self, Goal};
use frlg_route::observe::Observer;
use frlg_route::record::{Feed, Recorder};

const PLAYERS_HOUSE_1F: (u8, u8) = (4, 0);
const PALLET_TOWN: (u8, u8) = (3, 0);
const OAKS_LAB: (u8, u8) = (4, 3);

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rom = frlg_emu::default_rom_path().ok_or("no ROM")?;
    let syms = SymbolTable::load(&frlg_emu::default_sym_path().ok_or("no syms")?)?;
    let obs = Observer::new(syms)?;
    let mut rec = Recorder::from_reset(&rom)?;

    rec.mash_until("the overworld", keys::A, 6000, |emu| {
        obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu)
    })?;
    println!("intro:  {}", obs.snapshot(rec.emu()));

    for (label, goal) in [
        ("1F", Goal::on_map(PLAYERS_HOUSE_1F)),
        ("town", Goal::on_map(PALLET_TOWN)),
        ("lab", Goal::on_map(OAKS_LAB)),
    ] {
        match nav::walk_to(&mut rec, &obs, goal, 4000) {
            Ok(frames) => println!(
                "{label:>6}: {frames:>5} frames  {}",
                obs.snapshot(rec.emu())
            ),
            Err(e) => {
                println!("{label:>6}: FAILED {e}");
                println!("        at {}", obs.snapshot(rec.emu()));
                break;
            }
        }
    }
    rec.emu().write_png(std::path::Path::new("/tmp/walk.png"))?;
    Ok(())
}
