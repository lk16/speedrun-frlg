//! Replay the committed route and narrate what the RNG does, frame by frame,
//! with the Rust model ([`frlg_rng::Rng`]) shadowing the emulator's
//! `gRngValue` the whole way -- any disagreement aborts the trace, so every
//! line printed is also a correctness check of the model.
//!
//! Per segment it reports frames, total stream steps, and every frame that
//! consumed more than the per-frame VBlank call
//! (`decompiled/src/main.c:412`), which is exactly the set of frames where
//! something else rolled: NPC movement, battle rolls, seeding.
//! During the battle segment it also tracks the memory that decides the
//! fight: both `gBattleMons` HP values and `gBattleOutcome`.
//!
//!     cargo run --release -p frlg-rng --example rng-trace [-- --per-frame FILE]

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

/// Big moves are reseeds (see `tests/emulator.rs`); this bound is far above
/// any single frame's real consumption on this route.
const RESEED_THRESHOLD: u32 = 5_000;

fn main() {
    let mut per_frame_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--per-frame" => {
                per_frame_path = Some(PathBuf::from(
                    args.next().expect("--per-frame needs a file path"),
                ))
            }
            other => panic!("unknown argument {other:?}"),
        }
    }

    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1)
        .expect("no ROM matching the ledger's rom_sha1 in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios, "boot must match the ledger's");

    let mut per_frame = per_frame_path.map(|path| {
        use std::io::Write;
        let mut file = std::io::BufWriter::new(std::fs::File::create(path).expect("create"));
        writeln!(file, "frame\tsegment\tgRngValue\tsteps").unwrap();
        file
    });

    let mut model = Rng(observer.rng(&mut emu));
    let mut frame = 0u32;
    println!(
        "route: {} frames, {}, {}",
        ledger.total_frames, ledger.starter, boot
    );

    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("committed log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        let seg_start_rng = model;
        let seg_start_frame = frame;
        let mut seg_steps: u64 = 0;
        let mut busy_frames: Vec<(u32, u32)> = Vec::new(); // (frame, steps) where steps != 1
        let in_battle_segment = entry.name.contains("battle-win");
        let mut hp = (0u16, 0u16);

        if in_battle_segment {
            let ours = observer.battle_mon(&mut emu, 0);
            let theirs = observer.battle_mon(&mut emu, 1);
            hp = (ours.hp, theirs.hp);
            println!(
                "\n== {}: battle starts at frame {frame}, gRngValue {:#010x} ==",
                entry.name, model.0
            );
            println!(
                "   us: species {} lv{} {}/{} hp | rival: species {} lv{} {}/{} hp",
                ours.species,
                ours.level,
                ours.hp,
                ours.max_hp,
                theirs.species,
                theirs.level,
                theirs.hp,
                theirs.max_hp
            );
        }

        for &keys in &log.frames {
            emu.step(keys);
            frame += 1;
            let observed = Rng(observer.rng(&mut emu));
            let steps = model.distance_to(observed);
            if steps > RESEED_THRESHOLD {
                println!("frame {frame}: reseed -> {:#010x}", observed.0);
            } else {
                seg_steps += steps as u64;
                if steps != 1 {
                    busy_frames.push((frame, steps));
                }
            }
            model = observed;
            if let Some(file) = per_frame.as_mut() {
                use std::io::Write;
                writeln!(
                    file,
                    "{frame}\t{}\t{:#010x}\t{steps}",
                    entry.name, observed.0
                )
                .unwrap();
            }

            if in_battle_segment {
                let ours = observer.battle_mon(&mut emu, 0);
                let theirs = observer.battle_mon(&mut emu, 1);
                if (ours.hp, theirs.hp) != hp {
                    println!(
                        "   frame {frame} (battle frame {}): hp {} -> {} | {} -> {}  rng {:#010x}",
                        frame - seg_start_frame,
                        hp.0,
                        ours.hp,
                        hp.1,
                        theirs.hp,
                        observed.0
                    );
                    hp = (ours.hp, theirs.hp);
                }
            }
        }

        let frames = log.frames.len() as u64;
        println!(
            "{:<16} {:>5} frames, {:>5} rng steps ({:+} vs 1/frame), {} busy frames, rng {:#010x} -> {:#010x}",
            entry.name,
            frames,
            seg_steps,
            seg_steps as i64 - frames as i64,
            busy_frames.len(),
            seg_start_rng.0,
            model.0,
        );
        // The interesting frames: what consumed beyond the VBlank call.
        if !busy_frames.is_empty() {
            let shown: Vec<String> = busy_frames
                .iter()
                .take(24)
                .map(|(f, s)| format!("{f}:{s}"))
                .collect();
            let suffix = if busy_frames.len() > 24 { ", ..." } else { "" };
            println!("                 frame:steps  {}{suffix}", shown.join(" "));
        }

        if in_battle_segment {
            let outcome = observer.battle_outcome(&mut emu);
            println!(
                "   outcome {outcome} (1 = won), final rng {:#010x}",
                model.0
            );
        }
    }
}
