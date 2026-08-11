//! Raw declarations for the shim in `csrc/shim.c`.
//!
//! Nothing here mirrors a libmgba struct. `FrlgCore` is opaque and every entry
//! point takes plain scalars, so this file cannot drift out of layout sync with
//! the library the way a hand-transcribed `struct mCore` would.
//!
//! Safety contract for every function below: `core` must be a non-null pointer
//! returned by [`frlg_core_new`] and not yet passed to [`frlg_core_free`].

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_int, c_uint, c_void};

/// Opaque handle. Constructed and destroyed only by the shim.
#[repr(C)]
pub struct FrlgCore {
    _private: [u8; 0],
}

extern "C" {
    /// Installs a no-op logger. Idempotent. Until this is called the core
    /// writes DMA and BIOS-call chatter to stdout.
    pub fn frlg_silence_logs();

    /// Creates a core, loads `rom_path`, and resets. Returns null on failure.
    pub fn frlg_core_new(rom_path: *const c_char) -> *mut FrlgCore;
    /// Returns nonzero on success. Resets the core. `skip_intro = 0` runs the
    /// BIOS boot animation, which is how BizHawk plays back a movie; nonzero
    /// skips it (interactive experiments only).
    pub fn frlg_core_load_bios(
        core: *mut FrlgCore,
        bios_path: *const c_char,
        skip_intro: c_int,
    ) -> c_int;
    pub fn frlg_core_free(core: *mut FrlgCore);
    pub fn frlg_core_reset(core: *mut FrlgCore);

    pub fn frlg_run_frame(core: *mut FrlgCore, keys: u16);
    pub fn frlg_frame_counter(core: *const FrlgCore) -> u32;

    pub fn frlg_read8(core: *mut FrlgCore, addr: u32) -> u32;
    pub fn frlg_read16(core: *mut FrlgCore, addr: u32) -> u32;
    pub fn frlg_read32(core: *mut FrlgCore, addr: u32) -> u32;
    /// `out` must be valid for `len` bytes.
    pub fn frlg_read_range(core: *mut FrlgCore, addr: u32, out: *mut u8, len: usize);
    pub fn frlg_write8(core: *mut FrlgCore, addr: u32, value: u8);

    /// Base pointer of the memory block containing `addr`, or null. Writes the
    /// block size and the offset of `addr` within it.
    pub fn frlg_memory_block(
        core: *mut FrlgCore,
        addr: u32,
        size_out: *mut usize,
        offset_out: *mut u32,
    ) -> *mut c_void;

    pub fn frlg_state_size(core: *mut FrlgCore) -> usize;
    /// `buf` must be valid for [`frlg_state_size`] bytes.
    pub fn frlg_state_save(core: *mut FrlgCore, buf: *mut c_void) -> c_int;
    /// `buf` must be valid for [`frlg_state_size`] bytes.
    pub fn frlg_state_load(core: *mut FrlgCore, buf: *const c_void) -> c_int;
    pub fn frlg_state_save_file(core: *mut FrlgCore, path: *const c_char) -> c_int;
    pub fn frlg_state_load_file(core: *mut FrlgCore, path: *const c_char) -> c_int;

    pub fn frlg_width(core: *const FrlgCore) -> c_uint;
    pub fn frlg_height(core: *const FrlgCore) -> c_uint;
    /// `width * height` pixels, R in bits 0-7 through A in bits 24-31.
    pub fn frlg_video_buffer(core: *const FrlgCore) -> *const u32;

    /// `out` must be valid for 16 bytes.
    pub fn frlg_game_title(core: *const FrlgCore, out: *mut c_char);
    /// `out` must be valid for 8 bytes.
    pub fn frlg_game_code(core: *const FrlgCore, out: *mut c_char);
    pub fn frlg_rom_size(core: *const FrlgCore) -> usize;
}
