//! `.bk2` export: the canonical `.ilog` rendered as a BizHawk 2.11.1 movie.
//!
//! The container, header and `SyncSettings` come from `route/template.bk2`
//! **verbatim** -- that file was written by BizHawk's own movie serialiser
//! (`tools/bk2-template.sh`) and settles everything that is not derivable in
//! the sandbox. This module only replaces the `Input Log.txt` entry.
//!
//! Every export is round-tripped before it is reported written: the produced
//! `.bk2` is decoded back to key masks and compared against the source log.
//! A `.bk2` that cannot be decoded to exactly the frames it was written from
//! is not evidence of anything.

use std::fs;
use std::io::{Read, Write as _};
use std::path::Path;

use frlg_emu::{keys, InputLog};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// The `LogKey` this exporter is written against. It is the one fact the
/// column mapping below depends on, so an export against a template whose
/// LogKey differs (a future BizHawk changed its controller definition) must
/// fail instead of silently emitting misaligned columns.
pub const LOGKEY: &str =
    "#Tilt X|Tilt Y|Tilt Z|Light Sensor|Up|Down|Left|Right|Start|Select|B|A|L|R|Power|";

/// The four analogue columns of an idle GBA controller, as BizHawk prints
/// them: `%5d,` each for Tilt X/Y/Z and the light sensor. The route never
/// touches any of them, so both writer and reader treat exactly this prefix
/// as the only valid one.
const ANALOG_IDLE: &str = "|    0,    0,    0,    0,";

/// Button columns in `LogKey` order after the analogue block: the mnemonic
/// character BizHawk prints when the button is held, and the decomp key bit
/// it corresponds to. `Power` is a BizHawk pseudo-button with no GBA key bit;
/// the route cannot press it, and a movie that holds it cannot be represented
/// as an `.ilog`.
///
/// The mnemonics are BizHawk 2.11.1's own, read from
/// `ControllerDefinition.MnemonicsCache` and cross-checked against
/// `Bk2LogEntryGenerator.GenerateLogEntry` with each button pressed alone
/// (mono, host, 2026-08-11) -- not guessed from the button names.
const COLUMNS: [(char, u16); 11] = [
    ('U', keys::UP),
    ('D', keys::DOWN),
    ('L', keys::LEFT),
    ('R', keys::RIGHT),
    ('S', keys::START),
    ('s', keys::SELECT),
    ('B', keys::B),
    ('A', keys::A),
    ('l', keys::L),
    ('r', keys::R),
    ('P', 0),
];

const INPUT_LOG_NAME: &str = "Input Log.txt";

#[derive(Debug, thiserror::Error)]
pub enum Bk2Error {
    #[error("template {path}: {message}")]
    Template { path: String, message: String },
    #[error(
        "template LogKey is not the one this exporter is written against:\n  \
         template: {found}\n  expected: {LOGKEY}\n  \
         (BizHawk's controller definition moved; re-derive the column mapping)"
    )]
    LogKeyMismatch { found: String },
    #[error(
        "the log does not name the ROM it was routed against (all-zero \
         rom_sha1); a movie header cannot be written from it"
    )]
    UnknownRom,
    #[error("frame {frame}: {message}")]
    Frame { frame: usize, message: String },
    #[error("{path}: {message}")]
    Decode { path: String, message: String },
    #[error(
        "round-trip failed: the written .bk2 decodes to different frames \
         (first difference at frame {frame}); {path} was removed"
    )]
    RoundTrip { frame: usize, path: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
}

/// Renders one frame's key mask as a `.bk2` input line.
fn line(mask: u16, frame: usize) -> Result<String, Bk2Error> {
    let stray = mask & !keys::MASK;
    if stray != 0 {
        return Err(Bk2Error::Frame {
            frame,
            message: format!("bits {stray:#06x} outside KEYS_MASK"),
        });
    }
    let mut out = String::with_capacity(ANALOG_IDLE.len() + COLUMNS.len() + 1);
    out.push_str(ANALOG_IDLE);
    for (mnemonic, bit) in COLUMNS {
        out.push(if bit != 0 && mask & bit != 0 {
            mnemonic
        } else {
            '.'
        });
    }
    out.push('|');
    Ok(out)
}

/// Parses one `.bk2` input line back to a key mask. Strict: anything this
/// writer would not have produced is an error, not a guess.
fn parse_line(line: &str, frame: usize) -> Result<u16, Bk2Error> {
    let frame_err = |message: String| Bk2Error::Frame { frame, message };
    let rest = line.strip_prefix(ANALOG_IDLE).ok_or_else(|| {
        frame_err(format!(
            "line does not start with the idle analogue block {ANALOG_IDLE:?}: {line:?}"
        ))
    })?;
    let rest = rest
        .strip_suffix('|')
        .ok_or_else(|| frame_err(format!("line does not end with '|': {line:?}")))?;
    let cells: Vec<char> = rest.chars().collect();
    if cells.len() != COLUMNS.len() {
        return Err(frame_err(format!(
            "{} button cells, expected {}: {line:?}",
            cells.len(),
            COLUMNS.len()
        )));
    }
    let mut mask = 0u16;
    for (cell, (mnemonic, bit)) in cells.into_iter().zip(COLUMNS) {
        if cell == '.' {
            continue;
        }
        if cell != mnemonic {
            return Err(frame_err(format!(
                "unexpected {cell:?} in the {mnemonic:?} column: {line:?}"
            )));
        }
        if bit == 0 {
            return Err(frame_err(
                "the movie presses Power, which no .ilog can represent".into(),
            ));
        }
        mask |= bit;
    }
    Ok(mask)
}

/// Renders the whole `Input Log.txt` entry, LF-terminated like the template.
fn render_input_log(log: &InputLog) -> Result<String, Bk2Error> {
    let mut out = String::new();
    out.push_str("[Input]\n");
    out.push_str("LogKey:");
    out.push_str(LOGKEY);
    out.push('\n');
    for (frame, &mask) in log.frames.iter().enumerate() {
        out.push_str(&line(mask, frame)?);
        out.push('\n');
    }
    out.push_str("[/Input]\n");
    Ok(out)
}

fn template_err(path: &Path, message: impl Into<String>) -> Bk2Error {
    Bk2Error::Template {
        path: path.display().to_string(),
        message: message.into(),
    }
}

/// Writes `log` as a `.bk2` at `out`, copying every entry of `template`
/// verbatim except `Input Log.txt` (the movie itself) and the two
/// ROM-identity lines of `Header.txt`, then decodes the result back and
/// compares. On success returns the number of frames written.
///
/// The header rewrite is what lets one committed template serve both
/// versions: BizHawk stamps the loaded ROM's name and hash into a recorded
/// movie's `Header.txt` (`GameName`, `SHA1` -- see `route/template.bk2`),
/// and a replayed movie is checked against the loaded ROM by that hash. The
/// log knows exactly which ROM it was routed against, so the export writes
/// *that* identity rather than refusing anything the template was not
/// recorded on. Everything else in the header -- and every other entry,
/// SyncSettings.json above all -- is still copied byte-for-byte.
pub fn export(
    template: &Path,
    log: &InputLog,
    rom_name: &str,
    out: &Path,
) -> Result<usize, Bk2Error> {
    let mut archive = ZipArchive::new(
        fs::File::open(template)
            .map_err(|e| template_err(template, format!("cannot open: {e}")))?,
    )?;

    // The template's own log key, from its Input Log.txt. Checked before
    // anything is written: a LogKey drift means the column mapping is wrong.
    {
        let mut entry = archive
            .by_name(INPUT_LOG_NAME)
            .map_err(|_| template_err(template, format!("no {INPUT_LOG_NAME} entry")))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        let found = text
            .lines()
            .find_map(|l| l.strip_prefix("LogKey:"))
            .ok_or_else(|| template_err(template, "no LogKey line in its Input Log.txt"))?;
        if found != LOGKEY {
            return Err(Bk2Error::LogKeyMismatch {
                found: found.to_string(),
            });
        }
    }

    // A movie header that names no ROM is a movie nobody can check; refuse
    // to write one.
    if log.rom_sha1 == [0u8; 20] {
        return Err(Bk2Error::UnknownRom);
    }
    let header = {
        let mut entry = archive
            .by_name("Header.txt")
            .map_err(|_| template_err(template, "no Header.txt entry"))?;
        let mut text = String::new();
        entry.read_to_string(&mut text)?;
        // Template sanity: both lines must exist to be replaced. BizHawk
        // writes SHA1 in uppercase hex, so the rewrite does too.
        for key in ["SHA1 ", "GameName "] {
            if !text.lines().any(|l| l.starts_with(key)) {
                return Err(template_err(
                    template,
                    format!("no {} line in its Header.txt", key.trim_end()),
                ));
            }
        }
        text.lines()
            .map(|l| {
                if l.starts_with("SHA1 ") {
                    format!("SHA1 {}", hex::encode_upper(log.rom_sha1))
                } else if l.starts_with("GameName ") {
                    format!("GameName {rom_name}")
                } else {
                    l.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    };

    let input_log = render_input_log(log)?;

    let mut writer = ZipWriter::new(fs::File::create(out)?);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        let name = entry.name().to_string();
        writer.start_file(&*name, options)?;
        if name == INPUT_LOG_NAME {
            writer.write_all(input_log.as_bytes())?;
        } else if name == "Header.txt" {
            writer.write_all(header.as_bytes())?;
        } else {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            writer.write_all(&bytes)?;
        }
    }
    writer.finish()?;

    // The round trip is part of the export, not an optional check: a .bk2
    // that does not decode back to its source frames is deleted on the spot.
    let replay = decode(out)?;
    if replay != log.frames {
        let frame = replay
            .iter()
            .zip(&log.frames)
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| replay.len().min(log.frames.len()));
        let path = out.display().to_string();
        let _ = fs::remove_file(out);
        return Err(Bk2Error::RoundTrip { frame, path });
    }
    Ok(log.frames.len())
}

/// Decodes a `.bk2`'s `Input Log.txt` back to per-frame key masks. Strict on
/// purpose; see [`parse_line`].
pub fn decode(path: &Path) -> Result<Vec<u16>, Bk2Error> {
    let decode_err = |message: String| Bk2Error::Decode {
        path: path.display().to_string(),
        message,
    };
    let mut archive = ZipArchive::new(fs::File::open(path)?)?;
    let mut text = String::new();
    archive
        .by_name(INPUT_LOG_NAME)
        .map_err(|_| decode_err(format!("no {INPUT_LOG_NAME} entry")))?
        .read_to_string(&mut text)?;

    let mut lines = text.lines();
    match lines.next() {
        Some("[Input]") => {}
        other => return Err(decode_err(format!("expected [Input], found {other:?}"))),
    }
    match lines.next().and_then(|l| l.strip_prefix("LogKey:")) {
        Some(found) if found == LOGKEY => {}
        Some(found) => {
            return Err(Bk2Error::LogKeyMismatch {
                found: found.to_string(),
            })
        }
        None => return Err(decode_err("expected a LogKey line".into())),
    }

    let mut frames = Vec::new();
    let mut closed = false;
    for line in lines {
        if closed {
            return Err(decode_err(format!("content after [/Input]: {line:?}")));
        }
        if line == "[/Input]" {
            closed = true;
            continue;
        }
        frames.push(parse_line(line, frames.len())?);
    }
    if !closed {
        return Err(decode_err("no [/Input] terminator".into()));
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn template() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../route/template.bk2")
    }

    fn rom_sha1_of_template() -> [u8; 20] {
        // 41CB23D8... from route/template.bk2's Header.txt; tests build logs
        // against it so the ROM check passes without a ROM on disk.
        let mut out = [0u8; 20];
        hex::decode_to_slice("41cb23d8dccc8ebd7c649cd8fbb58eeace6e2fdc", &mut out).unwrap();
        out
    }

    #[test]
    fn masks_round_trip_through_a_real_bk2() {
        let dir = std::env::temp_dir().join("frlg-bk2-roundtrip");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("roundtrip.bk2");
        let frames: Vec<u16> = (0..2048u16)
            .map(|i| {
                // Every button bit pattern inside KEYS_MASK, plus quiet frames.
                // wrapping_mul: i * 37 passes u16::MAX from i = 1772 on, and a
                // debug build panics on the overflow where --release wraps.
                i.wrapping_mul(37) & keys::MASK
            })
            .collect();
        let log = InputLog::new(rom_sha1_of_template(), frames.clone());
        let written = export(&template(), &log, "pokefirered", &out).unwrap();
        assert_eq!(written, frames.len());
        assert_eq!(decode(&out).unwrap(), frames);
        std::fs::remove_file(&out).unwrap();
    }

    #[test]
    fn every_single_button_renders_and_parses() {
        for (_, bit) in COLUMNS.iter().filter(|(_, b)| *b != 0) {
            let rendered = line(*bit, 0).unwrap();
            assert_eq!(parse_line(&rendered, 0).unwrap(), *bit);
        }
    }

    #[test]
    fn a_power_press_is_rejected_not_guessed() {
        let rendered = format!("{ANALOG_IDLE}..........P|");
        let err = parse_line(&rendered, 3).unwrap_err();
        assert!(err.to_string().contains("Power"), "{err}");
    }

    #[test]
    fn the_header_carries_the_logs_rom_not_the_templates() {
        use std::io::Read;

        let dir = std::env::temp_dir().join("frlg-bk2-otherrom");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("otherrom.bk2");
        // A log routed against some other ROM than the template's: the
        // written movie must carry *its* identity, uppercased like BizHawk
        // writes it, with the given name.
        let log = InputLog::new([7u8; 20], vec![0, keys::A]);
        export(&template(), &log, "pokeleafgreen", &out).unwrap();
        let mut archive = ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let mut header = String::new();
        archive
            .by_name("Header.txt")
            .unwrap()
            .read_to_string(&mut header)
            .unwrap();
        assert!(
            header
                .lines()
                .any(|l| l == format!("SHA1 {}", hex::encode_upper([7u8; 20]))),
            "{header}"
        );
        assert!(
            header.lines().any(|l| l == "GameName pokeleafgreen"),
            "{header}"
        );
        std::fs::remove_file(&out).unwrap();
    }

    #[test]
    fn a_log_without_a_rom_refuses_to_export() {
        let dir = std::env::temp_dir().join("frlg-bk2-norom");
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("norom.bk2");
        let log = InputLog::new([0u8; 20], vec![0, keys::A]);
        let err = export(&template(), &log, "pokefirered", &out).unwrap_err();
        assert!(matches!(err, Bk2Error::UnknownRom), "{err}");
        assert!(!out.exists());
    }

    #[test]
    fn misaligned_and_alien_lines_are_errors() {
        assert!(parse_line("|    0,    0,    0,    0,..........|", 0).is_err()); // 10 cells
        assert!(parse_line("|    1,    0,    0,    0,...........|", 0).is_err()); // tilt moved
        assert!(parse_line("|    0,    0,    0,    0,X..........|", 0).is_err());
        // alien char
    }
}
