//! The route ledger: what has been routed, what it costs, and what proves it.
//!
//! The ledger is both the objective function and the memory. Building it runs
//! the segments; verifying it *re-derives* every claim from the committed input
//! logs by replaying them from reset in a single pass. A verifier that trusted
//! the builder's numbers would only be able to tell you that the builder was
//! self-consistent.

use std::fs;
use std::path::{Path, PathBuf};

use frlg_emu::{Emu, InputLog, SymbolTable};
use serde::{Deserialize, Serialize};

use crate::observe::Observer;
use crate::record::{Recorder, RouteError};
use crate::segments::{self, Segment, Starter, Target, Tuning};

/// Tier 2 is a BizHawk replay on the host; nothing in this sandbox can do it.
/// `frlg route export` writes the `.bk2` and queues it, but a queued request
/// is not a result: this stays the recorded state until a verdict from
/// `tools/verify-runner.sh` lands in `verify/results` and is written back.
/// Every entry says so out loud rather than leaving the field empty and
/// letting a reader assume.
pub const TIER2_BLOCKED: &str =
    "not replayed: queue with `frlg route export`, host runs tools/verify-runner.sh (docs/rival-1/route.md)";

#[derive(Debug, Serialize, Deserialize)]
pub struct Ledger {
    /// The ROM every log below was routed against.
    pub rom_sha1: String,
    /// How the core was booted when these logs were made: `"hle"` (mGBA's
    /// high-level BIOS, the only option until a real BIOS exists on the
    /// host), or `"bios+intro:<sha1>"` (real BIOS, boot animation played --
    /// the only boot BizHawk uses for a movie, since replaying a `.bk2`
    /// requests deterministic emulation and `MGBAHawk.cs:41` then overrides
    /// `SkipBios`). The retired `"bios:<sha1>"` meant intro-skipped and is
    /// refused like any other mismatch. Deliberately not `#[serde(default)]`,
    /// like `tuning`: the boot changes both SWI timing and the intro length,
    /// so logs replayed under another boot are not the same evidence.
    /// `verify` refuses a mismatch.
    pub bios: String,
    /// Which TAS these logs belong to (`segments::Target`). `#[serde(default)]`
    /// to "rival-1" is safe here, unlike `tuning`: every ledger written before
    /// the field existed *was* a rival-1 ledger.
    #[serde(default = "target_compat")]
    pub target: String,
    pub starter: String,
    /// The route-level knobs this build used. Recorded so `verify` rebuilds the
    /// same segment definitions the logs were made against, and so a sweep's
    /// answer is not folded back into the code as a magic number.
    ///
    /// Deliberately not `#[serde(default)]`: a ledger written before this field
    /// existed would then be read back with a tuning it was not built with, and
    /// silently claim knob values that are not what produced its logs. Failing
    /// to parse is the better outcome -- rebuild it.
    pub tuning: Tuning,
    pub total_frames: usize,
    pub segments: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub goal: String,
    /// The segment this one starts from, or null for the one that starts at
    /// power-on.
    pub parent: Option<String>,
    /// Repo-relative path of the segment's input log.
    pub log: String,
    /// `sha1` of the log's frame payload -- its identity in the ledger.
    pub digest: String,
    pub frames: usize,
    /// Frame number, counted from reset, at which this segment starts.
    pub start_frame: usize,
    /// EWRAM+IWRAM fingerprint at the end of the segment.
    pub ram_hash: String,
    /// Whether a replay from reset reached this segment's observable.
    pub tier1: bool,
    pub tier2: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
    #[error(transparent)]
    Route(#[from] RouteError),
    #[error(transparent)]
    Emu(#[from] frlg_emu::EmuError),
    #[error(transparent)]
    Log(#[from] frlg_emu::LogError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("segment {name} ran to completion but did not reach its goal: {goal}")]
    NotReached { name: String, goal: String },
    #[error("{0}")]
    Message(String),
}

fn target_compat() -> String {
    Target::Rival1.name().to_string()
}

/// Where a build writes.
pub struct Paths {
    /// Directory for the committed input logs, e.g. `route/rival-1/logs`.
    pub logs: PathBuf,
    /// The ledger file, e.g. `route/rival-1/ledger.json`.
    pub ledger: PathBuf,
    /// Checkpoint savestates. These do not survive the sandbox and are not
    /// committed, so a missing directory is not an error -- it just means no
    /// checkpoints.
    pub states: Option<PathBuf>,
}

fn observer(sym: &Path) -> Result<Observer, LedgerError> {
    let syms = SymbolTable::load(sym)?;
    Observer::new(syms).map_err(LedgerError::Message)
}

/// Which version the ROM is, from its header -- the route has one
/// version-dependent beat (`segments::Version`), and guessing it from a
/// filename would be exactly the kind of unforced error the header exists to
/// prevent.
fn version_of(rom: &Path) -> Result<segments::Version, LedgerError> {
    segments::Version::of_rom(rom)?.ok_or_else(|| {
        LedgerError::Message(format!(
            "{} has a game code other than BPRE/BPGE; not a FireRed or LeafGreen ROM",
            rom.display()
        ))
    })
}

/// Run the route, writing one log per segment plus the ledger.
///
/// Segments run in sequence on one emulator, so each starts exactly where the
/// last ended -- which is what makes the per-segment logs concatenate into a
/// single run.
pub fn build(
    rom: &Path,
    sym: &Path,
    target: Target,
    starter: Starter,
    tuning: Tuning,
    paths: &Paths,
    mut progress: impl FnMut(&str),
) -> Result<Ledger, LedgerError> {
    let obs = observer(sym)?;
    let mut rec = Recorder::from_reset(rom)?;
    fs::create_dir_all(&paths.logs)?;
    if let Some(states) = &paths.states {
        fs::create_dir_all(states)?;
    }

    let mut entries: Vec<Entry> = Vec::new();
    let mut consumed = 0usize;
    for segment in target.segments(version_of(rom)?, starter, tuning) {
        let start_frame = rec.frames();
        (segment.run)(&mut rec, &obs)?;
        if !(segment.reached)(&obs, rec.emu()) {
            return Err(LedgerError::NotReached {
                name: segment.name.to_string(),
                goal: segment.goal.clone(),
            });
        }

        // The segment's own frames, split off the running log.
        let whole = rec.log();
        let frames: Vec<u16> = whole.frames[consumed..].to_vec();
        consumed = whole.frames.len();
        let log = InputLog::new(whole.rom_sha1, frames);
        let path = paths.logs.join(format!("{}.ilog", segment.name));
        fs::write(&path, log.encode())?;
        if let Some(states) = &paths.states {
            rec.save_state_file(&states.join(format!("{}.state", segment.name)))?;
        }

        let entry = Entry {
            name: segment.name.to_string(),
            goal: segment.goal.clone(),
            parent: entries.last().map(|e| e.name.clone()),
            log: repo_relative(&path),
            digest: log.digest(),
            frames: log.len(),
            start_frame,
            ram_hash: rec.emu().ram_hash()?,
            // Set by `verify`, which is the only thing entitled to claim it.
            tier1: false,
            tier2: TIER2_BLOCKED.to_string(),
        };
        progress(&format!(
            "{:<16} {:>6} frames  (ends at {})  {}",
            entry.name,
            entry.frames,
            entry.start_frame + entry.frames,
            entry.goal
        ));
        entries.push(entry);
    }

    let ledger = Ledger {
        rom_sha1: hex::encode(rec.log().rom_sha1),
        bios: rec.boot().to_string(),
        target: target.name().to_string(),
        starter: starter.name().to_string(),
        tuning,
        total_frames: rec.frames(),
        segments: entries,
    };
    write(&ledger, &paths.ledger)?;
    Ok(ledger)
}

/// Replay the committed logs from reset and check every claim in the ledger.
///
/// Returns the ledger with `tier1` filled in from what the replay actually did.
pub fn verify(
    rom: &Path,
    sym: &Path,
    starter: Starter,
    ledger: &Ledger,
    mut progress: impl FnMut(&str),
) -> Result<Ledger, LedgerError> {
    let obs = observer(sym)?;
    let rom_sha1 = frlg_emu::file_sha1(rom)?;
    if hex::encode(rom_sha1) != ledger.rom_sha1 {
        return Err(LedgerError::Message(format!(
            "ledger was built against ROM {} but this one is {}",
            ledger.rom_sha1,
            hex::encode(rom_sha1)
        )));
    }

    let target = Target::parse(&ledger.target).ok_or_else(|| {
        LedgerError::Message(format!("ledger names unknown target {:?}", ledger.target))
    })?;
    let defined = target.segments(version_of(rom)?, starter, ledger.tuning);
    if defined.len() != ledger.segments.len() {
        return Err(LedgerError::Message(format!(
            "ledger has {} segments but the route defines {}",
            ledger.segments.len(),
            defined.len()
        )));
    }

    let mut emu = Emu::new(rom)?;
    let boot = frlg_emu::boot_with_default_bios(&mut emu)?;
    if boot != ledger.bios {
        return Err(LedgerError::Message(format!(
            "ledger was built with boot {} but this replay would boot {} -- \
             the logs are not evidence under a different boot; rebuild the route",
            ledger.bios, boot
        )));
    }
    emu.reset();
    let mut checked: Vec<Entry> = Vec::new();
    let mut frame = 0usize;

    for (segment, entry) in defined.iter().zip(&ledger.segments) {
        let (log, ok) = replay_segment(&mut emu, &obs, segment, entry, frame)?;
        frame += log.len();
        progress(&format!(
            "{} {:<16} {:>6} frames  ends f{:<6} {}",
            if ok { "ok  " } else { "FAIL" },
            entry.name,
            log.len(),
            frame,
            entry.goal
        ));
        checked.push(Entry {
            name: entry.name.clone(),
            goal: entry.goal.clone(),
            parent: entry.parent.clone(),
            log: entry.log.clone(),
            digest: log.digest(),
            frames: log.len(),
            start_frame: frame - log.len(),
            ram_hash: emu.ram_hash()?,
            tier1: ok,
            tier2: entry.tier2.clone(),
        });
    }

    Ok(Ledger {
        rom_sha1: ledger.rom_sha1.clone(),
        bios: ledger.bios.clone(),
        target: ledger.target.clone(),
        starter: ledger.starter.clone(),
        tuning: ledger.tuning,
        total_frames: frame,
        segments: checked,
    })
}

/// Replay one segment's log and report whether the game got where the segment
/// says it should.
fn replay_segment(
    emu: &mut Emu,
    obs: &Observer,
    segment: &Segment,
    entry: &Entry,
    frame: usize,
) -> Result<(InputLog, bool), LedgerError> {
    if segment.name != entry.name {
        return Err(LedgerError::Message(format!(
            "ledger segment {} does not match the route's {} at frame {frame}",
            entry.name, segment.name
        )));
    }
    let bytes = fs::read(&entry.log)?;
    let log = InputLog::decode(&bytes)?;
    if log.digest() != entry.digest {
        return Err(LedgerError::Message(format!(
            "{} does not match its ledger digest: {} on disk, {} recorded",
            entry.log,
            log.digest(),
            entry.digest
        )));
    }
    log.validate()?;
    emu.replay(&log, |_, _| {});
    Ok((log, (segment.reached)(obs, emu)))
}

pub fn write(ledger: &Ledger, path: &Path) -> Result<(), LedgerError> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut text = serde_json::to_string_pretty(ledger)?;
    text.push('\n');
    fs::write(path, text)?;
    Ok(())
}

pub fn read(path: &Path) -> Result<Ledger, LedgerError> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

/// Ledger paths are repo-relative so the file is the same on every machine.
fn repo_relative(path: &Path) -> String {
    let cwd = std::env::current_dir().unwrap_or_default();
    path.strip_prefix(&cwd)
        .unwrap_or(path)
        .display()
        .to_string()
}
