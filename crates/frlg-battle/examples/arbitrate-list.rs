//! Arbitrate an explicit list of plans on the committed battle-start state:
//! replay 01..08, then run each plan through the route's real drive.
//!
//!     cargo run --release -p frlg-battle --example arbitrate-list -- "4,3,3,0" "0,2,3,0" ...

use std::path::{Path, PathBuf};

use frlg_emu::{keys, Emu, InputLog, SaveState};
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

const WON: u8 = 1;
const FRAME_BUDGET: u32 = 20_000;

fn run_plan(
    emu: &mut Emu,
    observer: &Observer,
    start: &SaveState,
    mash: &[u16],
    plan: &[u32],
) -> (bool, u32) {
    emu.load_state(start).expect("load state");
    let mut frames = 0u32;
    for _ in 0..plan.first().copied().unwrap_or(0) {
        emu.step(0);
        frames += 1;
    }
    let mut turns = 0usize;
    let won = loop {
        let mut mash_phase = 0usize;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if observer.battle_outcome(emu) != 0
                || observer.battle_choosing_actions(emu)
                || frames >= FRAME_BUDGET
            {
                break;
            }
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
        if frames >= FRAME_BUDGET {
            break false;
        }
        turns += 1;
        for _ in 0..plan.get(turns).copied().unwrap_or(0) {
            emu.step(0);
            frames += 1;
        }
        let mut mash_phase = 0usize;
        loop {
            emu.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if observer.battle_outcome(emu) != 0
                || !observer.battle_choosing_actions(emu)
                || frames >= FRAME_BUDGET
            {
                break;
            }
        }
        let outcome = observer.battle_outcome(emu);
        if outcome != 0 {
            break outcome == WON;
        }
        if frames >= FRAME_BUDGET {
            break false;
        }
    };
    (won, frames)
}

fn main() {
    let plans: Vec<Vec<u32>> = std::env::args()
        .skip(1)
        .map(|s| {
            s.split(',')
                .map(|d| d.trim().parse().expect("delay"))
                .collect()
        })
        .collect();
    assert!(
        !plans.is_empty(),
        "pass at least one plan as \"d0,d1,d2,...\""
    );

    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
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
    assert!(
        observer.in_battle(&mut emu),
        "replay must end in the battle"
    );
    println!("battle-start gRngValue {:#010x}", emu.read32(rng_addr));
    let start = emu.save_state().expect("state at battle start");

    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };
    for plan in &plans {
        let (won, frames) = run_plan(&mut emu, &observer, &start, &mash, plan);
        println!(
            "plan {plan:?} -> {}",
            if won {
                format!("WIN {frames}")
            } else {
                "LOSS".to_string()
            }
        );
    }
}
