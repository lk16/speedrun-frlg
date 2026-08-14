//! Reproduce a walk from a build checkpoint state, with nav diagnostics.
//!
//!     FRLG_NAV_DEBUG=1 cargo run --release -p frlg-route --example debug-walk -- \
//!         $FRLG_ARTIFACTS/states/route-defeat-brock/10-exit-lab.state

use frlg_route::nav::{self, Goal};
use frlg_route::observe::Observer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let state_path = std::env::args().nth(1).expect("state file argument");
    let rom = frlg_emu::default_rom_path().ok_or("no default ROM")?;
    let sym = frlg_emu::default_sym_path().ok_or("no default sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = Observer::new(syms).map_err(std::io::Error::other)?;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    emu.load_state_file(std::path::Path::new(&state_path))?;
    println!("start: {}", obs.snapshot(&mut emu));

    // Leg 1: Pallet -> Route 1.
    let route1 = (3u8, 19u8);
    let viridian = (3u8, 1u8);
    let start = emu.save_state()?;
    let (path, reached) = nav::search_best_effort(
        &mut emu,
        &obs,
        &start,
        Goal::on_map_via(route1, (3, 0), (12, 1)),
        3000,
    )?;
    println!("leg1 route1: reached={reached} frames={}", path.frames);
    emu.load_state(&start)?;
    for &k in &path.inputs {
        emu.step(k);
    }
    println!("after leg1: {}", obs.snapshot(&mut emu));

    // Leg 2: Route 1 -> Viridian.
    let start = emu.save_state()?;
    let t0 = std::time::Instant::now();
    let (path, reached) = nav::search_best_effort(
        &mut emu,
        &obs,
        &start,
        Goal::on_map_via(viridian, route1, (12, 1)),
        6000,
    )?;
    println!(
        "leg2 viridian: reached={reached} frames={} in {:?}",
        path.frames,
        t0.elapsed()
    );
    emu.load_state(&start)?;
    for &k in &path.inputs {
        emu.step(k);
    }
    println!("after leg2: {}", obs.snapshot(&mut emu));
    Ok(())
}
