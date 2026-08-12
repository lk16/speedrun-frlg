//! What does delaying the first button press actually do to the RNG?
//!
//! Three measurements:
//!
//! 1. The stream advances before any input (VBlank `Random()`,
//!    `decompiled/src/main.c:412`) -- but both `SeedRng` calls overwrite it,
//!    so the pre-press stream is discarded twice.
//! 2. Timer 1 is started *inside* the title and naming screens
//!    (`title_screen.c:351`, `naming_screen.c:428`) and read into the seed at
//!    their exit presses (`SeedRngAndSetTrainerId`, `main.c:264`, reading
//!    `REG_TM1CNT_L`). Measured here: the timer's per-frame stride while the
//!    title screen is up, i.e. how far one frame of exit delay moves the seed.
//! 3. The whole committed movie is replayed with k = 0, 1, 2 idle frames
//!    prepended (a delayed first press and everything after it shifted).
//!    Reported: both reseed values per k. If a start delay merely "started
//!    later in the same stream", the seeds would match the baseline; they do
//!    not -- each k picks an unrelated 16-bit seed.
//!
//!     cargo run --release -p frlg-rng --example seed-probe

use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog};
use frlg_rng::Rng;
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

/// `REG_ADDR_TM1CNT_L`: 0x4000000 + 0x104
/// (`decompiled/include/gba/io_reg.h:471`, base offsets in the same file).
const TM1CNT_L: u32 = 0x0400_0104;

fn main() {
    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let observer = Observer::new(syms).expect("observer");

    let mut frames: Vec<u16> = Vec::new();
    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        frames.extend(InputLog::decode(&bytes).expect("log decodes").frames);
    }

    // 2. Timer stride per frame, while the title screen is up (the committed
    // title reseed lands at frame 595, so 500 is comfortably on the screen).
    let mut emu = Emu::new(&rom).expect("core");
    frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    for &mask in &frames[..500] {
        emu.step(mask);
    }
    let t0 = emu.read16(TM1CNT_L);
    emu.step(frames[500]);
    let t1 = emu.read16(TM1CNT_L);
    emu.step(frames[501]);
    let t2 = emu.read16(TM1CNT_L);
    println!(
        "timer 1 during the title screen: {t0} -> {t1} -> {t2} \
         (stride {} then {} per frame, mod 65536)",
        t1.wrapping_sub(t0),
        t2.wrapping_sub(t1)
    );

    // 3. Replay the whole movie with k idle frames prepended.
    for prepend in 0..3u32 {
        let mut emu = Emu::new(&rom).expect("core");
        frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
        let observer = observer.clone();
        let mut model = Rng(0);
        let mut reseeds: Vec<(u32, u32)> = Vec::new();
        let mut frame = 0u32;
        let step = |emu: &mut Emu, mask: u16, frame: &mut u32| {
            emu.step(mask);
            *frame += 1;
        };
        for _ in 0..prepend {
            step(&mut emu, 0, &mut frame);
        }
        for &mask in &frames {
            step(&mut emu, mask, &mut frame);
            let observed = Rng(observer.rng(&mut emu));
            if model.distance_to(observed) > 5_000 {
                reseeds.push((frame, observed.0));
            }
            model = observed;
        }
        let where_now = observer.map(&mut emu);
        let outcome = observer.battle_outcome(&mut emu);
        println!(
            "prepend {prepend}: reseeds {:x?}, ends on map {where_now:?}, \
             battle outcome {outcome}",
            reseeds
        );
    }
}
