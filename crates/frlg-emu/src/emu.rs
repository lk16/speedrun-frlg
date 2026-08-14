//! A safe, single-threaded wrapper around one libmgba GBA core.
//!
//! `Emu` holds a raw pointer, so it is neither `Send` nor `Sync`. That is the
//! intended shape: a parallel input search gives each worker its own `Emu`
//! rather than sharing one.

use std::ffi::{c_char, c_void, CString};
use std::path::Path;
use std::sync::Once;

use sha1::{Digest, Sha1};

use crate::inputlog::InputLog;

/// GBA main RAM regions, used for the RAM fingerprint.
pub const EWRAM_START: u32 = 0x0200_0000;
pub const EWRAM_LEN: u32 = 0x0004_0000;
pub const IWRAM_START: u32 = 0x0300_0000;
pub const IWRAM_LEN: u32 = 0x0000_8000;

#[derive(Debug, thiserror::Error)]
pub enum EmuError {
    #[error("path {0} contains a NUL byte")]
    BadPath(String),
    #[error("libmgba could not create a core or load the ROM at {0}")]
    Create(String),
    #[error("libmgba could not load the BIOS at {0}")]
    Bios(String),
    #[error(
        "{path} is not the World GBA BIOS (sha1 {sha1}, wanted {}); \
         refusing to boot from it",
        crate::GBA_BIOS_SHA1
    )]
    WrongBios { path: String, sha1: String },
    #[error("libmgba refused to {op} a savestate")]
    State { op: &'static str },
    #[error("savestate is {actual} bytes, this core wants {expected}")]
    StateSize { expected: usize, actual: usize },
    #[error("no memory block covers {addr:#010x}")]
    NoMemoryBlock { addr: u32 },
    #[error("input log was routed against ROM {expected}, but this ROM is {actual}")]
    RomMismatch { expected: String, actual: String },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// A raw core savestate: fixed size, no savedata, cheap to take and restore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveState(Vec<u8>);

impl SaveState {
    /// Wraps bytes that came from somewhere other than [`Emu::save_state`].
    /// The size is checked against the core at load time, not here.
    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

pub struct Emu {
    raw: *mut mgba_sys::FrlgCore,
}

static SILENCE: Once = Once::new();

impl Emu {
    /// Creates a core, loads the ROM and resets. The first call also installs a
    /// no-op log handler, without which the core prints DMA and BIOS chatter to
    /// stdout and drowns anything else reporting there.
    pub fn new(rom: &Path) -> Result<Self, EmuError> {
        SILENCE.call_once(|| unsafe { mgba_sys::frlg_silence_logs() });

        let display = rom.display().to_string();
        let path =
            CString::new(display.as_bytes()).map_err(|_| EmuError::BadPath(display.clone()))?;
        let raw = unsafe { mgba_sys::frlg_core_new(path.as_ptr()) };
        if raw.is_null() {
            return Err(EmuError::Create(display));
        }
        Ok(Self { raw })
    }

    /// Loads a real GBA BIOS and resets. Without this mGBA uses its HLE BIOS.
    ///
    /// `skip_intro: false` runs the BIOS boot animation, which is the only
    /// boot BizHawk uses for a movie: replaying a `.bk2` requests
    /// deterministic emulation, and `MGBAHawk.cs:41` (2.11.1) overrides the
    /// SyncSettings' `SkipBios: true` to false in exactly that case. `true`
    /// exists for interactive experiments only; a log built on it is shifted
    /// against tier 2 by the whole intro.
    pub fn load_bios(&mut self, bios: &Path, skip_intro: bool) -> Result<(), EmuError> {
        let display = bios.display().to_string();
        let path =
            CString::new(display.as_bytes()).map_err(|_| EmuError::BadPath(display.clone()))?;
        if unsafe { mgba_sys::frlg_core_load_bios(self.raw, path.as_ptr(), skip_intro.into()) } == 0
        {
            return Err(EmuError::Bios(display));
        }
        Ok(())
    }

    pub fn reset(&mut self) {
        unsafe { mgba_sys::frlg_core_reset(self.raw) }
    }

    /// Latches `keys` and advances exactly one frame.
    pub fn step(&mut self, keys: u16) {
        unsafe { mgba_sys::frlg_run_frame(self.raw, keys) }
    }

    /// Advances `count` frames with nothing held.
    pub fn idle(&mut self, count: u32) {
        for _ in 0..count {
            self.step(0);
        }
    }

    /// Replays a whole log from the current state, calling `on_frame` after each
    /// frame with the frame index within the log.
    pub fn replay<F: FnMut(&mut Self, usize)>(&mut self, log: &InputLog, mut on_frame: F) {
        for (index, &keys) in log.frames.iter().enumerate() {
            self.step(keys);
            on_frame(self, index);
        }
    }

    /// Frames since reset, as libmgba counts them.
    pub fn frame(&self) -> u32 {
        unsafe { mgba_sys::frlg_frame_counter(self.raw) }
    }

    pub fn read8(&mut self, addr: u32) -> u8 {
        unsafe { mgba_sys::frlg_read8(self.raw, addr) as u8 }
    }

    pub fn read16(&mut self, addr: u32) -> u16 {
        unsafe { mgba_sys::frlg_read16(self.raw, addr) as u16 }
    }

    pub fn read32(&mut self, addr: u32) -> u32 {
        unsafe { mgba_sys::frlg_read32(self.raw, addr) }
    }

    pub fn read_bytes(&mut self, addr: u32, len: u32) -> Vec<u8> {
        let mut out = vec![0u8; len as usize];
        unsafe { mgba_sys::frlg_read_range(self.raw, addr, out.as_mut_ptr(), out.len()) };
        out
    }

    pub fn write8(&mut self, addr: u32, value: u8) {
        unsafe { mgba_sys::frlg_write8(self.raw, addr, value) }
    }

    /// Borrows the emulator's memory block containing `addr` directly, for bulk
    /// reads that would be slow one bus access at a time.
    pub fn with_memory_block<T>(
        &mut self,
        addr: u32,
        f: impl FnOnce(&[u8], usize) -> T,
    ) -> Result<T, EmuError> {
        let mut size: usize = 0;
        let mut offset: u32 = 0;
        let base = unsafe { mgba_sys::frlg_memory_block(self.raw, addr, &mut size, &mut offset) };
        if base.is_null() || size == 0 {
            return Err(EmuError::NoMemoryBlock { addr });
        }
        // SAFETY: the block is owned by the core, stays mapped for as long as
        // `self` lives, and `f` cannot outlive this borrow.
        let block = unsafe { std::slice::from_raw_parts(base as *const u8, size) };
        Ok(f(block, offset as usize))
    }

    /// sha1 over EWRAM followed by IWRAM. This is the divergence fingerprint:
    /// unlike a raw savestate it contains no emulator-internal padding, so two
    /// identical runs always agree.
    ///
    /// Deliberately fallible rather than falling back to bus reads. A silent
    /// fallback would still produce a plausible hash while hiding the fact that
    /// the fast path stopped working.
    pub fn ram_hash(&mut self) -> Result<String, EmuError> {
        let mut hasher = Sha1::new();
        for (start, len) in [(EWRAM_START, EWRAM_LEN), (IWRAM_START, IWRAM_LEN)] {
            let chunk = self.with_memory_block(start, |block, offset| {
                let end = (offset + len as usize).min(block.len());
                block[offset..end].to_vec()
            })?;
            if chunk.len() != len as usize {
                return Err(EmuError::NoMemoryBlock { addr: start });
            }
            hasher.update(&chunk);
        }
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn state_size(&mut self) -> usize {
        unsafe { mgba_sys::frlg_state_size(self.raw) }
    }

    pub fn save_state(&mut self) -> Result<SaveState, EmuError> {
        let mut buf = vec![0u8; self.state_size()];
        let ok = unsafe { mgba_sys::frlg_state_save(self.raw, buf.as_mut_ptr() as *mut c_void) };
        if ok == 0 {
            return Err(EmuError::State { op: "take" });
        }
        Ok(SaveState(buf))
    }

    pub fn load_state(&mut self, state: &SaveState) -> Result<(), EmuError> {
        let expected = self.state_size();
        if state.0.len() != expected {
            return Err(EmuError::StateSize {
                expected,
                actual: state.0.len(),
            });
        }
        let ok = unsafe { mgba_sys::frlg_state_load(self.raw, state.0.as_ptr() as *const c_void) };
        if ok == 0 {
            return Err(EmuError::State { op: "restore" });
        }
        Ok(())
    }

    /// A full serialized state on disk, savedata included. This is what a route
    /// checkpoint should be; [`Emu::save_state`] is for the inner search loop.
    pub fn save_state_file(&mut self, path: &Path) -> Result<(), EmuError> {
        let display = path.display().to_string();
        let c = CString::new(display.as_bytes()).map_err(|_| EmuError::BadPath(display))?;
        if unsafe { mgba_sys::frlg_state_save_file(self.raw, c.as_ptr()) } == 0 {
            return Err(EmuError::State { op: "write" });
        }
        Ok(())
    }

    pub fn load_state_file(&mut self, path: &Path) -> Result<(), EmuError> {
        let display = path.display().to_string();
        let c = CString::new(display.as_bytes()).map_err(|_| EmuError::BadPath(display))?;
        if unsafe { mgba_sys::frlg_state_load_file(self.raw, c.as_ptr()) } == 0 {
            return Err(EmuError::State { op: "read" });
        }
        Ok(())
    }

    pub fn width(&self) -> u32 {
        unsafe { mgba_sys::frlg_width(self.raw) }
    }

    pub fn height(&self) -> u32 {
        unsafe { mgba_sys::frlg_height(self.raw) }
    }

    /// The current frame as RGBA8, `width * height * 4` bytes, alpha forced
    /// opaque.
    pub fn screen_rgba(&self) -> Vec<u8> {
        let pixels = (self.width() * self.height()) as usize;
        // SAFETY: the shim allocated width*height color_t and keeps it alive
        // for the lifetime of the core.
        let buffer =
            unsafe { std::slice::from_raw_parts(mgba_sys::frlg_video_buffer(self.raw), pixels) };
        let mut out = Vec::with_capacity(pixels * 4);
        for &pixel in buffer {
            out.extend_from_slice(&[
                (pixel & 0xff) as u8,
                ((pixel >> 8) & 0xff) as u8,
                ((pixel >> 16) & 0xff) as u8,
                0xff,
            ]);
        }
        out
    }

    /// Sample-frames per second the core is producing right now.
    ///
    /// Not a constant: the GBA derives it from SOUNDBIAS's resolution field
    /// (`mgba/src/gba/audio.c:231`), which is 9-bit at reset (32768 Hz) and
    /// writable by the game. A capture that assumes one rate should read this
    /// back and notice if it moves.
    pub fn audio_sample_rate(&self) -> u32 {
        unsafe { mgba_sys::frlg_audio_sample_rate(self.raw) }
    }

    /// Interleave width of [`Emu::drain_audio`]: 2 on the GBA.
    pub fn audio_channels(&mut self) -> u32 {
        unsafe { mgba_sys::frlg_audio_channels(self.raw) }
    }

    /// Discards buffered audio. Worth doing after a reset, so the first
    /// captured frame carries no samples from before it.
    pub fn clear_audio(&mut self) {
        unsafe { mgba_sys::frlg_audio_clear(self.raw) }
    }

    /// Appends every buffered sample-frame to `out`, interleaved, and returns
    /// how many were appended.
    ///
    /// The core's buffer is 0x4000 frames and it drops what does not fit, so
    /// this has to be called every frame to capture a run losslessly -- at
    /// 32768 Hz a video frame is only ~549 of them.
    pub fn drain_audio(&mut self, out: &mut Vec<i16>) -> usize {
        let channels = self.audio_channels() as usize;
        if channels == 0 {
            return 0;
        }
        let mut total = 0;
        loop {
            let want = 4096;
            let base = out.len();
            out.resize(base + want * channels, 0);
            // SAFETY: `out` has `want * channels` i16 of spare capacity from
            // `base`, which is what the shim is allowed to write.
            let got =
                unsafe { mgba_sys::frlg_audio_read(self.raw, out[base..].as_mut_ptr(), want) };
            out.truncate(base + got * channels);
            total += got;
            if got < want {
                return total;
            }
        }
    }

    pub fn write_png(&self, path: &Path) -> Result<(), EmuError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let file = std::fs::File::create(path)?;
        let mut encoder =
            png::Encoder::new(std::io::BufWriter::new(file), self.width(), self.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        writer
            .write_image_data(&self.screen_rgba())
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }

    pub fn game_title(&self) -> String {
        let mut buf = [0 as c_char; 16];
        unsafe { mgba_sys::frlg_game_title(self.raw, buf.as_mut_ptr()) };
        c_str(&buf)
    }

    pub fn game_code(&self) -> String {
        let mut buf = [0 as c_char; 8];
        unsafe { mgba_sys::frlg_game_code(self.raw, buf.as_mut_ptr()) };
        c_str(&buf)
    }

    pub fn rom_size(&self) -> usize {
        unsafe { mgba_sys::frlg_rom_size(self.raw) }
    }
}

fn c_str(buf: &[c_char]) -> String {
    buf.iter()
        .take_while(|&&byte| byte != 0)
        .map(|&byte| byte as u8 as char)
        .collect::<String>()
        .trim()
        .to_string()
}

impl Drop for Emu {
    fn drop(&mut self) {
        unsafe { mgba_sys::frlg_core_free(self.raw) }
    }
}
