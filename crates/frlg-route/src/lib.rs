//! The route: segments that take FireRed from power-on to a won rival battle,
//! written as code against the tier-1 harness.
//!
//! - [`record::Recorder`] -- an `Emu` that records one mask per frame it
//!   advances, so "wait until the menu appears" is still a replayable log.
//! - [`observe::Observer`] -- named RAM probes, each cited to the decomp.
//!
//! Tier 2 does not run in this sandbox. A segment that passes here has been
//! shown to work under mGBA and nothing more.

pub mod bk2;
pub mod ledger;
pub mod nav;
pub mod observe;
pub mod record;
pub mod search;
pub mod segments;

pub use observe::{Observer, Snapshot};
pub use record::{Feed, Recorder, RouteError};
pub use segments::{Segment, Starter};
