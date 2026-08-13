//! Probe: walk to waypoint 1 in the forest, then hold each direction and
//! narrate what the game does.

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
        Leg::Near(brock::VIRIDIAN_FOREST, 41, 44, 2),
        frlg_emu::keys::UP,
        1200,
    )?;
    println!("at wp1: {}", obs.snapshot(rec.emu()));

    for dir in [
        frlg_emu::keys::UP,
        frlg_emu::keys::LEFT,
        frlg_emu::keys::RIGHT,
        frlg_emu::keys::DOWN,
    ] {
        let save = rec.save_state()?;
        for i in 0..120 {
            rec.hold(dir, 1)?;
            let e = rec.emu();
            if i % 30 == 29 || obs.in_battle(e) {
                println!(
                    "dir {dir:#06x} f{i}: pos {:?} battle {} lock {} can_step {} prevent {}",
                    obs.pos(e),
                    obs.in_battle(e),
                    obs.field_controls_locked(e),
                    obs.player_can_step(e),
                    obs.prevent_step(e),
                );
            }
            if obs.in_battle(e) {
                break;
            }
        }
        rec.emu().load_state(&save)?;
    }
    Ok(())
}
