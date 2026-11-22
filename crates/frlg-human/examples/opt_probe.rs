//! Print the game options of a savestate (route-windows journal 2026-11-22: which text speed and
//! battle-scene setting the played run's battles were measured at).
//!
//!     cargo run --release --example opt_probe -- <rom> <sym> <state> [<state> ...]
//!
//! `struct SaveBlock2` (`decompiled/include/global.h`): u16 at 0x14 = `optionsTextSpeed:3`,
//! `optionsWindowFrameType:5`, `optionsSound:1`, `optionsBattleStyle:1`, `optionsBattleSceneOff:1`.
//! Text speed 0 SLOW, 1 MID (the `new_game.c:62` default), 2 FAST.
use std::path::Path;

use frlg_emu::{Emu, SymbolTable};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 3 {
        eprintln!("usage: opt_probe <rom> <sym> <state> [<state> ...]");
        std::process::exit(2);
    }
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let sb2 = syms.get("gSaveBlock2Ptr").expect("gSaveBlock2Ptr").addr;
    let mut emu = Emu::new(Path::new(&a[0])).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    for st in &a[2..] {
        emu.load_state_file(Path::new(st)).expect("state");
        let p = emu.read32(sb2);
        let o = emu.read16(p + 0x14);
        println!(
            "{st}: text speed {} battle style {} battle scene off {}",
            o & 7,
            (o >> 9) & 1,
            (o >> 10) & 1
        );
    }
}
