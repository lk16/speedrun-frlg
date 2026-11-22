//! Segment-cost calibration tracer (`human-run/calibration.md`, 2026-11-22 calibration section).
//!
//! Replays a played run's `.ilog` from one checkpoint savestate to a later frame and writes one CSV
//! row per frame with what the frame was spent on: battle flags, the running screen, the player's
//! map and tile, the avatar's movement state, script locks and fades. `human-run/tools/cal_decompose.py`
//! turns the CSV into walking / text-menu / battle / overhead totals.
//!
//!     cargo run --release --example cal_trace -- <rom> <sym> <run.ilog> <state> <start> <end> <out.csv>
//!
//! `start` is the frame the state was saved at (the checkpoint frame): frame `i` of the log is the
//! input fed after `i` frames. Prints the RAM hash at `end` so the caller can check it against
//! `checkpoints.jsonl`.
use std::io::Write;
use std::path::Path;

use frlg_emu::{Emu, InputLog, SymbolTable};
use frlg_route::Observer;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() != 7 {
        eprintln!("usage: cal_trace <rom> <sym> <run.ilog> <state> <start> <end> <out.csv>");
        std::process::exit(2);
    }
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let addr = |n: &str| syms.get(n).unwrap_or_else(|| panic!("{n}")).addr;
    // `struct PlayerAvatar` (`decompiled/include/global.fieldmap.h`): flags 0x00, runningState 0x02,
    // tileTransitionState 0x03, preventStep 0x06.
    let avatar = addr("gPlayerAvatar");
    // `sLockFieldControls` (`decompiled/src/script.c:34`), `sGlobalScriptContextStatus` (`script.c`),
    // `sMessageBoxType` (`decompiled/src/field_message_box.c`), `gPaletteFade.active` (bit 15 of
    // the u16 at offset 6, `decompiled/include/palette.h:37-56`), `gBattleTypeFlags`.
    let lock = addr("sLockFieldControls");
    let ctx = addr("sGlobalScriptContextStatus");
    let msg = addr("sMessageBoxType");
    let fade = addr("gPaletteFade");
    let btf = addr("gBattleTypeFlags");
    // `sWildEncounterData` (`decompiled/src/wild_encounter.c:24-34`): encounterRateBuff u16 at 6,
    // stepsSinceLastEncounter u8 at 8.
    let wild = addr("sWildEncounterData");
    let obs = Observer::new(syms.clone()).expect("observer");

    let log = InputLog::decode(&std::fs::read(&a[2]).expect("ilog")).expect("ilog decode");
    let start: usize = a[4].parse().unwrap();
    let end: usize = a[5].parse().unwrap();
    // Booted like `Recorder::from_reset` (BIOS, then reset): the BIOS changes SWI timing, so a state
    // replayed on the HLE core diverges.
    let mut emu = Emu::new(Path::new(&a[0])).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    emu.load_state_file(Path::new(&a[3])).expect("state");
    let mut out = std::io::BufWriter::new(std::fs::File::create(&a[6]).expect("out"));
    writeln!(
        out,
        "frame,keys,in_battle,btf,cb2,mg,mn,x,y,av_flags,run_state,tile_state,prevent,lock,ctx,msg,fade,enc_buff,enc_steps"
    )
    .unwrap();
    let mut last_cb2 = 0u32;
    let mut cb2_name = String::new();
    for f in start..end.min(log.frames.len()) {
        let k = log.frames[f];
        emu.step(k);
        let cb2 = obs.callback2(&mut emu);
        if cb2 != last_cb2 {
            last_cb2 = cb2;
            cb2_name = obs.callback2_name(&mut emu);
        }
        let (mg, mn) = obs.map(&mut emu).unwrap_or((255, 255));
        let (x, y) = obs.pos(&mut emu).unwrap_or((-1, -1));
        writeln!(
            out,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            f + 1,
            k,
            obs.in_battle(&mut emu) as u8,
            emu.read32(btf),
            cb2_name,
            mg,
            mn,
            x,
            y,
            emu.read8(avatar),
            emu.read8(avatar + 2),
            emu.read8(avatar + 3),
            emu.read8(avatar + 6),
            emu.read8(lock),
            emu.read8(ctx),
            emu.read8(msg),
            (emu.read8(fade + 7) & 0x80 != 0) as u8,
            emu.read16(wild + 6),
            emu.read8(wild + 8),
        )
        .unwrap();
    }
    out.flush().unwrap();
    println!("ram_hash {}", emu.ram_hash().expect("hash"));
}
