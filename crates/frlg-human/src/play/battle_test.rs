//! `frlg-human battle-test`: load a savestate at or just before a battle, mash into it, play it
//! with [`super::battle::fight`], and report. The development harness of S4.
//!
//! ```text
//! frlg-human battle-test --plan human-run/plans/1122-lgdescent.md --state X.state \
//!     --seg misty [--wild flee|catch|fight] [--seed 11] [--out DIR]
//! ```
//! Writes `DIR/events.jsonl`, `DIR/end.png` and `DIR/end.state` (DIR defaults to the scratch
//! directory `$FRLG_ARTIFACTS/scratch/battle-test`).
use std::path::{Path, PathBuf};

use frlg_emu::SymbolTable;
use frlg_route::{Observer, Recorder};

use super::battle::{fight, BattleCfg, WildPlan};
use super::events::EventLog;
use super::human::{HumanFeed, InputModel};
use crate::data::Dex;
use crate::eval::Plan;

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn artifacts() -> PathBuf {
    PathBuf::from(std::env::var("FRLG_ARTIFACTS").unwrap_or_else(|_| ".".into()))
}

/// Run the subcommand; `Err` carries the message for stderr.
pub fn run(dex: &'static Dex, args: &[String]) -> Result<(), String> {
    let plan_path = flag(args, "--plan").unwrap_or("human-run/plans/1122-lgdescent.md".into());
    let state = flag(args, "--state").ok_or("--state is required")?;
    let seg = flag(args, "--seg").unwrap_or("wild".into());
    let seed: u64 = flag(args, "--seed")
        .and_then(|s| s.parse().ok())
        .unwrap_or(11);
    let wild = match flag(args, "--wild").as_deref() {
        Some("catch") => WildPlan::Catch,
        Some("fight") => WildPlan::Fight,
        _ => WildPlan::Flee,
    };
    let rom = flag(args, "--rom")
        .map(PathBuf::from)
        .unwrap_or_else(|| artifacts().join("rom/pokeleafgreen.gba"));
    let sym = flag(args, "--sym")
        .map(PathBuf::from)
        .unwrap_or_else(|| rom.with_extension("sym"));
    let out = flag(args, "--out")
        .map(PathBuf::from)
        .unwrap_or_else(|| artifacts().join("scratch/battle-test"));
    std::fs::create_dir_all(&out).map_err(|e| format!("{}: {e}", out.display()))?;

    let text = std::fs::read_to_string(&plan_path).map_err(|e| format!("{plan_path}: {e}"))?;
    let plan = Plan::load(dex, &text)?;
    let ctx = plan.ctx();
    let syms = SymbolTable::load(&sym).map_err(|e| format!("{}: {e}", sym.display()))?;
    let obs = Observer::new(syms)?;
    let mut rec = Recorder::from_reset(&rom).map_err(|e| e.to_string())?;
    rec.emu()
        .load_state_file(Path::new(&state))
        .map_err(|e| format!("{state}: {e}"))?;
    // `--give ITEM_X:qty`: an experiment's bag change (items pocket), e.g. a Full Heal before Koga.
    if let Some(g) = flag(args, "--give") {
        let (name, q) = g.split_once(':').ok_or("--give ITEM:qty")?;
        let item = super::names::id(name);
        let ok = super::ram::Ram::new(&obs).give(
            rec.emu(),
            0,
            item,
            q.parse().map_err(|_| "--give qty")?,
        );
        if !ok {
            return Err("--give: no empty item slot".into());
        }
    }
    let base = rec.emu().frame() as u64;
    let mut f = HumanFeed::new(rec, InputModel::from_params(&plan.params), seed, 0, base);
    let mut log = EventLog::default();
    let cfg = BattleCfg {
        wild,
        ..BattleCfg::new(&ctx, &seg)
    };
    let res = fight(&mut f, &obs, &mut log, &cfg);
    log.save(&out.join("events.jsonl"))
        .map_err(|e| e.to_string())?;
    let _ = f.emu().write_png(&out.join("end.png"));
    let _ = f.emu().save_state_file(&out.join("end.state"));
    match res {
        Ok(r) => {
            println!(
                "{{\"end\":\"{:?}\",\"turns\":{},\"throws\":{},\"frames\":{},\"trainer\":{:?},\"events\":{}}}",
                r.end,
                r.turns,
                r.throws,
                r.frames,
                r.trainer.unwrap_or_default(),
                log.events.len()
            );
            Ok(())
        }
        Err(e) => Err(format!("battle failed: {e}")),
    }
}
