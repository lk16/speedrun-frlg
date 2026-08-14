//! `frlg` -- drive the tier-1 harness from a shell.
//!
//! Everything this prints describes an mGBA run. It says nothing about whether
//! BizHawk agrees; that is tier 2, and it does not run in this sandbox.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::{Args, Parser, Subcommand};
use frlg_emu::{check_log_rom, keys, Emu, InputLog, SymbolTable, Target};
use frlg_route::segments::Tuning;
use frlg_route::{ledger, RouteError, Starter};

#[derive(Parser)]
#[command(name = "frlg", about = "Headless mGBA harness for the FireRed TAS")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Report what the ROM and the core look like.
    Info(RomArgs),
    /// Replay an input log (or idle frames) and report the result.
    Run(RunArgs),
    /// Look symbols up in pokefirered.sym.
    Sym(SymArgs),
    /// Inspect and convert input logs.
    #[command(subcommand)]
    Log(LogCommand),
    /// Build and verify the route.
    #[command(subcommand)]
    Route(RouteCommand),
}

#[derive(Subcommand)]
enum RouteCommand {
    /// Run the segments, writing an input log each and the ledger.
    Build(RouteArgs),
    /// Replay the committed logs from reset and check the ledger's claims.
    Verify(RouteArgs),
    /// Sweep the route-level knobs end-to-end and keep the fastest run.
    ///
    /// A knob cannot be judged by the segment it sits in: a frame saved before
    /// the rival battle re-rolls the battle, which is worth far more than the
    /// frame. So each variant is built in full and scored on total frames.
    Tune(RouteArgs),
    /// Print the ledger as it stands, without running anything.
    Status {
        /// Which TAS's ledger, when --ledger is not given.
        #[arg(long, default_value = "rival-1")]
        target: String,
        #[arg(long)]
        ledger: Option<PathBuf>,
    },
    /// One line per TAS in the repo, from the committed ledgers.
    ///
    /// Reads `route/<target>/ledger.json` for every target directory. Like
    /// `status` it reports what the ledgers claim without running anything;
    /// replaying is `route verify` (tier 1) and the verify queue (tier 2).
    List {
        /// Directory holding the per-target route directories.
        #[arg(long, default_value = "route")]
        dir: PathBuf,
    },
    /// Export the committed logs as one BizHawk movie for tier 2.
    ///
    /// Concatenates the ledger's segment logs into a single .bk2 built on
    /// route/template.bk2, round-trips it back to key masks, and by default
    /// drops it in $FRLG_ARTIFACTS/verify/queue with a request json for the
    /// host-side runner. Writing the file is not verification: tier 2 has
    /// only happened once a result lands in verify/results.
    Export(ExportArgs),
}

#[derive(Args)]
struct ExportArgs {
    /// Which TAS's ledger, when --ledger is not given.
    #[arg(long, default_value = "rival-1")]
    target: String,
    #[arg(long)]
    ledger: Option<PathBuf>,

    /// The BizHawk-written movie whose container and settings are copied
    /// verbatim; only the input log is replaced.
    #[arg(long, default_value = "route/template.bk2")]
    template: PathBuf,

    /// Explicit output path. When absent the movie is queued in
    /// $FRLG_ARTIFACTS/verify/queue/<id>.bk2 alongside <id>.json.
    #[arg(long)]
    out: Option<PathBuf>,

    #[command(flatten)]
    rom: RomArgs,

    #[arg(long)]
    sym: Option<PathBuf>,
}

#[derive(Args)]
struct RouteArgs {
    #[command(flatten)]
    rom: RomArgs,

    #[arg(long)]
    sym: Option<PathBuf>,

    /// Which TAS to build or verify: "rival-1" or "defeat-brock". Picks the
    /// segment list and the default `route/<target>/` paths.
    #[arg(long, default_value = "rival-1")]
    target: String,

    /// Which starter to route. The rival always takes the one that beats it.
    /// Defaults to what the ledger records, else squirtle -- so a bare
    /// `frlg route verify` follows the committed route across starters.
    #[arg(long)]
    starter: Option<String>,

    /// Where the per-segment input logs go. Defaults to
    /// `route/<target>/logs`.
    #[arg(long)]
    logs: Option<PathBuf>,

    /// The ledger file. Defaults to `route/<target>/ledger.json`.
    #[arg(long)]
    ledger: Option<PathBuf>,

    /// Checkpoint savestates. Defaults to $FRLG_ARTIFACTS/states/route, and is
    /// skipped when that is not set -- they are convenience, not evidence.
    #[arg(long)]
    states: Option<PathBuf>,

    /// Write the ledger back with what this run established. `verify` only
    /// fills in tier-1 status, which is the only thing a replay can prove.
    #[arg(long)]
    write: bool,

    /// Frames of UP held to face the starter's ball. Defaults to the ledger's
    /// value if one is there, else the built-in default; `tune` sweeps it.
    #[arg(long)]
    turn_hold: Option<usize>,

    /// Frames A/B is held per one-frame release in dialogue mashes (1 = the
    /// plain press-release mash). Held frames print text at full speed
    /// (decompiled/src/text.c:639-650). Defaults like --turn-hold; `tune`
    /// sweeps it.
    #[arg(long)]
    text_hold: Option<usize>,

    /// Frames idled at power-on before the boot mash. Shifts the title-exit
    /// press, which seeds both gRngValue and the wild-encounter LCG
    /// (decompiled/src/title_screen.c:735, src/new_game.c:103) -- one frame
    /// buys a fresh battle-stream family and a fresh wild pass/fail
    /// sequence. Defaults like --turn-hold.
    #[arg(long)]
    seed_delay: Option<usize>,

    /// Build only: resume after an existing ledger's prefix. The segments
    /// before this one are replayed from the committed logs (seconds) and
    /// only this one onward are rebuilt. Requires the same starter and
    /// tuning as the ledger.
    #[arg(long)]
    from: Option<String>,
}

#[derive(Args)]
struct RomArgs {
    /// ROM path. Defaults to $FRLG_ROM, then $FRLG_ARTIFACTS/rom/pokefirered.gba.
    #[arg(long)]
    rom: Option<PathBuf>,
}

#[derive(Args)]
struct RunArgs {
    #[command(flatten)]
    rom: RomArgs,

    /// Symbol table, so --watch and --trace can use names. Defaults to
    /// $FRLG_SYM, then $FRLG_ARTIFACTS/rom/pokefirered.sym.
    #[arg(long)]
    sym: Option<PathBuf>,

    /// Input log to replay (binary .ilog, or .txt in the run-length text form).
    #[arg(long)]
    input: Option<PathBuf>,

    /// Run this many frames. With --input, truncates the log; without one,
    /// idles for this many frames.
    #[arg(long)]
    frames: Option<usize>,

    /// Start from this savestate file instead of a fresh reset.
    #[arg(long)]
    load_state: Option<PathBuf>,

    /// Write a savestate file when the run ends.
    #[arg(long)]
    save_state: Option<PathBuf>,

    /// Report this address after the run. `gRngValue`, `gMain+0x10:2`,
    /// `0x03005000:4`. Repeatable.
    #[arg(long = "watch", value_name = "SPEC")]
    watches: Vec<String>,

    /// Sample this address every frame into a CSV. Repeatable.
    #[arg(long = "trace", value_name = "SPEC")]
    traces: Vec<String>,

    /// Where the --trace CSV goes. Defaults to stdout.
    #[arg(long)]
    trace_out: Option<PathBuf>,

    /// Write a PNG of the final frame.
    #[arg(long)]
    png: Option<PathBuf>,

    /// Print the EWRAM+IWRAM fingerprint. This is the divergence check.
    #[arg(long)]
    ram_hash: bool,
}

#[derive(Args)]
struct SymArgs {
    #[arg(long)]
    sym: Option<PathBuf>,
    /// Substring to look for, case-insensitive.
    needle: String,
    /// Cap on how many matches to print.
    #[arg(long, default_value_t = 40)]
    limit: usize,
}

#[derive(Subcommand)]
enum LogCommand {
    /// Print a log's header, digest and key-press summary.
    Show { path: PathBuf },
    /// Binary log -> run-length text.
    ToText {
        path: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Run-length text -> binary log.
    FromText {
        path: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Join segment logs, in order, into one whole-run log.
    Cat {
        /// Logs to join. They must agree on the ROM they were routed against.
        paths: Vec<PathBuf>,
        #[arg(short, long)]
        out: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Info(args) => cmd_info(args),
        Command::Run(args) => cmd_run(args),
        Command::Sym(args) => cmd_sym(args),
        Command::Log(args) => cmd_log(args),
        Command::Route(args) => cmd_route(args),
    }
}

fn parse_target(name: &str) -> Result<frlg_route::Target> {
    frlg_route::Target::parse(name)
        .with_context(|| format!("unknown target {name:?}: rival-1 or defeat-brock"))
}

fn target_ledger(target: frlg_route::Target, explicit: &Option<PathBuf>) -> PathBuf {
    explicit
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("route/{}/ledger.json", target.name())))
}

fn target_logs(target: frlg_route::Target, explicit: &Option<PathBuf>) -> PathBuf {
    explicit
        .clone()
        .unwrap_or_else(|| PathBuf::from(format!("route/{}/logs", target.name())))
}

fn cmd_route(command: RouteCommand) -> Result<()> {
    match command {
        RouteCommand::Build(args) => {
            let target = parse_target(&args.target)?;
            let ledger_path = target_ledger(target, &args.ledger);
            let (rom, sym, starter) = route_setup(&args, &ledger_path)?;
            let paths = ledger::Paths {
                logs: target_logs(target, &args.logs),
                ledger: ledger_path.clone(),
                states: args
                    .states
                    .clone()
                    .or_else(|| default_states_dir(target.name())),
            };
            let tuning = tuning_for(&args, &ledger_path);
            let built = ledger::build_from(
                &rom,
                &sym,
                target,
                starter,
                tuning,
                &paths,
                args.from.as_deref(),
                |line| println!("{line}"),
            )?;
            println!("\n{} frames total", built.total_frames);
            println!("wrote {}", ledger_path.display());
            println!("tier 1 is claimed by `frlg route verify`, not by this command");
            Ok(())
        }
        RouteCommand::Verify(args) => {
            let target = parse_target(&args.target)?;
            let ledger_path = target_ledger(target, &args.ledger);
            let (rom, sym, starter) = route_setup(&args, &ledger_path)?;
            let recorded = ledger::read(&ledger_path)
                .with_context(|| format!("reading {}", ledger_path.display()))?;
            let checked =
                ledger::verify(&rom, &sym, starter, &recorded, |line| println!("{line}"))?;

            let mut drifted = Vec::new();
            for (was, now) in recorded.segments.iter().zip(&checked.segments) {
                if was.ram_hash != now.ram_hash {
                    drifted.push(format!(
                        "{}: ledger says RAM {} but the replay produced {}",
                        now.name, was.ram_hash, now.ram_hash
                    ));
                }
            }
            let failed: Vec<&str> = checked
                .segments
                .iter()
                .filter(|s| !s.tier1)
                .map(|s| s.name.as_str())
                .collect();

            if args.write {
                ledger::write(&checked, &ledger_path)?;
                println!("wrote {}", ledger_path.display());
            }
            for line in &drifted {
                println!("DRIFT {line}");
            }
            if !failed.is_empty() {
                bail!("segments did not reach their goal on replay: {failed:?}");
            }
            if !drifted.is_empty() {
                bail!("the replay diverged from the ledger's recorded RAM fingerprints");
            }
            println!("\n{} frames, tier 1 ok", checked.total_frames);
            Ok(())
        }
        RouteCommand::Tune(args) => {
            let target = parse_target(&args.target)?;
            let ledger_path = target_ledger(target, &args.ledger);
            let (rom, sym, starter) = route_setup(&args, &ledger_path)?;
            let mut best: Option<(Tuning, usize)> = None;
            for tuning in Tuning::variants(tuning_for(&args, &ledger_path)) {
                // Sweep into a scratch directory: a variant that loses must not
                // leave its logs behind claiming to be the route.
                let scratch = std::env::temp_dir().join(format!(
                    "frlg-tune-{}-{}",
                    tuning.turn_hold, tuning.text_hold
                ));
                let paths = ledger::Paths {
                    logs: scratch.join("logs"),
                    ledger: scratch.join("ledger.json"),
                    states: None,
                };
                // A variant whose stream cannot win its battle is an answer
                // -- this knob value loses -- not a reason to abandon the
                // sweep. turn_hold 7 on the 2026-08-12 route was exactly
                // that: none of 64 start delays won.
                match ledger::build(&rom, &sym, target, starter, tuning, &paths, |_| {}) {
                    Ok(built) => {
                        let total = built.total_frames;
                        println!(
                            "turn_hold {:>2}  text_hold {:>2}  {total:>6} frames",
                            tuning.turn_hold, tuning.text_hold
                        );
                        if best.as_ref().is_none_or(|(_, seen)| total < *seen) {
                            best = Some((tuning, total));
                        }
                    }
                    Err(ledger::LedgerError::Route(RouteError::Timeout { what, .. })) => {
                        println!(
                            "turn_hold {:>2}  text_hold {:>2}  loses ({what})",
                            tuning.turn_hold, tuning.text_hold
                        );
                    }
                    Err(other) => return Err(other.into()),
                }
            }
            let (tuning, total) = best.expect("Tuning::variants is not empty");
            println!(
                "\nbest: turn_hold {} text_hold {} at {total} frames",
                tuning.turn_hold, tuning.text_hold
            );

            let paths = ledger::Paths {
                logs: target_logs(target, &args.logs),
                ledger: ledger_path.clone(),
                states: args
                    .states
                    .clone()
                    .or_else(|| default_states_dir(target.name())),
            };
            ledger::build(&rom, &sym, target, starter, tuning, &paths, |_| {})?;
            println!("rebuilt {} with it", ledger_path.display());
            Ok(())
        }
        RouteCommand::Status { target, ledger } => {
            let path = target_ledger(parse_target(&target)?, &ledger);
            let led = ledger::read(&path).with_context(|| format!("reading {}", path.display()))?;
            println!("rom     {}", led.rom_sha1);
            println!("boot    {}", led.bios);
            println!("target  {}", led.target);
            println!("starter {}", led.starter);
            println!(
                "tuning  turn_hold {}  text_hold {}",
                led.tuning.turn_hold, led.tuning.text_hold
            );
            println!("frames  {}", led.total_frames);
            println!();
            for s in &led.segments {
                println!(
                    "{} {:<16} {:>6} frames  f{:<6} {:<8} {}",
                    if s.tier1 { "t1" } else { "--" },
                    s.name,
                    s.frames,
                    s.start_frame,
                    &s.digest[..8],
                    s.goal
                );
            }
            println!(
                "\ntier 2: {}",
                led.segments
                    .first()
                    .map(|s| s.tier2.as_str())
                    .unwrap_or("-")
            );
            Ok(())
        }
        RouteCommand::List { dir } => cmd_list(&dir),
        RouteCommand::Export(args) => cmd_export(args),
    }
}

/// One line per committed TAS. Nothing is replayed, so this reports the
/// ledgers' claims, not fresh evidence.
fn cmd_list(dir: &Path) -> Result<()> {
    // GBA frame rate, so the frame counts mean something on a clock. The
    // route docs publish times at this rate (docs/defeat-brock/route.md).
    const HZ: f64 = 59.7275;

    let mut ledgers = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path().join("ledger.json");
        if path.is_file() {
            let led = ledger::read(&path).with_context(|| format!("reading {}", path.display()))?;
            ledgers.push(led);
        }
    }
    if ledgers.is_empty() {
        bail!(
            "no <target>/ledger.json under {}; run from the repo root or pass --dir",
            dir.display()
        );
    }
    ledgers.sort_by(|a, b| a.target.cmp(&b.target));

    println!(
        "{:<14} {:<10} {:>7} {:>7} {:>4}  {:<5} tier2",
        "target", "starter", "frames", "~time", "segs", "tier1"
    );
    for led in &ledgers {
        let secs = led.total_frames as f64 / HZ;
        let tier1_ok = led.segments.iter().filter(|s| s.tier1).count();
        let tier1 = if tier1_ok == led.segments.len() {
            "yes".to_string()
        } else {
            format!("{tier1_ok}/{}", led.segments.len())
        };
        println!(
            "{:<14} {:<10} {:>7} {:>3}m{:02}s {:>4}  {:<5} {}",
            led.target,
            led.starter,
            led.total_frames,
            secs as u64 / 60,
            secs as u64 % 60,
            led.segments.len(),
            tier1,
            tier2_summary(&led.segments),
        );
    }
    Ok(())
}

/// The tier-2 column: the shared head of the segments' tier2 claims when they
/// all pass, a count when they disagree. The full sentences are in `status`.
fn tier2_summary(segments: &[ledger::Entry]) -> String {
    let passed = segments
        .iter()
        .filter(|s| s.tier2.starts_with("passed"))
        .count();
    if passed == segments.len() {
        let head = segments[0].tier2.split(':').next().unwrap_or("passed");
        if segments
            .iter()
            .all(|s| s.tier2.split(':').next() == Some(head))
        {
            head.to_string()
        } else {
            "passed, in more than one result (see status)".to_string()
        }
    } else if passed == 0 {
        "not replayed".to_string()
    } else {
        format!("{passed}/{} segments passed (see status)", segments.len())
    }
}

fn cmd_export(args: ExportArgs) -> Result<()> {
    use sha1::{Digest, Sha1};

    let ledger_path = target_ledger(parse_target(&args.target)?, &args.ledger);
    let led =
        ledger::read(&ledger_path).with_context(|| format!("reading {}", ledger_path.display()))?;
    let mut rom_sha1 = [0u8; 20];
    hex::decode_to_slice(&led.rom_sha1, &mut rom_sha1)
        .with_context(|| format!("ledger rom_sha1 {:?} is not sha1 hex", led.rom_sha1))?;

    // Re-derive the movie from the committed logs, and only from logs the
    // ledger vouches for: a digest mismatch means the logs on disk are not
    // the ones the ledger describes, and exporting them would put an
    // unverified movie behind a verified-looking id.
    let mut frames = Vec::with_capacity(led.total_frames);
    for entry in &led.segments {
        let log = read_log(Path::new(&entry.log))?;
        if log.digest() != entry.digest {
            bail!(
                "{}: digest {} does not match the ledger's {} -- rebuild or re-verify the route",
                entry.log,
                log.digest(),
                entry.digest
            );
        }
        check_log_rom(&log, rom_sha1)?;
        frames.extend_from_slice(&log.frames);
    }
    if frames.len() != led.total_frames {
        bail!(
            "segments sum to {} frames, the ledger claims {}",
            frames.len(),
            led.total_frames
        );
    }
    let combined = InputLog::new(rom_sha1, frames);
    let ilog_sha1 = combined.digest();
    let id = format!("route-{}f-{}", combined.len(), &ilog_sha1[..12]);

    if led.bios == "hle" {
        eprintln!(
            "warning: this route was built on mGBA's HLE BIOS, but BizHawk replays \
             movies from a real BIOS. Expect a desync: put the BIOS at \
             $BIZHAWK_HOME/Firmware/GBA_bios.rom and rebuild the route first \
             (docs/rival-1/route.md). Exporting anyway -- the movie still exercises the \
             tier-2 pipeline."
        );
    }

    let (out, queued) = match &args.out {
        Some(path) => (path.clone(), false),
        None => {
            let artifacts = std::env::var("FRLG_ARTIFACTS")
                .context("no --out and $FRLG_ARTIFACTS is not set; nowhere to queue")?;
            let queue = PathBuf::from(artifacts).join("verify/queue");
            fs::create_dir_all(&queue).with_context(|| format!("creating {}", queue.display()))?;
            (queue.join(format!("{id}.bk2")), true)
        }
    };

    // The ROM first: the movie's header carries its name and hash, and the
    // trace replay below runs on it. It must be the ledger's ROM -- an
    // export replayed against the wrong version would desync on frame 1 and
    // the error should say why, not where. No --rom means the ledger's sha1
    // finds it, same as `route verify`.
    let rom = match &args.rom.rom {
        Some(path) => path.clone(),
        None => frlg_emu::rom_path_for_sha1(&led.rom_sha1)
            .or_else(frlg_emu::default_rom_path)
            .context("no ROM matching the ledger's rom_sha1: pass --rom or build it into $FRLG_ARTIFACTS/rom")?,
    };
    let rom_file_sha1 = frlg_emu::file_sha1(&rom)?;
    if rom_file_sha1 != rom_sha1 {
        bail!(
            "{} is sha1 {}, but the ledger's logs were routed against {}; \
             pass the matching ROM with --rom",
            rom.display(),
            hex::encode(rom_file_sha1),
            led.rom_sha1
        );
    }
    let rom_name = rom
        .file_stem()
        .and_then(|s| s.to_str())
        .context("the ROM path has no printable file name for the movie header")?;

    let written = frlg_route::bk2::export(&args.template, &combined, rom_name, &out)
        .with_context(|| format!("exporting {}", out.display()))?;
    let bk2_sha1 = hex::encode(Sha1::digest(fs::read(&out)?));

    // Replay the exported route once on tier 1, sampling gRngValue after
    // every frame. The game advances it once per VBlank
    // (decompiled/src/main.c:412 calls Random(); decompiled/src/random.c),
    // so the trace lets tier 2 name the *first* divergent frame instead of
    // reporting "the final fingerprint differs". The replay also re-proves
    // the movie's frames reach the ledger's final fingerprint from reset.
    // Symbols follow the ROM: gRngValue's address differs per version.
    let sym_path = args.sym.clone().or_else(|| {
        let sibling = rom.with_extension("sym");
        sibling.is_file().then_some(sibling)
    });
    let syms = resolve_syms(&sym_path)?;
    let rng = syms
        .get("gRngValue")
        .context("gRngValue is not in the symbol table; pass --sym or set $FRLG_SYM")?;
    let mut emu = Emu::new(&rom)?;
    let boot = frlg_emu::boot_with_default_bios(&mut emu)?;
    if boot != led.bios {
        bail!(
            "this machine boots {boot:?} but the ledger's logs were built on {:?}; \
             the trace would describe a different run",
            led.bios
        );
    }
    let mut trace = Vec::with_capacity(combined.len() * 4);
    emu.replay(&combined, |emu, _| {
        trace.extend_from_slice(&emu.read32(rng.addr).to_le_bytes());
    });
    let final_ram = emu.ram_hash()?;
    if let Some(expected) = led.segments.last().map(|s| s.ram_hash.as_str()) {
        if final_ram != expected {
            bail!(
                "replaying the exported movie ends at fingerprint {final_ram}, \
                 the ledger claims {expected}; the logs and ledger disagree"
            );
        }
    }
    let trace_path = out.with_extension("trace");
    fs::write(&trace_path, &trace).with_context(|| format!("writing {}", trace_path.display()))?;

    // Where the probe lives in BizHawk's memory-domain terms. gRngValue is in
    // IWRAM (COMMON_DATA, decompiled/src/random.c:7); refuse to guess if a
    // future symbol table moves it somewhere this arithmetic does not cover.
    const IWRAM: std::ops::Range<u32> = 0x0300_0000..0x0300_8000;
    if !IWRAM.contains(&rng.addr) {
        bail!(
            "gRngValue at {:#010x} is not in IWRAM; teach export its domain",
            rng.addr
        );
    }
    let trace_json = serde_json::json!({
        "file": trace_path.file_name().and_then(|n| n.to_str()),
        "symbol": "gRngValue",
        "domain": "IWRAM",
        "offset": rng.addr - IWRAM.start,
        "size": 4,
        "frames": combined.len(),
    });

    if queued {
        // The request half of the tier-2 contract (docs/rival-1/route.md): what the
        // sandbox expects of this movie, so the runner's verdict can say
        // more than "it ran".
        let request = serde_json::json!({
            "id": id,
            "frames": combined.len(),
            "ilog_sha1": ilog_sha1,
            "bk2_sha1": bk2_sha1,
            "rom_sha1": led.rom_sha1,
            "bios": led.bios,
            "ram_hash": led.segments.last().map(|s| s.ram_hash.as_str()),
            "goal": led.segments.last().map(|s| s.goal.as_str()),
            "trace": trace_json,
        });
        let request_path = out.with_extension("json");
        fs::write(&request_path, format!("{request:#}\n"))
            .with_context(|| format!("writing {}", request_path.display()))?;
        println!("queued {}", out.display());
        println!("       {}", request_path.display());
    } else {
        println!("wrote {}", out.display());
    }
    println!("frames {written}");
    println!("ilog   {ilog_sha1}");
    println!("bk2    {bk2_sha1}");
    println!("round-trip: the .bk2 decodes back to the exported frames");
    println!("tier 2 is claimed by a result in verify/results, not by this command");
    Ok(())
}

/// The ROM, symbols and starter a route command should use. Explicit flags
/// win; otherwise the committed ledger decides -- it pins its ROM by sha1 and
/// records its starter, and a bare `frlg route verify` must follow the route
/// across versions and starters rather than assume FireRed/Squirtle. Only
/// with no ledger at all do the FireRed defaults apply.
fn route_setup(args: &RouteArgs, ledger_path: &Path) -> Result<(PathBuf, PathBuf, Starter)> {
    let led = ledger::read(ledger_path).ok();

    let rom = match &args.rom.rom {
        Some(path) => path.clone(),
        None => led
            .as_ref()
            .and_then(|l| frlg_emu::rom_path_for_sha1(&l.rom_sha1))
            .or_else(frlg_emu::default_rom_path)
            .context(
                "no ROM found: pass --rom, or set $FRLG_ROM, or build the ROM \
                 and copy it to $FRLG_ARTIFACTS/rom",
            )?,
    };
    let sym = match &args.sym {
        Some(path) => path.clone(),
        None => {
            let sibling = rom.with_extension("sym");
            if sibling.is_file() {
                sibling
            } else {
                frlg_emu::default_sym_path().context(
                    "no .sym found next to the ROM or in $FRLG_ARTIFACTS/rom: \
                     pass --sym or set $FRLG_SYM",
                )?
            }
        }
    };
    let starter_name = match &args.starter {
        Some(name) => name.clone(),
        None => led
            .as_ref()
            .map(|l| l.starter.clone())
            .unwrap_or_else(|| "squirtle".to_string()),
    };
    Ok((rom, sym, parse_starter(&starter_name)?))
}

fn parse_starter(name: &str) -> Result<Starter> {
    match name.to_lowercase().as_str() {
        "bulbasaur" => Ok(Starter::Bulbasaur),
        "squirtle" => Ok(Starter::Squirtle),
        "charmander" => Ok(Starter::Charmander),
        other => bail!("unknown starter {other:?}: bulbasaur, squirtle or charmander"),
    }
}

/// The knobs a build should use: what was asked for, else what the ledger the
/// build is about to overwrite already settled on, else the default.
fn tuning_for(args: &RouteArgs, ledger_path: &Path) -> Tuning {
    let mut tuning = ledger::read(ledger_path)
        .map(|led| led.tuning)
        .unwrap_or_default();
    if let Some(turn_hold) = args.turn_hold {
        tuning.turn_hold = turn_hold;
    }
    if let Some(text_hold) = args.text_hold {
        tuning.text_hold = text_hold;
    }
    if let Some(seed_delay) = args.seed_delay {
        tuning.seed_delay = seed_delay;
    }
    tuning
}

/// Per-target so a defeat-brock build cannot overwrite rival-1's checkpoint
/// states. rival-1 keeps its historical directory name.
fn default_states_dir(target: &str) -> Option<PathBuf> {
    let dir = std::env::var("FRLG_ARTIFACTS").ok()?;
    let sub = match target {
        "rival-1" => "route".to_string(),
        other => format!("route-{other}"),
    };
    Some(PathBuf::from(dir).join("states").join(sub))
}

fn resolve_rom(args: &RomArgs) -> Result<PathBuf> {
    if let Some(path) = &args.rom {
        return Ok(path.clone());
    }
    frlg_emu::default_rom_path().context(
        "no ROM found: pass --rom, or set $FRLG_ROM, or build the ROM and copy \
         it to $FRLG_ARTIFACTS/rom/pokefirered.gba",
    )
}

fn resolve_syms(explicit: &Option<PathBuf>) -> Result<SymbolTable> {
    let path = match explicit {
        Some(path) => Some(path.clone()),
        None => frlg_emu::default_sym_path(),
    };
    match path {
        Some(path) => SymbolTable::load(&path)
            .with_context(|| format!("reading symbols from {}", path.display())),
        // An empty table still resolves numeric addresses, so a missing sym
        // file is only fatal once a name is actually used.
        None => Ok(SymbolTable::default()),
    }
}

fn cmd_info(args: RomArgs) -> Result<()> {
    let rom = resolve_rom(&args)?;
    let sha1 = frlg_emu::file_sha1(&rom).with_context(|| format!("hashing {}", rom.display()))?;
    let mut emu = Emu::new(&rom)?;
    let boot = frlg_emu::boot_with_default_bios(&mut emu)?;

    println!("rom          {}", rom.display());
    println!("sha1         {}", hex::encode(sha1));
    println!("boot         {boot}");
    println!("title        {}", emu.game_title());
    println!("code         {}", emu.game_code());
    println!("rom size     {} bytes", emu.rom_size());
    println!("screen       {}x{}", emu.width(), emu.height());
    println!("state size   {} bytes", emu.state_size());
    println!("frame        {}", emu.frame());
    Ok(())
}

fn cmd_run(args: RunArgs) -> Result<()> {
    let rom = resolve_rom(&args.rom)?;
    let rom_sha1 =
        frlg_emu::file_sha1(&rom).with_context(|| format!("hashing {}", rom.display()))?;
    let syms = resolve_syms(&args.sym)?;

    let watches = resolve_targets(&syms, &args.watches)?;
    let traces = resolve_targets(&syms, &args.traces)?;

    let mut log = match &args.input {
        Some(path) => {
            let log = read_log(path)?;
            check_log_rom(&log, rom_sha1)?;
            log
        }
        None => InputLog::new(rom_sha1, Vec::new()),
    };

    match (args.frames, args.input.is_some()) {
        (Some(frames), true) => {
            if frames > log.frames.len() {
                bail!(
                    "--frames {frames} exceeds the log's {} frames",
                    log.frames.len()
                );
            }
            log.frames.truncate(frames);
        }
        (Some(frames), false) => log.frames = vec![0u16; frames],
        (None, true) => {}
        (None, false) => bail!("nothing to do: pass --input, --frames, or both"),
    }

    for (frame, &mask) in log.frames.iter().enumerate() {
        if keys::is_impossible_dpad(mask) {
            bail!(
                "frame {frame} holds {} -- opposing d-pad directions cannot \
                 happen on hardware and are a tier-2 desync risk",
                keys::Display(mask)
            );
        }
    }

    let mut emu = Emu::new(&rom)?;
    frlg_emu::boot_with_default_bios(&mut emu)?;
    if let Some(state) = &args.load_state {
        emu.load_state_file(state)
            .with_context(|| format!("loading state {}", state.display()))?;
    }

    let mut trace_csv = String::new();
    if !traces.is_empty() {
        let mut header = String::from("frame");
        for spec in &args.traces {
            let _ = write!(header, ",{spec}");
        }
        trace_csv.push_str(&header);
        trace_csv.push('\n');
    }

    let started = emu.frame();
    for (index, &mask) in log.frames.iter().enumerate() {
        emu.step(mask);
        if !traces.is_empty() {
            let mut row = index.to_string();
            for target in &traces {
                let _ = write!(row, ",{}", read_target(&mut emu, *target));
            }
            trace_csv.push_str(&row);
            trace_csv.push('\n');
        }
    }

    if !traces.is_empty() {
        match &args.trace_out {
            Some(path) => {
                write_out(path, trace_csv.as_bytes())?;
                eprintln!("trace -> {}", path.display());
            }
            None => print!("{trace_csv}"),
        }
    }

    if let Some(path) = &args.png {
        emu.write_png(path)
            .with_context(|| format!("writing {}", path.display()))?;
        eprintln!("png -> {}", path.display());
    }
    if let Some(path) = &args.save_state {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        emu.save_state_file(path)
            .with_context(|| format!("writing state {}", path.display()))?;
        eprintln!("state -> {}", path.display());
    }

    println!("frames       {}", log.frames.len());
    println!("digest       {}", log.digest());
    println!("core frame   {} -> {}", started, emu.frame());
    for (spec, target) in args.watches.iter().zip(&watches) {
        println!(
            "watch        {spec} @ {:#010x} = {}",
            target.addr,
            read_target(&mut emu, *target)
        );
    }
    if args.ram_hash {
        println!("ram sha1     {}", emu.ram_hash()?);
    }
    Ok(())
}

fn resolve_targets(syms: &SymbolTable, specs: &[String]) -> Result<Vec<Target>> {
    specs
        .iter()
        .map(|spec| {
            syms.resolve(spec).map_err(|message| {
                if syms.is_empty() {
                    anyhow::anyhow!("{message} (no symbol table loaded -- pass --sym)")
                } else {
                    anyhow::anyhow!("{message}")
                }
            })
        })
        .collect()
}

/// Widths of 1, 2 and 4 read as little-endian integers, since that is what a
/// watch on a variable means. Anything else is a byte dump.
fn read_target(emu: &mut Emu, target: Target) -> String {
    match target.len {
        1 => format!("{:#04x}", emu.read8(target.addr)),
        2 => format!("{:#06x}", emu.read16(target.addr)),
        4 => format!("{:#010x}", emu.read32(target.addr)),
        len => hex::encode(emu.read_bytes(target.addr, len)),
    }
}

fn read_log(path: &Path) -> Result<InputLog> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    // The text form is the exception, so try binary first and fall back only
    // when the magic is absent.
    match InputLog::decode(&bytes) {
        Ok(log) => Ok(log),
        Err(frlg_emu::LogError::BadMagic) => {
            let text = String::from_utf8(bytes).with_context(|| {
                format!("{} is neither a binary nor a text log", path.display())
            })?;
            Ok(InputLog::from_text(&text)?)
        }
        Err(other) => Err(other.into()),
    }
}

fn write_out(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

fn cmd_sym(args: SymArgs) -> Result<()> {
    let syms = resolve_syms(&args.sym)?;
    if syms.is_empty() {
        bail!("no symbol table: pass --sym, or set $FRLG_SYM, or run `make syms`");
    }
    let hits = syms.search(&args.needle);
    for (name, sym) in hits.iter().take(args.limit) {
        println!("{:08x} {:6} {name}", sym.addr, sym.size);
    }
    if hits.len() > args.limit {
        println!("... {} more", hits.len() - args.limit);
    }
    Ok(())
}

fn cmd_log(command: LogCommand) -> Result<()> {
    match command {
        LogCommand::Show { path } => {
            let log = read_log(&path)?;
            println!("frames       {}", log.frames.len());
            println!("digest       {}", log.digest());
            println!("rom sha1     {}", hex::encode(log.rom_sha1));
            let held = log.frames.iter().filter(|&&mask| mask != 0).count();
            println!("frames held  {held}");
            for (name, bit) in keys::ALL {
                let count = log.frames.iter().filter(|&&mask| mask & bit != 0).count();
                if count > 0 {
                    println!("  {name:<7} {count}");
                }
            }
            Ok(())
        }
        LogCommand::ToText { path, out } => {
            let text = read_log(&path)?.to_text();
            match out {
                Some(path) => write_out(&path, text.as_bytes()),
                None => {
                    print!("{text}");
                    Ok(())
                }
            }
        }
        LogCommand::FromText { path, out } => {
            let text =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            let log = InputLog::from_text(&text)?;
            write_out(&out, &log.encode())?;
            println!("{} frames, digest {}", log.frames.len(), log.digest());
            Ok(())
        }
        LogCommand::Cat { paths, out } => {
            if paths.is_empty() {
                bail!("nothing to join");
            }
            let mut frames = Vec::new();
            let mut rom_sha1 = [0u8; 20];
            for path in &paths {
                let log = read_log(path)?;
                // Joining logs routed against different ROMs would produce a
                // file that replays as nonsense, so it is refused. An unknown
                // (all-zero) hash joins with anything.
                if log.rom_sha1 != [0u8; 20] {
                    if rom_sha1 != [0u8; 20] && rom_sha1 != log.rom_sha1 {
                        bail!(
                            "{} was routed against ROM {}, the others against {}",
                            path.display(),
                            hex::encode(log.rom_sha1),
                            hex::encode(rom_sha1)
                        );
                    }
                    rom_sha1 = log.rom_sha1;
                }
                frames.extend_from_slice(&log.frames);
            }
            let joined = InputLog::new(rom_sha1, frames);
            joined.validate()?;
            write_out(&out, &joined.encode())?;
            println!(
                "{} logs, {} frames, digest {}",
                paths.len(),
                joined.frames.len(),
                joined.digest()
            );
            Ok(())
        }
    }
}
