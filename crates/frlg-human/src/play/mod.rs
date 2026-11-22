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
pub mod events;
pub mod human;

/// GBA frames per second (`human-run/PROMPT.md`, *The target*).
pub const FPS: f64 = 59.7275;
