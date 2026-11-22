//! The human input model on a real core. Same knobs and keying as the simulator (`sim.rs`
//! `jitter`/`react`, `params.rs` `jitter_n`, `react_lo`, `react_hi`): each mashed or menu input is
//! late by `U{0..N}` frames, each reaction to an unpredictable event costs `U{lo..hi}` frames.
//! The stream is keyed by `(seed, branch)`: a rollback bumps `branch`, so the replayed stretch
//! gets fresh timing and with it a different game RNG (`PROMPT.md`, *Roll back, don't restart*).
use frlg_emu::{Emu, SaveState};
use frlg_route::{Feed, Recorder, RouteError};

use crate::rng::Rng;

/// Stream key of the human input draws ("human").
const HUMAN_STREAM: u64 = 0x0068_756d_616e;

#[derive(Clone, Copy, Debug)]
pub struct InputModel {
    pub jitter_n: u32,
    pub react_lo: u32,
    pub react_hi: u32,
}

impl InputModel {
    pub fn from_params(p: &crate::params::Params) -> Self {
        InputModel {
            jitter_n: p.jitter_n,
            react_lo: p.react_lo,
            react_hi: p.react_hi,
        }
    }
}

/// A recorder with a human at the controls. All stepping goes through [`Feed`], so every helper
/// in `frlg-route` (nav, `mash_until`, ...) works on it unchanged; the human-specific helpers
/// below add the jitter.
pub struct HumanFeed {
    pub rec: Recorder,
    pub model: InputModel,
    pub seed: u64,
    pub branch: u64,
    rng: Rng,
    /// Absolute frame of the core when this recorder's log starts (a resumed prefix).
    pub base_frame: u64,
}

impl HumanFeed {
    pub fn new(rec: Recorder, model: InputModel, seed: u64, branch: u64, base_frame: u64) -> Self {
        HumanFeed {
            rec,
            model,
            seed,
            branch,
            rng: Rng::new(seed, branch, HUMAN_STREAM),
            base_frame,
        }
    }

    /// Re-key the human's randomness after a rollback (fresh human seed for the new branch).
    pub fn rekey(&mut self, branch: u64) {
        self.branch = branch;
        self.rng = Rng::new(self.seed, branch, HUMAN_STREAM);
    }

    /// Absolute emulator frame.
    pub fn frame(&mut self) -> u64 {
        self.rec.emu().frame() as u64
    }

    pub fn emu(&mut self) -> &mut Emu {
        self.rec.emu()
    }

    pub fn save_state(&mut self) -> Result<SaveState, RouteError> {
        self.rec.save_state()
    }

    /// `U{0..N}` frames of nothing: how late a human's input lands.
    pub fn late(&mut self) -> Result<u32, RouteError> {
        let n = self.model.jitter_n;
        let d = if n == 0 { 0 } else { self.rng.below(n + 1) };
        self.idle(d as usize)?;
        Ok(d)
    }

    /// `U{lo..hi}` frames: the human noticed something unpredictable and decides.
    pub fn react(&mut self) -> Result<u32, RouteError> {
        let d = self.rng.range(self.model.react_lo, self.model.react_hi);
        self.idle(d as usize)?;
        Ok(d)
    }

    /// One deliberate press (menu input): late by `U{0..N}`, then press and release.
    pub fn press(&mut self, keys: u16) -> Result<(), RouteError> {
        self.late()?;
        self.tap(keys)
    }

    /// Mash `keys` until `until` holds: every press is late by `U{0..N}` frames.
    pub fn human_mash_until(
        &mut self,
        what: &str,
        keys: u16,
        budget: usize,
        mut until: impl FnMut(&mut Emu) -> bool,
    ) -> Result<usize, RouteError> {
        let start = self.fed();
        loop {
            if until(self.rec.emu()) {
                return Ok(self.fed() - start);
            }
            if self.fed() - start >= budget {
                return Err(RouteError::Timeout {
                    what: what.to_string(),
                    budget,
                    frames: self.fed(),
                });
            }
            let n = self.model.jitter_n;
            let d = if n == 0 { 0 } else { self.rng.below(n + 1) };
            for _ in 0..d {
                self.step(0)?;
                if until(self.rec.emu()) {
                    return Ok(self.fed() - start);
                }
            }
            self.step(keys)?;
            if until(self.rec.emu()) {
                return Ok(self.fed() - start);
            }
            self.step(0)?;
        }
    }

    /// The human's own uniform draw in `0..n`, for anything else that needs one.
    pub fn below(&mut self, n: u32) -> u32 {
        self.rng.below(n)
    }
}

impl Feed for HumanFeed {
    fn core(&mut self) -> &mut Emu {
        self.rec.emu()
    }
    fn record(&mut self, keys: u16) {
        self.rec.record(keys)
    }
    fn fed(&self) -> usize {
        self.rec.fed()
    }
}
