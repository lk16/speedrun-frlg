//! The squirtle-fight model against the machine: replay the defeat-brock
//! route's committed `09-battle-win` on libmgba, reconstruct the plan the
//! log encodes (menu delays from the idle runs after each
//! `choosing_actions` detection) and the commit gates it resolved (the
//! `choosing_actions` fall), and require `engine::simulate_with` under
//! [`pacing::SQUIRTLE_LAB`] to enumerate a leaf that is exactly the battle
//! the emulator played -- same gates, same frame count.
//!
//! Run with `cargo test --release`; needs the ROM in `$FRLG_ARTIFACTS/rom`.

use std::path::{Path, PathBuf};

use frlg_battle::engine::{simulate_with, SimResult};
use frlg_battle::{pacing, Mon, Move};
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

#[test]
fn engine_reproduces_the_committed_brock_run_rival_battle() {
    assert!(pacing::SQUIRTLE_LAB_FITTED);
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/defeat-brock/ledger.json"))
        .expect("committed ledger");
    assert_eq!(ledger.starter, "squirtle");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let mons_base = syms.get("gBattleMons").expect("gBattleMons").addr;
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let observer = Observer::new(syms).expect("observer");

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);

    let mut battle_log: Option<InputLog> = None;
    for entry in &ledger.segments {
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        if entry.name == "09-battle-win" {
            battle_log = Some(log);
            break;
        }
        for &mask in &log.frames {
            emu.step(mask);
        }
    }
    let battle_log = battle_log.expect("route has 09-battle-win");
    let anchor = Rng(emu.read32(rng_addr));

    // Feed the committed log, watching the action-selection state: a rise
    // at frame f is a menu detection, the zero-mask run right after it is
    // that menu's planned delay, and the fall closes the commit gate.
    let mut choosing_prev = false;
    let mut detections: Vec<u32> = Vec::new();
    let mut falls: Vec<u32> = Vec::new();
    let mut outcome_frame: Option<u32> = None;
    let mut mons: Option<(Mon, Mon)> = None;
    for (frame, &mask) in battle_log.frames.iter().enumerate() {
        emu.step(mask);
        if mons.is_none() && emu.read16(mons_base + mon_off::HP) != 0 {
            mons = Some((
                read_mon(&mut emu, mons_base, 0),
                read_mon(&mut emu, mons_base, 1),
            ));
        }
        let choosing = observer.battle_choosing_actions(&mut emu);
        if choosing && !choosing_prev {
            detections.push(frame as u32);
        }
        if !choosing && choosing_prev {
            // The fitter's `loop_b_end`: the frame whose step flipped the
            // selection state off.
            falls.push(frame as u32);
        }
        choosing_prev = choosing;
        if outcome_frame.is_none() && observer.battle_outcome(&mut emu) != 0 {
            outcome_frame = Some(frame as u32);
        }
    }
    assert_eq!(observer.battle_outcome(&mut emu), 1, "committed battle won");
    let outcome_frame = outcome_frame.expect("outcome set");
    let (us, rival) = mons.expect("gBattleMons initialised");
    assert_eq!((us.hp, rival.hp), (20, 19), "battle-truth mons");

    // Reconstruct plan and gates. plan[0] is the drive's pre-battle idle
    // (zero in the committed log -- the intro mash starts immediately);
    // plan[k] is the zero-run after detection k-1.
    let mut plan: Vec<u32> = vec![0];
    let mut gates: Vec<u32> = Vec::new();
    for (k, &det) in detections.iter().enumerate() {
        let mut delay = 0u32;
        while battle_log.frames[(det + 1 + delay) as usize] == 0 {
            delay += 1;
        }
        plan.push(delay);
        // The commit gate: the fall of choosing_actions minus the commit
        // mash start (det + delay + 1).
        let fall = falls.get(k).copied().expect("every menu commits");
        gates.push(fall - (det + delay + 1));
    }
    let frames = outcome_frame + 1;
    assert_eq!(
        frames,
        battle_log.frames.len() as u32,
        "the committed log ends when the outcome lands"
    );

    // The engine must enumerate exactly this battle among its leaves.
    let leaves = simulate_with(
        &plan,
        anchor,
        us,
        rival,
        Move::Tackle,
        &pacing::SQUIRTLE_LAB,
    );
    assert!(
        leaves
            .iter()
            .any(|l| l.commit_durs == gates && l.result == SimResult::Win { frames }),
        "committed battle (plan {plan:?}, gates {gates:?}, {frames} frames, anchor \
         {anchor:#010x?}) missing from the engine's {} leaves: {leaves:?}",
        leaves.len()
    );
}
