//! From a forest-exit savestate, walk straight to Brock -- no Pokémon
//! Center -- and run the full battle search. Answers whether the heal
//! segment is load-bearing for the current run's arrival HP, before any
//! route restructuring.
//!
//! Usage: no-heal-probe <forest-exit-state>

use frlg_emu::keys;
use frlg_route::brock::{self, Leg};
use frlg_route::observe::Observer;
use frlg_route::segments::Tuning;
use frlg_route::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("forest-exit state file");
    let rom = frlg_emu::default_rom_path().ok_or("no ROM")?;
    let sym = frlg_emu::default_sym_path().ok_or("no sym")?;
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
        ball_delay: 0,
    };

    // The heal segment's walking, minus the Pokémon Center detour
    // (`brock::heal` cites the warps): north entrance exit (7,1), Route 2's
    // top meets Pewter's bottom, gym door at Pewter (15,16).
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::ROUTE2, brock::FOREST_NORTH_ENTRANCE, (7, 2)),
        keys::UP,
        400,
    )?;
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::PEWTER_CITY, brock::ROUTE2, (9, 1)),
        keys::UP,
        1000,
    )?;
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::MapVia(brock::PEWTER_GYM, brock::PEWTER_CITY, (15, 17)),
        keys::UP,
        1500,
    )?;
    let to_gym = rec.frames();
    println!("at the gym in {to_gym} frames from the forest exit");

    // Brock's talk tile is (6,6) (`brock::brock`).
    brock::walk_fleeing(
        &mut rec,
        &obs,
        tuning,
        Leg::Tile(brock::PEWTER_GYM, 6, 6),
        keys::UP,
        3000,
    )?;
    rec.wait_until("the player to settle", 240, |emu| obs.player_can_step(emu))?;
    rec.hold(keys::UP, 2)?;
    rec.idle(6)?;
    rec.mash_until("the battle to start", keys::A, 3000, |emu| {
        obs.in_battle(emu)
    })?;
    const MOVE_BUBBLE: u16 = 145;
    brock::win_battle(&mut rec, &obs, tuning, Some(MOVE_BUBBLE), "brock", 192)?;
    println!(
        "no-heal: forest exit -> Brock beaten in {} frames (committed path: 7706)",
        rec.frames()
    );
    Ok(())
}
