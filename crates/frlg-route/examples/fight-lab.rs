//! Re-run a battle's delay search from a savestate taken just after the
//! battle starts, at a given stage-1 width -- for measuring what widening
//! (or narrowing) the search actually buys before changing the route code.
//!
//! Usage: fight-lab <state-file> <start-delays> [preferred-move-id]

use frlg_route::brock;
use frlg_route::observe::Observer;
use frlg_route::segments::Tuning;
use frlg_route::Recorder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let state_path = args.next().expect("state file");
    let start_delays: usize = args.next().expect("start delays").parse()?;
    let preferred: Option<u16> = args.next().map(|s| s.parse()).transpose()?;

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
    brock::win_battle(&mut rec, &obs, tuning, preferred, "fight-lab", start_delays)?;
    println!("won in {} frames from the state", rec.frames());
    Ok(())
}
