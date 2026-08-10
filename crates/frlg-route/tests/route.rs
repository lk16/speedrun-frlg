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
