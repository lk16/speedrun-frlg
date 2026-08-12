//! The per-step wild encounter check as a state machine over the two RNGs.
//!
//! Mirrors `TryStandardWildEncounter` (`decompiled/src/wild_encounter.c:757-776`)
//! exactly, for the pre-Brock game: no repel, no flutes, no Cleanse Tag, no
//! bike, no roamer, lead ability neither Stench nor Illuminate. Each of those
//! is an absent branch documented at the site it would live, so a later
//! target can add it against the citation instead of re-deriving the flow.
//!
//! Not yet emulator-validated; `tests/` grows that proof when the route
//! reaches Route 1 grass. Until then this module's authority is the
//! transcription plus its unit tests.

use frlg_rng::{Rng, WildRng};

use crate::create::wild_mon_from_nature;
use crate::stats::Ivs;

/// One land encounter table: `encounter_rate` and 12 slots, from
/// `decompiled/src/data/wild_encounters.json` (cites per table below).
#[derive(Debug, Clone, Copy)]
pub struct MapWild {
    pub rate: u8,
    /// `(species, min_level, max_level)` per slot.
    pub slots: [(u16, u8, u8); 12],
}

impl MapWild {
    /// `GetMapBaseEncounterCooldown` for land
    /// (`decompiled/src/wild_encounter.c:673-699`): steps after which the
    /// cooldown gate passes for free.
    pub fn min_steps(&self) -> u8 {
        if self.rate >= 80 {
            0
        } else if self.rate < 10 {
            8
        } else {
            8 - self.rate / 10
        }
    }
}

use crate::species::{CATERPIE, KAKUNA, METAPOD, PIDGEY, PIKACHU, RATTATA, WEEDLE};

/// Route 1, both versions identical (`wild_encounters.json:8258-8325` FR,
/// `:20801-20867` LG).
pub const ROUTE1: MapWild = MapWild {
    rate: 21,
    slots: [
        (PIDGEY, 3, 3),
        (RATTATA, 3, 3),
        (PIDGEY, 3, 3),
        (RATTATA, 3, 3),
        (PIDGEY, 2, 2),
        (RATTATA, 2, 2),
        (PIDGEY, 3, 3),
        (RATTATA, 3, 3),
        (PIDGEY, 4, 4),
        (RATTATA, 4, 4),
        (PIDGEY, 5, 5),
        (RATTATA, 4, 4),
    ],
};

/// Route 2, both versions identical (`wild_encounters.json:8327-8394` FR,
/// `:20870-20936` LG).
pub const ROUTE2: MapWild = MapWild {
    rate: 21,
    slots: [
        (RATTATA, 3, 3),
        (PIDGEY, 3, 3),
        (RATTATA, 4, 4),
        (PIDGEY, 4, 4),
        (RATTATA, 2, 2),
        (PIDGEY, 2, 2),
        (RATTATA, 5, 5),
        (PIDGEY, 5, 5),
        (CATERPIE, 4, 4),
        (WEEDLE, 4, 4),
        (CATERPIE, 5, 5),
        (WEEDLE, 5, 5),
    ],
};

/// Viridian Forest, FireRed (`wild_encounters.json:563-631`).
pub const VIRIDIAN_FOREST_FR: MapWild = MapWild {
    rate: 14,
    slots: [
        (CATERPIE, 4, 4),
        (WEEDLE, 4, 4),
        (CATERPIE, 5, 5),
        (WEEDLE, 5, 5),
        (CATERPIE, 3, 3),
        (WEEDLE, 3, 3),
        (METAPOD, 5, 5),
        (KAKUNA, 5, 5),
        (KAKUNA, 4, 4),
        (PIKACHU, 3, 3),
        (KAKUNA, 6, 6),
        (PIKACHU, 5, 5),
    ],
};

/// Viridian Forest, LeafGreen (`wild_encounters.json:13106-13172`): the
/// FireRed table with Kakuna→Metapod at slots 6-8 and 10 swapped the other
/// way.
pub const VIRIDIAN_FOREST_LG: MapWild = MapWild {
    rate: 14,
    slots: [
        (CATERPIE, 4, 4),
        (WEEDLE, 4, 4),
        (CATERPIE, 5, 5),
        (WEEDLE, 5, 5),
        (CATERPIE, 3, 3),
        (WEEDLE, 3, 3),
        (KAKUNA, 5, 5),
        (METAPOD, 5, 5),
        (METAPOD, 4, 4),
        (PIKACHU, 3, 3),
        (METAPOD, 6, 6),
        (PIKACHU, 5, 5),
    ],
};

/// `ChooseWildMonIndex_Land`'s cumulative thresholds
/// (`decompiled/src/wild_encounter.c:71-99`; rates 20/20/10/10/10/10/5/5/
/// 4/4/1/1 from `wild_encounters.json:7-21`): slot = first index whose
/// threshold exceeds `Random() % 100`.
pub const LAND_SLOT_THRESHOLDS: [u8; 12] = [20, 40, 50, 60, 70, 80, 85, 90, 94, 98, 99, 100];

pub fn land_slot(roll: u16) -> u8 {
    let r = (roll % 100) as u8;
    LAND_SLOT_THRESHOLDS.iter().position(|&t| r < t).unwrap() as u8
}

/// `sWildEncounterData`'s modelled fields
/// (`decompiled/src/wild_encounter.c:24-34`; `abilityEffect` and
/// `leadMonHeldItem` are recomputed from the party each step and are
/// constant 0/none pre-Brock, `:334-346`, `:730-735`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WildState {
    pub rng: WildRng,
    pub prev_behavior: u16,
    pub rate_buff: u16,
    pub steps_since: u8,
}

impl WildState {
    /// The state right after `SeedWildEncounterRng(seed)`
    /// (`decompiled/src/wild_encounter.c:661-665`): seeded, modifiers reset.
    /// `prev_behavior` starts 0 (EWRAM zero-init, `:34`).
    pub fn seeded(seed: u16) -> Self {
        WildState {
            rng: WildRng::seeded(seed),
            prev_behavior: 0,
            rate_buff: 0,
            steps_since: 0,
        }
    }

    /// `ResetEncounterRateModifiers` (`decompiled/src/wild_encounter.c:701-705`),
    /// called on map load/warp (`src/overworld.c:764,799`) and battle start
    /// (`src/battle_setup.c:205`). Does *not* touch `prev_behavior`.
    pub fn reset_modifiers(&mut self) {
        self.rate_buff = 0;
        self.steps_since = 0;
    }

    /// One tile-center step. `map` is the current map's land table (`None`
    /// when the map has no wild header -- towns); `land` is the tile's
    /// encounter-type attribute == `TILE_ENCOUNTER_LAND`; `behavior` its
    /// behavior attribute. `grng` is `gRngValue` *as of the check* -- the
    /// caller owns every other consumer (VBlank, NPCs).
    pub fn step(
        &mut self,
        grng: &mut Rng,
        map: Option<&MapWild>,
        land: bool,
        behavior: u16,
    ) -> StepOutcome {
        // HandleWildEncounterCooldown (`wild_encounter.c:707-755`). The
        // encounter-type check precedes the header lookup.
        if !land {
            self.prev_behavior = behavior;
            return StepOutcome::NotEncounterTile;
        }
        let Some(map) = map else {
            self.prev_behavior = behavior;
            return StepOutcome::NoWildHeader;
        };
        // No flute/cleanse-tag/ability modifiers: minSteps and the 5%
        // gate rate stay at their base values (`:717-746` all no-ops here).
        if self.steps_since < map.min_steps() {
            self.steps_since += 1;
            if grng.random() % 100 >= 5 {
                self.prev_behavior = behavior;
                return StepOutcome::CooldownFailed;
            }
        }
        // StandardWildEncounter (`wild_encounter.c:355-403`), land branch.
        let prev = self.prev_behavior;
        if prev != behavior && grng.random() % 100 >= 60 {
            self.prev_behavior = behavior;
            return StepOutcome::BehaviorRollFailed;
        }
        // DoWildEncounterRateTest (`:309-332`): no bike, no mods. The dice
        // roll is the second LCG's, not gRngValue's.
        let mut rate = map.rate as u32 * 16 + self.rate_buff as u32 * 16 / 200;
        rate = rate.min(1600);
        if (self.rng.random() % 1600) as u32 >= rate {
            // AddToWildEncounterRateBuff (`:778-784`), no repel.
            self.rate_buff += map.rate as u16;
            self.prev_behavior = behavior;
            return StepOutcome::RateFailed;
        }
        // TryGenerateWildMon (`:269-292`) -> GenerateWildMon (`:226-241`).
        let slot = land_slot(grng.random());
        let (species, lo, hi) = map.slots[slot as usize];
        let level = lo + (grng.random() % (hi as u16 - lo as u16 + 1)) as u8;
        let nature = (grng.random() % 25) as u8;
        let (genome, _) = wild_mon_from_nature(grng, nature);
        self.rate_buff = 0;
        self.steps_since = 0;
        self.prev_behavior = behavior;
        StepOutcome::Encounter(Encounter {
            slot,
            species,
            level,
            nature,
            pid: genome.pid,
            ivs: genome.ivs,
        })
    }
}

/// What one step did. Everything except `Encounter` means "keep walking".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    /// Tile's encounter type is not LAND: no rolls at all.
    NotEncounterTile,
    /// Map has no wild header (towns): no rolls.
    NoWildHeader,
    /// Within `min_steps` of a reset and the 5% gate missed (the usual
    /// case): 1 `gRngValue` roll.
    CooldownFailed,
    /// Tile behavior changed and the 60% roll missed: 1-2 `gRngValue` rolls.
    BehaviorRollFailed,
    /// The second-LCG rate test missed; the rate buff grew.
    RateFailed,
    /// A wild battle starts.
    Encounter(Encounter),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Encounter {
    pub slot: u8,
    pub species: u16,
    pub level: u8,
    pub nature: u8,
    pub pid: u32,
    pub ivs: Ivs,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_steps_from_rates() {
        // 8 - rate/10 for 10 <= rate < 80 (`wild_encounter.c:673-699`).
        assert_eq!(ROUTE1.min_steps(), 6);
        assert_eq!(ROUTE2.min_steps(), 6);
        assert_eq!(VIRIDIAN_FOREST_FR.min_steps(), 7);
        assert_eq!(
            MapWild {
                rate: 80,
                slots: ROUTE1.slots
            }
            .min_steps(),
            0
        );
        assert_eq!(
            MapWild {
                rate: 9,
                slots: ROUTE1.slots
            }
            .min_steps(),
            8
        );
    }

    #[test]
    fn slot_thresholds_partition_100() {
        assert_eq!(land_slot(0), 0);
        assert_eq!(land_slot(19), 0);
        assert_eq!(land_slot(20), 1);
        assert_eq!(land_slot(97), 9);
        assert_eq!(land_slot(98), 10);
        assert_eq!(land_slot(99), 11);
        assert_eq!(land_slot(100), 0); // % 100 wraps
    }

    #[test]
    fn non_encounter_tiles_roll_nothing() {
        let mut st = WildState::seeded(1);
        let mut grng = Rng(42);
        let before = (st.rng, grng);
        assert_eq!(
            st.step(&mut grng, Some(&ROUTE1), false, 7),
            StepOutcome::NotEncounterTile
        );
        assert_eq!(st.step(&mut grng, None, true, 2), StepOutcome::NoWildHeader);
        assert_eq!((st.rng, grng), before);
        // But prev_behavior tracked anyway (`wild_encounter.c:761`).
        assert_eq!(st.prev_behavior, 2);
    }

    #[test]
    fn cooldown_consumes_one_groll_and_counts_up() {
        let mut st = WildState::seeded(1);
        // A gRng state whose next roll % 100 >= 5 (fails the gate).
        let mut grng = Rng(0);
        while (grng.next().0 >> 16) % 100 < 5 {
            grng = grng.next();
        }
        let g0 = grng;
        let out = st.step(&mut grng, Some(&ROUTE1), true, 2);
        assert_eq!(out, StepOutcome::CooldownFailed);
        assert_eq!(g0.distance_to(grng), 1);
        assert_eq!(st.steps_since, 1);
        // The wild LCG did not move.
        assert_eq!(st.rng, WildRng::seeded(1));
    }

    #[test]
    fn after_min_steps_the_gate_is_free() {
        let mut st = WildState::seeded(1);
        st.steps_since = 6; // Route 1's minSteps
        st.prev_behavior = 2;
        // Find a wild state whose next roll fails the 21-rate test, so the
        // step ends at RateFailed having consumed no gRng.
        loop {
            let mut probe = st.rng;
            if probe.random() % 1600 >= 21 * 16 {
                break;
            }
            st.rng = st.rng.next();
        }
        let mut grng = Rng(7);
        let g0 = grng;
        let out = st.step(&mut grng, Some(&ROUTE1), true, 2);
        assert_eq!(out, StepOutcome::RateFailed);
        assert_eq!(g0.distance_to(grng), 0);
        assert_eq!(st.rate_buff, 21);
        assert_eq!(st.steps_since, 6, "counter stops at minSteps");
    }

    #[test]
    fn rate_buff_raises_the_effective_rate() {
        // 200 failures at rate 21 give buff 4200 -> +336 effective, i.e.
        // rate 672/1600. Just check the arithmetic path doesn't overflow
        // and clamps at 1600.
        let mut st = WildState::seeded(99);
        st.steps_since = 6;
        st.prev_behavior = 2;
        let mut grng = Rng(1);
        for _ in 0..2000 {
            match st.step(&mut grng, Some(&ROUTE1), true, 2) {
                StepOutcome::RateFailed => {}
                StepOutcome::Encounter(e) => {
                    assert!(e.level >= 2 && e.level <= 5);
                    assert_eq!(e.pid % 25, e.nature as u32);
                    return;
                }
                other => panic!("unexpected {other:?}"),
            }
        }
        panic!("2000 eligible steps without an encounter is implausible at rate >= 21/1600");
    }
}
