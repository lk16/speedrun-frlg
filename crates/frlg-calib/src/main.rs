//! Measures, on libmgba, the frame cost of the wild-encounter pieces `frlg-human` models with
//! guessed constants (`ENC_INTRO`, `THROW`, `CATCH_END`, `FLEE`).
//!
//! These are floors: inputs are mashed with no human delay, so a human takes longer
//! (`text_factor` in the plans). usage: frlg-calib <state> [trials] [png-dir]
//!
//! Start: `route-defeat-brock-r78/to-forest.state`, Viridian Forest's south entrance with five Poke
//! Balls and a level-6ish starter (a TAS state; the battle UI is the same everywhere).

use frlg_emu::{
    boot_with_default_bios, default_rom_path, default_sym_path, keys, Emu, SymbolTable,
};
use frlg_route::Observer;
use std::path::PathBuf;

struct Ctx {
    emu: Emu,
    obs: Observer,
    g_battle_mons: u32,
}

impl Ctx {
    /// Steps with `key` mashed (pressed on even frames) until `done` or `limit` frames.
    fn mash(
        &mut self,
        key: u16,
        limit: u32,
        mut done: impl FnMut(&mut Emu, &Observer) -> bool,
    ) -> u32 {
        let t = self.emu.frame();
        let mut n = 0;
        while !done(&mut self.emu, &self.obs) && n < limit {
            self.emu.step(if n % 2 == 0 { key } else { 0 });
            n += 1;
        }
        self.emu.frame() - t
    }

    fn tap(&mut self, key: u16) {
        self.emu.step(key);
        self.emu.step(0);
    }
}

fn mean(v: &[u32]) -> f64 {
    v.iter().sum::<u32>() as f64 / v.len().max(1) as f64
}

/// A trainer battle from a state at its start: A mashed through everything. Prints frames per
/// phase: start to first menu, each turn cycle, and the end (battle over to free movement).
fn trainer(state: &std::path::Path) {
    let rom = default_rom_path().expect("rom");
    let mut emu = Emu::new(&rom).expect("emu");
    boot_with_default_bios(&mut emu).expect("bios");
    let sym_path = default_sym_path().expect("sym");
    let obs = Observer::new(SymbolTable::load(&sym_path).unwrap()).expect("observer");
    emu.load_state_file(state).expect("state");
    let t0 = emu.frame();
    let mut menus = vec![];
    let mut was = false;
    let mut n = 0u32;
    while obs.in_battle(&mut emu) || n < 30 {
        let m = obs.battle_choosing_actions(&mut emu);
        if m && !was {
            menus.push(emu.frame() - t0);
        }
        was = m;
        emu.step(if n.is_multiple_of(2) { keys::A } else { 0 });
        n += 1;
        if n > 20000 {
            break;
        }
    }
    let over = emu.frame() - t0;
    let mut j = 0u32;
    while !obs.player_can_step(&mut emu) && j < 3000 {
        emu.step(if j.is_multiple_of(2) { keys::A } else { 0 });
        j += 1;
    }
    println!(
        "trainer battle: menu frames {menus:?}, battle over at {over}, free at {}",
        emu.frame() - t0
    );
    let mut prev = 0;
    for m in &menus {
        println!("  menu at {m} (+{})", m - prev);
        prev = *m;
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--trainer") {
        trainer(&PathBuf::from(&args[2]));
        return;
    }
    let state = PathBuf::from(&args[1]);
    let trials: u32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(20);
    let png_dir = args.get(3).map(PathBuf::from);
    let rom = default_rom_path().expect("rom");
    let mut emu = Emu::new(&rom).expect("emu");
    boot_with_default_bios(&mut emu).expect("bios");
    let sym_path = default_sym_path().expect("sym");
    let g_battle_mons = SymbolTable::load(&sym_path)
        .unwrap()
        .get("gBattleMons")
        .unwrap()
        .addr;
    let obs = Observer::new(SymbolTable::load(&sym_path).unwrap()).expect("observer");
    emu.load_state_file(&state).expect("state");
    emu.idle(260);
    let base = emu.save_state().expect("save");
    let mut c = Ctx {
        emu,
        obs,
        g_battle_mons,
    };

    let (mut intro, mut throw_cycle, mut catch_end, mut flee_ok, mut flee_fail) =
        (vec![], vec![], vec![], vec![], vec![]);
    let (mut turn_cycle, mut kill_cycle) = (vec![], vec![]);
    for trial in 0..trials {
        c.emu.load_state(&base).expect("load");
        c.emu.idle(trial * 7);
        let mut battle_at = None;
        'walk: for leg in 0..80u32 {
            let (dir, steps) = match leg {
                0 => (keys::UP, 3),
                1 => (keys::LEFT, 8),
                2 => (keys::UP, 3),
                l if l % 2 == 1 => (keys::LEFT, 3),
                _ => (keys::RIGHT, 3),
            };
            for _ in 0..16 * steps {
                c.emu.step(dir);
                if c.obs.in_battle(&mut c.emu) {
                    battle_at = Some(c.emu.frame());
                    break 'walk;
                }
            }
        }
        let Some(t0) = battle_at else { continue };
        c.mash(keys::A, 3000, |e, o| o.battle_choosing_actions(e));
        intro.push(c.emu.frame() - t0);
        let enemy = c.g_battle_mons + 0x58;
        if trial % 4 == 3 {
            // Fight: a turn that does not kill (enemy HP 200), then a kill (HP 1).
            let t1 = c.emu.frame();
            c.emu.write8(enemy + 0x28, 200);
            c.emu.write8(enemy + 0x29, 0);
            c.emu.write8(enemy + 0x2C, 200);
            c.mash(keys::A, 3000, |e, o| {
                !o.in_battle(e) || (o.battle_choosing_actions(e) && e.frame() > t1 + 30)
            });
            if c.obs.in_battle(&mut c.emu) {
                turn_cycle.push(c.emu.frame() - t1);
                let t2 = c.emu.frame();
                c.emu.write8(enemy + 0x28, 1);
                c.emu.write8(enemy + 0x29, 0);
                c.mash(keys::A, 3000, |e, o| !o.in_battle(e));
                kill_cycle.push(c.emu.frame() - t2);
            }
        } else if trial % 2 == 0 {
            // Catch: one HP left, so the 255-rate forest bugs are caught on the first ball.
            c.emu.write8(enemy + 0x28, 1);
            c.emu.write8(enemy + 0x29, 0);
            let t1 = c.emu.frame();
            c.tap(keys::RIGHT); // Bag
            c.mash(keys::A, 3000, |e, o| {
                !o.in_battle(e) || (o.battle_choosing_actions(e) && e.frame() > t1 + 30)
            });
            if !c.obs.in_battle(&mut c.emu) {
                throw_cycle.push(c.emu.frame() - t1);
                let t2 = c.emu.frame();
                c.mash(keys::B, 3000, |e, o| o.player_can_step(e));
                catch_end.push(c.emu.frame() - t2);
            }
        } else {
            // Run: bottom-right, retried until it works.
            for _ in 0..6 {
                let t1 = c.emu.frame();
                c.tap(keys::DOWN);
                c.tap(keys::RIGHT);
                c.mash(keys::A, 3000, |e, o| {
                    !o.in_battle(e) || (o.battle_choosing_actions(e) && e.frame() > t1 + 30)
                });
                if !c.obs.in_battle(&mut c.emu) {
                    c.mash(keys::A, 3000, |e, o| o.player_can_step(e));
                    flee_ok.push(c.emu.frame() - t1);
                    break;
                }
                flee_fail.push(c.emu.frame() - t1);
            }
        }
        if let (Some(d), true) = (&png_dir, trial < 2) {
            let _ = c.emu.write_png(&d.join(format!("t{trial}.png")));
        }
    }
    let show = |name: &str, v: &Vec<u32>| {
        let mut s = v.clone();
        s.sort();
        println!(
            "{name:12} n={:3} mean {:7.1} min {:?} max {:?} {:?}",
            v.len(),
            mean(v),
            s.first(),
            s.last(),
            &s[..s.len().min(8)]
        );
    };
    show("fight-turn", &turn_cycle);
    show("fight-kill", &kill_cycle);
    show("intro", &intro);
    show("throw+catch", &throw_cycle);
    show("catch-end", &catch_end);
    show("flee-ok", &flee_ok);
    show("flee-fail", &flee_fail);
}
