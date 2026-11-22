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
            "party {i}: {} L{} exp {} hp {}/{} status {:#x} held {} moves {}",
            n.species(m.species),
            m.level,
            m.exp,
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
    // Pokédex owned & seen (`struct Pokedex` at SaveBlock2 + 0x18, owned 0x10, seen 0x44,
    // `decompiled/include/global.h:193-204`), and the PC boxes (`gPokemonStoragePtr`,
    // `decompiled/include/pokemon_storage_system.h:44-50`: currentBox, then 14 x 30 BoxPokemon).
    if let Some(sb2) = obs.save_block2(emu) {
        let own = emu.read_bytes(sb2 + 0x28, 49);
        let seen = emu.read_bytes(sb2 + 0x5C, 49);
        let has = |i: usize| (own[i / 8] & seen[i / 8]) & (1 << (i % 8)) != 0;
        let caught = (0..386).filter(|&i| has(i)).count();
        let missing: Vec<String> = (0..151)
            .filter(|&i| !has(i))
            .map(|i| {
                _dex.species
                    .iter()
                    .find(|sp| sp.dex as usize == i + 1)
                    .map_or(format!("#{}", i + 1), |sp| sp.name.clone())
            })
            .collect();
        println!(
            "caught {caught}; Kanto missing {}: {}",
            missing.len(),
            missing.join(" ")
        );
    }
    if let Some(p) = obs.symbols().get("gPokemonStoragePtr") {
        let st = emu.read32(p.addr);
        for b in 0..14u32 {
            let v: Vec<String> = (0..30u32)
                .filter_map(|i| {
                    let a = st + 4 + (b * 30 + i) * 80;
                    if emu.read32(a) == 0 && emu.read32(a + 4) == 0 {
                        None
                    } else {
                        let m = super::ram::read_mon(emu, a);
                        Some(format!(
                            "{i}:{}:L{}",
                            n.species(m.species).trim_start_matches("SPECIES_"),
                            m.level
                        ))
                    }
                })
                .collect();
            if !v.is_empty() {
                println!("box {b}: {}", v.join(" "));
            }
        }
    }
}
