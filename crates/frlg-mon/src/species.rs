//! The species this target touches, transcribed from
//! `decompiled/src/data/pokemon/species_info.h` (line cites per entry) and
//! `decompiled/include/constants/species.h` for the ids. Only what the
//! defeat-brock route can meet is here; growing the table is a transcription
//! chore, not a design change.

use crate::stats::Growth;

/// `decompiled/include/constants/pokemon.h:96-114`.
pub mod types {
    pub const NORMAL: u8 = 0;
    pub const FLYING: u8 = 2;
    pub const POISON: u8 = 3;
    pub const GROUND: u8 = 4;
    pub const ROCK: u8 = 5;
    pub const BUG: u8 = 6;
    pub const FIRE: u8 = 10;
    pub const WATER: u8 = 11;
    pub const GRASS: u8 = 12;
    pub const ELECTRIC: u8 = 13;
}

/// `decompiled/include/constants/species.h`.
pub const BULBASAUR: u16 = 1;
pub const CHARMANDER: u16 = 4;
pub const SQUIRTLE: u16 = 7;
pub const CATERPIE: u16 = 10;
pub const METAPOD: u16 = 11;
pub const WEEDLE: u16 = 13;
pub const KAKUNA: u16 = 14;
pub const PIDGEY: u16 = 16;
pub const RATTATA: u16 = 19;
pub const PIKACHU: u16 = 25;
pub const SANDSHREW: u16 = 27;
pub const GEODUDE: u16 = 74;
pub const ONIX: u16 = 95;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseStats {
    pub species: u16,
    pub name: &'static str,
    pub hp: u8,
    pub atk: u8,
    pub def: u8,
    pub spe: u8,
    pub spa: u8,
    pub spd: u8,
    pub types: (u8, u8),
    pub exp_yield: u8,
    pub growth: Growth,
}

use types::*;
use Growth::{MediumFast, MediumSlow};

/// `species_info.h` line cites, in order: Bulbasaur `:38-65`, Charmander
/// `:125-152`, Squirtle `:212-239`, Caterpie `:299-326`, Metapod `:328-355`,
/// Weedle `:386-413`, Kakuna `:415-442`, Pidgey `:473-500`, Rattata
/// `:560-587`, Pikachu `:734-761`, Sandshrew `:792-819`, Geodude
/// `:2155-2182`, Onix `:2764-2791`.
pub const ALL: [BaseStats; 13] = [
    BaseStats {
        species: BULBASAUR,
        name: "Bulbasaur",
        hp: 45,
        atk: 49,
        def: 49,
        spe: 45,
        spa: 65,
        spd: 65,
        types: (GRASS, POISON),
        exp_yield: 64,
        growth: MediumSlow,
    },
    BaseStats {
        species: CHARMANDER,
        name: "Charmander",
        hp: 39,
        atk: 52,
        def: 43,
        spe: 65,
        spa: 60,
        spd: 50,
        types: (FIRE, FIRE),
        exp_yield: 65,
        growth: MediumSlow,
    },
    BaseStats {
        species: SQUIRTLE,
        name: "Squirtle",
        hp: 44,
        atk: 48,
        def: 65,
        spe: 43,
        spa: 50,
        spd: 64,
        types: (WATER, WATER),
        exp_yield: 66,
        growth: MediumSlow,
    },
    BaseStats {
        species: CATERPIE,
        name: "Caterpie",
        hp: 45,
        atk: 30,
        def: 35,
        spe: 45,
        spa: 20,
        spd: 20,
        types: (BUG, BUG),
        exp_yield: 53,
        growth: MediumFast,
    },
    BaseStats {
        species: METAPOD,
        name: "Metapod",
        hp: 50,
        atk: 20,
        def: 55,
        spe: 30,
        spa: 25,
        spd: 25,
        types: (BUG, BUG),
        exp_yield: 72,
        growth: MediumFast,
    },
    BaseStats {
        species: WEEDLE,
        name: "Weedle",
        hp: 40,
        atk: 35,
        def: 30,
        spe: 50,
        spa: 20,
        spd: 20,
        types: (BUG, POISON),
        exp_yield: 52,
        growth: MediumFast,
    },
    BaseStats {
        species: KAKUNA,
        name: "Kakuna",
        hp: 45,
        atk: 25,
        def: 50,
        spe: 35,
        spa: 25,
        spd: 25,
        types: (BUG, POISON),
        exp_yield: 71,
        growth: MediumFast,
    },
    BaseStats {
        species: PIDGEY,
        name: "Pidgey",
        hp: 40,
        atk: 45,
        def: 40,
        spe: 56,
        spa: 35,
        spd: 35,
        types: (NORMAL, FLYING),
        exp_yield: 55,
        growth: MediumSlow,
    },
    BaseStats {
        species: RATTATA,
        name: "Rattata",
        hp: 30,
        atk: 56,
        def: 35,
        spe: 72,
        spa: 25,
        spd: 35,
        types: (NORMAL, NORMAL),
        exp_yield: 57,
        growth: MediumFast,
    },
    BaseStats {
        species: PIKACHU,
        name: "Pikachu",
        hp: 35,
        atk: 55,
        def: 30,
        spe: 90,
        spa: 50,
        spd: 40,
        types: (ELECTRIC, ELECTRIC),
        exp_yield: 82,
        growth: MediumFast,
    },
    BaseStats {
        species: SANDSHREW,
        name: "Sandshrew",
        hp: 50,
        atk: 75,
        def: 85,
        spe: 40,
        spa: 20,
        spd: 30,
        types: (GROUND, GROUND),
        exp_yield: 93,
        growth: MediumFast,
    },
    BaseStats {
        species: GEODUDE,
        name: "Geodude",
        hp: 40,
        atk: 80,
        def: 100,
        spe: 20,
        spa: 30,
        spd: 30,
        types: (ROCK, GROUND),
        exp_yield: 86,
        growth: MediumSlow,
    },
    BaseStats {
        species: ONIX,
        name: "Onix",
        hp: 35,
        atk: 45,
        def: 160,
        spe: 70,
        spa: 30,
        spd: 45,
        types: (ROCK, GROUND),
        exp_yield: 108,
        growth: MediumFast,
    },
];

pub fn by_id(species: u16) -> Option<&'static BaseStats> {
    ALL.iter().find(|b| b.species == species)
}
