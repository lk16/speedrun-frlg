//! Tier-1 tests: these run the real ROM under libmgba.
//!
//! They need `$FRLG_ARTIFACTS/rom/pokefirered.gba` (or `$FRLG_ROM`). That file
//! is produced by `make COMPARE=1` in the decomp, so a missing ROM is a setup
//! error rather than a reason to skip -- these fail loudly instead of passing
//! vacuously.

use std::path::PathBuf;

use frlg_emu::{keys, Emu, InputLog, SaveState};

/// mGBA reaches the FireRed title screen well inside this many frames.
const TITLE_FRAMES: u32 = 3000;

fn rom() -> PathBuf {
    frlg_emu::default_rom_path().expect(
        "no ROM: build it with `make -C ~/decomp COMPARE=1` and copy \
         pokefirered.gba into $FRLG_ARTIFACTS/rom/, or set $FRLG_ROM",
    )
}

fn emu() -> Emu {
    Emu::new(&rom()).expect("libmgba failed to load the ROM")
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("frlg-test-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creating scratch dir");
    dir.join(name)
}

#[test]
fn loads_the_firered_header() {
    let emu = emu();
    assert_eq!(emu.game_title(), "POKEMON FIRE");
    assert_eq!(emu.game_code(), "AGB-BPRE");
    assert_eq!(emu.rom_size(), 16 * 1024 * 1024);
    assert_eq!((emu.width(), emu.height()), (240, 160));
}

#[test]
fn boots_and_advances_the_rng() {
    let mut emu = emu();
    // gRngValue, 0x03005000 per pokefirered.sym.
    let at_reset = emu.read32(0x0300_5000);
    emu.idle(TITLE_FRAMES);

    assert_eq!(emu.frame(), TITLE_FRAMES);
    assert_ne!(
        emu.read32(0x0300_5000),
        at_reset,
        "gRngValue never moved -- the core is not actually running the game"
    );
}

#[test]
fn the_same_inputs_produce_the_same_ram() {
    let log = InputLog::new([0; 20], press_start_log());

    let mut first = emu();
    first.replay(&log, |_, _| {});

    let mut second = emu();
    second.replay(&log, |_, _| {});

    assert_eq!(
        first.ram_hash().expect("ram fingerprint"),
        second.ram_hash().expect("ram fingerprint"),
        "two identical replays diverged; the harness is not deterministic"
    );
}

#[test]
fn different_inputs_produce_different_ram() {
    let held = InputLog::new([0; 20], press_start_log());
    let idle = InputLog::new([0; 20], vec![0u16; held.len()]);

    let mut with_input = emu();
    with_input.replay(&held, |_, _| {});

    let mut without = emu();
    without.replay(&idle, |_, _| {});

    assert_ne!(
        with_input.ram_hash().expect("ram fingerprint"),
        without.ram_hash().expect("ram fingerprint"),
        "pressing START changed nothing; key input is not reaching the game"
    );
}

#[test]
fn an_in_memory_savestate_restores_the_run_exactly() {
    let mut emu = emu();
    emu.idle(1200);

    let checkpoint = emu.save_state().expect("saving state");
    assert_eq!(checkpoint.len(), emu.state_size());

    emu.idle(300);
    let expected = emu.ram_hash().expect("ram fingerprint");

    emu.load_state(&checkpoint).expect("restoring state");
    emu.idle(300);

    assert_eq!(
        emu.ram_hash().expect("ram fingerprint"),
        expected,
        "restore-then-rerun landed somewhere else; savestates are not usable \
         for the input search"
    );
}

#[test]
fn a_savestate_file_carries_across_cores() {
    let path = scratch("checkpoint.state");

    let mut writer = emu();
    writer.idle(1200);
    writer.save_state_file(&path).expect("writing state file");
    writer.idle(300);
    let expected = writer.ram_hash().expect("ram fingerprint");

    let mut reader = emu();
    reader.load_state_file(&path).expect("reading state file");
    reader.idle(300);

    assert_eq!(reader.ram_hash().expect("ram fingerprint"), expected);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn a_wrong_sized_savestate_is_refused_rather_than_accepted() {
    let mut emu = emu();
    emu.idle(60);

    let mut bytes = emu.save_state().unwrap().as_bytes().to_vec();
    bytes.truncate(bytes.len() - 1);

    // A short buffer handed to libmgba's loadState would read past its end.
    assert!(matches!(
        emu.load_state(&SaveState::from_bytes(bytes)),
        Err(frlg_emu::EmuError::StateSize { .. })
    ));
}

#[test]
fn the_memory_block_view_agrees_with_bus_reads() {
    let mut emu = emu();
    emu.idle(600);

    let via_bus = emu.read_bytes(frlg_emu::emu::IWRAM_START + 0x5000, 16);
    let via_block = emu
        .with_memory_block(frlg_emu::emu::IWRAM_START + 0x5000, |block, offset| {
            block[offset..offset + 16].to_vec()
        })
        .expect("IWRAM block");

    assert_eq!(via_bus, via_block);
}

#[test]
fn the_screenshot_is_the_right_shape_and_fully_opaque() {
    let mut emu = emu();
    emu.idle(TITLE_FRAMES);

    let rgba = emu.screen_rgba();
    assert_eq!(rgba.len(), (240 * 160 * 4) as usize);
    assert!(rgba.chunks_exact(4).all(|px| px[3] == 0xff));
    assert!(
        rgba.chunks_exact(4).any(|px| px[..3] != [0, 0, 0]),
        "the title screen came out entirely black"
    );

    let png = scratch("title.png");
    emu.write_png(&png).expect("writing png");
    let bytes = std::fs::read(&png).unwrap();
    assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n");
    let _ = std::fs::remove_file(&png);
}

#[test]
fn a_log_replayed_in_two_halves_matches_one_pass() {
    let frames = press_start_log();
    let (head, tail) = frames.split_at(frames.len() / 2);

    let mut whole = emu();
    whole.replay(&InputLog::new([0; 20], frames.clone()), |_, _| {});

    let mut split = emu();
    split.replay(&InputLog::new([0; 20], head.to_vec()), |_, _| {});
    split.replay(&InputLog::new([0; 20], tail.to_vec()), |_, _| {});

    assert_eq!(
        whole.ram_hash().expect("ram fingerprint"),
        split.ram_hash().expect("ram fingerprint")
    );
}

/// Idle to the title screen, then tap START a few times.
fn press_start_log() -> Vec<u16> {
    let mut frames = vec![0u16; TITLE_FRAMES as usize + 240];
    for tap in 0..4 {
        let at = TITLE_FRAMES as usize + tap * 60;
        frames[at] = keys::START;
        frames[at + 1] = keys::START;
    }
    frames
}
