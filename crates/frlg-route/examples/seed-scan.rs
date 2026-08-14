//! Map `Tuning::seed_delay` to the wild-encounter seed it buys.
//!
//! For each delay in the range, boot exactly the way `01-boot` does (mash to
//! `CB2_TitleScreenRun`, idle, mash to `CB2_NewGameScene`) and print the
//! seeded `sWildEncounterData.rngState` -- the whole pass/fail sequence of
//! the run's wild encounters is a pure function of it
//! (`docs/defeat-brock/research/wild-encounters.md`). Output is one line per
//! delay: `delay wild_seed boot_frames`.

use frlg_emu::keys;
use frlg_route::observe::Observer;
use frlg_route::{Feed, Recorder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max: usize = std::env::args()
        .nth(1)
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(48);
    let rom = frlg_emu::default_rom_path().ok_or("no ROM: $FRLG_ARTIFACTS/rom")?;
    let sym = frlg_emu::default_sym_path().ok_or("no sym: $FRLG_ARTIFACTS/rom")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = Observer::new(syms).map_err(std::io::Error::other)?;
    let wild = obs
        .symbols()
        .get("sWildEncounterData")
        .ok_or("sWildEncounterData not in the symbol table")?
        .addr;

    for delay in 0..max {
        let mut rec = Recorder::from_reset(&rom)?;
        rec.mash_until("the title screen", keys::A, 1200, |emu| {
            obs.callback2_is(emu, "CB2_TitleScreenRun")
        })?;
        rec.idle(delay)?;
        rec.mash_until("NEW GAME", keys::A, 2000, |emu| {
            obs.callback2_is(emu, "CB2_NewGameScene")
        })?;
        println!("{delay} {:#06x} {}", rec.emu().read32(wild), rec.frames());
    }
    Ok(())
}
