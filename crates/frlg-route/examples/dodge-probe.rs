//! Price the committed run's wild encounters against their alternatives.
//!
//! Replays the ledger's logs from reset (like audit-run) and, for every
//! same-map trail on a map with a wild table, reports:
//!
//! - what actually happened: steps, land-encounter (grass) steps, rate
//!   tests consumed, battles fought, frames spent;
//! - what the planner says of the same leg from the same entry state: the
//!   best plan at the honest `ENCOUNTER_COST`, and the cheapest *clean*
//!   plan (`FORBID_ENCOUNTERS`) -- the price of dodging instead of fleeing.
//!
//! Read-only: nothing is written, the committed logs are the input.
//!
//! Usage: dodge-probe <ledger.json>

use frlg_route::observe::{Observer, WildData};
use frlg_route::plan::{self, PlanRequest, StepKind};
use frlg_route::world::World;
use std::collections::HashSet;

struct Trail {
    seg: String,
    map: (u8, u8),
    entry_frame: u32,
    exit_frame: u32,
    entry: (i16, i16),
    exit: (i16, i16),
    wild_at_entry: WildData,
    steps: u32,
    grass_steps: u32,
    rate_tests: u32,
    battle_frames: u32,
    battles: Vec<(u32, bool)>, // (start frame, trainer?)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger_path = std::env::args().nth(1).expect("ledger.json path");
    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let obs = Observer::new(frlg_emu::SymbolTable::load(&sym)?).map_err(std::io::Error::other)?;
    let mut world = World::load()?;
    let version = frlg_route::segments::Version::of_rom(&rom)
        .ok()
        .flatten()
        .unwrap_or(frlg_route::segments::Version::FireRed);

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;

    let mut seg_of = Vec::new();
    let mut logs = Vec::new();
    for seg in &ledger.segments {
        let bytes = std::fs::read(&seg.log)?;
        let log = frlg_emu::InputLog::decode(&bytes)?;
        for _ in 0..log.frames.len() {
            seg_of.push(seg.name.clone());
        }
        logs.push(log);
    }

    let mut trails: Vec<Trail> = Vec::new();
    let mut cur: Option<Trail> = None;
    let mut prev_pos: Option<(i16, i16)> = None;
    let mut wild_prev: u32 = 0;
    let mut in_battle_prev = false;
    let mut battle_start = 0u32;

    let mut frame_abs = 0u32;
    for log in &logs {
        for &keys in &log.frames {
            emu.step(keys);
            let seg = &seg_of[frame_abs as usize];
            let map = obs.map(&mut emu);
            let pos = obs.pos(&mut emu);
            let ib = obs.in_battle(&mut emu);
            let wd = obs.wild_data(&mut emu);

            if let (Some(m), Some(p)) = (map, pos) {
                let switch = cur.as_ref().is_none_or(|t| t.map != m);
                if switch {
                    if let Some(t) = cur.take() {
                        trails.push(t);
                    }
                    cur = Some(Trail {
                        seg: seg.clone(),
                        map: m,
                        entry_frame: frame_abs,
                        exit_frame: frame_abs,
                        entry: p,
                        exit: p,
                        wild_at_entry: wd,
                        steps: 0,
                        grass_steps: 0,
                        rate_tests: 0,
                        battle_frames: 0,
                        battles: Vec::new(),
                    });
                    prev_pos = Some(p);
                }
                let t = cur.as_mut().unwrap();
                t.exit = p;
                t.exit_frame = frame_abs;
                if !ib {
                    if let Some(pp) = prev_pos {
                        if pp != p {
                            t.steps += 1;
                            if let Ok(data) = world.map(m) {
                                if data.tile(p.0, p.1).is_some_and(|tile| tile.land) {
                                    t.grass_steps += 1;
                                }
                            }
                        }
                    }
                    prev_pos = Some(p);
                }
                if wd.rng_state != wild_prev && frame_abs > 0 {
                    t.rate_tests += 1;
                }
                if ib && !in_battle_prev {
                    battle_start = frame_abs;
                    let trainer = obs.battle_type_flags(&mut emu)
                        & frlg_route::observe::BATTLE_TYPE_TRAINER
                        != 0;
                    t.battles.push((frame_abs, trainer));
                }
                if !ib && in_battle_prev {
                    t.battle_frames += frame_abs - battle_start;
                }
            }
            wild_prev = wd.rng_state;
            in_battle_prev = ib;
            frame_abs += 1;
        }
    }
    if let Some(t) = cur.take() {
        trails.push(t);
    }

    for t in &trails {
        let table = frlg_route::brock::wild_table(t.map, version);
        let name = world
            .map_name(t.map)
            .map(str::to_string)
            .unwrap_or_else(|| format!("{:?}", t.map));
        if table.is_none() && t.grass_steps == 0 {
            continue;
        }
        println!(
            "== {} {} ({},{}) f{}..f{} entry {:?} exit {:?}",
            t.seg, name, t.map.0, t.map.1, t.entry_frame, t.exit_frame, t.entry, t.exit
        );
        println!(
            "   walked {} steps ({} grass), {} rate tests, {} battles ({} battle frames), {} frames total",
            t.steps,
            t.grass_steps,
            t.rate_tests,
            t.battles.len(),
            t.battle_frames,
            t.exit_frame - t.entry_frame,
        );
        for &(f, trainer) in &t.battles {
            println!(
                "   battle at f{f} ({})",
                if trainer { "trainer" } else { "wild" }
            );
        }
        let Some(table) = table else {
            continue;
        };
        if t.entry == t.exit {
            continue;
        }
        let Ok(data) = world.map(t.map) else { continue };
        for (label, cost) in [
            ("honest", plan::ENCOUNTER_COST),
            ("clean-only", plan::FORBID_ENCOUNTERS),
        ] {
            let req = PlanRequest {
                map: data,
                wild: Some(table),
                start: t.entry,
                wild_data: t.wild_at_entry,
                targets: vec![t.exit],
                blocked: HashSet::new(),
                encounter_cost: cost,
                test_bias: 0,
            };
            match plan::plan(&req) {
                None => println!("   plan[{label}]: no path"),
                Some((steps, c)) => {
                    let flees = steps
                        .iter()
                        .filter(|s| {
                            matches!(
                                s.kind,
                                StepKind::Consume {
                                    fated_pass: true,
                                    ..
                                }
                            )
                        })
                        .count();
                    let grass = steps
                        .iter()
                        .filter(|s| !matches!(s.kind, StepKind::Free | StepKind::Jump))
                        .count();
                    let net = if cost == plan::FORBID_ENCOUNTERS {
                        c.saturating_sub(flees as u32 * plan::FORBID_ENCOUNTERS)
                    } else {
                        c
                    };
                    println!(
                        "   plan[{label}]: {} steps ({} grass-checked), {} planned encounters, model cost {} (walk part {})",
                        steps.len(),
                        grass,
                        flees,
                        c,
                        net.saturating_sub(flees as u32 * if cost == plan::ENCOUNTER_COST { 600 } else { 0 }),
                    );
                }
            }
        }
    }
    Ok(())
}
