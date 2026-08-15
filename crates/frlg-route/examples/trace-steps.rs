//! Print every player step (and map change) in a frame window of the
//! committed run. Usage: trace-steps <ledger.json> <from> <to>

use frlg_route::observe::Observer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let ledger_path = args.next().expect("ledger.json path");
    let from: u32 = args.next().expect("from").parse()?;
    let to: u32 = args.next().expect("to").parse()?;

    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let obs = Observer::new(frlg_emu::SymbolTable::load(&sym)?).map_err(std::io::Error::other)?;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    let mut n = 0u32;
    let mut prev: Option<((u8, u8), (i16, i16))> = None;
    'outer: for seg in &ledger.segments {
        let bytes = std::fs::read(&seg.log)?;
        let log = frlg_emu::InputLog::decode(&bytes)?;
        for &keys in &log.frames {
            emu.step(keys);
            n += 1;
            if n < from {
                continue;
            }
            if n > to {
                break 'outer;
            }
            if let (Some(m), Some(p)) = (obs.map(&mut emu), obs.pos(&mut emu)) {
                if prev != Some((m, p)) {
                    println!("f{n} map {m:?} pos {p:?}");
                    prev = Some((m, p));
                }
            }
        }
    }
    Ok(())
}
