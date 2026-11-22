//! Game ids <-> names, parsed from the decomp's own constant headers
//! (`decompiled/include/constants/{species,moves,items}.h`, `#define NAME number`), so nothing is
//! transcribed by hand.
use std::collections::HashMap;
use std::sync::OnceLock;

pub struct Names {
    by_name: HashMap<String, u16>,
    species: HashMap<u16, String>,
    moves: HashMap<u16, String>,
    items: HashMap<u16, String>,
    flags: HashMap<u16, String>,
}

fn decomp() -> std::path::PathBuf {
    for v in ["FRLG_DECOMP", "FRLG_DECOMP_RO"] {
        if let Some(p) = std::env::var_os(v) {
            let p = std::path::PathBuf::from(p);
            if p.join("include/constants/items.h").exists() {
                return p;
            }
        }
    }
    panic!("no decomp checkout ($FRLG_DECOMP / $FRLG_DECOMP_RO)")
}

fn parse(file: &str, prefix: &str, by_name: &mut HashMap<String, u16>) -> HashMap<u16, String> {
    let text = std::fs::read_to_string(decomp().join("include/constants").join(file))
        .unwrap_or_else(|e| panic!("{file}: {e}"));
    let mut out = HashMap::new();
    for line in text.lines() {
        let mut it = line.split_whitespace();
        if it.next() != Some("#define") {
            continue;
        }
        let Some(name) = it.next() else {
            continue;
        };
        let val: String = it.collect::<Vec<_>>().join(" ");
        // A trailing `// 0x800` comment states the value of an expression this parser cannot
        // evaluate (`#define SYS_FLAGS (TRAINER_FLAGS_END + 1) // 0x800`, `flags.h:1324`).
        let comment_val = val
            .split("//")
            .nth(1)
            .map(str::trim)
            .and_then(|c| c.strip_prefix("0x"))
            .and_then(|h| u16::from_str_radix(h, 16).ok());
        let val = val.split("//").next().unwrap_or_default().trim();
        if !name.starts_with(prefix) || val.is_empty() {
            continue;
        }
        // `N`, `0xN`, or a sum of those and earlier names, e.g. `(SYS_FLAGS + 0x20)`.
        let term = |t: &str, by_name: &HashMap<String, u16>| -> Option<u16> {
            let t = t.trim();
            if let Some(h) = t.strip_prefix("0x") {
                u16::from_str_radix(h, 16).ok()
            } else if t.chars().all(|c| c.is_ascii_digit()) {
                t.parse().ok()
            } else {
                by_name.get(t).copied()
            }
        };
        let v = val
            .trim_start_matches('(')
            .trim_end_matches(')')
            .split('+')
            .map(|t| term(t, by_name))
            .try_fold(0u16, |a, t| t.map(|t| a.wrapping_add(t)))
            .or(comment_val);
        if let Some(v) = v {
            by_name.insert(name.to_string(), v);
            out.entry(v).or_insert_with(|| name.to_string());
        }
    }
    out
}

pub fn names() -> &'static Names {
    static N: OnceLock<Names> = OnceLock::new();
    N.get_or_init(|| {
        let mut by_name = HashMap::new();
        let species = parse("species.h", "SPECIES_", &mut by_name);
        let moves = parse("moves.h", "MOVE_", &mut by_name);
        let items = parse("items.h", "ITEM_", &mut by_name);
        // Flags are defined relative to helpers (`SYS_FLAGS`, `TRAINER_FLAGS_START`, ...) that do
        // not start with FLAG_; read those first.
        parse("flags.h", "", &mut by_name);
        let flags = parse("flags.h", "FLAG_", &mut by_name);
        // Trainer ids (`TRAINER_*`, `decompiled/include/constants/opponents.h`): a trainer's
        // beaten flag is `TRAINER_FLAGS_START` + id (`include/constants/flags.h`).
        parse("opponents.h", "TRAINER_", &mut by_name);
        // Script variables (`VAR_*`, `decompiled/include/constants/vars.h`).
        parse("vars.h", "VAR", &mut by_name);
        Names {
            by_name,
            species,
            moves,
            items,
            flags,
        }
    })
}

impl Names {
    /// The id of `SPECIES_X` / `MOVE_X` / `ITEM_X`; panics on a name the decomp does not have.
    pub fn id(&self, name: &str) -> u16 {
        *self
            .by_name
            .get(name)
            .unwrap_or_else(|| panic!("{name} is not defined in the decomp's constants"))
    }
    pub fn species(&self, id: u16) -> &str {
        self.species.get(&id).map_or("?", String::as_str)
    }
    pub fn mv(&self, id: u16) -> &str {
        self.moves.get(&id).map_or("?", String::as_str)
    }
    pub fn flag(&self, id: u16) -> &str {
        self.flags.get(&id).map_or("?", String::as_str)
    }
    /// The id of a name if the decomp defines it.
    pub fn get(&self, name: &str) -> Option<u16> {
        self.by_name.get(name).copied()
    }
    pub fn item(&self, id: u16) -> &str {
        self.items.get(&id).map_or("?", String::as_str)
    }
}

/// Shorthand: the id of a decomp constant.
pub fn id(name: &str) -> u16 {
    names().id(name)
}
