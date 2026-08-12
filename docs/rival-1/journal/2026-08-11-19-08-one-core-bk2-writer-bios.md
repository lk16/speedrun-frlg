# 2026-08-11 (later, host) -- same core on both tiers, a .bk2 writer, and BIOS wiring; the route re-rolled to 12209

Worked the three items `docs/rival-1/route.md` still listed under tier 2, on the host (network, mono,
docker all present). Two are closed; the third is wired and waits on one file.

**Both tiers now run mGBA `94b1578f`** -- BizHawk 2.11.1's own submodule gitlink, self-reported
0.11.0. `MGBA_REF` defaults to it, the deps tree is rebuilt, and `bin/frlg-doctor`'s `mgba pin`
check now passes when our pin equals the recorded submodule. The shim port
(`crates/mgba-sys/csrc/shim.c`) took four changes: `getGameTitle`/`getGameCode` →
`getGameInfo` (the "AGB-BPRE" format is reconstructed so the Rust side is untouched),
`desiredVideoDimensions` → `baseVideoSize`, `color_t` → `mColor`, and an explicit
`#include <mgba/flags.h>` since 0.11's `common.h` no longer pulls it in. The trap worth
remembering: **the installed `flags.h` lies about `ENABLE_DIRECTORIES`** -- upstream
`CMakeLists.txt:869` appends the compile definition whenever `ENABLE_VFS` is on, but no cmake
*variable* of that name exists, so `#cmakedefine ENABLE_DIRECTORIES` stays undefined. The flag
gates a 4152-byte `struct mDirectorySet` embedded in `struct mCore` ahead of the vtable, so the
shim compiled clean and then called a NULL pointer. Diagnosed by dumping the real allocation
(vtable starts at byte 4856; our `offsetof(init)` said 704; the difference is exactly
`sizeof(mDirectorySet)`); the shim now defines the flag itself, with the citation.

**The pin moved the route: 11873 → 12209.** On the new core, segments 01-07 replay bit-identically
to their observables (same frame counts; RAM digests differ, as expected between core versions),
and the old `08-battle-win` log *loses* -- the battle RNG stream is not the same. `frlg route
build` re-searched the 16 start delays (8 win now), kept delay 0, and the chosen battle is 3797
frames. Every number that predates the pin is labelled as such in `docs/rival-1/route.md`. The lesson from
2026-08-10 generalises: the battle is a hash of everything upstream, *including the emulator*.

**`frlg route export` writes the `.bk2`** (`crates/frlg-route/src/bk2.rs`). Template entries are
copied verbatim, only `Input Log.txt` is generated; the ledger's digests gate which logs may be
exported; every export decodes its own output back to masks and compares before reporting
success, and deletes the file on mismatch. The button mnemonics (`U D L R S s B A l r P`) came
out of BizHawk's `ControllerDefinition.MnemonicsCache` under mono -- `Bk2MnemonicLookup`, which
older notes named, no longer exists in 2.11.1 -- and were cross-checked by generating a log entry
per button with BizHawk's own `Bk2LogEntryGenerator`. The exported route reads back through
BizHawk's `Bk2Movie.Load`: 12209 frames, header intact. Export queues
`verify/queue/<id>.bk2` + `<id>.json` (the `docs/rival-1/route.md` contract, plus `bios`), and the
ledger's `tier2` line now says "not replayed", not "blocked".

**The BIOS gap is wired shut from our side.** `frlg_emu::boot_with_default_bios` boots every
route/run/info core from `$FRLG_GBA_BIOS`, else `$BIZHAWK_HOME/Firmware/GBA_bios.rom`, the moment
the file exists -- sha1-pinned to the World BIOS (`300c20df…`), refusing anything else, intro
skipped via `opts.skipBios`, which lands in the same `GBASkipBIOS` BizHawk's glue calls
(`src/platform/bizhawk/bizinterface.c:171` at the pinned commit; its `skipbios` comes from the
movie SyncSettings, where `SkipBios` is true in our template). The ledger records `bios: "hle" |
"bios:<sha1>"` per build; `verify` refuses a boot mismatch; `export` warns on an HLE route;
doctor prints the BIOS state every startup. **When the file lands: rebuild, verify, export** --
the battle will re-roll again (real-BIOS SWIs are not HLE-cycle-identical), and that rebuild is
the point, not a regression.

**Unverified.** Everything tier 2 still: the runner has never replayed a movie, and the queued
`route-12209f-fb2fc4969219.bk2` is expected to desync if replayed before the route is rebuilt on
the real BIOS -- it exists to exercise the pipeline, and its request json says `"bios": "hle"`.
