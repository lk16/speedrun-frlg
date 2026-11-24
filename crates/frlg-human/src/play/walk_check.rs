//! `frlg-human play-walk` (review round 1, step R1): the walker regression set.
//!
//! Each case loads a savestate (usually a few seconds before a bump of a played run, cut by
//! `examples/walk_bumps.rs`) and walks one or more legs with the current walker; the report counts
//! bumps (`warn` events with `"bump": true`), waits for turning trainers, detours, and the trainer
//! battles that started on the way. A case fails on any bump or on a battle with a trainer it
//! lists in `no_fight`. The cases live in `human-run/walk-regress.json`:
//!
//! ```json
//! [{"name": "route3", "state": "states/human-play/1122-lgdescent-s11/0073608-brock-done.state",
//!   "legs": [["MAP_ROUTE4", 8, 5]], "opts": ["cut"], "no_fight": ["YOUNGSTER_BEN", "LASS_ROBIN"]}]
//! ```
//!
//! `state` is relative to `$FRLG_ARTIFACTS` unless absolute. Each case runs from a throwaway run
//! directory under `$FRLG_ARTIFACTS/scratch/r1-walk/`.
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::json;

use super::model::Model;
use super::session::{PlayCfg, PlayError, Session};
use super::walk::{self, Opts};
use crate::eval::Plan;

#[derive(Deserialize, Clone)]
pub struct Case {
    pub name: String,
    pub state: String,
    /// Goal tiles in order: (map, x, y); x < 0 means anywhere on the map.
    pub legs: Vec<(String, i16, i16)>,
    #[serde(default)]
    pub opts: Vec<String>,
    /// Trainer constants (as the battle log names them, e.g. `YOUNGSTER_BEN`) that must not be
    /// fought on the way.
    #[serde(default)]
    pub no_fight: Vec<String>,
    #[serde(default)]
    pub note: String,
}

pub struct Outcome {
    pub bumps: u32,
    pub waits: u32,
    pub detours: Vec<String>,
    pub battles: Vec<String>,
    pub frames: u64,
    pub error: Option<String>,
}

fn artifacts() -> PathBuf {
    std::env::var_os("FRLG_ARTIFACTS")
        .map(PathBuf::from)
        .expect("$FRLG_ARTIFACTS is not set")
}

fn leak(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// A run directory whose only checkpoint is `state` at frame 0, for `Session::resume`.
fn scratch_run(name: &str, state: &Path, rom: &Path) -> Result<PathBuf, String> {
    let dir = artifacts().join("scratch/r1-walk").join(name);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let rec = frlg_route::Recorder::from_reset(rom).map_err(|e| e.to_string())?;
    let log = frlg_emu::InputLog::new(rec.rom_sha1(), vec![]);
    std::fs::write(dir.join("run.ilog"), log.encode()).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("events.jsonl"), "").map_err(|e| e.to_string())?;
    let cp = json!({"name": "walk-check", "frame": 0, "ram_hash": "",
                    "state_file": state.display().to_string()});
    std::fs::write(dir.join("checkpoints.jsonl"), format!("{cp}\n")).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn run_case(
    plan: &Plan,
    rom: &Path,
    sym: &Path,
    seed: u64,
    c: &Case,
) -> Result<Outcome, String> {
    let state = if Path::new(&c.state).is_absolute() {
        PathBuf::from(&c.state)
    } else {
        artifacts().join(&c.state)
    };
    let dir = scratch_run(&c.name, &state, rom)?;
    let cfg = PlayCfg {
        rom: rom.to_path_buf(),
        sym: sym.to_path_buf(),
        seed,
        out_dir: dir.clone(),
        state_dir: dir.clone(),
        force_rollback: vec![],
        from: None,
        max_retries: 1,
    };
    let model = Model::build(plan, 0xC0_17_20_11, 4);
    let mut s = Session::resume(plan, cfg, &dir, model).map_err(|e| e.to_string())?;
    s.hf.rekey(seed);
    s.seg = format!("walk-check {}", c.name);
    let opts = Opts {
        cut: c.opts.iter().any(|o| o == "cut"),
        surf: c.opts.iter().any(|o| o == "surf"),
        ..Opts::default()
    };
    let ev0 = s.log.events.len();
    let f0 = s.frame();
    let mut error = None;
    for (map, x, y) in &c.legs {
        // x < 0: anywhere on the map (a warp's arrival tile, which the game walks the player off).
        let r = if *x < 0 {
            walk::go_map(&mut s, leak(map), &opts)
        } else {
            walk::go(&mut s, leak(map), *x, *y, &opts)
        };
        if let Err(e) = r {
            error = Some(match e {
                PlayError::Death(m) | PlayError::Fatal(m) => m,
            });
            break;
        }
    }
    let ev = &s.log.events[ev0..];
    let (bumps, _) = walk::bump_summary(ev);
    let waits = ev.iter().filter(|e| e.nums.get("wait").is_some()).count() as u32;
    let detours = ev
        .iter()
        .filter(|e| e.kind == "warn" && e.text.starts_with("detour"))
        .map(|e| e.text.clone())
        .collect();
    let battles = ev
        .iter()
        .filter(|e| e.kind == "battle" && e.text.contains(" sends out "))
        .map(|e| e.text.split(" sends out ").next().unwrap_or("").to_string())
        .collect();
    if std::env::var_os("FRLG_WALK_DEBUG").is_some() {
        for e in ev {
            eprintln!("  {} {} {}", e.frame, e.kind, e.text);
        }
    }
    Ok(Outcome {
        bumps,
        waits,
        detours,
        battles,
        frames: s.frame() - f0,
        error,
    })
}

/// `play-walk --plan <plan.md> <cases.json> [--case <name>] [--seed S] [--seeds n]`
pub fn main(args: &[String], plan: &Plan, rom: &Path, sym: &Path) {
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let file = args
        .iter()
        .skip(1)
        .find(|a| a.ends_with(".json"))
        .cloned()
        .unwrap_or_else(|| "human-run/walk-regress.json".into());
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{file}: {e}"));
    let cases: Vec<Case> = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{file}: {e}"));
    let only = flag("--case");
    let seed: u64 = flag("--seed").map_or(1, |s| s.parse().unwrap());
    let seeds: u64 = flag("--seeds").map_or(1, |s| s.parse().unwrap());
    let mut failed = 0;
    let mut total_bumps = 0;
    for c in cases
        .iter()
        .filter(|c| only.as_ref().is_none_or(|o| c.name.contains(o.as_str())))
    {
        for k in 0..seeds {
            let sd = seed + k;
            match run_case(plan, rom, sym, sd, c) {
                Ok(o) => {
                    total_bumps += o.bumps;
                    let bad: Vec<&String> = o
                        .battles
                        .iter()
                        .filter(|b| c.no_fight.iter().any(|n| b.contains(n.as_str())))
                        .collect();
                    let ok = o.bumps == 0 && bad.is_empty() && o.error.is_none();
                    if !ok {
                        failed += 1;
                    }
                    println!(
                        "{} {:<28} seed {sd}: bumps {} waits {} frames {} battles [{}]{}{}",
                        if ok { "ok  " } else { "FAIL" },
                        c.name,
                        o.bumps,
                        o.waits,
                        o.frames,
                        o.battles.join(", "),
                        o.error.map_or(String::new(), |e| format!(" error: {e}")),
                        if o.detours.is_empty() {
                            String::new()
                        } else {
                            format!(" | {}", o.detours.join(" | "))
                        }
                    );
                }
                Err(e) => {
                    failed += 1;
                    println!("FAIL {:<28} seed {sd}: {e}", c.name);
                }
            }
        }
    }
    println!("walk-check: {failed} failed, {total_bumps} bumps in total");
    if failed > 0 {
        std::process::exit(1);
    }
}
