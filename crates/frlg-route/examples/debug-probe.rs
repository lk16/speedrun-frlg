//! Probe: from the 17-to-gym checkpoint, walk to Brock and narrate the
//! A-mash.

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
        seed_delay: 0,
    };

    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::Tile(brock::PEWTER_GYM, 6, 6),
        frlg_emu::keys::UP,
        3000,
    )?;
    println!("at (6,6): {}", obs.snapshot(rec.emu()));

    rec.hold(frlg_emu::keys::UP, 2)?;
    rec.idle(1)?;
    for i in 0..2000usize {
        let key = if i % 2 == 0 { frlg_emu::keys::A } else { 0 };
        rec.hold(key, 1)?;
        let e = rec.emu();
        if i % 200 == 0 || obs.in_battle(e) {
            println!(
                "f{i}: pos {:?} battle {} lock {} can_step {} cb2 {}",
                obs.pos(e),
                obs.in_battle(e),
                obs.field_controls_locked(e),
                obs.player_can_step(e),
                obs.callback2_name(e),
            );
        }
        if obs.in_battle(e) {
            println!("battle at f{i}");
            break;
        }
    }
    Ok(())
}
