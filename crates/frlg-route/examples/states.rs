//! Exploration aid: mash a button from reset and print every state change.
//!
//! `cargo run --release --example states -- [frames] [key]`
//!
//! This is how a segment gets written: watch which `gMain.callback2` the game
//! walks through, then wait on those transitions by name instead of counting
//! frames by hand.

use frlg_emu::{keys, SymbolTable};
use frlg_route::observe::Observer;
use frlg_route::record::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let budget: usize = args.next().unwrap_or_else(|| "20000".into()).parse()?;
    let key = keys::parse(&args.next().unwrap_or_else(|| "A".into()))?;

    let rom = frlg_emu::default_rom_path().ok_or("no ROM")?;
    let syms = SymbolTable::load(&frlg_emu::default_sym_path().ok_or("no syms")?)?;
    let obs = Observer::new(syms)?;

    let mut rec = Recorder::from_reset(&rom)?;
    let mut last = String::new();
    for i in 0..budget {
        rec.step(if i % 2 == 0 { key } else { 0 })?;
        let snap = obs.snapshot(rec.emu());
        let now = format!(
            "{} {:?} {:?} {:?} {}",
            snap.callback2, snap.map, snap.pos, snap.party_count, snap.in_battle
        );
        if now != last {
            println!("{snap}");
            last = now;
        }
    }
    Ok(())
}
