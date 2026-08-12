//! What a Pokémon *is*, predicted from a `gRngValue`: creation rolls
//! (PID/nature/IVs), stat arithmetic, exp thresholds, and the wild-encounter
//! per-step state machine.
//!
//! Same contract as `frlg-battle`: every formula carries its decomp citation,
//! and correctness is not argued from the transcription -- `tests/` replays
//! real emulator runs and requires the model to reproduce what libmgba's RAM
//! says, roll for roll. Until a piece has an emulator test, its doc says so.
//!
//! The models deliberately cover only the pre-Brock game: no repel, no
//! flutes, no Cleanse Tag, no bike, no roamer, and a lead mon whose ability
//! is neither Stench nor Illuminate (the three starters carry
//! Overgrow/Blaze/Torrent, `decompiled/src/data/pokemon/species_info.h:62,
//! 149,236`). Each absent branch is a documented assumption rather than a
//! wrong formula.

pub mod create;
pub mod species;
pub mod stats;
pub mod wild;

pub use create::{gift_mon, wild_mon_from_nature, Genome};
pub use stats::{calc_stats, Ivs, Stats};
pub use wild::{StepOutcome, WildState};
