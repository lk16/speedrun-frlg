//! The model against the machine: replay the committed route on libmgba and
//! check `frlg_rng::Rng` against `gRngValue` on every single frame.
//!
//! This is the test that makes the pure-Rust RNG a fact rather than a
//! transcription: if `random.c`, the VBlank hook, or the seeding ever differ
//! from the model, some frame's predicted state will not match the one read
//! out of IWRAM.
//!
//! Run with `cargo test --release`; it needs the ROM in `$FRLG_ARTIFACTS/rom`.

use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog, SymbolTable};
use frlg_rng::Rng;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

/// A frame-to-frame move bigger than this is not consumption: nothing in one
/// frame of this route calls `Random()` thousands of times, so only `SeedRng`
/// (a fresh 16-bit state, `decompiled/src/random.c:16-19`) can look like it.
const RESEED_THRESHOLD: u32 = 5_000;

/// A `SeedRng(u16)` leaves a state <= 0xFFFF; by the time the frame ends the
/// VBlank `Random()` (and at most a couple more consumers) may have stepped
/// it. Walking that few steps back must land on a 16-bit state.
fn steps_since_16bit_seed(state: Rng) -> Option<u32> {
    let mut back = state;
    for steps in 0..4 {
        if back.0 <= 0xFFFF {
            return Some(steps);
        }
        back = back.prev();
    }
    None
}

/// Replays every committed log in ledger order and, on each frame, steps the
/// model by however many `Random()` calls the emulator's `gRngValue` moved
/// and demands exact agreement. The two `SeedRng` events the route contains
/// (title-screen exit and player naming-screen exit,
/// `decompiled/src/title_screen.c:735`, `naming_screen.c:722`) are the only
/// frames allowed to move the state by more than [`RESEED_THRESHOLD`] steps,
/// and each must be a fresh 16-bit seed.
#[test]
fn model_matches_grngvalue_on_every_frame_of_the_committed_route() {
    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");

    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1)
        .expect("no ROM matching the ledger's rom_sha1 in $FRLG_ARTIFACTS/rom");
    let syms = SymbolTable::load(&rom.with_extension("sym")).expect("sym file beside the ROM");
    let rng_addr = syms
        .get("gRngValue")
        .expect("gRngValue in the sym file")
        .addr;

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(
        boot, ledger.bios,
        "this test replays the ledger's logs and must boot the way they were built"
    );

    // gRngValue is COMMON_DATA-initialised to 0 and the VBlank handler steps
    // it from there even before the first SeedRng, so the model starts at 0.
    let mut model = Rng(emu.read32(rng_addr));
    assert_eq!(model.0, 0, "gRngValue starts zeroed at reset");

    let mut frames = 0u32;
    let mut max_step = 0u32;
    let mut reseeds: Vec<(u32, u32)> = Vec::new();

    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("committed log");
        let log = InputLog::decode(&bytes).expect("committed log decodes");
        for &keys in &log.frames {
            emu.step(keys);
            frames += 1;
            let observed = Rng(emu.read32(rng_addr));

            let steps = model.distance_to(observed);
            if steps > RESEED_THRESHOLD {
                assert!(
                    steps_since_16bit_seed(observed).is_some(),
                    "frame {frames}: gRngValue moved {steps} steps \
                     ({:#010x} -> {:#010x}) but no 16-bit seed is within \
                     reach -- model divergence, not a reseed",
                    model.0,
                    observed.0
                );
                reseeds.push((frames, observed.0));
            } else {
                max_step = max_step.max(steps);
                assert_eq!(model.jump(steps), observed, "frame {frames}");
            }
            model = observed;
        }
    }

    assert_eq!(
        frames as usize, ledger.total_frames,
        "replayed the whole route"
    );
    assert_eq!(
        reseeds.len(),
        2,
        "SeedRngAndSetTrainerId runs exactly twice on this route \
         (title-screen exit and player naming-screen exit); saw {reseeds:?}"
    );
    // The route contains a battle; something must consume more than the
    // per-frame VBlank call, or this test watched nothing interesting.
    assert!(
        max_step > 1,
        "no frame ever consumed more than the VBlank call -- suspicious"
    );
}
