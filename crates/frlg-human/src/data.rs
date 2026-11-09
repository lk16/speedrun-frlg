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
}

#[derive(Deserialize)]
struct Raw {
    species: HashMap<String, RawSpecies>,
    sites: HashMap<String, RawSite>,
    sites_lg: HashMap<String, RawSite>,
    trainers: HashMap<String, RawTrainer>,
    map_trainers: HashMap<String, Vec<String>>,
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
    map_trainers: HashMap<String, Vec<String>>,
}

impl Dex {
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
        Dex {
            species,
            by_name,
            sites: raw.sites,
            sites_lg: raw.sites_lg,
            trainers,
            map_trainers: raw.map_trainers,
        }
    }

    pub fn id(&self, name: &str) -> usize {
        *self
            .by_name
            .get(name)
            .unwrap_or_else(|| panic!("unknown species {name}"))
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
    ) -> Option<(u32, Vec<Slot>, &'static [u32])> {
        let s = if lg { &self.sites_lg } else { &self.sites }.get(site)?;
        let (a, rates): (&RawArea, &'static [u32]) = match area {
            Area::Land => (s.land_mons.as_ref()?, &LAND_RATES),
            Area::Water => (s.water_mons.as_ref()?, &WATER_RATES),
            Area::Fish => (s.fishing_mons.as_ref()?, &FISH_RATES),
        };
        let slots = a
            .slots
            .iter()
            .filter_map(|(mn, mx, sp)| {
                self.by_name.get(sp).map(|&i| Slot {
                    min: *mn,
                    max: *mx,
                    species: i,
                })
            })
            .collect();
        Some((a.rate, slots, rates))
    }

    /// Parties of the trainers placed on the given maps (decomp `object_events` that resolve to
    /// a `trainerbattle_*`), hardest first (by summed level).
    pub fn map_parties(&self, maps: &[&str]) -> Vec<Vec<(u32, usize)>> {
        let mut out: Vec<Vec<(u32, usize)>> = maps
            .iter()
            .flat_map(|m| {
                self.map_trainers
                    .get(*m)
                    .unwrap_or_else(|| panic!("no trainers on map {m}"))
            })
            .filter_map(|t| self.trainers.get(t).cloned())
            .collect();
        out.sort_by_key(|p| std::cmp::Reverse(p.iter().map(|x| x.0).sum::<u32>()));
        out
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
        let (rate, slots, w) = d.area("MAP_ROUTE24", Area::Land, false).unwrap();
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
