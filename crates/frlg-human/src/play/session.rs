//! One played attempt: a resumed prefix, a [`HumanFeed`] continuing it, checkpoints and rollback
//! (`human-run/PROMPT.md`, *Roll back, don't restart*).
//!
//! The route is a list of [`Stage`]s. Each stage that returns `Ok` ends in a checkpoint (a raw
//! in-memory savestate for rollback, a savestate file, and the RAM hash `frlg run --ram-hash` would
//! print). A stage that dies (blackout, soft-lock timeout, or a forced debug death) rolls back to
//! the last checkpoint: the core is restored, the input log and the events are truncated, a
//! `rollback` event is logged, and the stage is replayed with a fresh human key. The first branch
//! that survives is kept.
//!
//! Human keys are per stage: stage `i`, retry `r` plays with `HumanFeed::rekey(i * 1000 + r)`. So a
//! run resumed at any checkpoint continues exactly as the uninterrupted run would have.
use std::path::{Path, PathBuf};

use frlg_emu::{InputLog, SaveState};
use frlg_route::{Observer, Recorder, RouteError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::events::EventLog;
use super::human::{HumanFeed, InputModel};
use super::model::Model;
use super::ram::Ram;
use crate::eval::Plan;

/// Why a branch died.
#[derive(Debug)]
pub enum PlayError {
    /// A dead branch: roll back and replay with a fresh human key.
    Death(String),
    /// A controller bug or an impossible state: stop.
    Fatal(String),
}

impl std::fmt::Display for PlayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlayError::Death(s) => write!(f, "dead branch: {s}"),
            PlayError::Fatal(s) => write!(f, "fatal: {s}"),
        }
    }
}

impl From<RouteError> for PlayError {
    fn from(e: RouteError) -> Self {
        match e {
            // A wait that never ended: the game soft-locked, or the controller lost track. Either
            // way the branch is dead; a stage that times out on every key ends as Fatal after the
            // retry cap.
            RouteError::Timeout { .. } => PlayError::Death(format!("soft-lock: {e}")),
            other => PlayError::Fatal(other.to_string()),
        }
    }
}

impl From<String> for PlayError {
    fn from(e: String) -> Self {
        PlayError::Fatal(e)
    }
}

impl From<&str> for PlayError {
    fn from(e: &str) -> Self {
        PlayError::Fatal(e.to_string())
    }
}

pub type PResult<T> = Result<T, PlayError>;

/// One step of the route; a checkpoint named `name` is saved when it returns `Ok`.
pub struct Stage {
    pub name: &'static str,
    pub run: fn(&mut Session) -> PResult<()>,
}

/// A checkpoint as stored in `checkpoints.jsonl`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckpointRec {
    pub name: String,
    /// Absolute frame (frames since power-on).
    pub frame: u64,
    /// sha1 over EWRAM then IWRAM (`Emu::ram_hash`).
    pub ram_hash: String,
    pub state_file: String,
}

struct Checkpoint {
    rec: CheckpointRec,
    state: SaveState,
    ev_len: usize,
    stage: usize,
}

pub struct PlayCfg {
    pub rom: PathBuf,
    pub sym: PathBuf,
    pub seed: u64,
    pub out_dir: PathBuf,
    pub state_dir: PathBuf,
    /// Stage names that die once (after succeeding) to exercise the rollback (`--force-rollback`).
    pub force_rollback: Vec<String>,
    pub max_retries: u32,
    /// Resume from this checkpoint of the resume directory instead of its last one (`--from`).
    pub from: Option<String>,
}

pub struct Session<'p> {
    pub hf: HumanFeed,
    pub obs: Observer,
    pub world: frlg_route::world::World,
    graph: std::rc::Rc<super::walk::Graph>,
    /// Every (name, size) of the sym file -> address, for names the sym file has twice.
    sized_syms: std::collections::HashMap<(String, u32), u32>,
    pub log: EventLog,
    pub plan: &'p Plan,
    /// The simulator context the battle policy reads (`Plan::ctx`, built once).
    pub ctx: crate::sim::Ctx<'p>,
    /// The plan segment being played (battle tactics key, `Params::fight_tactics`).
    pub seg: String,
    /// Use the stand-in battle driver instead of the sim-policy bridge (`--fallback-battle`).
    pub fallback: bool,
    /// What to do with wild encounters right now (flee unless a hunt says otherwise).
    pub wild_plan: super::battle::WildPlan,
    /// The dex fill's hunt state (`play/dex.rs`): which wild mons to catch, Safari counts.
    pub dex: super::dex::DexState,
    /// Tiles the walker bumped into during this stage, with counts: a tile bumped on two walks
    /// is treated as blocked for the rest of the stage (cleared at every stage start, so a run
    /// resumed at a checkpoint plans exactly as the uninterrupted one).
    pub bumped: std::collections::HashMap<super::walk::Node, u32>,
    pub model: Model,
    pub cfg: PlayCfg,
    prefix: Vec<u16>,
    ckpts: Vec<Checkpoint>,
    pub rollbacks: u32,
    /// Rollbacks already logged in the resumed prefix (the numbering continues from them).
    prefix_rollbacks: u32,
    /// Index of the stage being played.
    pub stage: usize,
    pub retry: u32,
    forced_done: Vec<String>,
    /// Added to the human key after a rollback past a dead-end checkpoint, so the earlier stage is
    /// replayed with inputs it has not had before; cleared once the run is past that stage.
    key_offset: u64,
    /// Stage index up to which `key_offset` applies, and how many deep rollbacks happened.
    deep_until: usize,
    deep: u32,
}

pub(crate) fn read_ilog(path: &Path) -> Result<InputLog, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    InputLog::decode(&bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The checkpoints of a run directory: `checkpoints.jsonl` if it exists, else the last
/// `checkpoint <name> saved at frame <f>` event whose frame is the log's end (the 2026-11-21
/// prefix predates `checkpoints.jsonl`), with its state file looked up next to the log or under
/// `state_dir`.
fn resume_checkpoints(
    dir: &Path,
    events: &EventLog,
    ilog_len: u64,
    state_dir: &Path,
) -> Result<Vec<CheckpointRec>, String> {
    let cp = dir.join("checkpoints.jsonl");
    if cp.exists() {
        let text = std::fs::read_to_string(&cp).map_err(|e| e.to_string())?;
        return text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(|e| format!("{}: {e}", cp.display())))
            .collect();
    }
    let ev = events
        .events
        .iter()
        .rev()
        .find(|e| e.frame == ilog_len && e.nums.get("checkpoint").is_some())
        .ok_or("no checkpoints.jsonl and no checkpoint event at the log's end")?;
    let name = ev.nums["checkpoint"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let candidates = [
        dir.join(format!("{name}-{ilog_len}.state")),
        state_dir.join(format!("{ilog_len:07}-{name}.state")),
    ];
    let file = candidates
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| format!("no state file for checkpoint {name} at {ilog_len}"))?;
    Ok(vec![CheckpointRec {
        name,
        frame: ilog_len,
        ram_hash: String::new(),
        state_file: file.display().to_string(),
    }])
}

impl<'p> Session<'p> {
    /// Resume from the last checkpoint of `resume` (a run directory).
    pub fn resume(plan: &'p Plan, cfg: PlayCfg, resume: &Path, model: Model) -> PResult<Self> {
        let syms = frlg_emu::SymbolTable::load(&cfg.sym).map_err(|e| e.to_string())?;
        let obs = Observer::new(syms)?;
        let ilog = read_ilog(&resume.join("run.ilog"))?;
        let mut log = EventLog::load(&resume.join("events.jsonl")).map_err(|e| e.to_string())?;
        let mut recs = resume_checkpoints(resume, &log, ilog.len() as u64, &cfg.state_dir)?;
        if let Some(from) = &cfg.from {
            let k = recs
                .iter()
                .rposition(|r| &r.name == from)
                .ok_or_else(|| format!("no checkpoint {from} in {}", resume.display()))?;
            recs.truncate(k + 1);
        }
        let last = recs.last().ok_or("no checkpoint to resume from")?.clone();
        if (last.frame as usize) > ilog.len() {
            return Err(PlayError::Fatal(format!(
                "checkpoint {} at {} is past the log's end {}",
                last.name,
                last.frame,
                ilog.len()
            )));
        }
        let prefix = ilog.frames[..last.frame as usize].to_vec();
        log.truncate(last.frame);
        // Cut any event after the checkpoint event itself (several can share its frame).
        if let Some(p) = log.events.iter().rposition(|e| {
            e.nums.get("checkpoint").and_then(Value::as_str) == Some(last.name.as_str())
        }) {
            log.events.truncate(p + 1);
        }

        let mut rec = Recorder::from_reset(&cfg.rom)?;
        rec.emu()
            .load_state_file(Path::new(&last.state_file))
            .map_err(|e| PlayError::Fatal(format!("{}: {e}", last.state_file)))?;
        let hash = rec.emu().ram_hash().map_err(|e| e.to_string())?;
        if !last.ram_hash.is_empty() && last.ram_hash != hash {
            return Err(PlayError::Fatal(format!(
                "state {} hashes {hash}, checkpoints.jsonl says {}",
                last.state_file, last.ram_hash
            )));
        }
        let state = rec.save_state()?;
        let model_in = InputModel::from_params(&plan.params);
        let hf = HumanFeed::new(rec, model_in, cfg.seed, 0, last.frame);
        let ev_len = log.events.len();
        let prefix_rollbacks = log.events.iter().filter(|e| e.kind == "rollback").count() as u32;
        let mut ckpts: Vec<Checkpoint> = recs[..recs.len() - 1]
            .iter()
            .map(|r| Checkpoint {
                rec: r.clone(),
                state: SaveState::from_bytes(vec![]),
                ev_len: 0,
                stage: usize::MAX,
            })
            .collect();
        ckpts.push(Checkpoint {
            rec: CheckpointRec {
                ram_hash: hash,
                ..last
            },
            state,
            ev_len,
            stage: usize::MAX,
        });
        let mut sized_syms = std::collections::HashMap::new();
        for line in std::fs::read_to_string(&cfg.sym)
            .map_err(|e| e.to_string())?
            .lines()
        {
            let f: Vec<&str> = line.split_whitespace().collect();
            if f.len() == 4 {
                if let (Ok(a), Ok(z)) =
                    (u32::from_str_radix(f[0], 16), u32::from_str_radix(f[2], 16))
                {
                    sized_syms.insert((f[3].to_string(), z), a);
                }
            }
        }
        let mut world = frlg_route::world::World::load()?;
        let graph = std::rc::Rc::new(super::walk::Graph::load(&mut world)?);
        Ok(Session {
            hf,
            obs,
            world,
            graph,
            sized_syms,
            log,
            plan,
            ctx: plan.ctx(),
            seg: String::new(),
            fallback: false,
            wild_plan: super::battle::WildPlan::Flee,
            dex: Default::default(),
            bumped: Default::default(),
            model,
            cfg,
            prefix,
            ckpts,
            rollbacks: 0,
            prefix_rollbacks,
            stage: 0,
            retry: 0,
            forced_done: vec![],
            key_offset: 0,
            deep_until: 0,
            deep: 0,
        })
    }

    /// A fresh attempt at power-on: no resumed prefix, the core straight out of reset, and a
    /// checkpoint `power-on` at frame 0 (the first stage of `legs::stages()`, `early.rs`), so
    /// rollback, `checkpoints.jsonl` and `play-verify` work from boot like any other checkpoint.
    pub fn power_on(plan: &'p Plan, cfg: PlayCfg, model: Model) -> PResult<Self> {
        let dir = cfg.out_dir.clone();
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let mut rec = Recorder::from_reset(&cfg.rom)?;
        std::fs::create_dir_all(&cfg.state_dir).map_err(|e| e.to_string())?;
        let file = cfg.state_dir.join(format!("{:07}-power-on.state", 0));
        rec.save_state_file(&file)?;
        let hash = rec.emu().ram_hash().map_err(|e| e.to_string())?;
        let mut log = EventLog::default();
        log.push(
            0,
            "decision",
            format!("plan {}: power-on", plan.params.name),
            json!({"checkpoint": "power-on", "ram_hash": hash}),
        );
        std::fs::write(
            dir.join("run.ilog"),
            InputLog::new(rec.rom_sha1(), vec![]).encode(),
        )
        .map_err(|e| e.to_string())?;
        log.save(&dir.join("events.jsonl"))
            .map_err(|e| e.to_string())?;
        let rec_line = CheckpointRec {
            name: "power-on".into(),
            frame: 0,
            ram_hash: hash,
            state_file: file.display().to_string(),
        };
        std::fs::write(
            dir.join("checkpoints.jsonl"),
            format!("{}\n", serde_json::to_string(&rec_line).unwrap()),
        )
        .map_err(|e| e.to_string())?;
        Self::resume(plan, cfg, &dir, model)
    }

    /// Absolute frame since power-on.
    pub fn frame(&self) -> u64 {
        self.prefix.len() as u64 + self.hf.rec.frames() as u64
    }

    /// Run timer in seconds (None before New Game).
    pub fn timer(&self) -> Option<f64> {
        self.log.timer(self.frame())
    }

    pub fn event(&mut self, kind: &str, text: impl Into<String>, nums: Value) {
        let f = self.frame();
        self.log.push(f, kind, text, nums);
    }

    /// The overworld map graph (shared, immutable).
    pub fn graph(&self) -> std::rc::Rc<super::walk::Graph> {
        self.graph.clone()
    }

    /// Address of the symbol `name` with byte size `size` (the sym file repeats some static names).
    pub fn sym_sized(&self, name: &str, size: u32) -> u32 {
        *self
            .sized_syms
            .get(&(name.to_string(), size))
            .unwrap_or_else(|| panic!("no symbol {name} of size {size:#x}"))
    }

    /// Address of a symbol (first entry of its name).
    pub fn sym(&self, name: &str) -> u32 {
        self.obs
            .symbols()
            .get(name)
            .unwrap_or_else(|| panic!("no symbol {name}"))
            .addr
    }

    pub fn ram(&self) -> Ram<'_> {
        Ram::new(&self.obs)
    }

    pub fn last_checkpoint(&self) -> &str {
        &self.ckpts.last().unwrap().rec.name
    }

    /// Run `stages` from the one after the resumed checkpoint until `until` (a stage name,
    /// inclusive) or the end.
    pub fn play(&mut self, stages: &[Stage], until: Option<&str>) -> PResult<()> {
        let start = self.last_checkpoint().to_string();
        let mut i = stages
            .iter()
            .position(|s| s.name == start)
            .map(|p| p + 1)
            .ok_or_else(|| format!("checkpoint {start} is not a stage of this route"))?;
        if let Some(c) = self.ckpts.last_mut() {
            c.stage = i - 1;
        }
        while i < stages.len() {
            self.stage = i;
            if i > self.deep_until {
                self.key_offset = 0;
            }
            self.hf
                .rekey(i as u64 * 1000 + self.retry as u64 + self.key_offset);
            let st = &stages[i];
            eprintln!(
                "== stage {i} {} (retry {}) at frame {}",
                st.name,
                self.retry,
                self.frame()
            );
            self.seg = st.name.to_string();
            self.wild_plan = super::battle::WildPlan::Flee;
            self.bumped.clear();
            let mut res = (st.run)(self);
            if res.is_ok()
                && self.cfg.force_rollback.iter().any(|n| n == st.name)
                && !self.forced_done.iter().any(|n| n == st.name)
            {
                self.forced_done.push(st.name.to_string());
                res = Err(PlayError::Death("forced rollback (debug flag)".into()));
            }
            match res {
                Ok(()) => {
                    self.checkpoint(st.name)?;
                    self.retry = 0;
                    if until == Some(st.name) {
                        break;
                    }
                    i += 1;
                }
                Err(PlayError::Death(cause)) => {
                    self.dump(&format!("{}-r{}", st.name, self.retry));
                    // Every branch from this checkpoint dies: it is a dead end (e.g. Lance with
                    // the PP and items the earlier rooms left). Roll back one checkpoint further,
                    // with human keys not used there before (at most 3 times per session).
                    let n = self.ckpts.len();
                    if self.retry + 1 >= self.cfg.max_retries
                        && self.deep < 3
                        && n >= 2
                        && self.ckpts[n - 2].stage != usize::MAX
                    {
                        let dead = self.ckpts.pop().unwrap();
                        self.deep += 1;
                        // The k-th dead end goes k checkpoints back (one more each time).
                        for _ in 1..self.deep {
                            let m = self.ckpts.len();
                            if m >= 2 && self.ckpts[m - 2].stage != usize::MAX {
                                self.ckpts.pop();
                            }
                        }
                        let back = self.ckpts.last().unwrap();
                        let j = back.stage + 1;
                        let msg = format!(
                            "{cause}; checkpoint {} is a dead end ({} deaths): back to {}",
                            dead.rec.name,
                            self.retry + 1,
                            back.rec.name
                        );
                        self.rollback(&msg)?;
                        self.retry = 0;
                        self.key_offset = 100 * self.deep as u64;
                        self.deep_until = i;
                        i = j;
                        continue;
                    }
                    if self.retry + 1 >= self.cfg.max_retries {
                        self.write_outputs()?;
                        return Err(PlayError::Fatal(format!(
                            "stage {} died {} times, last: {cause}",
                            st.name,
                            self.retry + 1
                        )));
                    }
                    self.rollback(&cause)?;
                }
                Err(e) => {
                    self.dump(&format!("{}-fatal", st.name));
                    self.write_outputs()?;
                    return Err(e);
                }
            }
        }
        self.write_outputs()?;
        Ok(())
    }

    /// Save a checkpoint now: in-memory state, state file, RAM hash; then flush the outputs.
    pub fn checkpoint(&mut self, name: &str) -> PResult<()> {
        let frame = self.frame();
        std::fs::create_dir_all(&self.cfg.state_dir).map_err(|e| e.to_string())?;
        let file = self.cfg.state_dir.join(format!("{frame:07}-{name}.state"));
        self.hf.rec.save_state_file(&file)?;
        let state = self.hf.save_state()?;
        let ram_hash = self.hf.emu().ram_hash().map_err(|e| e.to_string())?;
        self.event(
            "decision",
            format!("checkpoint {name} saved at frame {frame}"),
            json!({"checkpoint": name, "ram_hash": ram_hash}),
        );
        let ev_len = self.log.events.len();
        self.ckpts.push(Checkpoint {
            rec: CheckpointRec {
                name: name.to_string(),
                frame,
                ram_hash,
                state_file: file.display().to_string(),
            },
            state,
            ev_len,
            stage: self.stage,
        });
        self.write_outputs()
    }

    /// The dead branch's last picture and state, for debugging (not part of the run).
    fn dump(&mut self, tag: &str) {
        let died_at = self.frame();
        let dbg = self.cfg.out_dir.join("dead");
        if std::fs::create_dir_all(&dbg).is_ok() {
            let stem = dbg.join(format!("{died_at:07}-{tag}"));
            let _ = self.hf.emu().write_png(&stem.with_extension("png"));
            let _ = self.hf.rec.save_state_file(&stem.with_extension("state"));
            eprintln!(
                "dead at {} ({}); picture {}",
                died_at,
                self.obs.snapshot(self.hf.rec.emu()),
                stem.with_extension("png").display()
            );
        }
    }

    fn rollback(&mut self, cause: &str) -> PResult<()> {
        let c = self.ckpts.last().unwrap();
        let (name, frame, ev_len) = (c.rec.name.clone(), c.rec.frame, c.ev_len);
        let state = c.state.clone();
        let died_at = self.frame();
        self.hf
            .emu()
            .load_state(&state)
            .map_err(|e| e.to_string())?;
        self.hf
            .rec
            .truncate((frame - self.prefix.len() as u64) as usize);
        // The dead branch's events go, but earlier rollbacks of this checkpoint stay: every
        // death is counted (`PROMPT.md`, *Count the deaths*).
        let kept: Vec<_> = self.log.events[ev_len.min(self.log.events.len())..]
            .iter()
            .filter(|e| e.kind == "rollback")
            .cloned()
            .collect();
        self.log.events.truncate(ev_len);
        self.log.events.extend(kept);
        self.rollbacks += 1;
        self.retry += 1;
        let n = self.rollbacks + self.prefix_rollbacks;
        self.event(
            "rollback",
            format!("rollback #{n} at {name}: {cause}"),
            json!({"n": n, "checkpoint": name, "cause": cause, "died_at": died_at,
                   "lost_frames": died_at - frame, "retry": self.retry}),
        );
        Ok(())
    }

    /// The whole stitched log: prefix plus what this session recorded.
    pub fn full_log(&self) -> InputLog {
        let mut frames = self.prefix.clone();
        frames.extend_from_slice(self.hf.rec.masks());
        InputLog::new(self.hf.rec.rom_sha1(), frames)
    }

    pub fn write_outputs(&self) -> PResult<()> {
        let d = &self.cfg.out_dir;
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
        // Only up to the last checkpoint: a branch in flight is not part of the run yet.
        let c = &self.ckpts.last().unwrap();
        let mut log = self.full_log();
        log.frames.truncate(c.rec.frame as usize);
        std::fs::write(d.join("run.ilog"), log.encode()).map_err(|e| e.to_string())?;
        let mut ev = EventLog {
            events: self.log.events.clone(),
            new_game_frame: self.log.new_game_frame,
        };
        ev.events.truncate(c.ev_len);
        ev.save(&d.join("events.jsonl"))
            .map_err(|e| e.to_string())?;
        let mut cp = String::new();
        for c in &self.ckpts {
            cp.push_str(&serde_json::to_string(&c.rec).unwrap());
            cp.push('\n');
        }
        std::fs::write(d.join("checkpoints.jsonl"), cp).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// Replay `dir/run.ilog` from power-on and compare the RAM hash at every checkpoint in
/// `dir/checkpoints.jsonl`. Returns the number of checkpoints checked.
pub fn verify(rom: &Path, dir: &Path) -> Result<usize, String> {
    let ilog = read_ilog(&dir.join("run.ilog"))?;
    let text = std::fs::read_to_string(dir.join("checkpoints.jsonl")).map_err(|e| e.to_string())?;
    let recs: Vec<CheckpointRec> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    let mut rec = Recorder::from_reset(rom).map_err(|e| e.to_string())?;
    let emu = rec.emu();
    let mut at = 0usize;
    let mut checked = 0;
    for c in &recs {
        let f = c.frame as usize;
        if f > ilog.len() {
            return Err(format!(
                "checkpoint {} at {f} past the log end {}",
                c.name,
                ilog.len()
            ));
        }
        while at < f {
            emu.step(ilog.frames[at]);
            at += 1;
        }
        let h = emu.ram_hash().map_err(|e| e.to_string())?;
        let ok = c.ram_hash.is_empty() || h == c.ram_hash;
        eprintln!(
            "{:>8} {:<24} {} {}",
            f,
            c.name,
            h,
            if ok { "ok" } else { "MISMATCH" }
        );
        if !ok {
            return Err(format!(
                "checkpoint {} at frame {f}: replay {h} vs recorded {}",
                c.name, c.ram_hash
            ));
        }
        checked += 1;
    }
    Ok(checked)
}
