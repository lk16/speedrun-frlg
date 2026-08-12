//! The model against the machine: replay the committed `09-battle-win` on
//! libmgba, extract the battle's logic rolls (everything beyond the 2/frame
//! VBlank pair; within a busy frame the pair leads and the game's rolls
//! trail -- established by matching damage arithmetic in both orders), and
//! drive `frlg-battle`'s turn model over exactly that roll list. The model
//! must consume every roll, predict every HP change, and end on the win.
//!
//! Run with `cargo test --release`; needs the ROM in `$FRLG_ARTIFACTS/rom`.

use std::path::{Path, PathBuf};

use frlg_battle::{execute_move, rival_choose_move, FirstBattleFlags, Mon, Move, Outcome};
use frlg_emu::{Emu, InputLog};
use frlg_rng::Rng;

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
fn model_reproduces_every_roll_and_hp_change_of_the_committed_battle() {
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let mons_base = syms.get("gBattleMons").expect("gBattleMons").addr;
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
    let outcome_addr = syms.get("gBattleOutcome").expect("gBattleOutcome").addr;

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

    // Extract the emulator's ground truth: the ordered logic-roll values and
    // the HP trajectory.
    let mut model_rng = Rng(emu.read32(rng_addr));
    let mut rolls: Vec<u16> = Vec::new();
    let mut hp_events: Vec<(u16, u16)> = Vec::new();
    let mut hp = (0u16, 0u16);
    let mut mons: Option<(Mon, Mon)> = None;
    for &mask in &battle_log.frames {
        emu.step(mask);
        let observed = Rng(emu.read32(rng_addr));
        let steps = model_rng.distance_to(observed);
        assert!(steps >= 2, "battle frames roll twice in VBlank");
        // The VBlank pair leads the frame window; the game's own rolls trail.
        let mut cursor = model_rng.jump(2);
        for _ in 0..steps - 2 {
            cursor = cursor.next();
            rolls.push((cursor.0 >> 16) as u16);
        }
        model_rng = observed;

        let ours = emu.read16(mons_base + mon_off::HP);
        let theirs = emu.read16(mons_base + mon_off::SIZE + mon_off::HP);
        if (ours, theirs) != hp {
            if hp == (0, 0) {
                mons = Some((
                    read_mon(&mut emu, mons_base, 0),
                    read_mon(&mut emu, mons_base, 1),
                ));
            } else if theirs != 0 || hp.1 != 0 {
                hp_events.push((ours, theirs));
            }
            hp = (ours, theirs);
        }
    }
    assert_eq!(emu.read8(outcome_addr), 1, "the committed battle is a win");
    let (mut us, mut rival) = mons.expect("gBattleMons initialised");
    assert!(
        us.speed != rival.speed,
        "speed tie would consume turn-order rolls the model does not include"
    );
    assert!(us.speed > rival.speed, "this route's player acts first");

    // Drive the model over the extracted rolls.
    let mut pos = 0usize;
    let mut src = || {
        let value = rolls
            .get(pos)
            .copied()
            .expect("model wants more rolls than the emulator consumed");
        pos += 1;
        value
    };
    let mut flags = FirstBattleFlags::default();
    let mut predicted_hp: Vec<(u16, u16)> = Vec::new();

    // TryDoEventsBeforeFirstTurn's trailing gRandomTurnNumber
    // (decompiled/src/battle_main.c:2926).
    let _ = src();

    let mut turns = 0;
    while rival.hp > 0 {
        turns += 1;
        assert!(turns <= 16, "model diverged: battle would have ended");
        // The AI answers during action selection, before the player commits
        // (src/battle_controller_opponent.c:1350-1359).
        let rival_move = rival_choose_move(&us, &rival, &mut src);
        // We act first (speed), always Tackle (the mash takes FIGHT + slot 1).
        match execute_move(&us, &mut rival, Move::Tackle, true, &mut flags, &mut src) {
            Outcome::Hit { .. } => predicted_hp.push((us.hp, rival.hp)),
            other => panic!("player Tackle cannot miss here, got {other:?}"),
        }
        if rival.hp == 0 {
            break;
        }
        match execute_move(&rival, &mut us, rival_move, false, &mut flags, &mut src) {
            Outcome::Hit { .. } | Outcome::AttackLowered => {
                predicted_hp.push((us.hp, rival.hp));
            }
            Outcome::Missed => {}
        }
        assert!(us.hp > 0, "model diverged: we fainted, the emulator won");
        // BattleTurnPassed's gRandomTurnNumber (battle_main.c:2999).
        let _ = src();
    }

    // Growl events change stages, not HP; the emulator's hp_events only has
    // HP changes -- filter the model's predictions the same way.
    let model_hp_changes: Vec<(u16, u16)> = {
        let mut out = Vec::new();
        let mut last = (us.max_hp, rival.max_hp);
        for &event in &predicted_hp {
            if event != last {
                out.push(event);
                last = event;
            }
        }
        out
    };
    assert_eq!(
        model_hp_changes, hp_events,
        "the model's HP trajectory must match the emulator's exactly"
    );

    // Every extracted roll must be accounted for: no unmodelled consumers.
    assert_eq!(
        pos,
        rolls.len(),
        "the emulator consumed rolls the model does not explain: {:?}",
        &rolls[pos..]
    );
}
