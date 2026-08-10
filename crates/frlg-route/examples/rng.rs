//! How much does the rival battle depend on luck?
//!
//! `cargo run --release --example rng -- <07-battle-start.state> [tries]`
//!
//! Delays the battle by 0, 1, 2, ... frames and mashes the same A pattern
//! through it. Everything else is identical, so any spread in the result is the
//! RNG stream moving underneath -- which is the whole question the optimisation
//! pass has to answer before it can claim the current win is anything but a
//! roll that went the route's way.

use std::path::PathBuf;

use frlg_emu::{keys, SaveState, SymbolTable};
use frlg_route::observe::Observer;
use frlg_route::record::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let state_path = PathBuf::from(args.next().ok_or("usage: rng <state> [tries]")?);
    let tries: usize = args.next().unwrap_or_else(|| "12".into()).parse()?;

    let rom = frlg_emu::default_rom_path().ok_or("no ROM")?;
    let syms = SymbolTable::load(&frlg_emu::default_sym_path().ok_or("no syms")?)?;
    let obs = Observer::new(syms)?;

    // The checkpoint is a savestate *file* (savedata and RTC included); read it
    // once through a throwaway core so every try starts from the same bytes.
    let mut loader = Recorder::from_reset(&rom)?;
    loader.emu().load_state_file(&state_path)?;
    let start: SaveState = loader.save_state()?;

    println!("delay  frames  outcome  player-hp  rival-hp  turns  rng-at-start");
    for delay in 0..tries {
        let mut rec = Recorder::from_state(&rom, &start)?;
        rec.idle(delay)?;
        let rng = obs.rng(rec.emu());

        // Count the times either side's HP moved: two per turn where both act,
        // so it is a proxy for battle length rather than an exact turn count.
        let mut hits = 0usize;
        let mut last = (0u16, 0u16);
        let frames = rec.mash_until("the battle to end", keys::A, 20000, |emu| {
            let now = (obs.battle_mon(emu, 0).hp, obs.battle_mon(emu, 1).hp);
            if now != last {
                hits += 1;
                last = now;
            }
            obs.battle_outcome(emu) != 0
        })?;

        let player = obs.battle_mon(rec.emu(), 0);
        let rival = obs.battle_mon(rec.emu(), 1);
        println!(
            "{delay:>5}  {frames:>6}  {:>7}  {:>9}  {:>8}  {:>5}  {rng:#010x}",
            obs.battle_outcome(rec.emu()),
            player.hp,
            rival.hp,
            hits.saturating_sub(1),
        );
    }
    Ok(())
}
