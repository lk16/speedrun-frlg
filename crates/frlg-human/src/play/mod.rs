//! The playing controller (`human-run/PROMPT.md`, *Next phase*): a bot that plays the CURRENT
//! plan in libmgba one input per frame with the human input model, and writes an `.ilog` plus a
//! decision log (`events.jsonl`) that `frlg video --overlay` renders.
//!
//! - [`human::HumanFeed`] -- a [`frlg_route::Recorder`] whose presses are late by `U{0..N}` and
//!   whose reactions cost `U{lo..hi}`, keyed by `(seed, branch, stream)` like the simulator.
//! - [`events::EventLog`] -- the decision log, one JSON line per event.
//!
//! Every decision goes through the simulator's own policy code (`battle.rs`, `sim.rs`); nothing
//! here re-implements a choice the model makes.
pub mod battle;
pub mod battle_audit;
pub mod battle_ram;
pub mod battle_test;
pub mod cli;
pub mod dex;
pub mod dex_test;
pub mod dexfin;
pub mod early2;
pub mod events;
pub mod fallback_battle;
pub mod human;
pub mod ids;
pub mod inspect;
pub mod items;
pub mod legs;
pub mod legs2;
pub mod legs3;
pub mod menu;
pub mod model;
pub mod names;
pub mod prims;
pub mod ram;
pub mod session;
pub mod shop;
pub mod walk;
pub mod walk_check;
#[cfg(test)]
mod walk_test;

/// GBA frames per second (`human-run/PROMPT.md`, *The target*).
pub const FPS: f64 = 59.7275;
