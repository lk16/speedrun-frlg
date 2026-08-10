# The tier-1 harness

Load the ROM into libmgba, feed it one key mask per frame, read RAM, dump a frame. This is the
loop the route is optimised against. Tier 2 -- BizHawk replaying a `.bk2` -- does not run in this
sandbox and nothing here claims a route is accepted.

    cargo build --release
    ./target/release/frlg info

## Layout

| Crate | What it is |
| --- | --- |
| `crates/mgba-sys` | `csrc/shim.c`, a flat C ABI over `struct mCore`, plus its `extern "C"` block |
| `crates/frlg-emu` | `Emu`, the input-log format, the decomp's key bits, `pokefirered.sym` |
| `crates/frlg-cli` | the `frlg` binary |

### Why there is C in a Rust project

`struct mCore` is ~90 function pointers with four structs embedded by value, and its layout moves
with the flags the library was built with -- `$MGBA_PREFIX/include/mgba/flags.h` records
`USE_DEBUGGERS` and `ENABLE_SCRIPTING` on, both of which add fields. Transcribing that by hand into
Rust is a silent-corruption bug waiting to happen, and bindgen is not available (no clang here).
Compiling a shim against the *installed* headers makes the C compiler derive the layout from the
same `flags.h` the `.so` was built with, and Rust only ever sees opaque pointers and scalars.

`cc` is vendored, so `cc::Build` works offline. `build.rs` reads `$MGBA_PREFIX` and fails with a
pointed message if it is unset.

## The canonical artifact

One `u16` key mask per frame, in the decomp's bit order -- `A_BUTTON 0x0001` through
`L_BUTTON 0x0200`, `KEYS_MASK 0x03FF`, from `include/gba/io_reg.h`. That is what the game reads and
what `setKeys` wants.

It is **not** the `.bk2` Input Log column order, which is BizHawk's, lives in compiled CIL, and is
not derivable from anything mounted here. So the raw log stays canonical and `.bk2` will be an
export of it: a column-order mistake then costs a re-export, not a re-route.

`.ilog` is a 40-byte header (magic, version, frame count, the sha1 of the ROM it was routed
against) followed by the masks, little-endian. A log's ledger identity is
`sha1(frame payload)` -- the header is excluded so it does not move when the header gains fields.
`frlg log to-text` / `from-text` convert to a run-length text form for review and diffing; the
round trip is byte-identical.

Two things are refused rather than passed through: bits outside `KEYS_MASK`, and opposing d-pad
directions (`LEFT|RIGHT`, `UP|DOWN`), which hardware cannot produce and which the two emulators may
filter differently.

## Commands

    frlg info                                   # rom sha1, title, code, state size, screen
    frlg run --frames 3000 --png title.png      # boot with nothing held
    frlg run --input seg.ilog --ram-hash        # replay a log, fingerprint the result
    frlg run --input seg.ilog --watch gRngValue --watch gMain+0x10:2
    frlg run --frames 600 --trace gRngValue --trace-out rng.csv
    frlg run --load-state a.state --frames 120 --save-state b.state
    frlg sym Rng                                # search pokefirered.sym
    frlg log show seg.ilog

The ROM defaults to `$FRLG_ROM`, then `$FRLG_ARTIFACTS/rom/pokefirered.gba`; symbols to `$FRLG_SYM`,
then `$FRLG_ARTIFACTS/rom/pokefirered.sym`. `--watch` and `--trace` take `name`, `name+0x10`,
`0x03005000`, any of them with `:len`; widths 1, 2 and 4 read as little-endian integers, anything
else is a byte dump.

Replaying a log against the wrong ROM is refused by comparing sha1s. An all-zero hash in a log means
"unknown" and is allowed.

## The divergence fingerprint

`--ram-hash` is sha1 over EWRAM (`0x02000000`, 256K) then IWRAM (`0x03000000`, 32K), read straight
out of the emulator's memory blocks. Prefer it to hashing a raw savestate: a savestate carries
emulator-internal padding, and a fingerprint that can differ between two identical runs is worse
than none. It is deliberately fallible -- an earlier version fell back to bus reads when the block
lookup failed, which produced a plausible hash while hiding that the fast path had broken.

## Savestates

Two kinds, on purpose:

- `Emu::save_state` -- the raw core state, 397312 bytes, in memory. No savedata. This is the one for
  the inner search loop: save, try inputs, restore.
- `Emu::save_state_file` -- `mCoreSaveStateNamed` with savedata and RTC, 528448 bytes on disk. This
  is what a route checkpoint should be, since it does not need an SRAM file alongside it.

`mCoreSaveStateNamed` needs a VFile opened `O_RDWR`; it maps the file to write the core state and
returns false on a write-only one, after having written most of the file. Confirmed against the
library, not assumed.

## What is measured

- ~1000 frames/sec per core in release on this sandbox (3000 frames in 3.0 s, single-threaded).
- FireRed reaches the title screen inside 3000 frames from reset.
- `Emu` holds a raw pointer, so it is neither `Send` nor `Sync`. A parallel input search gives each
  worker its own `Emu` rather than sharing one; the type system enforces it.

## Caveats worth carrying into tier 2

- **HLE BIOS.** No GBA BIOS exists in this sandbox (`$BIZHAWK_HOME/Firmware` is empty), so mGBA runs
  its HLE BIOS. Whether BizHawk's mGBA core does the same is unverified, and HLE-vs-real is a
  plausible divergence axis. `Emu::load_bios` exists so testing that needs a file, not a code change.
- **No `.bk2` writer yet**, deliberately -- see the column-order note above.
- **`cargo fmt` and `cargo clippy` are unavailable.** The prebuilt toolchain under `$FRLG_DEPS/rust`
  ships only cargo/rustc/rustdoc, and components cannot be added offline. Fixing that is a host-side
  change to `tools/host-prep.sh` (`rustup component add rustfmt clippy`) plus a fresh sandbox.

## Tests

`cargo test --release` runs 18 unit tests and 10 that drive the real ROM: boot, determinism across
two replays, input actually reaching the game, savestate round trips in memory and on disk, the
memory-block view agreeing with bus reads, split-replay equalling one pass, and the screenshot being
the right shape and opaque. They need the ROM and fail loudly without it rather than skipping.

Note that `cargo test --release` does not relink `target/release/frlg`; run `cargo build --release`
before trusting the CLI binary.
