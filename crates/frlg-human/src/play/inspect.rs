//! `frlg-human play-inspect`: what a savestate holds (map, position, party, bag, money).
use std::path::Path;

use frlg_route::{Observer, Recorder};

use super::names::names;
use super::ram::Ram;
use crate::data::Dex;

pub fn print(rom: &Path, sym: &Path, state: &Path, _dex: &Dex) {
    let syms = frlg_emu::SymbolTable::load(sym).expect("symbols");
    let obs = Observer::new(syms).expect("observer");
    let mut rec = Recorder::from_reset(rom).expect("rom");
    rec.emu().load_state_file(state).expect("state");
    let emu = rec.emu();
    let world = frlg_route::world::World::load().ok();
    let map = obs.map(emu);
    let mname = map
        .and_then(|m| {
            world
                .as_ref()
                .and_then(|w| w.map_name(m).map(str::to_string))
        })
        .unwrap_or_default();
    println!("{}", obs.snapshot(emu));
    println!("map {map:?} {mname} pos {:?}", obs.pos(emu));
    println!("ram sha1 {}", emu.ram_hash().unwrap());
    let ram = Ram::new(&obs);
    let n = names();
    for (i, m) in ram.party(emu).iter().enumerate() {
        if !m.valid {
            println!("party {i}: checksum mismatch");
        }
        println!(
            "party {i}: {} L{} hp {}/{} status {:#x} held {} moves {}",
            n.species(m.species),
            m.level,
            m.hp,
            m.max_hp,
            m.status,
            n.item(m.held),
            m.moves
                .iter()
                .zip(m.pp)
                .filter(|(mv, _)| **mv != 0)
                .map(|(mv, pp)| format!("{}({pp})", n.mv(*mv)))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!("money {}", ram.money(emu));
    for p in 0..super::ram::POCKETS.len() {
        let items = ram.pocket(emu, p);
        println!(
            "pocket {p}: {}",
            items
                .iter()
                .map(|(i, q)| format!("{}x{q}", n.item(*i)))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }
    println!(
        "pc items: {:?}",
        ram.pc_items(emu)
            .iter()
            .map(|(i, q)| format!("{}x{q}", n.item(*i)))
            .collect::<Vec<_>>()
    );
    for o in ram.objects(emu) {
        println!(
            "object {} gfx {} at ({}, {})",
            o.local_id, o.graphics, o.x, o.y
        );
    }
    println!("facing {}", ram.player_facing(emu));
}
