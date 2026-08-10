//! GBA key bits, as the *game* reads them.
//!
//! Values transcribed from `include/gba/io_reg.h` in the decomp (the "// keys"
//! block, `A_BUTTON` through `KEYS_MASK`). This is the bit order libmgba's
//! `setKeys` wants and the order the canonical input log stores.
//!
//! It is emphatically *not* the `.bk2` Input Log column order, which is
//! BizHawk's and is not derivable from anything mounted in this sandbox. Any
//! `.bk2` writer must map from these bits explicitly.

use std::fmt;

pub const A: u16 = 0x0001;
pub const B: u16 = 0x0002;
pub const SELECT: u16 = 0x0004;
pub const START: u16 = 0x0008;
pub const RIGHT: u16 = 0x0010;
pub const LEFT: u16 = 0x0020;
pub const UP: u16 = 0x0040;
pub const DOWN: u16 = 0x0080;
pub const R: u16 = 0x0100;
pub const L: u16 = 0x0200;

/// `KEYS_MASK`. Bits outside this are not key bits.
pub const MASK: u16 = 0x03FF;

/// In decomp bit order, least significant first.
pub const ALL: [(&str, u16); 10] = [
    ("A", A),
    ("B", B),
    ("SELECT", SELECT),
    ("START", START),
    ("RIGHT", RIGHT),
    ("LEFT", LEFT),
    ("UP", UP),
    ("DOWN", DOWN),
    ("R", R),
    ("L", L),
];

/// Parses `"A"`, `"A+UP"`, `"start+select+a+b"`, `""` or `"-"` for nothing.
pub fn parse(spec: &str) -> Result<u16, String> {
    let spec = spec.trim();
    if spec.is_empty() || spec == "-" {
        return Ok(0);
    }
    let mut keys = 0u16;
    for part in spec.split(['+', ',']) {
        let name = part.trim();
        if name.is_empty() {
            continue;
        }
        let bit = ALL
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, b)| *b)
            .ok_or_else(|| format!("unknown key {name:?}"))?;
        keys |= bit;
    }
    Ok(keys)
}

/// Renders a mask as `"A+UP"`, or `"-"` when empty. Round-trips with [`parse`].
pub struct Display(pub u16);

impl fmt::Display for Display {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut wrote = false;
        for (name, bit) in ALL {
            if self.0 & bit != 0 {
                if wrote {
                    f.write_str("+")?;
                }
                f.write_str(name)?;
                wrote = true;
            }
        }
        if !wrote {
            f.write_str("-")?;
        }
        Ok(())
    }
}

/// Left+Right and Up+Down held together. Real hardware cannot produce these;
/// BizHawk's mGBA core may or may not filter them, so a route that relies on
/// one is a tier-2 desync risk and worth refusing up front.
pub fn is_impossible_dpad(keys: u16) -> bool {
    (keys & (LEFT | RIGHT)) == (LEFT | RIGHT) || (keys & (UP | DOWN)) == (UP | DOWN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_through_display() {
        for mask in [0, A, A | B, START | SELECT, MASK, UP | LEFT | R] {
            let text = Display(mask).to_string();
            assert_eq!(parse(&text).unwrap(), mask, "{text}");
        }
    }

    #[test]
    fn parse_is_case_insensitive_and_accepts_separators() {
        assert_eq!(parse("a+Up").unwrap(), A | UP);
        assert_eq!(parse("A,UP").unwrap(), A | UP);
        assert_eq!(parse("  ").unwrap(), 0);
        assert_eq!(parse("-").unwrap(), 0);
        assert!(parse("Z").is_err());
    }

    #[test]
    fn every_bit_is_inside_the_decomp_mask() {
        for (name, bit) in ALL {
            assert_eq!(bit & MASK, bit, "{name}");
        }
    }

    #[test]
    fn impossible_dpad_detects_both_axes() {
        assert!(is_impossible_dpad(LEFT | RIGHT));
        assert!(is_impossible_dpad(UP | DOWN));
        assert!(!is_impossible_dpad(UP | LEFT));
        assert!(!is_impossible_dpad(A | B | START));
    }
}
