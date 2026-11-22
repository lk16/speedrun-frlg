//! `frlg-human dex-test` / `shop-test`: load a savestate, run a list of dex-fill and shop actions
//! with the playing controller's session (walking, battles, the human input model), and report.
//! The development harness of S7.
//!
//! ```text
//! frlg-human dex-test --state X.state [--plan P] [--seed 11] [--out DIR] --do ACTION [--do ACTION ...]
//! ACTION:
//!   hunt:<i>                          the plan's hunt #i
//!   hunt:<MAP>:<land|water|fish>:<steps>[:<stop_after>]   an ad-hoc hunt (plan skip rules apply)
//!   special:<i> | evolve:<i>          the plan's special / evolution #i
//!   evo:<SPECIES_X>:<stone|candy>[:<STONE>]  an ad-hoc evolution (e.g. evo:SPECIES_JOLTEON:stone:THUNDER)
//!   hook:<segment>                    dex::dex_hook (everything the plan anchors after it)
//!   buy:<MAP>[@x,y]:<ITEM>=<n>[,<ITEM>=<n>...]   shop::buy
//!   deposit:<CENTER_MAP>:<slot> | withdraw:<CENTER_MAP>:<SPECIES_X>   shop::pc
//!   use:<ITEM>[:<slot>]               shop::use_item / use_item_on
//!   talk:<MAP>:<object script>        talk to someone and answer YES (a gift item, e.g. the Good Rod)
//!   go:<MAP>[:x:y] | fly:<MAPSEC> | heal:<CENTER_MAP> | bike
//!   save:<name>                       write DIR/<name>.state
//! ```
//! Writes `DIR/events.jsonl`, `DIR/end.png`, `DIR/end.state` (DIR defaults to
//! `$FRLG_ARTIFACTS/scratch/dex-test`), and prints one JSON line per action.
use std::path::{Path, PathBuf};

use frlg_route::Feed;
use serde_json::json;

use super::model::Model;
use super::names::id;
use super::session::{CheckpointRec, PResult, PlayCfg, PlayError, Session};
use super::walk::Opts;
use crate::data::Dex;
use crate::eval::Plan;
use crate::params::{Evolve, Hunt};

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn artifacts() -> PathBuf {
    PathBuf::from(std::env::var("FRLG_ARTIFACTS").unwrap_or_else(|_| ".".into()))
}

/// A session that starts at a savestate: a throwaway run directory with an empty log and one
/// checkpoint (frame 0, the state file), resumed like any other run.
fn session_at<'p>(
    plan: &'p Plan,
    state: &Path,
    out: &Path,
    seed: u64,
) -> Result<Session<'p>, String> {
    let (rom, sym) = super::cli::rom_for(plan);
    let run = out.join("run");
    std::fs::create_dir_all(&run).map_err(|e| e.to_string())?;
    std::fs::write(
        run.join("run.ilog"),
        frlg_emu::InputLog::new([0; 20], vec![]).encode(),
    )
    .map_err(|e| e.to_string())?;
    std::fs::write(run.join("events.jsonl"), "").map_err(|e| e.to_string())?;
    let rec = CheckpointRec {
        name: "start".into(),
        frame: 0,
        ram_hash: String::new(),
        state_file: state.display().to_string(),
    };
    std::fs::write(
        run.join("checkpoints.jsonl"),
        serde_json::to_string(&rec).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())?;
    let cfg = PlayCfg {
        rom,
        sym,
        seed,
        out_dir: out.to_path_buf(),
        state_dir: out.join("states"),
        force_rollback: vec![],
        max_retries: 1,
        from: None,
    };
    let model = Model::build(plan, 0xC0_17_20_11, 40);
    Session::resume(plan, cfg, &run, model).map_err(|e| e.to_string())
}

fn act(s: &mut Session, a: &str, out: &Path) -> PResult<serde_json::Value> {
    let parts: Vec<&str> = a.split(':').collect();
    let plan = s.plan;
    let p = &plan.params;
    let idx = |i: usize| -> PResult<usize> {
        parts
            .get(i)
            .and_then(|x| x.parse().ok())
            .ok_or_else(|| PlayError::Fatal(format!("{a}: index expected")))
    };
    let v = match parts[0] {
        "hunt" => {
            let h = if parts.len() >= 4 {
                Hunt {
                    after: "dex-test".into(),
                    site: parts[1].into(),
                    area: parts[2].into(),
                    steps: parts[3].parse().unwrap_or(100),
                    stop_after: parts.get(4).and_then(|x| x.parse().ok()).unwrap_or(0),
                    on_route: true,
                    ..Hunt::default()
                }
            } else {
                p.hunts
                    .get(idx(1)?)
                    .cloned()
                    .ok_or_else(|| PlayError::Fatal(format!("{a}: no such hunt")))?
            };
            let r = super::dex::hunt(s, &h)?;
            json!({"site": r.site, "why": r.why, "steps": r.steps, "encounters": r.encounters,
                   "caught": r.caught, "frames": r.frames})
        }
        "special" => {
            let sp = p
                .specials
                .get(idx(1)?)
                .cloned()
                .ok_or_else(|| PlayError::Fatal(format!("{a}: no such special")))?;
            json!({"obtained": super::dex::special(s, &sp)?})
        }
        "evolve" | "evo" => {
            let e = if parts[0] == "evolve" {
                p.evolutions
                    .get(idx(1)?)
                    .cloned()
                    .ok_or_else(|| PlayError::Fatal(format!("{a}: no such evolution")))?
            } else {
                Evolve {
                    after: "dex-test".into(),
                    species: parts.get(1).copied().unwrap_or("").into(),
                    via: parts.get(2).copied().unwrap_or("stone").into(),
                    item: parts.get(3).copied().unwrap_or("").into(),
                    ..Evolve::default()
                }
            };
            json!({"evolved": super::dex::evolve(s, &e)?})
        }
        "hook" => {
            super::dex::dex_hook(s, parts.get(1).copied().unwrap_or(""))?;
            json!({})
        }
        "buy" => {
            let (map, clerk) = match parts[1].split_once('@') {
                Some((m, xy)) => {
                    let (x, y) = xy.split_once(',').unwrap_or(("0", "0"));
                    (m, Some((x.parse().unwrap_or(0), y.parse().unwrap_or(0))))
                }
                None => (parts[1], None),
            };
            let want: Vec<(&str, u16)> = parts[2]
                .split(',')
                .filter_map(|w| w.split_once('='))
                .map(|(i, n)| (i, n.parse().unwrap_or(1)))
                .collect();
            let got = super::shop::buy(s, map, clerk, &want)?;
            json!(got
                .iter()
                .map(|b| json!({"item": b.item, "qty": b.qty, "cost": b.cost}))
                .collect::<Vec<_>>())
        }
        "deposit" => {
            super::shop::pc(s, parts[1], &[super::shop::PcOp::Deposit(idx(2)?)])?;
            json!({"party": s.obs.party_count(s.hf.rec.emu())})
        }
        "withdraw" => {
            super::shop::pc(s, parts[1], &[super::shop::PcOp::Withdraw(id(parts[2]))])?;
            json!({"party": s.obs.party_count(s.hf.rec.emu())})
        }
        "use" => {
            if let Some(slot) = parts.get(2).and_then(|x| x.parse().ok()) {
                let sp = super::shop::use_item_on(s, parts[1], slot)?;
                json!({"species": super::names::names().species(sp)})
            } else {
                super::shop::use_item(s, parts[1])?;
                json!({})
            }
        }
        "talk" => {
            let g = s.graph();
            let local = g.maps[&g.id(parts[1])]
                .objects
                .iter()
                .find(|o| o.script == parts[2])
                .map(|o| o.local_id)
                .ok_or_else(|| PlayError::Fatal(format!("{a}: no such object")))?;
            super::legs::talk_to(s, parts[1], local)?;
            // Answer YES to whatever it asks.
            let start = s.hf.fed();
            loop {
                if s.hf.fed() - start > 20_000 {
                    return Err(PlayError::Death("soft-lock: talk".into()));
                }
                if super::prims::is_free(s) {
                    break;
                }
                if super::menu::yesno_up(s) {
                    super::menu::answer(s, true)?;
                    continue;
                }
                s.hf.late()?;
                s.hf.step(frlg_emu::keys::A)?;
                s.hf.step(0)?;
            }
            super::prims::settle(s)?;
            json!({})
        }
        "go" => {
            if parts.len() >= 4 {
                super::walk::go(
                    s,
                    parts[1],
                    parts[2].parse().unwrap_or(0),
                    parts[3].parse().unwrap_or(0),
                    &Opts::default(),
                )?;
            } else {
                super::walk::go_map(s, parts[1], &Opts::default())?;
            }
            json!({})
        }
        "fly" => {
            super::items::fly(s, parts[1])?;
            json!({})
        }
        "heal" => {
            super::legs::center_heal(s, parts[1])?;
            json!({})
        }
        "bike" => json!({"on": super::items::mount_bike(s)?}),
        "save" => {
            let f = out.join(format!("{}.state", parts.get(1).copied().unwrap_or("save")));
            s.hf.rec.save_state_file(&f)?;
            json!({"state": f.display().to_string()})
        }
        other => return Err(PlayError::Fatal(format!("unknown action {other}"))),
    };
    Ok(v)
}

/// Run the subcommand; `Err` carries the message for stderr.
pub fn main(dex: &'static Dex, args: &[String]) -> Result<(), String> {
    let plan_path = flag(args, "--plan").unwrap_or("human-run/plans/1122-lgdescent.md".into());
    let state = flag(args, "--state").ok_or("--state is required")?;
    let seed: u64 = flag(args, "--seed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(11);
    let out = flag(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| artifacts().join("scratch/dex-test"));
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;
    let text = std::fs::read_to_string(&plan_path).map_err(|e| format!("{plan_path}: {e}"))?;
    let plan = Plan::load(dex, &text)?;
    let mut s = session_at(&plan, Path::new(&state), &out, seed)?;
    s.seg = "dex-test".into();
    s.hf.rekey(seed);
    let actions: Vec<String> = args
        .windows(2)
        .filter(|w| w[0] == "--do")
        .map(|w| w[1].clone())
        .collect();
    let mut res = Ok(());
    for a in &actions {
        let f0 = s.frame();
        match act(&mut s, a, &out) {
            Ok(v) => println!(
                "{}",
                json!({"action": a, "ok": true, "frames": s.frame() - f0,
                       "owned": super::dex::caught_count(&mut s), "result": v})
            ),
            Err(e) => {
                println!(
                    "{}",
                    json!({"action": a, "ok": false, "frames": s.frame() - f0, "error": e.to_string()})
                );
                res = Err(format!("{a}: {e}"));
                break;
            }
        }
    }
    s.log
        .save(&out.join("events.jsonl"))
        .map_err(|e| e.to_string())?;
    let _ = s.hf.emu().write_png(&out.join("end.png"));
    let _ = s.hf.rec.save_state_file(&out.join("end.state"));
    res
}
