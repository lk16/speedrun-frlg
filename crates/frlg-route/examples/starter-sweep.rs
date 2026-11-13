//! Starter-manipulation sweep: what nature/IVs does the starter get as a
//! function of the two dials a human has (docs/tricks/starter-manip.md).
//!
//! Dial N: idle frames on the player naming screen before the final OK press.
//!   TM1 is restarted on naming-screen entry and read at exit, which reseeds
//!   `gRngValue` (`decompiled/src/naming_screen.c:427-428,722`,
//!   `decompiled/src/main.c:259-269`). Every other frame of the route is the
//!   committed `frlg-route` pipeline (names, options, house, to-lab, lab walk).
//! Dial j: idle frames facing the ball before the A that runs `givemon`.
//!
//! Usage: starter-sweep <out.tsv> <n_lo> <n_hi> <j_max> <j_stride_for_N> [seed_delay] [threads] [starter]
//!   For every N in n_lo..n_hi (exclusive) it records the j=0 genome; for N with
//!   (N % j_stride == 0) it records the genome for every j in 0..=j_max.
//! Output rows (TSV): N val rng_at_ball ball_frames j pid hp atk def spe spa spd frames m
//!   val = gTrainerId (the u16 TM1 value the naming exit seeded with)
//!   m   = smallest m with gift_mon(Rng(rng_at_ball).jump(m)) == created genome (-1: none <= 700)

use frlg_emu::keys;
use frlg_mon::create::gift_mon;
use frlg_rng::Rng;
use frlg_route::observe::VAR_OAKS_LAB_SCENE;
use frlg_route::record::Feed;
use frlg_route::segments::{self, Starter, Tuning, Version};
use frlg_route::{nav::Goal, Observer, Recorder};
use std::io::Write;
use std::sync::{Arc, Mutex};

const SUBSTRUCT3_SLOT: [u32; 24] = [
    3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 3, 2, 3, 2, 1, 1, 0, 0, 0, 0, 0, 0,
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().collect();
    let out = a.get(1).expect("out.tsv").clone();
    let n_lo: usize = a[2].parse()?;
    let n_hi: usize = a[3].parse()?;
    let j_max: usize = a[4].parse()?;
    let j_stride: usize = a[5].parse()?;
    let seed_delay: usize = a.get(6).map(|s| s.parse().unwrap()).unwrap_or(0);
    let threads: usize = a.get(7).map(|s| s.parse().unwrap()).unwrap_or(12);
    let starter = match a.get(8).map(String::as_str).unwrap_or("squirtle") {
        "bulbasaur" => Starter::Bulbasaur,
        "charmander" => Starter::Charmander,
        _ => Starter::Squirtle,
    };
    let rom = std::env::var("FRLG_ROM")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("FRLG_ARTIFACTS").unwrap())
                .join("rom/pokefirered.gba")
        });
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let syms = frlg_emu::SymbolTable::load(&sym)?;
    let version = Version::of_rom(&rom)?.ok_or("not FR/LG")?;
    let party = syms.get("gPlayerParty").ok_or("gPlayerParty")?.addr;
    let trainer_id = syms.get("gTrainerId").ok_or("gTrainerId")?.addr;
    let tuning = Tuning {
        seed_delay,
        ..Tuning::default()
    };

    let next = Arc::new(Mutex::new(n_lo));
    let rows = Arc::new(Mutex::new(Vec::<(usize, Vec<String>)>::new()));
    let mut hs = vec![];
    for _ in 0..threads {
        let (next, rows, rom, syms) = (next.clone(), rows.clone(), rom.clone(), syms.clone());
        hs.push(std::thread::spawn(move || -> Result<(), String> {
            let obs = Observer::new(syms).map_err(|e| e.to_string())?;
            let e2s = |e: frlg_route::RouteError| e.to_string();
            let segs = segments::all(version, starter, tuning);
            let seg = |name: &str| segs.iter().find(|s| s.name == name).unwrap();
            // Checkpoint: naming screen up.
            let mut rec = Recorder::from_reset(&rom).map_err(e2s)?;
            for n in ["01-boot", "02-intro-oak"] {
                (seg(n).run)(&mut rec, &obs).map_err(e2s)?;
            }
            let base_state = rec.save_state().map_err(e2s)?;
            let base_frames = rec.frames();
            // Path cache: the walk searches (05-house, 06-to-lab) are the expensive
            // part and the route does not depend on the seed unless an NPC blocks
            // it, so the first N's inputs are replayed and re-checked (`reached`).
            let mut cache: Vec<Vec<u16>> = vec![];
            loop {
                let n = {
                    let mut g = next.lock().unwrap();
                    let n = *g;
                    *g += 1;
                    n
                };
                if n >= n_hi {
                    break;
                }
                let mut rec = Recorder::from_state(&rom, &base_state).map_err(e2s)?;
                segments::set_naming_delay(n);
                let mut fail = None;
                let mut fresh: Vec<Vec<u16>> = vec![];
                for (idx, name) in ["03-names", "04-options", "05-house", "06-to-lab"].iter().enumerate() {
                    let start = rec.frames();
                    let mut done = false;
                    if idx >= 2 && cache.len() > idx {
                        let ck = rec.save_state().map_err(e2s)?;
                        rec.play(&cache[idx]).map_err(e2s)?;
                        if (seg(name).reached)(&obs, rec.emu()) {
                            done = true;
                        } else {
                            rec.emu().load_state(&ck).map_err(|e| e.to_string())?;
                        }
                    }
                    if !done {
                        if let Err(e) = (seg(name).run)(&mut rec, &obs) {
                            fail = Some(format!("{name}: {e}"));
                            break;
                        }
                    }
                    let log = rec.log().frames;
                    fresh.push(log[start..].to_vec());
                }
                if fail.is_none() && cache.len() < 4 {
                    cache = fresh;
                }
                if let Some(f) = fail {
                    rows.lock().unwrap().push((n, vec![format!("{n}\tFAIL\t{f}")]));
                    continue;
                }
                let val = rec.emu().read16(trainer_id);
                // Lab: Oak's offer, walk to the ball, face it (07-starter's prefix).
                let r = (|| -> Result<(), frlg_route::RouteError> {
                    rec.hold_mash_until("Oak's offer", keys::A, 1, 4000, |emu| {
                        obs.var(emu, VAR_OAKS_LAB_SCENE) == Some(2) && obs.player_can_step(emu)
                    })?;
                    let bx = match starter {
                        Starter::Bulbasaur => 8,
                        Starter::Squirtle => 9,
                        Starter::Charmander => 10,
                    };
                    frlg_route::nav::walk_to(
                        &mut rec,
                        &obs,
                        Goal::tile(segments::OAKS_LAB, bx, 5),
                        4000,
                    )?;
                    rec.wait_until("settle", 240, |emu| obs.player_can_step(emu))?;
                    rec.hold(keys::UP, tuning.turn_hold)?;
                    rec.idle(1)?;
                    Ok(())
                })();
                if let Err(e) = r {
                    rows.lock().unwrap().push((n, vec![format!("{n}\t{val}\tFAIL\tlab: {e}")]));
                    continue;
                }
                let ball_frames = rec.frames() - base_frames;
                let rng0 = obs.rng(rec.emu());
                let s_ball = rec.save_state().map_err(e2s)?;
                let js: Vec<usize> = if n % j_stride == 0 {
                    (0..=j_max).collect()
                } else {
                    vec![0]
                };
                let mut out_rows = vec![];
                for j in js {
                    rec.emu().load_state(&s_ball).map_err(|e| e.to_string())?;
                    rec.idle(j).map_err(e2s)?;
                    let before = rec.frames();
                    if rec
                        .hold_mash_until("party", keys::A, 1, 1200, |emu| obs.party_count(emu) == 1)
                        .is_err()
                    {
                        out_rows.push(format!("{n}\t{val}\t{rng0:#x}\t{ball_frames}\t{j}\tFAIL"));
                        continue;
                    }
                    let frames = rec.frames() - before;
                    let emu = rec.emu();
                    let pid = emu.read32(party);
                    let key = pid ^ emu.read32(party + 4);
                    let slot = SUBSTRUCT3_SLOT[(pid % 24) as usize];
                    let w = emu.read32(party + 0x20 + slot * 12 + 4) ^ key;
                    let iv = |s: u32| (w >> s) & 0x1F;
                    let g = (pid, [iv(0), iv(5), iv(10), iv(15), iv(20), iv(25)]);
                    // Which stream offset from rng_at_ball reproduces it?
                    let mut m: i64 = -1;
                    let mut s = Rng(rng0);
                    for k in 0..700 {
                        let mut t = s;
                        let c = gift_mon(&mut t);
                        if c.pid == g.0
                            && [c.ivs.hp, c.ivs.atk, c.ivs.def, c.ivs.spe, c.ivs.spa, c.ivs.spd]
                                .map(u32::from)
                                == g.1
                        {
                            m = k;
                            break;
                        }
                        s = s.next();
                    }
                    out_rows.push(format!(
                        "{n}\t{val}\t{rng0:#x}\t{ball_frames}\t{j}\t{:#010x}\t{}\t{}\t{}\t{}\t{}\t{}\t{frames}\t{m}",
                        g.0, g.1[0], g.1[1], g.1[2], g.1[3], g.1[4], g.1[5]
                    ));
                }
                rows.lock().unwrap().push((n, out_rows));
                eprintln!("N={n} done");
            }
            Ok(())
        }));
    }
    for h in hs {
        h.join().unwrap()?;
    }
    let mut rows = rows.lock().unwrap();
    rows.sort_by_key(|r| r.0);
    let mut f = std::fs::File::create(&out)?;
    for (_, rs) in rows.iter() {
        for r in rs {
            writeln!(f, "{r}")?;
        }
    }
    Ok(())
}
