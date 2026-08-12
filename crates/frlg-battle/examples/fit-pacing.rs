//! Fit the battle's frame pacing, per semantic event, from emulator runs.
//!
//! Replays the committed route to the end of `08-battle-start`, then runs a
//! training set of delay plans through the exact drive the route's search
//! uses (`run_plan` in frlg-rng's battle-plan-scan). For every run it
//! extracts the logic rolls (the window beyond the 2-per-frame VBlank pair,
//! pair leading -- the established order, `docs/journal.md` 2026-08-12),
//! labels each roll with the v1 semantics from this crate, cross-checks the
//! predicted damage against the observed `gBattleMons` HP writes, and emits
//! (transition key -> frame gap) observations.
//!
//! The output is the evidence for `Pacing::measured()` in `src/pacing.rs`:
//! every key is printed with every gap value seen and how often, so a key
//! with two values is a key whose context is too coarse -- refine it here
//! before trusting it there.
//!
//!     cargo run --release -p frlg-battle --example fit-pacing

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};

use frlg_battle::{apply_variance, base_damage, rival_choose_move, Mon, Move};
use frlg_emu::{keys, Emu, InputLog, SaveState};
use frlg_rng::Rng;
use frlg_route::observe::Observer;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives two directories below the repo root")
        .to_path_buf()
}

/// `B_OUTCOME_WON`, `decompiled/include/constants/battle.h:76`.
const WON: u8 = 1;
/// Same bound the route's search uses.
const FRAME_BUDGET: u32 = 20_000;

/// `struct BattlePokemon` offsets (`decompiled/include/pokemon.h:170-206`).
mod mon_off {
    pub const ATTACK: u32 = 0x02;
    pub const DEFENSE: u32 = 0x04;
    pub const SPEED: u32 = 0x06;
    /// `s8 statStages[]`; index 1 is ATK (`include/constants/battle.h`).
    pub const STAT_STAGES: u32 = 0x18;
    pub const HP: u32 = 0x28;
    pub const LEVEL: u32 = 0x2A;
    pub const MAX_HP: u32 = 0x2C;
    pub const SIZE: u32 = 0x58;
}

fn read_mon(emu: &mut Emu, base: u32, index: u32) -> Mon {
    let a = base + index * mon_off::SIZE;
    Mon {
        hp: emu.read16(a + mon_off::HP),
        max_hp: emu.read16(a + mon_off::MAX_HP),
        attack: emu.read16(a + mon_off::ATTACK),
        defense: emu.read16(a + mon_off::DEFENSE),
        speed: emu.read16(a + mon_off::SPEED),
        level: emu.read8(a + mon_off::LEVEL),
        atk_stage: 6,
        def_stage: 6,
    }
}

/// The frame spans of one turn's two input loops in `run_plan`: the "to
/// action selection" mash (loop A, which also paces the previous turn's
/// resolution texts), and the commit mash (loop B). These are what decide
/// where the press grid sits, so the ±5 splits in input-gated gaps can be
/// resolved against them.
#[derive(Debug, Clone, Copy)]
struct Marks {
    loop_a_start: u32,
    detection: u32,
    loop_b_start: u32,
    loop_b_end: u32,
}

/// Everything one battle run leaves behind, at frame granularity.
struct Trace {
    plan: Vec<u32>,
    shift: i64,
    /// (battle frame, roll value) for every logic roll, in stream order.
    rolls: Vec<(u32, u16)>,
    /// (battle frame, (our hp, rival hp)) for every change after init.
    hp_events: Vec<(u32, (u16, u16))>,
    /// (battle frame, our raw ATK stage) for every change after init.
    stage_events: Vec<(u32, u8)>,
    /// First frame `gBattleOutcome` went nonzero, and its value.
    outcome: Option<(u32, u8)>,
    /// What the search's `run_plan` would have returned.
    won: bool,
    frames: u32,
    budget_exceeded: bool,
    mons: Option<(Mon, Mon)>,
    marks: Vec<Marks>,
}

/// Steps the emulator while extracting rolls/HP/stages, exactly like
/// frlg-rng's battle-truth example does.
struct Recorder<'a> {
    emu: &'a mut Emu,
    observer: &'a Observer,
    mons_base: u32,
    model: Rng,
    frame: u32,
    rolls: Vec<(u32, u16)>,
    hp: (u16, u16),
    hp_events: Vec<(u32, (u16, u16))>,
    stage: u8,
    stage_events: Vec<(u32, u8)>,
    outcome: Option<(u32, u8)>,
    mons: Option<(Mon, Mon)>,
}

impl<'a> Recorder<'a> {
    fn step(&mut self, mask: u16) {
        self.emu.step(mask);
        let observed = Rng(self.observer.rng(self.emu));
        let steps = self.model.distance_to(observed);
        assert!(steps >= 2, "battle frames roll twice in VBlank");
        // The VBlank pair leads the frame's window; the game's own rolls
        // trail (docs/journal.md 2026-08-12, proven by damage arithmetic).
        let mut cursor = self.model.jump(2);
        for _ in 0..steps - 2 {
            cursor = cursor.next();
            self.rolls.push((self.frame, (cursor.0 >> 16) as u16));
        }
        self.model = observed;

        let ours = self.emu.read16(self.mons_base + mon_off::HP);
        let theirs = self
            .emu
            .read16(self.mons_base + mon_off::SIZE + mon_off::HP);
        if (ours, theirs) != self.hp {
            if self.hp == (0, 0) {
                self.mons = Some((
                    read_mon(self.emu, self.mons_base, 0),
                    read_mon(self.emu, self.mons_base, 1),
                ));
            } else if theirs != 0 || self.hp.1 != 0 {
                self.hp_events.push((self.frame, (ours, theirs)));
            }
            self.hp = (ours, theirs);
        }
        let stage = self.emu.read8(self.mons_base + mon_off::STAT_STAGES + 1);
        if self.mons.is_some() && stage != self.stage {
            self.stage_events.push((self.frame, stage));
            self.stage = stage;
        }
        if self.outcome.is_none() {
            let outcome = self.observer.battle_outcome(self.emu);
            if outcome != 0 {
                self.outcome = Some((self.frame, outcome));
            }
        }
        self.frame += 1;
    }
}

/// The route search's `run_plan`, verbatim in control flow (frlg-rng's
/// battle-plan-scan), but stepping through the recorder.
#[allow(clippy::too_many_arguments)]
fn run_plan_recorded(
    emu: &mut Emu,
    observer: &Observer,
    start: &SaveState,
    mons_base: u32,
    rng_addr: u32,
    mash: &[u16],
    state: Rng,
    shift: i64,
    plan: &[u32],
) -> Trace {
    emu.load_state(start).expect("load state");
    for (i, byte) in state.0.to_le_bytes().iter().enumerate() {
        emu.write8(rng_addr + i as u32, *byte);
    }
    let model = Rng(observer.rng(emu));
    let mut rec = Recorder {
        emu,
        observer,
        mons_base,
        model,
        frame: 0,
        rolls: Vec::new(),
        hp: (0, 0),
        hp_events: Vec::new(),
        stage: 6,
        stage_events: Vec::new(),
        outcome: None,
        mons: None,
    };

    let mut frames = 0u32;
    let mut budget_exceeded = false;
    let mut marks: Vec<Marks> = Vec::new();
    for _ in 0..plan.first().copied().unwrap_or(0) {
        rec.step(0);
        frames += 1;
    }
    let mut turns = 0usize;
    let won = loop {
        let loop_a_start = rec.frame;
        let mut mash_phase = 0usize;
        let mut over = false;
        loop {
            rec.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if rec.observer.battle_outcome(rec.emu) != 0
                || rec.observer.battle_choosing_actions(rec.emu)
            {
                break;
            }
            if frames >= FRAME_BUDGET {
                over = true;
                break;
            }
        }
        if over {
            budget_exceeded = true;
            break false;
        }
        let outcome = rec.observer.battle_outcome(rec.emu);
        if outcome != 0 {
            break outcome == WON;
        }
        let detection = rec.frame - 1;
        turns += 1;
        for _ in 0..plan.get(turns).copied().unwrap_or(0) {
            rec.step(0);
            frames += 1;
        }
        let loop_b_start = rec.frame;
        mash_phase = 0;
        loop {
            rec.step(mash[mash_phase % mash.len()]);
            mash_phase += 1;
            frames += 1;
            if rec.observer.battle_outcome(rec.emu) != 0
                || !rec.observer.battle_choosing_actions(rec.emu)
            {
                break;
            }
            if frames >= FRAME_BUDGET {
                over = true;
                break;
            }
        }
        marks.push(Marks {
            loop_a_start,
            detection,
            loop_b_start,
            loop_b_end: rec.frame - 1,
        });
        if over {
            budget_exceeded = true;
            break false;
        }
        let outcome = rec.observer.battle_outcome(rec.emu);
        if outcome != 0 {
            break outcome == WON;
        }
    };

    Trace {
        plan: plan.to_vec(),
        shift,
        rolls: rec.rolls,
        hp_events: rec.hp_events,
        stage_events: rec.stage_events,
        outcome: rec.outcome,
        won,
        frames,
        budget_exceeded,
        mons: rec.mons,
        marks,
    }
}

/// One labeled event: a transition key (with the pacing-relevant context
/// baked in) and the battle frame it happened on.
#[derive(Debug)]
struct Ev {
    key: String,
    frame: u32,
}

/// Walk the trace's roll stream with the v1 semantics and name every roll.
/// Errors instead of guessing when the stream disagrees with the model --
/// a failed label is evidence against v1, and gets printed, not skipped.
fn label(trace: &Trace) -> Result<Vec<Ev>, String> {
    let (mut us, mut rival) = trace.mons.ok_or("gBattleMons never initialised")?;
    let rolls = &trace.rolls;
    let mut ri = 0usize;
    let mut hi = 0usize;
    let mut si = 0usize;
    let mut events: Vec<Ev> = Vec::new();
    let mut crit_enabled = false; // FIRST_BATTLE_MSG_FLAG_INFLICT_DMG
    let mut player_hits = 0u32;
    let mut rival_hits = 0u32;
    let mut growls = 0u32;

    let take = |ri: &mut usize| -> Result<(u32, u16), String> {
        let &(f, v) = rolls.get(*ri).ok_or("roll stream ended early")?;
        *ri += 1;
        Ok((f, v))
    };

    // TryDoEventsBeforeFirstTurn's trailing gRandomTurnNumber
    // (decompiled/src/battle_main.c:2926).
    let (f, _) = take(&mut ri)?;
    events.push(Ev {
        key: format!("preturn d{}", trace.plan.first().copied().unwrap_or(0)),
        frame: f,
    });

    let mut turn = 0usize;
    loop {
        turn += 1;
        if turn > 24 {
            return Err("more than 24 turns".into());
        }
        let delay = trace.plan.get(turn).copied().unwrap_or(0);

        // The AI block: all its rolls land on one frame.
        let mut ai_frames: Vec<u32> = Vec::new();
        let mut err = None;
        let mv = {
            let mut src = || match rolls.get(ri) {
                Some(&(f, v)) => {
                    ri += 1;
                    ai_frames.push(f);
                    v
                }
                None => {
                    err = Some("roll stream ended inside AI block");
                    0
                }
            };
            rival_choose_move(&us, &rival, &mut src)
        };
        if let Some(e) = err {
            return Err(e.into());
        }
        if ai_frames.windows(2).any(|w| w[0] != w[1]) {
            return Err(format!("AI rolls span frames {ai_frames:?}"));
        }
        events.push(Ev {
            key: format!("ai d{delay}"),
            frame: ai_frames[0],
        });

        // Player Tackle: no accuracy roll on this route (the ACC_CURR_MOVE
        // quirk, src/lib.rs), then crit, damage, HP write, secondary.
        let (f, crit_roll) = take(&mut ri)?;
        let crit = crit_roll.is_multiple_of(16) && crit_enabled;
        events.push(Ev {
            key: format!("pcrit d{delay}"),
            frame: f,
        });
        let (f, dmg_roll) = take(&mut ri)?;
        events.push(Ev {
            key: "pdmg".into(),
            frame: f,
        });
        let damage = apply_variance(base_damage(&us, &rival, Move::Tackle, crit), dmg_roll);
        let &(f, hp) = trace
            .hp_events
            .get(hi)
            .ok_or("missing rival HP write for player hit")?;
        hi += 1;
        let expect = (us.hp, rival.hp.saturating_sub(damage as u16));
        if hp != expect {
            return Err(format!(
                "turn {turn}: player hit predicted hp {expect:?}, emulator wrote {hp:?}"
            ));
        }
        // The HP bar drains what the target actually lost, so a kill that
        // overshoots is keyed by the delta, not the computed damage.
        let delta = rival.hp - hp.1;
        rival.hp = hp.1;
        player_hits += 1;
        events.push(Ev {
            key: format!(
                "rhp delta{delta}{}{}",
                if crit { " crit" } else { "" },
                if player_hits == 1 { " first" } else { "" }
            ),
            frame: f,
        });
        crit_enabled = true;
        let (f, _) = take(&mut ri)?;
        events.push(Ev {
            key: format!("psec{}", if crit { " crit" } else { "" }),
            frame: f,
        });
        if rival.hp == 0 {
            let (of, ov) = trace.outcome.ok_or("rival fainted but no outcome")?;
            if ov != WON {
                return Err(format!("rival at 0 HP but outcome {ov}"));
            }
            events.push(Ev {
                key: "outcome-win".into(),
                frame: of,
            });
            break;
        }

        // The rival's move.
        let (f, _acc_roll) = take(&mut ri)?; // both rival moves are 100 acc
        events.push(Ev {
            key: format!("racc {mv:?}"),
            frame: f,
        });
        match mv {
            Move::Growl => {
                growls += 1;
                if us.atk_stage > 0 {
                    us.atk_stage -= 1;
                    let &(f, stage) = trace
                        .stage_events
                        .get(si)
                        .ok_or("missing stage write for Growl")?;
                    si += 1;
                    if stage != us.atk_stage {
                        return Err(format!(
                            "turn {turn}: Growl predicted stage {}, emulator wrote {stage}",
                            us.atk_stage
                        ));
                    }
                    events.push(Ev {
                        key: format!("stagefall{}", if growls == 1 { " first" } else { "" }),
                        frame: f,
                    });
                }
            }
            mv => {
                let (f, crit_roll) = take(&mut ri)?;
                let crit = crit_roll.is_multiple_of(16) && crit_enabled;
                events.push(Ev {
                    key: "rcrit".into(),
                    frame: f,
                });
                let (f, dmg_roll) = take(&mut ri)?;
                events.push(Ev {
                    key: "rdmg".into(),
                    frame: f,
                });
                let damage = apply_variance(base_damage(&rival, &us, mv, crit), dmg_roll);
                let &(f, hp) = trace
                    .hp_events
                    .get(hi)
                    .ok_or("missing our HP write for rival hit")?;
                hi += 1;
                let expect = (us.hp.saturating_sub(damage as u16), rival.hp);
                if hp != expect {
                    return Err(format!(
                        "turn {turn}: rival hit predicted hp {expect:?}, emulator wrote {hp:?}"
                    ));
                }
                let delta = us.hp - hp.0;
                us.hp = hp.0;
                rival_hits += 1;
                events.push(Ev {
                    key: format!(
                        "uhp delta{delta}{}{}",
                        if crit { " crit" } else { "" },
                        if rival_hits == 1 { " first" } else { "" }
                    ),
                    frame: f,
                });
                // The fatal hit still burns the seteffectwithchance roll
                // before the battle ends (observed on every loss run).
                let (f, _) = take(&mut ri)?;
                events.push(Ev {
                    key: format!(
                        "rsec{}{}",
                        if crit { " crit" } else { "" },
                        if us.hp == 0 { " fatal" } else { "" }
                    ),
                    frame: f,
                });
                if us.hp == 0 {
                    let (of, ov) = trace.outcome.ok_or("we fainted but no outcome")?;
                    events.push(Ev {
                        key: format!("outcome-loss v{ov}"),
                        frame: of,
                    });
                    break;
                }
            }
        }

        // BattleTurnPassed's gRandomTurnNumber (battle_main.c:2999).
        let (f, _) = take(&mut ri)?;
        events.push(Ev {
            key: "turnend".into(),
            frame: f,
        });
    }

    if ri != rolls.len() {
        return Err(format!(
            "{} rolls left unexplained: {:?}",
            rolls.len() - ri,
            &rolls[ri..rolls.len().min(ri + 8)]
        ));
    }
    Ok(events)
}

type GapTable = BTreeMap<String, BTreeMap<u32, u32>>;

fn observe_gaps(events: &[Ev], table: &mut GapTable) {
    let mut prev: Option<&Ev> = None;
    for ev in events {
        let (from, gap) = match prev {
            None => ("start".to_string(), ev.frame),
            Some(p) => (
                p.key.split(" d").next().unwrap_or(&p.key).to_string(),
                ev.frame - p.frame,
            ),
        };
        *table
            .entry(format!("{from} -> {}", ev.key))
            .or_default()
            .entry(gap)
            .or_default() += 1;
        prev = Some(ev);
    }
}

fn main() {
    let root = repo_root();
    let ledger =
        frlg_route::ledger::read(&root.join("route/ledger.json")).expect("committed ledger");
    let rom = frlg_emu::rom_path_for_sha1(&ledger.rom_sha1).expect("ROM in $FRLG_ARTIFACTS/rom");
    let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
    let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;

    let mut emu = Emu::new(&rom).expect("core");
    let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
    assert_eq!(boot, ledger.bios);
    for entry in &ledger.segments {
        if entry.name == "09-battle-win" {
            break;
        }
        let bytes = std::fs::read(root.join(&entry.log)).expect("log");
        let log = InputLog::decode(&bytes).expect("log decodes");
        for &mask in &log.frames {
            emu.step(mask);
        }
    }
    let start = emu.save_state().expect("state at battle start");
    let base = Rng(emu.read32(rng_addr));
    drop(emu);
    println!("battle-start gRngValue {:#010x}", base.0);

    let mash: Vec<u16> = {
        let mut m = vec![keys::A; ledger.tuning.text_hold.max(1)];
        m.push(0);
        m
    };

    // The training set: the search's own space, plus stream shifts for
    // vocabulary breadth (more crit patterns, kill deltas, Growl turns).
    // Start delays collapse mod 5 (the mash period) -- verified over the
    // full 0..64 range in an earlier fit -- so two periods suffice.
    let mut jobs_list: Vec<(i64, Vec<u32>)> = Vec::new();
    for d in 0..10 {
        jobs_list.push((0, vec![d]));
        jobs_list.push((0, vec![d, 3, 3, 3]));
    }
    for t in 1..=4 {
        for d in 0..16 {
            let mut plan = vec![4, 3, 3, 3];
            if plan.len() < t + 1 {
                plan.resize(t + 1, 0);
            }
            plan[t] = d;
            jobs_list.push((0, plan));
        }
    }
    for shift in -10i64..=10 {
        if shift == 0 {
            continue;
        }
        for d in 0..5 {
            jobs_list.push((shift, vec![d]));
        }
        jobs_list.push((shift, vec![4, 3, 3, 3]));
    }
    jobs_list.sort();
    jobs_list.dedup();

    let total = jobs_list.len();
    let jobs = Arc::new(Mutex::new(jobs_list));
    struct RunReport {
        shift: i64,
        plan: Vec<u32>,
        won: bool,
        frames: u32,
        budget_exceeded: bool,
        label: Result<(GapTable, Vec<(String, u32)>), String>,
        marks: Vec<Marks>,
    }
    let (tx, rx) = mpsc::channel::<RunReport>();

    let workers = std::thread::available_parallelism()
        .map(|n| n.get().saturating_sub(2).clamp(1, 12))
        .unwrap_or(4);
    for _ in 0..workers {
        let jobs = Arc::clone(&jobs);
        let tx = tx.clone();
        let rom = rom.clone();
        let start: SaveState = start.clone();
        let mash = mash.clone();
        let bios = ledger.bios.clone();
        std::thread::spawn(move || {
            let mut emu = Emu::new(&rom).expect("core");
            let boot = frlg_emu::boot_with_default_bios(&mut emu).expect("boot");
            assert_eq!(boot, bios);
            let syms = frlg_emu::SymbolTable::load(&rom.with_extension("sym")).expect("syms");
            let rng_addr = syms.get("gRngValue").expect("gRngValue").addr;
            let mons_base = syms.get("gBattleMons").expect("gBattleMons").addr;
            let observer = Observer::new(syms).expect("observer");
            loop {
                let (shift, plan) = match jobs.lock().unwrap().pop() {
                    Some(p) => p,
                    None => break,
                };
                let state = if shift >= 0 {
                    base.jump(shift as u32)
                } else {
                    let mut s = base;
                    for _ in 0..-shift {
                        s = s.prev();
                    }
                    s
                };
                let trace = run_plan_recorded(
                    &mut emu, &observer, &start, mons_base, rng_addr, &mash, state, shift, &plan,
                );
                let label = if trace.budget_exceeded {
                    Err("frame budget exceeded".to_string())
                } else {
                    label(&trace).map(|events| {
                        let mut t = GapTable::new();
                        observe_gaps(&events, &mut t);
                        (t, events.into_iter().map(|e| (e.key, e.frame)).collect())
                    })
                };
                let _ = tx.send(RunReport {
                    shift: trace.shift,
                    plan: trace.plan,
                    won: trace.won,
                    frames: trace.frames,
                    budget_exceeded: trace.budget_exceeded,
                    label,
                    marks: trace.marks,
                });
            }
        });
    }
    drop(tx);

    let tsv_path = std::env::args().nth(1).unwrap_or_else(|| {
        std::env::var("FRLG_ARTIFACTS")
            .map(|a| format!("{a}/scratch/fit-pacing.tsv"))
            .unwrap_or_else(|_| "fit-pacing.tsv".into())
    });
    let mut tsv = std::io::BufWriter::new(std::fs::File::create(&tsv_path).expect("tsv"));
    use std::io::Write;

    let mut table = GapTable::new();
    let mut failures: Vec<(i64, Vec<u32>, String)> = Vec::new();
    let mut results: Vec<(i64, Vec<u32>, bool, u32)> = Vec::new();
    let mut done = 0usize;
    for report in rx {
        done += 1;
        if done.is_multiple_of(25) {
            eprintln!("  {done}/{total} runs");
        }
        results.push((report.shift, report.plan.clone(), report.won, report.frames));
        let run_id = format!("s{}:{:?}", report.shift, report.plan);
        for (i, m) in report.marks.iter().enumerate() {
            writeln!(
                tsv,
                "{run_id}\tmark\t{i}\t{}\t{}\t{}\t{}",
                m.loop_a_start, m.detection, m.loop_b_start, m.loop_b_end
            )
            .unwrap();
        }
        match report.label {
            Ok((t, events)) => {
                for (key, frame) in events {
                    writeln!(tsv, "{run_id}\tev\t{frame}\t{key}").unwrap();
                }
                for (key, gaps) in t {
                    let entry = table.entry(key).or_default();
                    for (gap, n) in gaps {
                        *entry.entry(gap).or_default() += n;
                    }
                }
            }
            Err(e) if report.budget_exceeded => {
                let _ = e;
            }
            Err(e) => failures.push((report.shift, report.plan, e)),
        }
    }
    println!("\nraw event/mark dump: {tsv_path}");

    results.sort();
    println!("\nper-plan results (shift, plan, won, frames):");
    for (shift, plan, won, frames) in &results {
        println!(
            "  s{shift} {plan:?}\t{}\t{frames}",
            if *won { "win" } else { "loss" }
        );
    }

    println!("\ngap table ({} keys):", table.len());
    let mut conflicts = 0;
    for (key, gaps) in &table {
        let marker = if gaps.len() > 1 {
            conflicts += 1;
            "  <-- CONFLICT"
        } else {
            ""
        };
        let shown: Vec<String> = gaps.iter().map(|(g, n)| format!("{g} (x{n})")).collect();
        println!("  {key}: {}{marker}", shown.join(", "));
    }
    println!(
        "\n{conflicts} conflicting keys, {} label failures",
        failures.len()
    );
    for (shift, plan, err) in &failures {
        println!("  FAILED s{shift} {plan:?}: {err}");
    }
}
