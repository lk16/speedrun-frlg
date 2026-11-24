//! `frlg-human play | play-verify | play-inspect` (`human-run/PROMPT.md`, step S2).
use std::path::{Path, PathBuf};

use crate::data::Dex;
use crate::eval::Plan;

use super::model::Model;
use super::session::{verify, PlayCfg, Session};

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn artifacts() -> PathBuf {
    std::env::var_os("FRLG_ARTIFACTS")
        .map(PathBuf::from)
        .expect("$FRLG_ARTIFACTS is not set")
}

/// The ROM and symbols for a plan's version (`"LG"` / `"FR"`).
pub fn rom_for(plan: &Plan) -> (PathBuf, PathBuf) {
    let base = artifacts().join("rom");
    if plan.params.version == "LG" {
        (
            base.join("pokeleafgreen.gba"),
            base.join("pokeleafgreen.sym"),
        )
    } else {
        (base.join("pokefirered.gba"), base.join("pokefirered.sym"))
    }
}

fn load_plan(dex: &'static Dex, path: &str) -> Plan {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    Plan::load(dex, &text).unwrap_or_else(|e| panic!("invalid plan {path}: {e}"))
}

pub fn usage() -> &'static str {
    "  play --plan <plan.md> --seed S --resume <dir> [--until <stage>] [--out <dir>]\n\
     \x20      [--force-rollback <stage>[,<stage>]] [--retries n] [--model-R n]\n\
     \x20                                      continue a played attempt from its last checkpoint\n\
     \x20 play-verify --plan <plan.md> <dir>    replay <dir>/run.ilog from boot, check checkpoint RAM hashes\n\
     \x20 play-battles --plan <plan.md> <dir>   replay <dir>/run.ilog, one line per battle (trainer, exp)\n\
     \x20 play-inspect --plan <plan.md> <state> print map, position, party, bag of a savestate\n\
     \x20 play-walk --plan <plan.md> [cases.json] [--case name] [--seed S] [--seeds n]\n\
     \x20                                      walker regression set (R1): bumps, waits, trainer fights"
}

pub fn main(cmd: &str, args: &[String], dex: &'static Dex) {
    let plan_path = flag(args, "--plan").unwrap_or_else(|| {
        let cur = std::fs::read_to_string("human-run/plans/CURRENT").expect("--plan");
        format!("human-run/plans/{}.md", cur.trim())
    });
    let plan = load_plan(dex, &plan_path);
    let (rom, sym) = rom_for(&plan);
    match cmd {
        "play" => {
            let seed: u64 = flag(args, "--seed").map_or(1, |s| s.parse().expect("--seed"));
            let tag = format!("{}-s{seed}", plan.params.name);
            let out = flag(args, "--out").map_or_else(
                || artifacts().join("runs/human-play").join(&tag),
                PathBuf::from,
            );
            let resume = flag(args, "--resume").map_or_else(|| out.clone(), PathBuf::from);
            let state_dir = flag(args, "--states").map_or_else(
                || artifacts().join("states/human-play").join(&tag),
                PathBuf::from,
            );
            let r: u64 = flag(args, "--model-R").map_or(400, |s| s.parse().unwrap());
            let t0 = std::time::Instant::now();
            let model = Model::build(&plan, 0xC0_17_20_11, r);
            eprintln!(
                "model: {} segments, {r} no-reset attempts in {:.1}s",
                model.names.len(),
                t0.elapsed().as_secs_f64()
            );
            let cfg = PlayCfg {
                rom,
                sym,
                seed,
                out_dir: out.clone(),
                state_dir,
                force_rollback: flag(args, "--force-rollback")
                    .map(|s| s.split(',').map(str::to_string).collect())
                    .unwrap_or_default(),
                from: flag(args, "--from"),
                max_retries: flag(args, "--retries").map_or(12, |s| s.parse().unwrap()),
            };
            let mut s = Session::resume(&plan, cfg, &resume, model).unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1)
            });
            s.fallback = args.iter().any(|a| a == "--fallback-battle");
            let until = flag(args, "--until");
            let stages = super::legs::stages();
            let ev0 = s.log.events.len();
            let res = s.play(&stages, until.as_deref());
            // Review round 1 (R1): every bump is a controller bug; the summary reports them.
            let (session_bumps, per) =
                super::walk::bump_summary(&s.log.events[ev0.min(s.log.events.len())..]);
            let (all_bumps, _) = super::walk::bump_summary(&s.log.events);
            let per: Vec<String> = per.iter().map(|(st, n)| format!("{st} {n}")).collect();
            eprintln!(
                "bumps: {session_bumps} this session ({}); {all_bumps} in the whole log",
                if per.is_empty() {
                    "none".to_string()
                } else {
                    per.join(", ")
                }
            );
            match res {
                Ok(()) => eprintln!(
                    "done: last checkpoint {} at frame {}, {} rollbacks; outputs in {}",
                    s.last_checkpoint(),
                    s.frame(),
                    s.rollbacks,
                    out.display()
                ),
                Err(e) => {
                    eprintln!(
                        "stopped: {e}; last checkpoint {}; outputs in {}",
                        s.last_checkpoint(),
                        out.display()
                    );
                    std::process::exit(1)
                }
            }
        }
        "play-verify" => {
            let dir = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with("--"))
                .find(|a| Path::new(a).is_dir())
                .expect("play-verify <dir>");
            match verify(&rom, Path::new(dir)) {
                Ok(n) => println!("verified: {n} checkpoints match the replay from boot"),
                Err(e) => {
                    println!("VERIFY FAILED: {e}");
                    std::process::exit(1)
                }
            }
        }
        "play-battles" => {
            let dir = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with("--"))
                .find(|a| Path::new(a).is_dir())
                .expect("play-battles <dir>");
            if let Err(e) = super::battle_audit::run(&rom, &sym, Path::new(dir)) {
                println!("play-battles failed: {e}");
                std::process::exit(1)
            }
        }
        "play-walk" => super::walk_check::main(args, &plan, &rom, &sym),
        "play-inspect" => {
            let state = args
                .iter()
                .skip(1)
                .find(|a| a.ends_with(".state"))
                .expect("play-inspect <file.state>");
            super::inspect::print(&rom, &sym, Path::new(state), dex);
        }
        _ => unreachable!(),
    }
}
