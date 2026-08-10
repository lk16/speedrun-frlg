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
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::Info(args) => cmd_info(args),
        Command::Run(args) => cmd_run(args),
        Command::Sym(args) => cmd_sym(args),
        Command::Log(args) => cmd_log(args),
    }
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
    }
}
