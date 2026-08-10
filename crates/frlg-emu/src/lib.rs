//! Tier-1 verification harness: drive a headless libmgba GBA core frame by
//! frame, feed it a key mask per frame, read RAM, dump a screenshot.
//!
//! The pieces:
//!
//! - [`emu::Emu`] -- one core, safe wrapper over the C shim in `mgba-sys`.
//! - [`inputlog::InputLog`] -- the canonical artifact, one `u16` per frame.
//! - [`keys`] -- the decomp's key bits, which are what the game reads.
//! - [`syms`] -- `pokefirered.sym`, so watches can be named rather than numeric.
//!
//! Tier 2 (BizHawk replaying a `.bk2`) does not run in this sandbox. Nothing
//! here claims a route is accepted; it only establishes that a log does what it
//! is supposed to do under mGBA.

pub mod emu;
pub mod inputlog;
pub mod keys;
pub mod syms;

pub use emu::{Emu, EmuError, SaveState};
pub use inputlog::{InputLog, LogError};
pub use syms::{SymbolTable, Target};

use std::path::{Path, PathBuf};

use sha1::{Digest, Sha1};

/// sha1 of a file, matching what `sha1sum` prints and what the ROM artifact is
/// identified by.
pub fn file_sha1(path: &Path) -> std::io::Result<[u8; 20]> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buf)?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }
    Ok(hasher.finalize().into())
}

/// A log records the ROM it was routed against; replaying it elsewhere is
/// meaningless. An all-zero hash in the log means "unknown" and is allowed.
pub fn check_log_rom(log: &InputLog, rom_sha1: [u8; 20]) -> Result<(), EmuError> {
    if log.rom_sha1 == [0u8; 20] || log.rom_sha1 == rom_sha1 {
        return Ok(());
    }
    Err(EmuError::RomMismatch {
        expected: hex::encode(log.rom_sha1),
        actual: hex::encode(rom_sha1),
    })
}

/// `$FRLG_ROM`, else `$FRLG_ARTIFACTS/rom/pokefirered.gba`, if it exists.
pub fn default_rom_path() -> Option<PathBuf> {
    artifact_path("FRLG_ROM", "pokefirered.gba")
}

/// `$FRLG_SYM`, else `$FRLG_ARTIFACTS/rom/pokefirered.sym`, if it exists.
pub fn default_sym_path() -> Option<PathBuf> {
    artifact_path("FRLG_SYM", "pokefirered.sym")
}

fn artifact_path(env_var: &str, file: &str) -> Option<PathBuf> {
    if let Ok(explicit) = std::env::var(env_var) {
        let path = PathBuf::from(explicit);
        return path.is_file().then_some(path);
    }
    let artifacts = std::env::var("FRLG_ARTIFACTS").ok()?;
    let path = PathBuf::from(artifacts).join("rom").join(file);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_log_rom_allows_unknown_but_rejects_mismatch() {
        let rom = [0x11u8; 20];

        let unknown = InputLog::new([0u8; 20], vec![0]);
        assert!(check_log_rom(&unknown, rom).is_ok());

        let matching = InputLog::new(rom, vec![0]);
        assert!(check_log_rom(&matching, rom).is_ok());

        let other = InputLog::new([0x22u8; 20], vec![0]);
        assert!(matches!(
            check_log_rom(&other, rom),
            Err(EmuError::RomMismatch { .. })
        ));
    }
}
