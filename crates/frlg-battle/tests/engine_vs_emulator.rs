//! The engine against the machine, on battles the pacing fit never saw:
//! held-out delay plans and stream shifts outside the fitter's training set.
//! For every case the emulator plays the battle with the route search's
//! exact drive (`run_plan`), the engine enumerates its gate leaves, and the
//! leaf whose commit durations match the emulator's marks must predict the
//! outcome exactly -- frame-exact on wins.
//!
//! This is also the evidence for the search claim: if the emulator wins, the
//! engine's leaf set must contain that win, so a plan whose every leaf loses
//! can be discarded without emulation.
//!
//! Run with `cargo test --release`; needs the ROM in `$FRLG_ARTIFACTS/rom`.

use std::path::{Path, PathBuf};

use frlg_battle::engine::{simulate, SimResult};
use frlg_battle::Mon;
use frlg_emu::{keys, Emu, InputLog};
use frlg_rng::Rng;
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

/// `B_OUTCOME_WON`, `decompiled/include/constants/battle.h:76`.
const WON: u8 = 1;
const FRAME_BUDGET: u32 = 20_000;

/// `struct BattlePokemon` offsets (`decompiled/include/pokemon.h:170-206`).
mod mon_off {
    pub const ATTACK: u32 = 0x02;
    pub const DEFENSE: u32 = 0x04;
    pub const SPEED: u32 = 0x06;
    pub const HP: u32 = 0x28;
    pub const LEVEL: u32 = 0x2A;
    pub const MAX_HP: u32 = 0x2C;
    pub const SIZE: u32 = 0x58;
}

fn read_mon(emu: &mut Emu, base: u32, index: u32) -> Mon {
    let a = base + index * mon_off::SIZE;
    Mon {
        hp: emu.read16(a + mon_off::HP),
        max_hp: emu.read16(a + mon_off::MAX_HP),
        attack: emu.read16(a + mon_off::ATTACK),
        defense: emu.read16(a + mon_off::DEFENSE),
        speed: emu.read16(a + mon_off::SPEED),
        level: emu.read8(a + mon_off::LEVEL),
        atk_stage: 6,
        def_stage: 6,
    }
}

/// The route search's `run_plan` (frlg-rng's battle-plan-scan), returning
/// what it returns plus the commit durations its two loops imply -- the key
/// that selects the engine leaf to compare against.
fn run_plan(
    emu: &mut Emu,
    observer: &Observer,
    start: &frlg_emu::SaveState,
    rng_addr: u32,
    mash: &[u16],
    state: Rng,
    plan: &[u32],
) -> (bool, u32, Vec<u32>) {
    emu.load_state(start).expect("load state");
    for (i, byte) in state.0.to_le_bytes().iter().enumerate() {
        emu.write8(rng_addr + i as u32, *byte);
    }
    let mut frame = 0u32;
    let mut durs = Vec::new();
    for _ in 0..plan.first().copied().unwrap_or(0) {
        emu.step(0);
        frame += 1;
    }
    let mut turns = 0usize;
    let won = loop {
        let mut mash_phase = 0usize;
        let mut over = false;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frame += 1;
            if observer.battle_outcome(emu) != 0 || observer.battle_choosing_actions(emu) {
                break;
            }
            if frame >= FRAME_BUDGET {
                over = true;
                break;
            }
        }
        if over {
            break false;
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
        turns += 1;
        for _ in 0..plan.get(turns).copied().unwrap_or(0) {
            emu.step(0);
            frame += 1;
        }
        let loop_b_start = frame;
        mash_phase = 0;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frame += 1;
            if observer.battle_outcome(emu) != 0 || !observer.battle_choosing_actions(emu) {
                break;
            }
            if frame >= FRAME_BUDGET {
                over = true;
                break;
            }
        }
        durs.push(frame - 1 - loop_b_start);
        if over {
            break false;
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
    };
    (won, frame, durs)
}

#[test]
fn engine_leaves_match_fresh_emulator_battles() {
    // None of these (shift, plan) pairs is in fit-pacing's training set:
    // shifts beyond +-10, and plan shapes the sweeps never visited.
    let cases: &[(i64, &[u32])] = &[
        (0, &[4, 6, 2, 9]),
        (0, &[2, 5, 1]),
        (0, &[1, 12, 4, 4, 4]),
        (0, &[3, 15, 15, 15]),
        (12, &[0]),
        (12, &[4, 3, 3, 3]),
        (13, &[1]),
        (-13, &[2]),
        (-13, &[4, 4, 4, 4]),
        (17, &[3]),
        (17, &[0, 9, 9]),
        (-20, &[4]),
    ];

    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let mons_base = syms.get("gBattleMons").expect("gBattleMons").addr;
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
    }
    let start = emu.save_state().expect("state at battle start");
    let base = Rng(emu.read32(rng_addr));

    // The mons never depend on the shift: the party is in the savestate.
    // Idle past gBattleMons initialisation to read them; every case below
    // reloads the savestate anyway.
    let (us, rival) = {
        for _ in 0..40 {
            emu.step(0);
        }
        let mons = (
            read_mon(&mut emu, mons_base, 0),
            read_mon(&mut emu, mons_base, 1),
        );
        assert_ne!(
            mons.0.hp, 0,
            "gBattleMons initialised within 40 idle frames"
        );
        mons
    };

    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };

    for &(shift, plan) in cases {
        let state = if shift >= 0 {
            base.jump(shift as u32)
        } else {
            let mut s = base;
            for _ in 0..-shift {
                s = s.prev();
            }
            s
        };
        let (won, frames, durs) =
            run_plan(&mut emu, &observer, &start, rng_addr, &mash, state, plan);
        let leaves = simulate(plan, state, us, rival);

        let matching: Vec<_> = leaves.iter().filter(|l| l.commit_durs == durs).collect();
        assert!(
            !matching.is_empty(),
            "shift {shift} plan {plan:?}: emulator committed {durs:?}, \
             no leaf did (gate sets too narrow); leaves: {leaves:?}"
        );
        if won {
            assert!(
                matching
                    .iter()
                    .any(|l| l.result == SimResult::Win { frames }),
                "shift {shift} plan {plan:?}: emulator won in {frames}, \
                 matching leaves predicted {matching:?}"
            );
        } else {
            assert!(
                matching.iter().all(|l| l.result == SimResult::Loss),
                "shift {shift} plan {plan:?}: emulator lost (frames {frames}), \
                 matching leaves predicted {matching:?}"
            );
        }
    }
}
