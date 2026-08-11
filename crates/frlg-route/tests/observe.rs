//! The probes are transcribed struct offsets, so they are checked against the
//! running game rather than against the header they came from. A wrong offset
//! reads a plausible number; only the game can say it is the wrong one.
//!
//! Run with `cargo test --release` -- these boot the real ROM.

use frlg_emu::{keys, SymbolTable};
use frlg_route::observe::Observer;
use frlg_route::record::{Feed, Recorder};

/// `data/maps/map_groups.json`: group 4 index 1 is
/// `PalletTown_PlayersHouse_2F`, which is where a new game starts.
const PLAYERS_HOUSE_2F: (u8, u8) = (4, 1);

fn setup() -> (Observer, Recorder) {
    let rom = frlg_emu::default_rom_path()
        .expect("no ROM: build it and copy it into $FRLG_ARTIFACTS/rom");
    let syms = SymbolTable::load(
        &frlg_emu::default_sym_path().expect("no pokefirered.sym in $FRLG_ARTIFACTS/rom"),
    )
    .unwrap();
    let obs = Observer::new(syms).unwrap();
    let rec = Recorder::from_reset(&rom).unwrap();
    (obs, rec)
}

/// Mash A from reset until the player is standing in the bedroom. This is not
/// the route -- it is the cheapest way to get a live overworld to probe.
fn to_overworld(obs: &Observer, rec: &mut Recorder) {
    rec.mash_until("the overworld", keys::A, 6000, |emu| {
        obs.callback2_is(emu, "CB2_Overworld") && obs.player_can_step(emu)
    })
    .expect("mashing A should reach the overworld");
}

#[test]
fn callback2_resolves_to_the_screen_the_game_is_actually_on() {
    let (obs, mut rec) = setup();
    // The copyright screen is the first thing `AgbMain` hands over to -- after
    // the BIOS boot animation (~272 frames on the intro boot) has played.
    rec.wait_until("the copyright screen", 600, |emu| {
        obs.callback2_is(emu, "CB2_InitCopyrightScreenAfterBootup")
    })
    .unwrap();

    to_overworld(&obs, &mut rec);
    assert_eq!(obs.callback2_name(rec.emu()), "CB2_Overworld");
    assert!(obs.callback2_is(rec.emu(), "CB2_Overworld"));
    assert!(!obs.callback2_is(rec.emu(), "CB2_MainMenu"));
}

#[test]
fn the_save_block_probes_agree_with_where_a_new_game_starts() {
    let (obs, mut rec) = setup();
    // No save block exists before the game allocates one.
    assert_eq!(obs.save_block1(rec.emu()), None);
    assert_eq!(obs.map(rec.emu()), None);

    to_overworld(&obs, &mut rec);
    assert_eq!(obs.map(rec.emu()), Some(PLAYERS_HOUSE_2F));
    // The bedroom is 12x9 (`data/layouts/layouts.json`), so a position outside
    // that would mean the offsets are pointing at something else entirely.
    let (x, y) = obs.pos(rec.emu()).unwrap();
    assert!((0..12).contains(&x) && (0..9).contains(&y), "pos ({x},{y})");
    assert_eq!(obs.party_count(rec.emu()), 0);
    assert!(!obs.in_battle(rec.emu()));
}

#[test]
fn walking_moves_the_position_probe_by_one_tile_in_the_direction_pressed() {
    // Which way is clear from the bed is the room's business; that a step
    // moves the probe by exactly one tile the right way is the offsets'.
    let expected = [
        (keys::UP, (0, -1)),
        (keys::DOWN, (0, 1)),
        (keys::LEFT, (-1, 0)),
        (keys::RIGHT, (1, 0)),
    ];

    let mut moved = 0;
    for (dir, delta) in expected {
        let (obs, mut rec) = setup();
        to_overworld(&obs, &mut rec);
        let before = obs.pos(rec.emu()).unwrap();

        // Blocked directions turn the player in place and never move them, so
        // a timeout here is a legitimate answer, not a failure.
        if rec
            .advance_while("a step", &[dir], 120, |emu| obs.pos(emu) != Some(before))
            .is_err()
        {
            continue;
        }

        let after = obs.pos(rec.emu()).unwrap();
        assert_eq!(
            (after.0 - before.0, after.1 - before.1),
            delta,
            "pressing {} moved {before:?} -> {after:?}",
            keys::Display(dir)
        );
        assert_eq!(obs.map(rec.emu()), Some(PLAYERS_HOUSE_2F));
        moved += 1;
    }
    assert!(moved > 0, "no direction moved the player at all");
}

#[test]
fn the_game_sees_the_keys_the_recorder_feeds() {
    let (obs, mut rec) = setup();
    to_overworld(&obs, &mut rec);
    // gMain.newKeys is the game's own view of the press, so this is the probe
    // that would catch a key-bit order mistake.
    rec.step(keys::START).unwrap();
    assert_eq!(obs.new_keys(rec.emu()) & keys::START, keys::START);
    rec.step(keys::START).unwrap();
    assert_eq!(
        obs.new_keys(rec.emu()) & keys::START,
        0,
        "a held key is new only once"
    );
}
