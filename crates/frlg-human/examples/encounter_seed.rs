//! Encounter-seed tools (journal `human-run/journal/2026-11-27-*-encounter-seed.md`).
//!
//!     encounter_seed title <rom> <sym> <lo> <hi> <threads> > title.tsv
//!         Power-on with **no input** until one A tap on frame `d` (absolute frames from reset,
//!         `lo <= d < hi`), then run to the title exit. Prints `d  ready  tid  wild_state`:
//!         `ready` 1 if the title was reading A on that frame (`Task_TitleScreenMain` scene 3 /
//!         state 1, `decompiled/src/title_screen.c:443-444,615-653`), `tid` = `gTrainerId` = the TM1
//!         value of `SeedRngAndSetTrainerId` (`src/main.c:264-269`), `wild_state` =
//!         `sWildEncounterData.rngState` after `SeedWildEncounterRng(Random())`
//!         (`src/new_game.c:103`, `src/wild_encounter.c:661-664`).
//!     encounter_seed trace <rom> <sym> <run.ilog> <end> <out.tsv> [<state> <start>]
//!         Replays a played run's log from reset and writes one row per completed player step
//!         (map or tile change) and per battle start: frame, map, x, y, land, behavior, the wild
//!         data (`wild_encounter.c:24-32`), `gRngValue`, battle type flags.
//!     encounter_seed poke <rom> <sym> <state_in> <state_out> <wild_state>
//!         Writes `sWildEncounterData.rngState` in a savestate (validation only: plays the path
//!         from a checkpoint as if the title press had produced `wild_state`).
use std::io::Write;
use std::path::Path;

use frlg_emu::{keys, Emu, InputLog, SymbolTable};
use frlg_route::world::World;
use frlg_route::Observer;

fn title_ready(e: &mut Emu, syms: &SymbolTable, tasks: u32) -> Option<u16> {
    for i in 0..16u32 {
        let base = tasks + i * 40;
        if e.read8(base + 4) != 0
            && matches!(syms.covering(e.read32(base)), Some((n, _)) if n == "Task_TitleScreenMain")
        {
            return Some(e.read16(base + 8) * 16 + e.read16(base + 10));
        }
    }
    None
}

fn boot(rom: &Path) -> Emu {
    let mut emu = Emu::new(rom).expect("rom");
    frlg_emu::boot_with_default_bios(&mut emu).expect("bios");
    emu.reset();
    emu
}

fn title(a: &[String]) {
    let rom = Path::new(&a[0]);
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let (lo, hi): (usize, usize) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let threads: usize = a[4].parse().unwrap();
    let tasks = syms.get("gTasks").unwrap().addr;
    let tid = syms.get("gTrainerId").unwrap().addr;
    let wild = syms.get("sWildEncounterData").unwrap().addr;
    let cb2 = syms.get("gMain").unwrap().addr + 4;
    let chunk = (hi - lo).div_ceil(threads);
    let rows: Vec<Vec<String>> = std::thread::scope(|sc| {
        let hs: Vec<_> = (0..threads)
            .map(|t| {
                let syms = &syms;
                sc.spawn(move || {
                    let a0 = lo + t * chunk;
                    let a1 = (a0 + chunk).min(hi);
                    let mut out = vec![];
                    if a0 >= a1 {
                        return out;
                    }
                    let mut e = boot(rom);
                    e.idle(a0 as u32);
                    for d in a0..a1 {
                        let st = e.save_state().unwrap();
                        let ready = title_ready(&mut e, syms, tasks) == Some(3 * 16 + 1);
                        e.step(keys::A);
                        let mut wst = 0u32;
                        let mut tv = 0u16;
                        if ready {
                            // Until the title's CB2 is gone (the seed is read on the fade's end).
                            for _ in 0..600 {
                                let f = e.read32(cb2);
                                if !matches!(syms.covering(f), Some((n, _)) if n == "CB2_TitleScreenRun")
                                {
                                    break;
                                }
                                e.step(0);
                            }
                            wst = e.read32(wild);
                            tv = e.read16(tid);
                        }
                        out.push(format!("{d}\t{}\t{tv}\t{wst}", ready as u8));
                        e.load_state(&st).unwrap();
                        e.step(0);
                    }
                    out
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut o = std::io::stdout().lock();
    for r in rows.into_iter().flatten() {
        writeln!(o, "{r}").unwrap();
    }
}

fn trace(a: &[String]) {
    let rom = Path::new(&a[0]);
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let wild = syms.get("sWildEncounterData").unwrap().addr;
    let obs = Observer::new(syms.clone()).expect("obs");
    let log = InputLog::decode(&std::fs::read(&a[2]).expect("ilog")).expect("ilog");
    let end: usize = a[3].parse().unwrap();
    let mut out = std::io::BufWriter::new(std::fs::File::create(&a[4]).expect("out"));
    writeln!(
        out,
        "frame\tmap\tx\ty\tland\tbehavior\twild_rng\tprev_beh\tbuff\tsteps\tgrng\tbattle\tbflags"
    )
    .unwrap();
    let mut world = World::load().expect("world");
    let mut e = boot(rom);
    // Optional: start from a savestate taken at log frame `start` (a poked checkpoint).
    let start: usize = if a.len() >= 7 {
        e.load_state_file(Path::new(&a[5])).expect("state");
        a[6].parse().unwrap()
    } else {
        0
    };
    let mut last: Option<((u8, u8), (i16, i16))> = None;
    let mut last_wild = (0u32, 0u16, 0u16, 0u8);
    let mut was_battle = false;
    for (i, &k) in log.frames.iter().enumerate().take(end).skip(start) {
        e.step(k);
        if obs.player_avatar_flags(&mut e) == 0 {
            continue;
        }
        let (Some(m), Some(p)) = (obs.map(&mut e), obs.pos(&mut e)) else {
            continue;
        };
        let w = (
            e.read32(wild),
            e.read16(wild + 4),
            e.read16(wild + 6),
            e.read8(wild + 8),
        );
        let b = obs.in_battle(&mut e);
        let started = b && !was_battle;
        was_battle = b;
        if Some((m, p)) != last || w != last_wild || started {
            let (land, beh) = world
                .map(m)
                .ok()
                .and_then(|md| md.tile(p.0, p.1))
                .map_or((0, 0), |t| (t.land as u8, t.behavior));
            let name = world.map_name(m).unwrap_or("?").to_string();
            writeln!(
                out,
                "{}\t{name}\t{}\t{}\t{land}\t{beh}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                i + 1,
                p.0,
                p.1,
                w.0,
                w.1,
                w.2,
                w.3,
                obs.rng(&mut e),
                b as u8,
                if started {
                    obs.battle_type_flags(&mut e)
                } else {
                    0
                }
            )
            .unwrap();
            last = Some((m, p));
            last_wild = w;
        }
    }
}

fn poke(a: &[String]) {
    let syms = SymbolTable::load(Path::new(&a[1])).expect("sym");
    let wild = syms.get("sWildEncounterData").unwrap().addr;
    let v: u32 = a[4].parse().unwrap();
    let mut e = boot(Path::new(&a[0]));
    e.load_state_file(Path::new(&a[2])).expect("state");
    for (i, b) in v.to_le_bytes().iter().enumerate() {
        e.write8(wild + i as u32, *b);
    }
    assert_eq!(e.read32(wild), v);
    e.save_state_file(Path::new(&a[3])).expect("save");
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match a.first().map(String::as_str) {
        Some("title") => title(&a[1..]),
        Some("trace") => trace(&a[1..]),
        Some("poke") => poke(&a[1..]),
        _ => {
            eprintln!("usage: encounter_seed title|trace ... (see the module doc)");
            std::process::exit(2);
        }
    }
}
