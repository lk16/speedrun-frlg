//! Audit a committed route: replay the ledger's logs from reset and report
//! everything a human watching the movie would flag -- battle episodes (which
//! are trainer fights, which are wild flees), steps that immediately undo the
//! previous step, and stretches where the player stands free doing nothing.
//!
//! Usage: audit-run <ledger.json>
//!
//! Read-only: nothing is written, the committed logs are the input.

use frlg_route::observe::Observer;
use std::collections::HashMap;

#[derive(Debug)]
struct Battle {
    start: u32,
    end: u32,
    seg: String,
    trainer: bool,
    foe_species: u16,
    foe_level: u8,
    /// atk/def/spe/spa/spd, `struct BattlePokemon`
    /// (`decompiled/include/pokemon.h:170`: u16s right after species).
    foe_stats: [u16; 5],
    our_stats: [u16; 5],
    our_level: u8,
    our_hp: (u16, u16),
    /// (frame, side 0=us/1=foe, hp before, hp after): every HP write seen.
    hp_events: Vec<(u32, u8, u16, u16)>,
    outcome: u8,
    wild_state_at_start: u32,
    steps_since: u8,
}

fn species_name(id: u16) -> String {
    // `decompiled/include/constants/species.h` -- gen-1 internal ids equal
    // national dex numbers for everything this route can meet.
    match id {
        1 => "Bulbasaur".into(),
        4 => "Charmander".into(),
        7 => "Squirtle".into(),
        10 => "Caterpie".into(),
        11 => "Metapod".into(),
        13 => "Weedle".into(),
        14 => "Kakuna".into(),
        16 => "Pidgey".into(),
        19 => "Rattata".into(),
        25 => "Pikachu".into(),
        74 => "Geodude".into(),
        95 => "Onix".into(),
        n => format!("species#{n}"),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ledger_path = std::env::args().nth(1).expect("ledger.json path");
    let ledger = frlg_route::ledger::read(std::path::Path::new(&ledger_path))?;
    // The ledger names its logs repo-relative; run from the repository root.
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).ok_or("rom for ledger sha1")?;
    let sym = frlg_emu::sym_path_for_rom(&rom).ok_or("sym for rom")?;
    let obs = Observer::new(frlg_emu::SymbolTable::load(&sym)?).map_err(std::io::Error::other)?;

    let mut emu = frlg_emu::Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;

    // Segment name per absolute frame, for labelling findings.
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

    let mut battles: Vec<Battle> = Vec::new();
    let mut in_battle_prev = false;
    let mut pending: Option<Battle> = None;

    // Movement traces.
    let mut prev_pos: Option<(u8, u8, i16, i16)> = None;
    // (frame, map, from-tile, to-tile) per step.
    type Step = (u32, (u8, u8), (i16, i16), (i16, i16));
    let mut steps: Vec<Step> = Vec::new();
    // Free-idle runs: player_can_step && !locked && !in_battle && standing still.
    let mut idle_run_start: Option<u32> = None;
    let mut idle_runs: Vec<(u32, u32, String)> = Vec::new();
    // Wild rate tests: the second LCG advances only per rate test.
    let mut wild_prev = 0u32;
    let mut rate_tests: HashMap<String, u32> = HashMap::new();

    let mut frame_abs = 0u32;
    for log in &logs {
        for &keys in &log.frames {
            emu.step(keys);
            let seg = &seg_of[frame_abs as usize];

            let ib = obs.in_battle(&mut emu);
            if ib && !in_battle_prev {
                let wd = obs.wild_data(&mut emu);
                pending = Some(Battle {
                    start: frame_abs,
                    end: 0,
                    seg: seg.clone(),
                    trainer: false,
                    foe_species: 0,
                    foe_level: 0,
                    outcome: 0,
                    wild_state_at_start: wd.rng_state,
                    steps_since: wd.steps_since,
                    foe_stats: [0; 5],
                    our_stats: [0; 5],
                    our_level: 0,
                    our_hp: (0, 0),
                    hp_events: Vec::new(),
                });
            }
            if ib {
                if let Some(b) = pending.as_mut() {
                    // Battle mons load a few frames in; keep the latest nonzero.
                    let foe = obs.battle_mon(&mut emu, 1);
                    if foe.species != 0 {
                        b.foe_species = foe.species;
                        b.foe_level = foe.level;
                        b.foe_stats = foe.stats;
                    }
                    let us = obs.battle_mon(&mut emu, 0);
                    if us.species != 0 && b.our_level == 0 {
                        // First sight only: start-of-battle level and HP.
                        b.our_stats = us.stats;
                        b.our_level = us.level;
                        b.our_hp = (us.hp, us.max_hp);
                    }
                    // HP writes, both sides, keyed off the last event's
                    // after-value (start values are the first "before").
                    for (side, mon) in [(0u8, &us), (1u8, &foe)] {
                        if mon.species == 0 {
                            continue;
                        }
                        let prev = b
                            .hp_events
                            .iter()
                            .rev()
                            .find(|e| e.1 == side)
                            .map(|e| e.3)
                            .unwrap_or(if side == 0 { b.our_hp.0 } else { mon.max_hp });
                        if mon.hp != prev {
                            b.hp_events.push((frame_abs, side, prev, mon.hp));
                        }
                    }
                    b.trainer = obs.battle_type_flags(&mut emu)
                        & frlg_route::observe::BATTLE_TYPE_TRAINER
                        != 0;
                }
            }
            if !ib && in_battle_prev {
                if let Some(mut b) = pending.take() {
                    b.end = frame_abs;
                    b.outcome = obs.battle_outcome(&mut emu);
                    battles.push(b);
                }
            }
            in_battle_prev = ib;

            // Steps.
            if let (Some((g, n)), Some((x, y))) = (obs.map(&mut emu), obs.pos(&mut emu)) {
                if let Some((pg, pn, px, py)) = prev_pos {
                    if (pg, pn) == (g, n) && (px, py) != (x, y) {
                        steps.push((frame_abs, (g, n), (px, py), (x, y)));
                    }
                }
                prev_pos = Some((g, n, x, y));
            }

            // Idle-free runs (outside battle).
            let free = !ib && obs.player_can_step(&mut emu) && !obs.field_controls_locked(&mut emu);
            if free {
                if idle_run_start.is_none() {
                    idle_run_start = Some(frame_abs);
                }
            } else if let Some(s) = idle_run_start.take() {
                if frame_abs - s >= 8 {
                    idle_runs.push((s, frame_abs, seg.clone()));
                }
            }

            // Wild rate tests.
            let wd = obs.wild_data(&mut emu);
            if wd.rng_state != wild_prev && wild_prev != 0 {
                *rate_tests.entry(seg.clone()).or_default() += 1;
            }
            wild_prev = wd.rng_state;

            frame_abs += 1;
        }
    }

    println!("== battles ({}) ==", battles.len());
    for b in &battles {
        println!(
            "  f{:>6}..{:>6} ({:>5} fr) {:<14} {} {} L{} outcome {} steps_since {}",
            b.start,
            b.end,
            b.end - b.start,
            b.seg,
            if b.trainer { "TRAINER" } else { "WILD   " },
            species_name(b.foe_species),
            b.foe_level,
            b.outcome,
            b.steps_since,
        );
        println!(
            "      us L{} hp {}/{} a/d/s/sa/sd {:?} | foe {:?} | wild {:#010x}",
            b.our_level, b.our_hp.0, b.our_hp.1, b.our_stats, b.foe_stats, b.wild_state_at_start,
        );
        if b.trainer {
            for (f, side, from, to) in &b.hp_events {
                println!(
                    "      f{f} {} hp {from} -> {to} ({}{})",
                    if *side == 0 { "us " } else { "foe" },
                    if to > from { "+" } else { "-" },
                    to.abs_diff(*from),
                );
            }
        }
    }

    // Doglegs: lateral (or vertical) waste per contiguous same-map trail.
    // A trail that takes L left-steps and R right-steps wastes
    // 2*min(L,R) steps against its net displacement (likewise up/down) --
    // the "walks left, then up, then right again" a viewer flags. Waste
    // is not automatically a bug (grass-lane weaving, ledges, NPC cones,
    // rate-test index shaping all force doglegs); this lists where to
    // look, with the trail's tile ranges so each can be checked against
    // `frlg map`.
    println!("\n== direction waste per same-map trail (>= 2 wasted steps) ==");
    let mut i = 0usize;
    while i < steps.len() {
        let map = steps[i].1;
        // A trail ends on a map change or a >600-frame gap (a battle or
        // scripted scene splits the walk).
        let end = (i..steps.len())
            .take_while(|&j| steps[j].1 == map && (j == i || steps[j].0 - steps[j - 1].0 <= 600))
            .last()
            .unwrap()
            + 1;
        let trail = &steps[i..end];
        let (mut l, mut r, mut u, mut d) = (0i32, 0i32, 0i32, 0i32);
        for (_, _, from, to) in trail {
            match (to.0 - from.0, to.1 - from.1) {
                (dx, _) if dx < 0 => l += 1,
                (dx, _) if dx > 0 => r += 1,
                (_, dy) if dy < 0 => u += 1,
                _ => d += 1,
            }
        }
        let waste = 2 * l.min(r) + 2 * u.min(d);
        if waste >= 2 {
            let (f0, _, from0, _) = trail[0];
            let (f1, _, _, to1) = trail[trail.len() - 1];
            println!(
                "  {} map {:?} f{}..{} {:?}->{:?}: {} steps (L{l} R{r} U{u} D{d}), {} wasted (~{} fr)",
                seg_of[f0 as usize],
                map,
                f0,
                f1,
                from0,
                to1,
                trail.len(),
                waste,
                waste * 16,
            );
        }
        i = end;
    }

    // AUDIT_TRAIL="f0-f1": dump every step in the frame window, for
    // checking one flagged trail against the map.
    if let Ok(win) = std::env::var("AUDIT_TRAIL") {
        if let Some((a, b)) = win.split_once('-') {
            let (a, b): (u32, u32) = (a.parse().unwrap_or(0), b.parse().unwrap_or(u32::MAX));
            println!("\n== trail {a}..{b} ==");
            for (f, m, from, to) in steps.iter().filter(|(f, ..)| (a..=b).contains(f)) {
                println!("  f{f} map {m:?} {from:?} -> {to:?}");
            }
        }
    }

    // Reversal steps: step i+1 returns to step i's origin (an undo).
    println!("\n== reversal steps (step that undoes the previous one) ==");
    for w in steps.windows(2) {
        let (f0, m0, from0, _to0) = &w[0];
        let (f1, m1, _from1, to1) = &w[1];
        if m0 == m1 && to1 == from0 && f1 - f0 < 120 {
            println!(
                "  f{f0}->f{f1} {} map {:?}: {:?} -> back to {:?}",
                seg_of[*f1 as usize], m0, _to0, to1
            );
        }
    }

    // Steps within 300 frames after each battle end, with positions --
    // the "steps directly after encounters" the user flagged.
    println!("\n== steps within 300 frames after each battle ==");
    for b in &battles {
        let after: Vec<_> = steps
            .iter()
            .filter(|(f, ..)| *f > b.end && *f <= b.end + 300)
            .collect();
        println!(
            "  after f{} ({} {}): {} steps: {}",
            b.end,
            if b.trainer { "trainer" } else { "wild" },
            species_name(b.foe_species),
            after.len(),
            after
                .iter()
                .map(|(f, _, from, to)| format!("f{f} {from:?}->{to:?}"))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }

    println!("\n== free-idle runs >= 8 frames (player standing, nothing holding it) ==");
    let mut by_seg: HashMap<String, (u32, u32)> = HashMap::new();
    for (s, e, seg) in &idle_runs {
        let ent = by_seg.entry(seg.clone()).or_default();
        ent.0 += 1;
        ent.1 += e - s;
    }
    for seg in ledger.segments.iter().map(|s| &s.name) {
        if let Some((count, total)) = by_seg.get(seg.as_str()) {
            println!("  {seg:<16} {count:>3} runs, {total:>5} frames");
        }
    }
    println!("  (long individual runs)");
    for (s, e, seg) in idle_runs.iter().filter(|(s, e, _)| e - s >= 30) {
        println!("    f{s}..{e} ({} fr) {seg}", e - s);
    }

    println!("\n== wild rate tests per segment ==");
    for seg in ledger.segments.iter().map(|s| &s.name) {
        if let Some(n) = rate_tests.get(seg.as_str()) {
            println!("  {seg:<16} {n}");
        }
    }

    println!("\ntotal steps: {}", steps.len());
    Ok(())
}
