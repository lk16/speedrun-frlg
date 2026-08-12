//! Ground truth for a battle model: replay the committed battle and log, per
//! frame, every `Random()` consumed beyond the battle's 2-per-frame VBlank
//! pair -- with the u16 values those calls returned, reconstructed from the
//! RNG model (within a busy frame the VBlank pair leads and the game's own
//! rolls trail; established by matching damage arithmetic in both orders,
//! `docs/rival-1/journal/` 2026-08-12) -- plus both mons' full battle stats, stat
//! stages, and every HP change. This is the dataset `frlg-battle`'s
//! predictions get checked against.
//!
//!     cargo run --release -p frlg-rng --example battle-truth [-- TSV_PATH]

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

/// `struct BattlePokemon`, `decompiled/include/pokemon.h:170-206`.
mod mon_off {
    pub const SPECIES: u32 = 0x00;
    pub const ATTACK: u32 = 0x02;
    pub const DEFENSE: u32 = 0x04;
    pub const SPEED: u32 = 0x06;
    pub const MOVES: u32 = 0x0C;
    /// `s8 statStages[NUM_BATTLE_STATS]` -- index 1 is ATK, 2 DEF
    /// (`decompiled/include/constants/battle.h`, STAT_ATK/STAT_DEF).
    pub const STAT_STAGES: u32 = 0x18;
    pub const HP: u32 = 0x28;
    pub const LEVEL: u32 = 0x2A;
    pub const MAX_HP: u32 = 0x2C;
    pub const SIZE: u32 = 0x58;
}

fn dump_mon(emu: &mut Emu, base: u32, index: u32) -> String {
    let a = base + index * mon_off::SIZE;
    let moves: Vec<u16> = (0..4)
        .map(|m| emu.read16(a + mon_off::MOVES + 2 * m))
        .collect();
    let stages: Vec<i8> = (0..8)
        .map(|s| emu.read8(a + mon_off::STAT_STAGES + s) as i8)
        .collect();
    format!(
        "species {} lv{} hp {}/{} atk {} def {} spe {} moves {:?} stages {:?}",
        emu.read16(a + mon_off::SPECIES),
        emu.read8(a + mon_off::LEVEL),
        emu.read16(a + mon_off::HP),
        emu.read16(a + mon_off::MAX_HP),
        emu.read16(a + mon_off::ATTACK),
        emu.read16(a + mon_off::DEFENSE),
        emu.read16(a + mon_off::SPEED),
        moves,
        stages,
    )
}

fn main() {
    let tsv_path = std::env::args().nth(1).map(PathBuf::from);
    let root = repo_root();
    let ledger = frlg_route::ledger::read(&root.join("route/rival-1/ledger.json"))
        .expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let mons_base = syms.get("gBattleMons").expect("gBattleMons").addr;
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

    let mut tsv = tsv_path.map(|p| {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(p).expect("create tsv"));
        writeln!(
            f,
            "battle_frame\tgame_rolls\troll_values\thp0\thp1\tstages0\tstages1"
        )
        .unwrap();
        f
    });

    let mut model = Rng(observer.rng(&mut emu));
    let mut hp = (0u16, 0u16);
    let mut stats_shown = false;
    for (frame, &mask) in battle_log.frames.iter().enumerate() {
        emu.step(mask);
        let observed = Rng(observer.rng(&mut emu));
        let steps = model.distance_to(observed);
        assert!(steps >= 2, "battle frames roll twice in VBlank");
        // ALL of this frame's outputs, in stream order. Which of them are
        // the VBlank pair and which the game's own rolls is a question the
        // consumer answers against known damage arithmetic -- printing the
        // full window keeps that decidable from the data.
        let mut cursor = model;
        let game_rolls: Vec<u16> = (0..steps)
            .map(|_| {
                cursor = cursor.next();
                (cursor.0 >> 16) as u16
            })
            .collect();
        let game_rolls = if steps > 2 { game_rolls } else { Vec::new() };
        model = observed;

        let ours_hp = emu.read16(mons_base + mon_off::HP);
        let theirs_hp = emu.read16(mons_base + mon_off::SIZE + mon_off::HP);
        let hp_now = (ours_hp, theirs_hp);

        if !stats_shown && ours_hp != 0 {
            stats_shown = true;
            println!("battle frame {frame}: mons initialised");
            println!("  us   : {}", dump_mon(&mut emu, mons_base, 0));
            println!("  rival: {}", dump_mon(&mut emu, mons_base, 1));
        }

        if !game_rolls.is_empty() || hp_now != hp {
            let stages0: Vec<i8> = (0..8)
                .map(|s| emu.read8(mons_base + mon_off::STAT_STAGES + s) as i8)
                .collect();
            let stages1: Vec<i8> = (0..8)
                .map(|s| emu.read8(mons_base + mon_off::SIZE + mon_off::STAT_STAGES + s) as i8)
                .collect();
            println!(
                "battle frame {frame:>4}: rolls {game_rolls:?} hp {:?} -> {hp_now:?} \
                 stages atk/def {}{} | {}{}",
                hp, stages0[1], stages0[2], stages1[1], stages1[2],
            );
            if let Some(f) = tsv.as_mut() {
                use std::io::Write;
                writeln!(
                    f,
                    "{frame}\t{}\t{game_rolls:?}\t{}\t{}\t{stages0:?}\t{stages1:?}",
                    game_rolls.len(),
                    hp_now.0,
                    hp_now.1
                )
                .unwrap();
            }
        }
        hp = hp_now;
    }
    println!("outcome {} (1 = won)", observer.battle_outcome(&mut emu));
}
