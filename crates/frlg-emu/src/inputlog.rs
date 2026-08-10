//! The canonical input artifact: one `u16` key mask per frame, in decomp bit
//! order (see [`crate::keys`]).
//!
//! `.bk2` is an *export* of this, not the source of truth, because the `.bk2`
//! Input Log column order cannot be determined in this sandbox. Keeping the raw
//! log canonical means a column-order mistake costs a re-export rather than a
//! re-route.
//!
//! Binary layout, all little-endian:
//!
//! ```text
//! 0x00  8   magic "FRLGILOG"
//! 0x08  4   version = 1
//! 0x0c  4   frame count
//! 0x10 20   sha1 of the ROM this was routed against
//! 0x24  4   reserved, zero
//! 0x28  ..  frame count * u16 key mask
//! ```

use std::fmt::Write as _;

use sha1::{Digest, Sha1};

use crate::keys;

const MAGIC: &[u8; 8] = b"FRLGILOG";
const VERSION: u32 = 1;
const HEADER_LEN: usize = 0x28;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputLog {
    /// sha1 of the ROM the log was routed against. Zero when unknown.
    pub rom_sha1: [u8; 20],
    pub frames: Vec<u16>,
}

#[derive(Debug, thiserror::Error)]
pub enum LogError {
    #[error("not an FRLGILOG file (bad magic)")]
    BadMagic,
    #[error("unsupported log version {0}, this build understands {VERSION}")]
    BadVersion(u32),
    #[error("truncated: header claims {claimed} frames, file holds {actual}")]
    Truncated { claimed: usize, actual: usize },
    #[error("frame {frame} has bits {bits:#06x} outside KEYS_MASK")]
    StrayBits { frame: usize, bits: u16 },
    #[error("line {line}: {message}")]
    Text { line: usize, message: String },
}

impl InputLog {
    pub fn new(rom_sha1: [u8; 20], frames: Vec<u16>) -> Self {
        Self { rom_sha1, frames }
    }

    pub fn len(&self) -> usize {
        self.frames.len()
    }

    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// The ledger identity of this log: sha1 over the frame payload alone, so
    /// it does not move when the header gains fields.
    pub fn digest(&self) -> String {
        let mut hasher = Sha1::new();
        for keys in &self.frames {
            hasher.update(keys.to_le_bytes());
        }
        hex::encode(hasher.finalize())
    }

    /// Rejects masks with bits outside `KEYS_MASK`, which would otherwise reach
    /// `setKeys` and mean something unintended.
    pub fn validate(&self) -> Result<(), LogError> {
        for (frame, &keys) in self.frames.iter().enumerate() {
            let stray = keys & !keys::MASK;
            if stray != 0 {
                return Err(LogError::StrayBits { frame, bits: stray });
            }
        }
        Ok(())
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(HEADER_LEN + self.frames.len() * 2);
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&(self.frames.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.rom_sha1);
        out.extend_from_slice(&0u32.to_le_bytes());
        for keys in &self.frames {
            out.extend_from_slice(&keys.to_le_bytes());
        }
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, LogError> {
        if bytes.len() < HEADER_LEN || &bytes[..8] != MAGIC {
            return Err(LogError::BadMagic);
        }
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if version != VERSION {
            return Err(LogError::BadVersion(version));
        }
        let claimed = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        let mut rom_sha1 = [0u8; 20];
        rom_sha1.copy_from_slice(&bytes[16..36]);

        let payload = &bytes[HEADER_LEN..];
        let actual = payload.len() / 2;
        if actual != claimed {
            return Err(LogError::Truncated { claimed, actual });
        }
        let frames = payload
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();

        let log = Self { rom_sha1, frames };
        log.validate()?;
        Ok(log)
    }

    /// Run-length text form, for review and diffing. One `frame  count  keys`
    /// row per run, `#` comments ignored on read.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "# frlg input log v{VERSION}");
        let _ = writeln!(out, "# rom-sha1 {}", hex::encode(self.rom_sha1));
        let _ = writeln!(out, "# frames {}", self.frames.len());
        let _ = writeln!(out, "# digest {}", self.digest());
        let _ = writeln!(out, "# start\tcount\tkeys");

        let mut frame = 0usize;
        while frame < self.frames.len() {
            let keys = self.frames[frame];
            let mut run = 1usize;
            while frame + run < self.frames.len() && self.frames[frame + run] == keys {
                run += 1;
            }
            let _ = writeln!(out, "{frame}\t{run}\t{}", keys::Display(keys));
            frame += run;
        }
        out
    }

    pub fn from_text(text: &str) -> Result<Self, LogError> {
        let mut rom_sha1 = [0u8; 20];
        let mut frames: Vec<u16> = Vec::new();

        for (index, raw) in text.lines().enumerate() {
            let line = index + 1;
            if let Some(rest) = raw.trim().strip_prefix("# rom-sha1 ") {
                let bytes = hex::decode(rest.trim()).map_err(|e| LogError::Text {
                    line,
                    message: format!("bad rom-sha1: {e}"),
                })?;
                if bytes.len() != 20 {
                    return Err(LogError::Text {
                        line,
                        message: format!("rom-sha1 is {} bytes, want 20", bytes.len()),
                    });
                }
                rom_sha1.copy_from_slice(&bytes);
                continue;
            }
            let body = raw.split('#').next().unwrap_or("").trim();
            if body.is_empty() {
                continue;
            }

            let fields: Vec<&str> = body.split_whitespace().collect();
            if fields.len() != 3 {
                return Err(LogError::Text {
                    line,
                    message: format!("want 3 fields (start count keys), got {}", fields.len()),
                });
            }
            let start: usize = fields[0].parse().map_err(|_| LogError::Text {
                line,
                message: format!("bad start frame {:?}", fields[0]),
            })?;
            let count: usize = fields[1].parse().map_err(|_| LogError::Text {
                line,
                message: format!("bad count {:?}", fields[1]),
            })?;
            let keys = keys::parse(fields[2]).map_err(|message| LogError::Text { line, message })?;

            if start != frames.len() {
                return Err(LogError::Text {
                    line,
                    message: format!("run starts at {start}, expected {}", frames.len()),
                });
            }
            frames.resize(frames.len() + count, keys);
        }

        let log = Self { rom_sha1, frames };
        log.validate()?;
        Ok(log)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> InputLog {
        let mut frames = vec![0u16; 40];
        frames[10] = keys::START;
        frames[11] = keys::START;
        frames[20] = keys::A;
        frames[30] = keys::UP | keys::B;
        InputLog::new([0xab; 20], frames)
    }

    #[test]
    fn binary_round_trips() {
        let log = sample();
        let decoded = InputLog::decode(&log.encode()).unwrap();
        assert_eq!(decoded, log);
    }

    #[test]
    fn text_round_trips_including_rom_hash() {
        let log = sample();
        let decoded = InputLog::from_text(&log.to_text()).unwrap();
        assert_eq!(decoded, log);
        assert_eq!(decoded.digest(), log.digest());
    }

    #[test]
    fn empty_log_round_trips() {
        let log = InputLog::new([0; 20], vec![]);
        assert_eq!(InputLog::decode(&log.encode()).unwrap(), log);
        assert_eq!(InputLog::from_text(&log.to_text()).unwrap(), log);
    }

    #[test]
    fn digest_ignores_the_header() {
        let a = InputLog::new([0x00; 20], vec![1, 2, 3]);
        let b = InputLog::new([0xff; 20], vec![1, 2, 3]);
        assert_eq!(a.digest(), b.digest());

        let c = InputLog::new([0x00; 20], vec![1, 2, 4]);
        assert_ne!(a.digest(), c.digest());
    }

    #[test]
    fn rejects_bad_magic_and_version() {
        assert!(matches!(
            InputLog::decode(b"not a log at all, really no"),
            Err(LogError::BadMagic)
        ));

        let mut bytes = sample().encode();
        bytes[8] = 9;
        assert!(matches!(
            InputLog::decode(&bytes),
            Err(LogError::BadVersion(9))
        ));
    }

    #[test]
    fn rejects_truncation() {
        let mut bytes = sample().encode();
        bytes.truncate(bytes.len() - 4);
        assert!(matches!(
            InputLog::decode(&bytes),
            Err(LogError::Truncated { .. })
        ));
    }

    #[test]
    fn rejects_bits_outside_the_key_mask() {
        let log = InputLog::new([0; 20], vec![0x4000]);
        assert!(matches!(log.validate(), Err(LogError::StrayBits { .. })));
        assert!(matches!(
            InputLog::decode(&log.encode()),
            Err(LogError::StrayBits { frame: 0, .. })
        ));
    }

    #[test]
    fn text_rejects_a_gap_in_the_frame_numbering() {
        let err = InputLog::from_text("0\t5\tA\n99\t1\tB\n").unwrap_err();
        assert!(matches!(err, LogError::Text { line: 2, .. }), "{err}");
    }
}
