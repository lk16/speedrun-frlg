//! Probe: from the 15-to-forest checkpoint, walk to row 39 and hold UP,
//! narrating position / battle / wild state per relevant frame.

use frlg_route::brock::{self, Leg};
use frlg_route::observe::Observer;
use frlg_route::segments::Tuning;
use frlg_route::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("state file");
    let rom = frlg_emu::default_rom_path().ok_or("rom")?;
    let sym = frlg_emu::default_sym_path().ok_or("sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = Observer::new(syms).map_err(std::io::Error::other)?;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    emu.load_state_file(std::path::Path::new(&state_path))?;
    let state = emu.save_state()?;
    drop(emu);
    let mut rec = Recorder::from_state(&rom, &state)?;
    let tuning = Tuning {
        turn_hold: 2,
        text_hold: 4,
    };

    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::Near(brock::VIRIDIAN_FOREST, 7, 40, 1),
        frlg_emu::keys::LEFT,
        1500,
    )?;
    println!("at row ~39/40: {}", obs.snapshot(rec.emu()));

    // Hold UP, narrating.
    let mut last = (obs.pos(rec.emu()), false);
    for i in 0..400 {
        rec.hold(frlg_emu::keys::UP, 1)?;
        let e = rec.emu();
        let now = (obs.pos(e), obs.in_battle(e));
        if now != last || i % 60 == 0 {
            println!(
                "f{i}: pos {:?} battle {} lock {} can_step {} wild {:#018x}",
                now.0,
                now.1,
                obs.field_controls_locked(e),
                obs.player_can_step(e),
                obs.wild_key(e),
            );
            last = now;
        }
        if now.1 {
            println!("battle started at f{i}");
            break;
        }
    }
    Ok(())
}
