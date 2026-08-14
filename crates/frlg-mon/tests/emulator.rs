//! The creation model against the machine.
//!
//! Replays the committed rival-1 route and requires:
//!
//! - the starter's PID and IVs, decrypted out of `gPlayerParty`, to be
//!   exactly what [`frlg_mon::gift_mon`] predicts from the `gRngValue`
//!   stream at the `givemon` frame -- which also pins the half-order of
//!   `Random32()` (`include/random.h:14`), the one thing the C source
//!   leaves to the compiler;
//! - the ROM's own `gExperienceTables` to match the crate's formula-derived
//!   thresholds;
//! - `sWildEncounterData.rngState` to be seeded exactly twice (copyright
//!   screen and title-screen exit, `decompiled/src/intro.c:1004`,
//!   `src/title_screen.c:737`), each value a `Random()` output of the
//!   observed stream (`decompiled/src/new_game.c:103`).
//!
//! Run with `cargo test --release`; needs the ROM in `$FRLG_ARTIFACTS/rom`.

use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog, SymbolTable};
use frlg_mon::stats::{exp_for_level, Growth};
use frlg_mon::{gift_mon, Ivs};
use frlg_rng::Rng;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

struct Replay {
    emu: Emu,
    /// End-of-frame `gRngValue` per frame, index = frame number.
    rng_trace: Vec<u32>,
    /// (frame, value) at the frame where the wild RNG state changed.
    wild_seeds: Vec<(usize, u32)>,
    /// Frame at which `gPlayerPartyCount` went 0 -> 1.
    starter_frame: usize,
}

/// Replays every committed log, recording what the three watched addresses
/// did. One replay serves all the tests below; ~10 s in release.
fn replay_committed_route() -> Replay {
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1)
        .expect("no ROM matching the ledger's rom_sha1 in $FRLG_ARTIFACTS/rom");
    let syms = SymbolTable::load(&rom.with_extension("sym")).expect("sym file beside the ROM");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let wild_addr = syms
        .get("sWildEncounterData")
        .expect("sWildEncounterData (static, but `make syms` keeps locals)")
        .addr;
    let count_addr = syms
        .get("gPlayerPartyCount")
        .expect("gPlayerPartyCount")
        .addr;

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(
        boot, ledger.bios,
        "replaying the ledger's logs under its boot"
    );

    let mut rng_trace = Vec::with_capacity(ledger.total_frames);
    let mut wild_seeds = Vec::new();
    let mut wild_last = emu.read32(wild_addr);
    assert_eq!(wild_last, 0, "sWildEncounterData.rngState starts zeroed");
    let mut starter_frame = 0usize;

    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("committed log");
        let log = InputLog::decode(&bytes).expect("committed log decodes");
        for &keys in &log.frames {
            emu.step(keys);
            rng_trace.push(emu.read32(rng_addr));
            let wild = emu.read32(wild_addr);
            if wild != wild_last {
                wild_seeds.push((rng_trace.len() - 1, wild));
                wild_last = wild;
            }
            if starter_frame == 0 && emu.read8(count_addr) == 1 {
                starter_frame = rng_trace.len() - 1;
            }
        }
    }
    assert!(starter_frame > 0, "the route acquires a starter");
    Replay {
        emu,
        rng_trace,
        wild_seeds,
        starter_frame,
    }
}

/// `GetSubstruct`'s permutation table (`decompiled/src/pokemon.c:2863-2898`):
/// row = PID % 24, column = substruct *type*, value = position among the four
/// 12-byte blocks.
const SUBSTRUCT_ORDER: [[usize; 4]; 24] = [
    [0, 1, 2, 3],
    [0, 1, 3, 2],
    [0, 2, 1, 3],
    [0, 3, 1, 2],
    [0, 2, 3, 1],
    [0, 3, 2, 1],
    [1, 0, 2, 3],
    [1, 0, 3, 2],
    [2, 0, 1, 3],
    [3, 0, 1, 2],
    [2, 0, 3, 1],
    [3, 0, 2, 1],
    [1, 2, 0, 3],
    [1, 3, 0, 2],
    [2, 1, 0, 3],
    [3, 1, 0, 2],
    [2, 3, 0, 1],
    [3, 2, 0, 1],
    [1, 2, 3, 0],
    [1, 3, 2, 0],
    [2, 1, 3, 0],
    [3, 1, 2, 0],
    [2, 3, 1, 0],
    [3, 2, 1, 0],
];

/// Reads a party mon's IVs the way `GetMonData` would: XOR-decrypt the
/// secure region with `personality ^ otId` (`EncryptBoxMon`,
/// `decompiled/src/pokemon.c:2797-2805`), pick substruct 3 by the PID%24
/// permutation, and unpack the IV word at its offset 4
/// (`include/pokemon.h:49-57`: hp/atk/def/spe/spa/spd, 5 bits each from
/// bit 0).
fn read_party_ivs(emu: &mut Emu, mon_addr: u32) -> (u32, u32, Ivs) {
    let pid = emu.read32(mon_addr);
    let ot_id = emu.read32(mon_addr + 4);
    let key = pid ^ ot_id;
    // secure.substructs at offset 32 (`include/pokemon.h:106-124`:
    // 4+4+10+1+1+7+1+2+2 bytes before the union).
    let pos = SUBSTRUCT_ORDER[(pid % 24) as usize][3];
    let iv_word = emu.read32(mon_addr + 32 + (pos as u32) * 12 + 4) ^ key;
    let ivs = Ivs {
        hp: (iv_word & 31) as u8,
        atk: ((iv_word >> 5) & 31) as u8,
        def: ((iv_word >> 10) & 31) as u8,
        spe: ((iv_word >> 15) & 31) as u8,
        spa: ((iv_word >> 20) & 31) as u8,
        spd: ((iv_word >> 25) & 31) as u8,
    };
    (pid, ot_id, ivs)
}

#[test]
fn creation_exp_and_wild_seed_match_the_rom() {
    let mut replay = replay_committed_route();
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("rom");
    let syms = SymbolTable::load(&rom.with_extension("sym")).expect("syms");

    // --- The starter's creation rolls.
    let party = syms.get("gPlayerParty").expect("gPlayerParty").addr;
    let (pid, _ot, ivs) = read_party_ivs(&mut replay.emu, party);

    // The four creation rolls happen a couple of frames before the party
    // count byte reads 1 (measured: rolls at flip-2). Walk the stream from
    // a few frames earlier and find the two consecutive outputs that form
    // the PID -- in either half order -- then demand the model, started at
    // that offset, reproduce PID and IVs exactly.
    let before = Rng(replay.rng_trace[replay.starter_frame - 8]);
    let mut outputs = [0u16; 40];
    {
        let mut probe = before;
        probe.fill(&mut outputs);
    }
    let mut found = None;
    for i in 0..outputs.len() - 3 {
        let (a, b) = (outputs[i] as u32, outputs[i + 1] as u32);
        let first_high = (a << 16) | b;
        let first_low = a | (b << 16);
        if first_high == pid || first_low == pid {
            assert!(found.is_none(), "PID matched at two stream offsets");
            found = Some((i, first_high == pid));
        }
    }
    let (offset, first_high) = found.expect("starter PID is two consecutive Random() outputs");
    assert!(
        !first_high,
        "Random32()'s first call is the LOW half on this ROM (measured \
         2026-08-12); frlg_mon::create::random32 says so and must stay in \
         agreement"
    );
    let mut model_rng = before.jump(offset as u32);
    let genome = gift_mon(&mut model_rng);
    assert_eq!(genome.pid, pid, "model PID");
    assert_eq!(genome.ivs, ivs, "model IVs vs decrypted party IVs");
    println!(
        "starter: frame {}, +{offset} rolls in, pid {pid:#010x}, nature {} ({}), ivs {ivs:?}",
        replay.starter_frame,
        genome.nature(),
        frlg_mon::stats::NATURE_NAMES[genome.nature() as usize],
    );

    // --- The exp thresholds against the ROM's own table.
    // gExperienceTables[6][MAX_LEVEL + 1] (`src/data/pokemon/experience_tables.h:18`),
    // row order per GROWTH_* (`include/constants/pokemon.h:246-251`):
    // MediumFast 0, MediumSlow 3.
    let exp_tables = syms
        .get("gExperienceTables")
        .expect("gExperienceTables")
        .addr;
    let (row_mf, row_ms) = (0u32, 3u32);
    for level in 2..=20u32 {
        let medium_fast = replay.emu.read32(exp_tables + (101 * row_mf + level) * 4);
        let medium_slow = replay.emu.read32(exp_tables + (101 * row_ms + level) * 4);
        assert_eq!(
            medium_fast,
            exp_for_level(Growth::MediumFast, level as u8),
            "MF L{level}"
        );
        assert_eq!(
            medium_slow,
            exp_for_level(Growth::MediumSlow, level as u8),
            "MS L{level}"
        );
    }

    // --- The wild RNG's seedings.
    // SeedWildEncounterRng(Random()) runs from ResetMenuAndMonGlobals
    // (`decompiled/src/new_game.c:103`), which a boot reaches twice: the
    // copyright screen (`src/intro.c:1004`) and the title-screen exit
    // (`src/title_screen.c:737`, right after SeedRngAndSetTrainerId at
    // `:735`). The second is the one gameplay runs on, so the wild stream
    // is pinned by the title-exit press -- the same dial as the main seed.
    assert_eq!(
        replay.wild_seeds.len(),
        2,
        "seeded at the copyright screen and the title-screen exit"
    );
    for &(frame, seed) in &replay.wild_seeds {
        assert!(seed <= 0xFFFF, "SeedWildEncounterRng stores a u16");
        // The seed is a Random() output of that frame, i.e. the top 16 bits
        // of some recent gRngValue state. Walk *backward* from the frame's
        // end state: forward from the previous frame would cross the
        // title-exit SeedRng on the second seeding and prove nothing.
        let mut back = Rng(replay.rng_trace[frame]);
        let hit = (0..8).any(|_| {
            let matched = (back.0 >> 16) == seed;
            back = back.prev();
            matched
        });
        assert!(
            hit,
            "the wild seed {seed:#06x} is a Random() output within frame {frame}"
        );
        println!("wild rng seeded at frame {frame} with {seed:#06x}");
    }
}
