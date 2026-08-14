//! Scan the starter-genome dial: replay a committed ledger's prefix to the
//! end of `06-to-lab`, then run `07-starter` once per candidate `ball_delay`
//! and read the starter the game actually created -- nature, IVs (decrypted
//! from the box substruct), and the computed L5 stats.
//!
//! Usage: ball-scan <ledger.json> [max_delay]
//!
//! Each delay costs its own frames 1:1 *and* re-picks every downstream
//! battle stream; this scan only reports genomes -- whether a genome pays
//! is a full-build question.

use frlg_mon::stats::NATURE_NAMES;

/// Which of the four 12-byte encrypted slots holds `PokemonSubstruct3`
/// (the IVs), by `personality % 24` -- the v4 column of `SUBSTRUCT_CASE`
/// (`decompiled/src/pokemon.c:2863-2896`).
const SUBSTRUCT3_SLOT: [u32; 24] = [
    3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0,
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger_path = std::env::args().nth(1).expect("ledger.json path");
    let max_delay: usize = std::env::args()
        .nth(2)
        .map(|s| s.parse().expect("max_delay"))
        .unwrap_or(64);
    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::default_sym_path().ok_or("sym")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let obs = frlg_route::Observer::new(syms.clone()).map_err(std::io::Error::other)?;
    let party = syms.get("gPlayerParty").ok_or("gPlayerParty")?.addr;

    let version = frlg_route::Version::of_rom(&rom)?.ok_or("not FR/LG")?;
    let starter = match ledger.starter.as_str() {
        "bulbasaur" => frlg_route::Starter::Bulbasaur,
        "squirtle" => frlg_route::Starter::Squirtle,
        "charmander" => frlg_route::Starter::Charmander,
        other => return Err(format!("unknown starter {other}").into()),
    };

    // Replay the committed prefix through 06-to-lab.
    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    for seg in &ledger.segments {
        let log = frlg_emu::InputLog::decode(&std::fs::read(&seg.log)?)?;
        for &keys in &log.frames {
            emu.step(keys);
        }
        if seg.name == "06-to-lab" {
            break;
        }
    }
    let state = emu.save_state()?;
    drop(emu);

    println!("delay  pid         nature   ivs h/a/d/s/sa/sd   L5 hp/a/d/s/sa/sd  frames");
    for delay in 0..=max_delay {
        let tuning = frlg_route::segments::Tuning {
            ball_delay: delay,
            ..ledger.tuning
        };
        let mut rec = frlg_route::Recorder::from_state(&rom, &state)?;
        let before = rec.frames();
        let segs = frlg_route::segments::all(version, starter, tuning);
        let seg = segs
            .iter()
            .find(|s| s.name == "07-starter")
            .expect("07-starter exists");
        if let Err(e) = (seg.run)(&mut rec, &obs) {
            println!("{delay:>5}  segment failed: {e}");
            continue;
        }
        let frames = rec.frames() - before;
        let emu = rec.emu();
        let pid = emu.read32(party);
        let ot_id = emu.read32(party + 4);
        let key = pid ^ ot_id;
        // `secure.substructs` at box offset 0x20
        // (`decompiled/include/pokemon.h:105-126`: personality, otId,
        // nickname[10], language, flags byte, otName[7], markings,
        // checksum u16, unknown u16), 4 x 12-byte slots, each u32 XORed
        // with `otId ^ personality` (`EncryptBoxMon`).
        let slot = SUBSTRUCT3_SLOT[(pid % 24) as usize];
        let iv_word = emu.read32(party + 0x20 + slot * 12 + 4) ^ key;
        let iv = |shift: u32| (iv_word >> shift) & 0x1F;
        println!(
            "{delay:>5}  {pid:#010x}  {:<8} {:>2}/{:>2}/{:>2}/{:>2}/{:>2}/{:>2}   {:>2}/{:>2}/{:>2}/{:>2}/{:>2}/{:>2}  {frames}",
            NATURE_NAMES[(pid % 25) as usize],
            iv(0),
            iv(5),
            iv(10),
            iv(15),
            iv(20),
            iv(25),
            emu.read16(party + 0x58),
            emu.read16(party + 0x5A),
            emu.read16(party + 0x5C),
            emu.read16(party + 0x5E),
            emu.read16(party + 0x60),
            emu.read16(party + 0x62),
        );
    }
    Ok(())
}
