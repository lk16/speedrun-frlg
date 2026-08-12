//! IVs, natures, stat arithmetic and exp thresholds -- the deterministic
//! half of what creation rolls decide.

/// One mon's six IVs, 0..=31 each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ivs {
    pub hp: u8,
    pub atk: u8,
    pub def: u8,
    pub spe: u8,
    pub spa: u8,
    pub spd: u8,
}

impl Ivs {
    /// The two IV words as `CreateBoxMon` unpacks them
    /// (`decompiled/src/pokemon.c:1836-1852`): word 1 carries HP/Atk/Def in
    /// bits 0-4/5-9/10-14, word 2 carries Speed/SpAtk/SpDef the same way;
    /// bit 15 of each is unused (`MAX_IV_MASK 31`,
    /// `include/constants/pokemon.h:231`).
    pub fn unpack(word1: u16, word2: u16) -> Ivs {
        Ivs {
            hp: (word1 & 31) as u8,
            atk: ((word1 >> 5) & 31) as u8,
            def: ((word1 >> 10) & 31) as u8,
            spe: (word2 & 31) as u8,
            spa: ((word2 >> 5) & 31) as u8,
            spd: ((word2 >> 10) & 31) as u8,
        }
    }
}

/// `sNatureStatTable` (`decompiled/src/pokemon.c:1360-1385`), rows indexed by
/// nature (= PID % 25), columns Attack/Defense/Speed/SpAtk/SpDef.
pub const NATURE_STAT_TABLE: [[i8; 5]; 25] = [
    [0, 0, 0, 0, 0],  // Hardy
    [1, -1, 0, 0, 0], // Lonely
    [1, 0, -1, 0, 0], // Brave
    [1, 0, 0, -1, 0], // Adamant
    [1, 0, 0, 0, -1], // Naughty
    [-1, 1, 0, 0, 0], // Bold
    [0, 0, 0, 0, 0],  // Docile
    [0, 1, -1, 0, 0], // Relaxed
    [0, 1, 0, -1, 0], // Impish
    [0, 1, 0, 0, -1], // Lax
    [-1, 0, 1, 0, 0], // Timid
    [0, -1, 1, 0, 0], // Hasty
    [0, 0, 0, 0, 0],  // Serious
    [0, 0, 1, -1, 0], // Jolly
    [0, 0, 1, 0, -1], // Naive
    [-1, 0, 0, 1, 0], // Modest
    [0, -1, 0, 1, 0], // Mild
    [0, 0, -1, 1, 0], // Quiet
    [0, 0, 0, 0, 0],  // Bashful
    [0, 0, 0, 1, -1], // Rash
    [-1, 0, 0, 0, 1], // Calm
    [0, -1, 0, 0, 1], // Gentle
    [0, 0, -1, 0, 1], // Sassy
    [0, 0, 0, -1, 1], // Careful
    [0, 0, 0, 0, 0],  // Quirky
];

pub const NATURE_NAMES: [&str; 25] = [
    "Hardy", "Lonely", "Brave", "Adamant", "Naughty", "Bold", "Docile", "Relaxed", "Impish", "Lax",
    "Timid", "Hasty", "Serious", "Jolly", "Naive", "Modest", "Mild", "Quiet", "Bashful", "Rash",
    "Calm", "Gentle", "Sassy", "Careful", "Quirky",
];

/// The six computed stats at a level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stats {
    pub hp: u16,
    pub atk: u16,
    pub def: u16,
    pub spe: u16,
    pub spa: u16,
    pub spd: u16,
}

/// `CalculateMonStats` (`decompiled/src/pokemon.c:2102-2170`):
/// HP = `((2·base + iv + ev/4)·level)/100 + level + 10` (`:2130-2131`, never
/// nature-modified); the rest `((2·base + iv + ev/4)·level)/100 + 5`
/// (`CALC_STAT`, `:2093-2100`) then `ModifyStatByNature`
/// (`:5404-5438`: ×110/100 or ×90/100, truncating, applied after).
/// EVs are all 0 for everything this crate routes, so they are not a
/// parameter yet.
pub fn calc_stats(base: &crate::species::BaseStats, ivs: Ivs, level: u8, nature: u8) -> Stats {
    let level = level as i32;
    let hp = ((2 * base.hp as i32 + ivs.hp as i32) * level) / 100 + level + 10;
    let raw = |b: u8, iv: u8| ((2 * b as i32 + iv as i32) * level) / 100 + 5;
    let with_nature = |stat: i32, column: usize| -> i32 {
        match NATURE_STAT_TABLE[nature as usize][column] {
            1 => stat * 110 / 100,
            -1 => stat * 90 / 100,
            _ => stat,
        }
    };
    Stats {
        hp: hp as u16,
        atk: with_nature(raw(base.atk, ivs.atk), 0) as u16,
        def: with_nature(raw(base.def, ivs.def), 1) as u16,
        spe: with_nature(raw(base.spe, ivs.spe), 2) as u16,
        spa: with_nature(raw(base.spa, ivs.spa), 3) as u16,
        spd: with_nature(raw(base.spd, ivs.spd), 4) as u16,
    }
}

/// Growth-rate groups this crate needs
/// (`decompiled/include/constants/pokemon.h:246-251`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Growth {
    MediumFast,
    MediumSlow,
}

/// Cumulative exp to *be* level `n`
/// (`decompiled/src/data/pokemon/experience_tables.h:4-7` macros; the table
/// itself stores these macro applications per level, `:18+`):
/// MEDIUM_FAST = n³, MEDIUM_SLOW = (6n³)/5 − 15n² + 100n − 140, both with C
/// integer truncation. Levels 0 and 1 are literal 0 and 1 in the table.
pub fn exp_for_level(growth: Growth, level: u8) -> u32 {
    let n = level as i64;
    if level <= 1 {
        return level as u32;
    }
    let v = match growth {
        Growth::MediumFast => n * n * n,
        Growth::MediumSlow => (6 * n * n * n) / 5 - 15 * n * n + 100 * n - 140,
    };
    v as u32
}

/// `GetLevelFromMonExp`'s answer: the highest level whose threshold the exp
/// meets (`decompiled/src/pokemon.c:2172-2181` walks the table upward).
pub fn level_from_exp(growth: Growth, exp: u32) -> u8 {
    let mut level = 1u8;
    while level < 100 && exp >= exp_for_level(growth, level + 1) {
        level += 1;
    }
    level
}

/// One defeated enemy's exp before splitting: `expYield · level / 7`
/// (`decompiled/src/battle_script_commands.c:3166`), then ×1.5 if a trainer
/// battle (`:3231-3232`), integer math throughout, single participant
/// assumed (`SAFE_DIV(calculatedExp, 1)`, `:3179`).
pub fn exp_gain(yield_: u8, level: u8, trainer: bool) -> u32 {
    let base = (yield_ as u32) * (level as u32) / 7;
    if trainer {
        base * 150 / 100
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::species;

    #[test]
    fn iv_unpack_masks_and_shifts() {
        let ivs = Ivs::unpack(0b0101_01010_10101, 0b11111_00000_11111);
        assert_eq!((ivs.hp, ivs.atk, ivs.def), (0b10101, 0b01010, 0b00101));
        assert_eq!((ivs.spe, ivs.spa, ivs.spd), (31, 0, 31));
        // Bit 15 is dead.
        assert_eq!(Ivs::unpack(0x8000, 0x8000), Ivs::unpack(0, 0));
    }

    #[test]
    fn medium_slow_matches_the_quoted_table() {
        // The values the research note derived from the macro
        // (docs/defeat-brock/research/starter-and-brock.md); an emulator test
        // cross-checks one against the ROM's own table.
        let expect = [
            (5u8, 135u32),
            (6, 179),
            (7, 236),
            (8, 314),
            (9, 419),
            (10, 560),
            (11, 742),
            (12, 973),
            (13, 1261),
            (14, 1612),
            (15, 2035),
            (16, 2535),
        ];
        for (level, exp) in expect {
            assert_eq!(exp_for_level(Growth::MediumSlow, level), exp, "L{level}");
            assert_eq!(level_from_exp(Growth::MediumSlow, exp), level);
            assert_eq!(level_from_exp(Growth::MediumSlow, exp - 1), level - 1);
        }
    }

    #[test]
    fn stat_calc_neutral_nature_level_5_bulbasaur() {
        // Worked by hand from the formula: base 45/49/49/45/65/65, all IVs 0,
        // L5: hp = (90*5)/100 + 15 = 19, atk/def = (98*5)/100+5 = 9,
        // spe = (90*5)/100+5 = 9, spa/spd = (130*5)/100+5 = 11.
        let s = calc_stats(
            species::by_id(species::BULBASAUR).unwrap(),
            Ivs {
                hp: 0,
                atk: 0,
                def: 0,
                spe: 0,
                spa: 0,
                spd: 0,
            },
            5,
            0,
        );
        assert_eq!(
            (s.hp, s.atk, s.def, s.spe, s.spa, s.spd),
            (19, 9, 9, 9, 11, 11)
        );
    }

    #[test]
    fn nature_modifier_truncates_after() {
        // Adamant (+Atk −SpA) on the same mon: 9*110/100 = 9 (truncation
        // makes small stats immune), 11*90/100 = 9.
        let s = calc_stats(
            species::by_id(species::BULBASAUR).unwrap(),
            Ivs {
                hp: 0,
                atk: 0,
                def: 0,
                spe: 0,
                spa: 0,
                spd: 0,
            },
            5,
            3,
        );
        assert_eq!((s.atk, s.spa), (9, 9));
    }
}
