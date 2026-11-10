//! Measures, on libmgba, the frame cost of the wild-encounter pieces `frlg-human` models with
//! guessed constants (`ENC_INTRO`, `THROW`, `CATCH_END`, `FLEE`).
//!
//! These are floors: inputs are mashed with no human delay, so a human takes longer
//! (`text_factor` in the plans). usage: frlg-calib <state> [trials] [png-dir]

use frlg_emu::{boot_with_default_bios, default_rom_path, default_sym_path, keys, Emu, SymbolTable};
use frlg_route::Observer;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let state = PathBuf::from(&args[1]);
    let trials: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    let png_dir = args.get(3).map(PathBuf::from);
    let rom = default_rom_path().expect("rom");
    let mut emu = Emu::new(&rom).expect("emu");
    boot_with_default_bios(&mut emu).expect("bios");
    let syms = SymbolTable::load(&default_sym_path().expect("sym")).expect("syms");
    let obs = Observer::new(syms).expect("observer");
    emu.load_state_file(&state).expect("state");
    emu.idle(260);
    if let Some(d) = &png_dir {
        let _ = emu.write_png(&d.join("start.png"));
        println!("pos {:?} map {:?}", obs.pos(&mut emu), obs.map(&mut emu));
    }
    let base = emu.save_state().expect("save");
    let mut results: Vec<(String, u32, u32, u32, u32)> = vec![];
    for trial in 0..trials {
        let mode = ["flee", "throw", "throw2"][(trial % 3) as usize];
        emu.load_state(&base).expect("load");
        emu.idle(trial * 5);
        // Walk up and down until a battle starts.
        let mut battle_at = None;
        for leg in 0..80u32 {
            // Two steps up onto the grass row, then back and forth sideways along it.
            let (dir, steps) = match leg {
                0 => (keys::UP, 4),
                l if l % 2 == 1 => (keys::LEFT, 4),
                _ => (keys::RIGHT, 4),
            };
            for _ in 0..16 * steps {
                emu.step(dir);
                if obs.in_battle(&mut emu) {
                    battle_at = Some(emu.frame());
                    break;
                }
            }
            if battle_at.is_some() {
                break;
            }
            if trial == 0 && leg < 6 {
                println!("leg {leg} pos {:?}", obs.pos(&mut emu));
                if let Some(d) = &png_dir {
                    let _ = emu.write_png(&d.join(format!("leg{leg}.png")));
                }
            }
        }
        let Some(t0) = battle_at else {
            println!("trial {trial}: no encounter");
            continue;
        };
        // Intro: mash A until the action menu.
        let mut n = 0u32;
        while !obs.battle_choosing_actions(&mut emu) && n < 3000 {
            emu.step(if n % 2 == 0 { keys::A } else { 0 });
            n += 1;
        }
        let intro = emu.frame() - t0;
        if let Some(d) = &png_dir {
            let _ = emu.write_png(&d.join(format!("t{trial}-menu.png")));
        }
        let t1 = emu.frame();
        let mut throws = 0;
        match mode {
            "flee" => {
                // Down, Right: bottom-right is Run.
                emu.step(keys::DOWN);
                emu.step(0);
                emu.step(keys::RIGHT);
                emu.step(0);
                let mut k = 0u32;
                while obs.in_battle(&mut emu) && k < 3000 {
                    emu.step(if k % 2 == 0 { keys::A } else { 0 });
                    k += 1;
                }
                let end = emu.frame() - t1;
                // Until the player can step again.
                let mut j = 0;
                while !obs.player_can_step(&mut emu) && j < 600 {
                    emu.step(0);
                    j += 1;
                }
                results.push(("flee".into(), intro, end, emu.frame() - t1, 0));
            }
            _ => {
                // Bag (top right), then mash A through the pocket, the item and USE.
                emu.step(keys::RIGHT);
                emu.step(0);
                let mut k = 0u32;
                let mut shot = false;
                loop {
                    emu.step(if k % 2 == 0 { keys::A } else { 0 });
                    k += 1;
                    if let (Some(d), false, true) = (&png_dir, shot, k == 40) {
                        let _ = emu.write_png(&d.join(format!("t{trial}-bag.png")));
                        shot = true;
                    }
                    if !obs.in_battle(&mut emu) || obs.battle_choosing_actions(&mut emu) && k > 20 || k > 3000 {
                        break;
                    }
                }
                throws += 1;
                let cycle = emu.frame() - t1;
                if !obs.in_battle(&mut emu) {
                    // Caught (or the battle ended): until the player can step again; B declines the nickname.
                    let mut j = 0;
                    while !obs.player_can_step(&mut emu) && j < 3000 {
                        emu.step(if j % 2 == 0 { keys::B } else { 0 });
                        j += 1;
                    }
                    results.push(("throw-end".into(), intro, cycle, emu.frame() - t1, throws));
                } else {
                    results.push(("throw-fail".into(), intro, cycle, cycle, throws));
                }
            }
        }
    }
    for r in &results {
        println!("{:11} intro {:4} cycle {:4} total {:4}", r.0, r.1, r.2, r.3);
    }
}
