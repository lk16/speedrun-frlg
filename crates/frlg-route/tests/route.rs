//! The committed route, replayed.
//!
//! This is the regression test the ledger's claims rest on: it does not run the
//! segment code, it replays the logs that were committed and asks the game
//! whether each segment's observable holds. A change to the segment code that
//! makes the route better must come with regenerated logs, and this test is
//! what notices when it does not.
//!
//! Run with `cargo test --release`; it needs the ROM.

use std::path::{Path, PathBuf};

use frlg_route::ledger;
use frlg_route::segments::Starter;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/frlg-route.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

#[test]
fn the_committed_route_replays_from_reset_and_beats_the_rival() {
    let root = repo_root();
    let ledger_path = root.join("route/ledger.json");
    let recorded = ledger::read(&ledger_path).expect("route/ledger.json should be committed");

    let rom = frlg_emu::default_rom_path().expect("no ROM: build it into $FRLG_ARTIFACTS/rom");
    let sym = frlg_emu::default_sym_path().expect("no pokefirered.sym in $FRLG_ARTIFACTS/rom");
    let starter = match recorded.starter.as_str() {
        "bulbasaur" => Starter::Bulbasaur,
        "squirtle" => Starter::Squirtle,
        "charmander" => Starter::Charmander,
        other => panic!("ledger names an unknown starter {other:?}"),
    };

    // The logs are recorded repo-relative.
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(&root).unwrap();
    let checked = ledger::verify(&rom, &sym, starter, &recorded, |_| {});
    std::env::set_current_dir(previous).unwrap();
    let checked = checked.expect("replaying the committed logs");

    for (was, now) in recorded.segments.iter().zip(&checked.segments) {
        assert!(
            now.tier1,
            "{} did not reach its goal: {}",
            now.name, now.goal
        );
        assert_eq!(
            was.ram_hash, now.ram_hash,
            "{} replayed to a different RAM fingerprint than the ledger records",
            now.name
        );
        assert_eq!(was.frames, now.frames, "{} changed length", now.name);
    }
    assert_eq!(checked.total_frames, recorded.total_frames);
}

/// Measure what `Tuning::text_hold` is worth on the intro alone -- the
/// MID-text stretch (`01-boot` through `03-names`) that no options menu can
/// reach (`docs/route.md`). This is a measurement, not a regression test:
/// run it by hand with
///
///     cargo test --release -p frlg-route --test route -- --ignored --nocapture text_hold
///
/// The intro is upstream of the naming-screen reseed
/// (`decompiled/src/naming_screen.c:722`), so unlike everything after the
/// bedroom, these numbers do not touch the battle's RNG stream and are
/// comparable in isolation. The full-route answer still comes from
/// `frlg route tune`, because downstream segments both gain (their text) and
/// re-roll (their battle).
#[test]
#[ignore = "a measurement, minutes long; run explicitly with --ignored"]
fn text_hold_on_the_intro_alone() {
    use frlg_route::segments::{self, Tuning, Version};
    use frlg_route::{Observer, Recorder};

    let rom = frlg_emu::default_rom_path().expect("no ROM: build it into $FRLG_ARTIFACTS/rom");
    let sym = frlg_emu::default_sym_path().expect("no pokefirered.sym in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&sym).expect("loading symbols");
    let obs = Observer::new(syms).expect("building the observer");
    let version = Version::of_rom(&rom)
        .expect("reading the ROM header")
        .expect("not a FireRed/LeafGreen ROM");

    println!("text_hold  01-boot  02-intro-oak  03-names  total(01-03)");
    for text_hold in [1usize, 2, 3, 4, 7, 15, 31] {
        let tuning = Tuning {
            text_hold,
            ..Tuning::default()
        };
        let mut rec = Recorder::from_reset(&rom).expect("booting");
        let mut cells = Vec::new();
        for segment in segments::all(version, Starter::Squirtle, tuning)
            .into_iter()
            .take(3)
        {
            let before = rec.frames();
            (segment.run)(&mut rec, &obs).unwrap_or_else(|e| panic!("{}: {e}", segment.name));
            assert!(
                (segment.reached)(&obs, rec.emu()),
                "{} did not reach its goal under text_hold {text_hold}",
                segment.name
            );
            cells.push(rec.frames() - before);
        }
        println!(
            "{:>9}  {:>7}  {:>12}  {:>8}  {:>12}",
            text_hold,
            cells[0],
            cells[1],
            cells[2],
            rec.frames()
        );
    }
}
