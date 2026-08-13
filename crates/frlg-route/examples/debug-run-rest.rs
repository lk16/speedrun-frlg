//! Run the defeat-brock segments from a checkpoint state onward, without
//! replaying the prefix: segment-code validation at iteration speed. The
//! resulting log is NOT route material (it did not start at reset); the
//! committed build still runs from power-on.
//!
//!     cargo run --release -p frlg-route --example debug-run-rest -- \
//!         <state file> <first segment index, 0-based within the brock list>

use frlg_route::brock;
use frlg_route::observe::Observer;
use frlg_route::segments::{Starter, Tuning};
use frlg_route::Recorder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("state file argument");
    let first: usize = std::env::args()
        .nth(2)
        .expect("first brock-segment index")
        .parse()?;
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

    for segment in brock::segments(Starter::Squirtle, tuning)
        .drain(..)
        .skip(first)
    {
        let t0 = std::time::Instant::now();
        let before = rec.frames();
        (segment.run)(&mut rec, &obs)?;
        let ok = (segment.reached)(&obs, rec.emu());
        println!(
            "{:<16} {:>6} frames  reached={ok}  ({:?})",
            segment.name,
            rec.frames() - before,
            t0.elapsed()
        );
        if !ok {
            return Err(format!("{} did not reach its goal", segment.name).into());
        }
        if let Ok(dir) = std::env::var("DEBUG_STATES") {
            std::fs::create_dir_all(&dir)?;
            rec.save_state_file(std::path::Path::new(&dir).join(format!("{}.state", segment.name)).as_path())?;
        }
    }
    println!("done: {}", obs.snapshot(rec.emu()));
    Ok(())
}
