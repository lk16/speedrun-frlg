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
| `crates/frlg-route` | the route: cited RAM probes, a recorder, path search, the ledger (`docs/rival-1/route.md`) |
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

It is **not** the `.bk2` Input Log column order, which is BizHawk's and lives in compiled CIL.
`route/template.bk2` now states that order outright (`docs/rival-1/route.md`), but the raw log stays
canonical and `.bk2` remains an export of it: a column-order mistake then costs a re-export, not a
re-route.

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
    frlg log cat a.ilog b.ilog -o whole.ilog    # join segments into one run
    frlg route build                            # run the route (docs/rival-1/route.md)
    frlg route build --target defeat-brock      # a longer target: same builder, more segments,
                                                #   paths default to route/<target>/
    frlg route verify                           # replay the committed logs and check the ledger
    frlg route tune                             # sweep the route knobs, score on total frames
    frlg route status --target defeat-brock     # print one ledger, without running anything
    frlg route list                             # every TAS in the repo, one ledger line each
    frlg video --target rival-1                 # the publishable recording (see below)

The ROM defaults to `$FRLG_ROM`, then `$FRLG_ARTIFACTS/rom/pokefirered.gba`; symbols to `$FRLG_SYM`,
then `$FRLG_ARTIFACTS/rom/pokefirered.sym`. `--watch` and `--trace` take `name`, `name+0x10`,
`0x03005000`, any of them with `:len`; widths 1, 2 and 4 read as little-endian integers, anything
else is a byte dump.

Replaying a log against the wrong ROM is refused by comparing sha1s. An all-zero hash in a log means
"unknown" and is allowed.

## Recording a run

`frlg video` is the only command that produces something for people who will never read a ledger,
and the only one that needs a tool the sandbox does not have: **ffmpeg** (`sudo apt-get install
ffmpeg` on the host). It writes one dated folder under `ignored/videos/`, named like a journal
entry, holding the video and a markdown file with the title and description to publish it with.

It refuses unless the run is **tier-2 verified and committed**. Tier 1 is not enough: a video
cannot be corrected after upload, so the gate asks for BizHawk's verdict in every segment's
`tier2` field, for the ledger and every log it names to be tracked and unmodified, and it links the
commit that *introduced* that verdict rather than whatever HEAD happens to be
(`crates/frlg-route/src/publish.rs`).

- **Format.** `--format mp4` (default) is lossless H.264 in RGB (`libx264rgb -qp 0`) plus ALAC;
  `--format mkv` is FFV1 plus FLAC. Both are bit-exact -- checked by encoding a raw stream,
  decoding it back and comparing bytes -- but the mkv is 3-4x larger, because FFV1 codes every
  frame independently and a GBA screen mostly does not move. rival-1's 9658 frames come to 32 MiB
  as mp4.
- **Scale.** `--scale 4` (default) is a nearest-neighbour upscale to 960x640: every output pixel is
  a copy of one input pixel, so it stays reversible, but a video site will not throw away a 240-line
  source's bitrate. `--scale 1` keeps the native frame.
- **Sound.** Captured from the core, not synthesised. The GBA's sample rate is not constant -- the
  game rewrites SOUNDBIAS, and FireRed/LeafGreen move between 32768 and 65536 Hz during the boot --
  so each frame's samples are laid onto the *video's* clock at the highest rate seen. That is what
  keeps picture and sound in step across a rate change instead of drifting by the samples the
  slower stretch did not produce.
- **Cost.** Two replays, one for sound and one for picture, plus the encode: about a minute for
  rival-1. ffmpeg needs its audio input to exist before it starts, and the raw picture is far too
  large to spool, so the deterministic replay is done twice rather than buffered once.

`--preview-frames N` encodes only the first N frames and writes no description; it is for checking
the pipeline, not for publishing.

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

Two of these used to be "plausible" and are now measured. Both are divergences the tier-1 loop is
structurally incapable of noticing, which is what makes them worth carrying rather than filing.

- **The boot is the real BIOS *with the intro played*, because that is the only boot BizHawk
  uses for a movie.** Loading a movie sets `DeterministicEmulationRequested`; without a BIOS
  `MGBAHawk`'s constructor then throws
  `MissingFirmwareException("A BIOS is required for deterministic recordings!")`, and *with* one
  it overrides the SyncSettings' `SkipBios: true` to false (`MGBAHawk.cs:41`,
  `skipBios: _syncSettings.SkipBios && !lp.DeterministicEmulationRequested`), so the ~272-frame
  boot animation always runs with movie input already being consumed. Tier 1 booting
  skip-intro was exactly the 2026-08-11 bedroom desync: the whole log ran ~272 frames early and
  the first frame-exact walking died. `frlg_emu::boot_with_default_bios` boots from
  `$FRLG_GBA_BIOS`/`$BIZHAWK_HOME/Firmware/GBA_bios.rom` the moment it exists, sha1-pinned to
  the World BIOS, intro *not* skipped (`Emu::load_bios(_, false)`); the ledger records the boot
  per build (`"hle"` or `"bios+intro:<sha1>"`, the retired `"bios:<sha1>"` meant skip-intro),
  and `frlg route verify` refuses to replay logs under another boot. A log built HLE or
  skip-intro must be expected to desync at tier 2.
- **The two tiers run the same mGBA commit since 2026-08-11.** `MGBA_REF` is `94b1578f`, the exact
  submodule gitlink BizHawk 2.11.1 ships (self-reported 0.11.0). The shim port this took:
  `getGameTitle`/`getGameCode` became `getGameInfo`; `desiredVideoDimensions` became
  `baseVideoSize`; `color_t` became `mColor`; and -- the sharp edge -- 0.11's headers no longer
  include `flags.h`, and the installed `flags.h` *lies about `ENABLE_DIRECTORIES`* (upstream
  `CMakeLists.txt:869` adds the define without a cmake variable behind it), which silently shifts
  every function pointer in `struct mCore` by 4152 bytes. `csrc/shim.c` documents and corrects
  both. `bin/frlg-doctor` confirms the pin at every startup; re-check the shim whenever the pin
  moves. Re-pinning re-rolled the battle RNG (`docs/rival-1/route.md`).
- **The `.bk2` writer exists**: `frlg route export`, built on `route/template.bk2`, round-trip
  checked on every export (`docs/rival-1/route.md`). The `.ilog` stays canonical; a `.bk2` is an export
  of it.

## Tests

`cargo test --release` runs 30 unit tests (20 in `frlg-emu`, 10 in `frlg-route`) and 17 that drive
the real ROM (10 `harness.rs`, 6 `observe.rs`, 1 `route.rs`): boot, determinism across two replays,
input actually reaching the game, savestate round trips in memory and on disk, the memory-block view
agreeing with bus reads, split-replay equalling one pass, and the screenshot being the right shape
and opaque. They need the ROM and fail loudly without it rather than skipping — `route.rs` looks
its ROM up by the ledger's `rom_sha1`, so it follows the route across versions. The `frlg-route`
unit tests include the `.bk2` round trip, which uses the committed `route/template.bk2` and no ROM.
One more is `#[ignore]`d on purpose: `text_hold_on_the_intro_alone` is a minutes-long measurement,
not a regression test (`docs/rival-1/route.md`).

Note that `cargo test --release` does not relink `target/release/frlg`; run `cargo build --release`
before trusting the CLI binary.
