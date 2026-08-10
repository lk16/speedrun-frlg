//! Drive a core and keep the input log honest.
//!
//! The rule this type exists to enforce: **every frame the emulator advances,
//! exactly one mask is appended to the log**. A route written as "advance until
//! the menu appears" is then still replayable frame-for-frame, because the
//! waiting itself was recorded. Nothing else in the crate is allowed to call
//! `Emu::step`.

use std::path::Path;

use frlg_emu::{keys, Emu, EmuError, InputLog, SaveState};

#[derive(Debug, thiserror::Error)]
pub enum RouteError {
    #[error(transparent)]
    Emu(#[from] EmuError),
    /// A wait ran out of frames. The message names what was being waited for,
    /// because "timed out" on its own is unactionable when a route has thirty
    /// of them.
    #[error("waiting for {what} timed out after {budget} frames (log is {frames} frames long)")]
    Timeout {
        what: String,
        budget: usize,
        frames: usize,
    },
    #[error("{0:?} is not a usable key mask: {1}")]
    BadKeys(u16, &'static str),
}

/// An emulator plus the log of what has been fed to it.
pub struct Recorder {
    emu: Emu,
    frames: Vec<u16>,
    rom_sha1: [u8; 20],
}

impl Recorder {
    /// A fresh core at power-on. The log starts empty and its first frame is
    /// the first frame after reset.
    pub fn from_reset(rom: &Path) -> Result<Self, RouteError> {
        let mut emu = Emu::new(rom)?;
        emu.reset();
        Ok(Self {
            rom_sha1: frlg_emu::file_sha1(rom).map_err(EmuError::from)?,
            emu,
            frames: Vec::new(),
        })
    }

    /// Resume from a checkpoint. The log records only the frames this recorder
    /// adds, which is what makes segments composable: a segment's log is
    /// meaningful relative to its parent's end state, and the ledger is what
    /// ties the two together.
    pub fn from_state(rom: &Path, state: &SaveState) -> Result<Self, RouteError> {
        let mut me = Self::from_reset(rom)?;
        me.emu.load_state(state)?;
        Ok(me)
    }

    pub fn emu(&mut self) -> &mut Emu {
        &mut self.emu
    }

    /// Frames recorded so far -- the cost of the segment.
    pub fn frames(&self) -> usize {
        self.frames.len()
    }

    pub fn log(&self) -> InputLog {
        InputLog::new(self.rom_sha1, self.frames.clone())
    }

    /// One frame with `keys` held.
    pub fn step(&mut self, keys: u16) -> Result<(), RouteError> {
        if keys & !keys::MASK != 0 {
            return Err(RouteError::BadKeys(keys, "bits outside KEYS_MASK"));
        }
        if keys::is_impossible_dpad(keys) {
            return Err(RouteError::BadKeys(keys, "opposing d-pad directions"));
        }
        self.frames.push(keys);
        self.emu.step(keys);
        Ok(())
    }

    /// `keys` held for `frames` frames.
    pub fn hold(&mut self, keys: u16, frames: usize) -> Result<(), RouteError> {
        for _ in 0..frames {
            self.step(keys)?;
        }
        Ok(())
    }

    /// Nothing held, for `frames` frames.
    pub fn idle(&mut self, frames: usize) -> Result<(), RouteError> {
        self.hold(0, frames)
    }

    /// One frame pressed, one frame released.
    ///
    /// The game acts on `gMain.newKeys` (`decompiled/include/main.h:32`), which
    /// is the difference between this frame's and last frame's held keys, so a
    /// button held across frames registers once. Releasing afterwards is what
    /// makes the next press register at all.
    pub fn tap(&mut self, keys: u16) -> Result<(), RouteError> {
        self.step(keys)?;
        self.step(0)
    }

    /// Advance, cycling `pattern` one mask per frame, until `until` says the
    /// game got where it was going.
    ///
    /// The predicate is checked after each frame, so the returned count is the
    /// number of frames this call added. `pattern` is the input to feed while
    /// waiting: `&[0]` to sit still, `&[keys::A, 0]` to mash A.
    pub fn advance_while(
        &mut self,
        what: &str,
        pattern: &[u16],
        budget: usize,
        mut until: impl FnMut(&mut Emu) -> bool,
    ) -> Result<usize, RouteError> {
        assert!(!pattern.is_empty(), "advance_while needs a pattern");
        let start = self.frames.len();
        for i in 0..budget {
            self.step(pattern[i % pattern.len()])?;
            if until(&mut self.emu) {
                return Ok(self.frames.len() - start);
            }
        }
        Err(RouteError::Timeout {
            what: what.to_string(),
            budget,
            frames: self.frames.len(),
        })
    }

    /// Sit still until `until` holds.
    pub fn wait_until(
        &mut self,
        what: &str,
        budget: usize,
        until: impl FnMut(&mut Emu) -> bool,
    ) -> Result<usize, RouteError> {
        self.advance_while(what, &[0], budget, until)
    }

    /// Mash `keys` (press, release, press, ...) until `until` holds.
    pub fn mash_until(
        &mut self,
        what: &str,
        keys: u16,
        budget: usize,
        until: impl FnMut(&mut Emu) -> bool,
    ) -> Result<usize, RouteError> {
        self.advance_while(what, &[keys, 0], budget, until)
    }

    pub fn save_state(&mut self) -> Result<SaveState, RouteError> {
        Ok(self.emu.save_state()?)
    }

    pub fn save_state_file(&mut self, path: &Path) -> Result<(), RouteError> {
        Ok(self.emu.save_state_file(path)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rom() -> std::path::PathBuf {
        frlg_emu::default_rom_path().expect("no ROM: build it and copy it to $FRLG_ARTIFACTS/rom")
    }

    #[test]
    fn every_advanced_frame_is_recorded() {
        let mut rec = Recorder::from_reset(&rom()).unwrap();
        rec.idle(10).unwrap();
        rec.tap(keys::A).unwrap();
        rec.hold(keys::B, 3).unwrap();
        assert_eq!(rec.frames(), 15);
        assert_eq!(rec.emu().frame(), 15);
        assert_eq!(rec.log().len(), 15);
    }

    #[test]
    fn a_recorded_log_replays_to_the_same_state() {
        let mut rec = Recorder::from_reset(&rom()).unwrap();
        rec.mash_until("frame 400", keys::A, 400, |emu| emu.frame() >= 400)
            .unwrap();
        let expected = rec.emu().ram_hash().unwrap();
        let log = rec.log();

        let mut fresh = Emu::new(&rom()).unwrap();
        fresh.reset();
        fresh.replay(&log, |_, _| {});
        assert_eq!(fresh.ram_hash().unwrap(), expected);
    }

    #[test]
    fn timeouts_name_what_they_were_waiting_for() {
        let mut rec = Recorder::from_reset(&rom()).unwrap();
        let err = rec
            .wait_until("the impossible", 5, |_| false)
            .expect_err("should time out");
        assert!(err.to_string().contains("the impossible"), "{err}");
        // The frames it burned are still in the log: the recorder never lies
        // about what the emulator was fed.
        assert_eq!(rec.frames(), 5);
    }

    #[test]
    fn impossible_input_is_refused_before_it_reaches_the_core() {
        let mut rec = Recorder::from_reset(&rom()).unwrap();
        assert!(rec.step(keys::LEFT | keys::RIGHT).is_err());
        assert!(rec.step(0x8000).is_err());
        assert_eq!(rec.frames(), 0);
    }
}
