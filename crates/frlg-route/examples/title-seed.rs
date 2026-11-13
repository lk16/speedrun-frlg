//! Title-exit seed per `Tuning::seed_delay`: boot to NEW GAME, then read
//! `gTrainerId` (= the u16 TM1 value `SeedRngAndSetTrainerId` seeded with,
//! `decompiled/src/main.c:264-269`) and `sWildEncounterData.rngState`
//! (`decompiled/src/wild_encounter.c:661-664`). Usage: title-seed <lo> <hi>
//! Rows: seed_delay  tm1_val  wild_rng_state
use frlg_route::segments::{self, Starter, Tuning, Version};
use frlg_route::{Observer, Recorder};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let (lo, hi): (usize, usize) = (a[1].parse()?, a[2].parse()?);
    let rom =
        std::path::PathBuf::from(std::env::var("FRLG_ARTIFACTS")?).join("rom/pokefirered.gba");
    let syms = frlg_emu::SymbolTable::load(&frlg_emu::sym_path_for_rom(&rom).ok_or("sym")?)?;
    let obs = Observer::new(syms.clone())?;
    let version = Version::of_rom(&rom)?.ok_or("ver")?;
    let tid = syms.get("gTrainerId").ok_or("tid")?.addr;
    let wild = syms.get("sWildEncounterData").ok_or("wild")?.addr;
    for s in lo..hi {
        let t = Tuning {
            seed_delay: s,
            ..Tuning::default()
        };
        let segs = segments::all(version, Starter::Squirtle, t);
        let mut rec = Recorder::from_reset(&rom)?;
        (segs[0].run)(&mut rec, &obs)?;
        let e = rec.emu();
        println!("{s}\t{}\t{}", e.read16(tid), e.read32(wild));
    }
    Ok(())
}
