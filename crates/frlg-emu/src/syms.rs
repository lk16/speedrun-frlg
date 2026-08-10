//! `pokefirered.sym`, as produced by `make syms`.
//!
//! Each line is `AAAAAAAA t SSSSSSSS name` -- address, symbol type letter,
//! size, name -- per the `objdump | perl` rule in the decomp's Makefile. This
//! is what turns "the simulation diverged somewhere" into "`gRngValue` differs
//! at frame N".

use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Symbol {
    pub addr: u32,
    pub size: u32,
}

#[derive(Debug, Default, Clone)]
pub struct SymbolTable {
    by_name: HashMap<String, Symbol>,
    /// `(addr, size, name)`, sorted by address, for the reverse lookup.
    by_addr: Vec<(u32, u32, String)>,
}

/// An address and a byte length to read there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub addr: u32,
    pub len: u32,
}

impl SymbolTable {
    pub fn parse(text: &str) -> Self {
        let mut by_name = HashMap::new();
        for line in text.lines() {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() != 4 {
                continue;
            }
            let (Ok(addr), Ok(size)) = (
                u32::from_str_radix(fields[0], 16),
                u32::from_str_radix(fields[2], 16),
            ) else {
                continue;
            };
            // The sym file is `sort -u`'d, so a name can appear more than once
            // with different sizes; the first entry wins and stays stable.
            by_name
                .entry(fields[3].to_string())
                .or_insert(Symbol { addr, size });
        }
        let mut by_addr: Vec<(u32, u32, String)> = by_name
            .iter()
            .map(|(name, sym)| (sym.addr, sym.size, name.clone()))
            .collect();
        // Ties are broken by the larger symbol first, then by name, so the
        // reverse lookup is deterministic where several names share an address
        // (the sym file has plenty: a function and the label at its entry).
        by_addr.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)).then(a.2.cmp(&b.2)));
        Self { by_name, by_addr }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        Ok(Self::parse(&fs::read_to_string(path)?))
    }

    pub fn get(&self, name: &str) -> Option<Symbol> {
        self.by_name.get(name).copied()
    }

    pub fn len(&self) -> usize {
        self.by_name.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// The symbol covering `addr`, and how far into it the address is.
    ///
    /// Thumb function pointers carry bit 0 set (`gMain.callback2` is one), so
    /// that bit is cleared before the search -- otherwise every callback lands
    /// one byte past its own symbol. A symbol with a recorded size of 0 covers
    /// only its own address; anything else would have this claim the rest of
    /// the ROM.
    pub fn covering(&self, addr: u32) -> Option<(&str, u32)> {
        let addr = addr & !1;
        let idx = self.by_addr.partition_point(|(a, _, _)| *a <= addr);
        let (sym_addr, size, name) = self.by_addr.get(idx.checked_sub(1)?)?;
        let offset = addr - sym_addr;
        (offset < (*size).max(1)).then_some((name.as_str(), offset))
    }

    /// `covering`, rendered as `CB2_MainMenu` or `CB2_MainMenu+0x4`, falling
    /// back to the bare address when nothing covers it.
    pub fn describe(&self, addr: u32) -> String {
        match self.covering(addr) {
            Some((name, 0)) => name.to_string(),
            Some((name, off)) => format!("{name}+{off:#x}"),
            None => format!("{addr:#010x}"),
        }
    }

    /// Names containing `needle`, case-insensitively, sorted.
    pub fn search(&self, needle: &str) -> Vec<(String, Symbol)> {
        let needle = needle.to_lowercase();
        let mut hits: Vec<(String, Symbol)> = self
            .by_name
            .iter()
            .filter(|(name, _)| name.to_lowercase().contains(&needle))
            .map(|(name, sym)| (name.clone(), *sym))
            .collect();
        hits.sort_by(|a, b| a.0.cmp(&b.0));
        hits
    }

    /// Resolves `gRngValue`, `gRngValue:4`, `gMain+0x10:2`, `0x03005000:4`.
    ///
    /// Without an explicit `:len`, a symbol uses its recorded size (capped, so
    /// a huge symbol does not silently dump megabytes) and a bare address uses
    /// 4 bytes.
    pub fn resolve(&self, spec: &str) -> Result<Target, String> {
        const DEFAULT_MAX: u32 = 64;

        let (locus, explicit_len) = match spec.rsplit_once(':') {
            Some((locus, len)) => {
                let len = parse_int(len).ok_or_else(|| format!("bad length in {spec:?}"))?;
                (locus, Some(len))
            }
            None => (spec, None),
        };

        let (base, offset) = match locus.split_once('+') {
            Some((base, off)) => (
                base.trim(),
                parse_int(off.trim()).ok_or_else(|| format!("bad offset in {spec:?}"))?,
            ),
            None => (locus.trim(), 0),
        };

        let (addr, natural_len) = if let Some(sym) = self.get(base) {
            (sym.addr, sym.size.clamp(1, DEFAULT_MAX))
        } else if let Some(addr) = parse_int(base) {
            (addr, 4)
        } else {
            return Err(format!("{base:?} is not a known symbol or an address"));
        };

        let len = explicit_len.unwrap_or(natural_len);
        if len == 0 {
            return Err(format!("zero length in {spec:?}"));
        }
        Ok(Target {
            addr: addr.wrapping_add(offset),
            len,
        })
    }
}

fn parse_int(text: &str) -> Option<u32> {
    let text = text.trim();
    match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
02023be4 g 00000160 gBattleMons
03005000 g 00000004 gRngValue
030030f0 g 0000043c gMain
02024284 g 00000258 gPlayerParty
garbage line
";

    fn table() -> SymbolTable {
        SymbolTable::parse(SAMPLE)
    }

    #[test]
    fn parses_and_skips_junk() {
        let t = table();
        assert_eq!(t.len(), 4);
        assert_eq!(
            t.get("gRngValue"),
            Some(Symbol {
                addr: 0x0300_5000,
                size: 4
            })
        );
        assert_eq!(t.get("nope"), None);
    }

    #[test]
    fn resolves_symbols_addresses_offsets_and_lengths() {
        let t = table();
        assert_eq!(
            t.resolve("gRngValue").unwrap(),
            Target {
                addr: 0x0300_5000,
                len: 4
            }
        );
        assert_eq!(
            t.resolve("gRngValue:2").unwrap(),
            Target {
                addr: 0x0300_5000,
                len: 2
            }
        );
        assert_eq!(
            t.resolve("gMain+0x10:2").unwrap(),
            Target {
                addr: 0x0300_3100,
                len: 2
            }
        );
        assert_eq!(
            t.resolve("0x02000000").unwrap(),
            Target {
                addr: 0x0200_0000,
                len: 4
            }
        );
        assert_eq!(
            t.resolve("0x02000000:16").unwrap(),
            Target {
                addr: 0x0200_0000,
                len: 16
            }
        );
    }

    #[test]
    fn a_large_symbol_does_not_default_to_dumping_everything() {
        let t = table();
        // gMain is 0x43c bytes; the default read is capped.
        assert_eq!(t.resolve("gMain").unwrap().len, 64);
        // ...but asking explicitly still works.
        assert_eq!(t.resolve("gMain:0x43c").unwrap().len, 0x43c);
    }

    #[test]
    fn rejects_unknown_names_and_zero_lengths() {
        let t = table();
        assert!(t.resolve("gNotAThing").is_err());
        assert!(t.resolve("gRngValue:0").is_err());
        assert!(t.resolve("gRngValue:xyz").is_err());
    }

    #[test]
    fn covering_finds_the_symbol_an_address_falls_inside() {
        let t = table();
        assert_eq!(t.covering(0x0300_3100), Some(("gMain", 0x10)));
        assert_eq!(t.covering(0x0300_5000), Some(("gRngValue", 0)));
        // Past the end of gRngValue (4 bytes) and before the next symbol.
        assert_eq!(t.covering(0x0300_5004), None);
        assert_eq!(t.covering(0x0000_0000), None);
    }

    #[test]
    fn covering_ignores_the_thumb_bit_so_callbacks_resolve() {
        // gMain.callback2 holds a Thumb pointer, i.e. the entry address | 1.
        let t = SymbolTable::parse("0800c2d4 l 00000016 CB2_MainMenu\n");
        assert_eq!(t.covering(0x0800_c2d5), Some(("CB2_MainMenu", 0)));
        assert_eq!(t.describe(0x0800_c2d5), "CB2_MainMenu");
        assert_eq!(t.describe(0x0800_c2db), "CB2_MainMenu+0x6");
        assert_eq!(t.describe(0x0800_0000), "0x08000000");
    }

    #[test]
    fn search_is_case_insensitive_and_sorted() {
        let hits = table().search("rng");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, "gRngValue");
        assert_eq!(table().search("g").len(), 4);
    }
}
