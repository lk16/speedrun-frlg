//! Species, wild-encounter and trainer tables, generated from `decompiled/` by
//! `human-run/tools/gen_dex.py` into `human-run/data/dex.json`.

use serde::Deserialize;
use std::collections::HashMap;

const DEX_JSON: &str = include_str!("../../../human-run/data/dex.json");

/// Slot weights, `decompiled/src/data/wild_encounters.json` `fields[].encounter_rates`.
pub const LAND_RATES: [u32; 12] = [20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1];
pub const WATER_RATES: [u32; 5] = [60, 30, 5, 4, 1];
pub const FISH_RATES: [u32; 10] = [70, 30, 60, 20, 20, 40, 40, 15, 4, 1];

#[derive(Deserialize)]
struct RawSpecies {
    safari_flee: u32,
    types: (String, String),
    catch: u32,
    #[serde(rename = "yield_")]
    yield_: u32,
    growth: String,
    evo: Vec<(String, String, String)>,
    dex: u32,
}

#[derive(Deserialize, Clone)]
pub struct RawArea {
    pub rate: u32,
    pub slots: Vec<(u32, u32, String)>,
}

#[derive(Deserialize, Clone, Default)]
pub struct RawSite {
    pub land_mons: Option<RawArea>,
    pub water_mons: Option<RawArea>,
    pub rock_smash_mons: Option<RawArea>,
    pub fishing_mons: Option<RawArea>,
}

#[derive(Deserialize)]
struct RawTrainer {
    party: Vec<(u32, String)>,
    money: u32,
}

#[derive(Deserialize)]
struct Raw {
    species: HashMap<String, RawSpecies>,
    type_chart: Vec<(String, String, u32)>,
    sites: HashMap<String, RawSite>,
    sites_lg: HashMap<String, RawSite>,
    trainers: HashMap<String, RawTrainer>,
    map_trainers: HashMap<String, Vec<String>>,
    scripts: HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct Species {
    pub name: String,
    pub catch_rate: u32,
    pub exp_yield: u32,
    pub medium_slow: bool,
    /// National number if in the Kanto dex (1..=151), else 0.
    pub dex: u32,
    pub evo: Vec<(String, String, usize)>,
    /// `decompiled/include/constants/pokemon.h` type numbers.
    pub types: (u8, u8),
    /// `safariZoneFleeRate` (`species_info.h`).
    pub safari_flee: u32,
}

/// `TYPE_*` numbers, `decompiled/include/constants/pokemon.h:96-114`.
pub fn type_num(name: &str) -> u8 {
    match name {
        "TYPE_NORMAL" => 0,
        "TYPE_FIGHTING" => 1,
        "TYPE_FLYING" => 2,
        "TYPE_POISON" => 3,
        "TYPE_GROUND" => 4,
        "TYPE_ROCK" => 5,
        "TYPE_BUG" => 6,
        "TYPE_GHOST" => 7,
        "TYPE_STEEL" => 8,
        "TYPE_FIRE" => 10,
        "TYPE_WATER" => 11,
        "TYPE_GRASS" => 12,
        "TYPE_ELECTRIC" => 13,
        "TYPE_PSYCHIC" => 14,
        "TYPE_ICE" => 15,
        "TYPE_DRAGON" => 16,
        "TYPE_DARK" => 17,
        _ => 9,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Area {
    Land,
    Water,
    Fish,
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub min: u32,
    pub max: u32,
    pub species: usize,
}

pub struct Dex {
    pub species: Vec<Species>,
    by_name: HashMap<String, usize>,
    sites: HashMap<String, RawSite>,
    sites_lg: HashMap<String, RawSite>,
    trainers: HashMap<String, Vec<(u32, usize)>>,
    /// Prize money per trainer id (`battle_script_commands.c:5373`).
    money: HashMap<String, u32>,
    map_trainers: HashMap<String, Vec<String>>,
    scripts: HashMap<String, String>,
    /// Damage multiplier of attacking type (row) on defending type (column),
    /// `decompiled/src/battle_main.c:312` `gTypeEffectiveness`.
    chart: [[f64; 18]; 18],
}

impl Dex {
    /// Multiplier of a move of type `att` on a defender with the given types.
    pub fn eff(&self, att: u8, def: (u8, u8)) -> f64 {
        let mut m = self.chart[att as usize][def.0 as usize];
        if def.1 != def.0 {
            m *= self.chart[att as usize][def.1 as usize];
        }
        m
    }

    pub fn load() -> Dex {
        let raw: Raw = serde_json::from_str(DEX_JSON).expect("dex.json parses");
        let mut names: Vec<&String> = raw.species.keys().collect();
        names.sort();
        let mut by_name = HashMap::new();
        for (i, n) in names.iter().enumerate() {
            by_name.insert((*n).clone(), i);
        }
        let species = names
            .iter()
            .map(|n| {
                let s = &raw.species[*n];
                Species {
                    name: (*n).clone(),
                    catch_rate: s.catch,
                    exp_yield: s.yield_,
                    medium_slow: s.growth == "GROWTH_MEDIUM_SLOW",
                    dex: s.dex,
                    types: (type_num(&s.types.0), type_num(&s.types.1)),
                    safari_flee: s.safari_flee,
                    evo: s
                        .evo
                        .iter()
                        .filter_map(|(k, p, t)| {
                            by_name.get(t).map(|&ti| (k.clone(), p.clone(), ti))
                        })
                        .collect(),
                }
            })
            .collect();
        let trainers = raw
            .trainers
            .iter()
            .map(|(k, t)| {
                (
                    k.clone(),
                    t.party
                        .iter()
                        .filter_map(|(l, s)| by_name.get(s).map(|&i| (*l, i)))
                        .collect(),
                )
            })
            .collect();
        let mut chart = [[1.0; 18]; 18];
        for (a, d, m) in &raw.type_chart {
            chart[type_num(a) as usize][type_num(d) as usize] = *m as f64 / 10.0;
        }
        let money = raw
            .trainers
            .iter()
            .map(|(k, t)| (k.clone(), t.money))
            .collect();
        Dex {
            money,
            chart,
            species,
            by_name,
            sites: raw.sites,
            sites_lg: raw.sites_lg,
            trainers,
            map_trainers: raw.map_trainers,
            scripts: raw.scripts,
        }
    }

    pub fn id(&self, name: &str) -> usize {
        *self
            .by_name
            .get(name)
            .unwrap_or_else(|| panic!("unknown species {name}"))
    }

    pub fn trainer_money(&self, name: &str) -> u32 {
        self.money.get(name).copied().unwrap_or(0)
    }

    pub fn trainer(&self, name: &str) -> &[(u32, usize)] {
        self.trainers
            .get(name)
            .unwrap_or_else(|| panic!("unknown trainer {name}"))
    }

    /// Slots of a site's area, with the cumulative-weight table that applies to it.
    pub fn area(
        &self,
        site: &str,
        area: Area,
        lg: bool,
        rod: &str,
    ) -> Option<(u32, Vec<Slot>, Vec<u32>)> {
        let s = if lg { &self.sites_lg } else { &self.sites }.get(site)?;
        let (a, rates): (&RawArea, &[u32]) = match area {
            Area::Land => (s.land_mons.as_ref()?, &LAND_RATES),
            Area::Water => (s.water_mons.as_ref()?, &WATER_RATES),
            Area::Fish => (s.fishing_mons.as_ref()?, &FISH_RATES),
        };
        // Fishing slots by rod: old 0-1, good 2-4, super 5-9, each rod's weights summing to 100
        // (`decompiled/src/wild_encounter.c:120-150`). "mixed" keeps all ten (the old model).
        let range = match (area, rod) {
            (Area::Fish, "old") => 0..2,
            (Area::Fish, "good") => 2..5,
            (Area::Fish, "super") => 5..10,
            _ => 0..a.slots.len(),
        };
        let weights: Vec<u32> = rates[range.start.min(rates.len())..range.end.min(rates.len())].to_vec();
        let slots = a
            .slots
            .iter()
            .enumerate()
            .filter(|(i, _)| range.contains(i))
            .map(|(_, x)| x)
            .filter_map(|(mn, mx, sp)| {
                self.by_name.get(sp).map(|&i| Slot {
                    min: *mn,
                    max: *mx,
                    species: i,
                })
            })
            .collect();
        Some((a.rate, slots, weights))
    }

    /// Parties of the trainers placed on the given maps (decomp `object_events` that resolve to
    /// a `trainerbattle_*`), hardest first (by summed level).
    pub fn map_parties(&self, maps: &[&str]) -> Vec<(Vec<(u32, usize)>, u32)> {
        let mut out: Vec<(Vec<(u32, usize)>, u32)> = maps
            .iter()
            .flat_map(|m| {
                self.map_trainers
                    .get(*m)
                    .unwrap_or_else(|| panic!("no trainers on map {m}"))
            })
            .filter_map(|t| {
                self.trainers
                    .get(t)
                    .map(|p| (p.clone(), self.trainer_money(t)))
            })
            .collect();
        out.sort_by_key(|p| std::cmp::Reverse(p.0.iter().map(|x| x.0).sum::<u32>()));
        out
    }

    /// The party of the trainer a map script label fights
    /// (`trainerbattle_*` in the label's body, `human-run/tools/gen_dex.py`).
    pub fn script_party(&self, label: &str) -> &[(u32, usize)] {
        self.trainer(self.script_trainer(label))
    }

    pub fn script_trainer(&self, label: &str) -> &str {
        self.scripts
            .get(label)
            .unwrap_or_else(|| panic!("no trainer script {label}"))
    }

    pub fn has_site(&self, site: &str) -> bool {
        self.sites.contains_key(site)
    }
}

/// Fixed-size caught set over the Kanto dex (national numbers 1..=151).
#[derive(Clone, Copy, Default)]
pub struct Caught([u64; 3]);

impl Caught {
    pub fn insert(&mut self, dex: u32) -> bool {
        if dex == 0 || dex > 151 {
            return false;
        }
        let (w, b) = ((dex as usize - 1) / 64, (dex - 1) % 64);
        let was = self.0[w] >> b & 1 == 1;
        self.0[w] |= 1 << b;
        !was
    }
    pub fn has(&self, dex: u32) -> bool {
        if dex == 0 || dex > 151 {
            return true;
        }
        let (w, b) = ((dex as usize - 1) / 64, (dex - 1) % 64);
        self.0[w] >> b & 1 == 1
    }
    pub fn count(&self) -> u32 {
        self.0.iter().map(|w| w.count_ones()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_resolves() {
        let d = Dex::load();
        let sq = d.id("SPECIES_SQUIRTLE");
        assert_eq!(d.species[sq].dex, 7);
        assert!(d.species[sq].medium_slow);
        assert_eq!(d.trainer("TRAINER_LEADER_BROCK").len(), 2);
        let (rate, slots, w) = d.area("MAP_ROUTE24", Area::Land, false, "mixed").unwrap();
        assert_eq!(rate, 21);
        assert_eq!(slots.len(), w.len());
        assert_eq!(w.iter().sum::<u32>(), 100);
    }

    #[test]
    fn caught_set_counts() {
        let mut c = Caught::default();
        assert!(c.insert(151));
        assert!(!c.insert(151));
        assert!(c.insert(1));
        assert_eq!(c.count(), 2);
        assert!(!c.insert(152));
    }
}
