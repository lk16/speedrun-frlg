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
use frlg_route::{ledger, Starter};

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
    /// Print the ledger as it stands, without running anything.
    Status {
        #[arg(long, default_value = "route/ledger.json")]
        ledger: PathBuf,
    },
}

#[derive(Args)]
struct RouteArgs {
    #[command(flatten)]
    rom: RomArgs,

    #[arg(long)]
    sym: Option<PathBuf>,

    /// Which starter to route. The rival always takes the one that beats it.
    #[arg(long, default_value = "squirtle")]
    starter: String,

    /// Where the per-segment input logs go.
    #[arg(long, default_value = "route/logs")]
    logs: PathBuf,

    #[arg(long, default_value = "route/ledger.json")]
    ledger: PathBuf,

    /// Checkpoint savestates. Defaults to $FRLG_ARTIFACTS/states/route, and is
    /// skipped when that is not set -- they are convenience, not evidence.
    #[arg(long)]
    states: Option<PathBuf>,

    /// Write the ledger back with what this run established. `verify` only
    /// fills in tier-1 status, which is the only thing a replay can prove.
    #[arg(long)]
    write: bool,
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

fn cmd_route(command: RouteCommand) -> Result<()> {
    match command {
        RouteCommand::Build(args) => {
            let (rom, sym, starter) = route_setup(&args)?;
            let paths = ledger::Paths {
                logs: args.logs.clone(),
                ledger: args.ledger.clone(),
                states: args.states.clone().or_else(default_states_dir),
            };
            let built = ledger::build(&rom, &sym, starter, &paths, |line| println!("{line}"))?;
            println!("\n{} frames total", built.total_frames);
            println!("wrote {}", args.ledger.display());
            println!("tier 1 is claimed by `frlg route verify`, not by this command");
            Ok(())
        }
        RouteCommand::Verify(args) => {
            let (rom, sym, starter) = route_setup(&args)?;
            let recorded = ledger::read(&args.ledger)
                .with_context(|| format!("reading {}", args.ledger.display()))?;
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
                ledger::write(&checked, &args.ledger)?;
                println!("wrote {}", args.ledger.display());
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
        RouteCommand::Status { ledger: path } => {
            let led = ledger::read(&path).with_context(|| format!("reading {}", path.display()))?;
            println!("rom     {}", led.rom_sha1);
            println!("starter {}", led.starter);
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
    }
}

fn route_setup(args: &RouteArgs) -> Result<(PathBuf, PathBuf, Starter)> {
    let rom = resolve_rom(&args.rom)?;
    let sym = match &args.sym {
        Some(path) => path.clone(),
        None => frlg_emu::default_sym_path().context(
            "no pokefirered.sym found: pass --sym, or set $FRLG_SYM, or copy it \
             into $FRLG_ARTIFACTS/rom",
        )?,
    };
    let starter = match args.starter.to_lowercase().as_str() {
        "bulbasaur" => Starter::Bulbasaur,
        "squirtle" => Starter::Squirtle,
        "charmander" => Starter::Charmander,
        other => bail!("unknown starter {other:?}: bulbasaur, squirtle or charmander"),
    };
    Ok((rom, sym, starter))
}

fn default_states_dir() -> Option<PathBuf> {
    std::env::var("FRLG_ARTIFACTS")
        .ok()
        .map(|dir| PathBuf::from(dir).join("states").join("route"))
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

    println!("rom          {}", rom.display());
    println!("sha1         {}", hex::encode(sha1));
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
