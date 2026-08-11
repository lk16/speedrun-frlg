# Session journal

Newest first. Continuity is something you write down; a sandbox ends mid-thought. Anything
unverified says so.

## 2026-08-12 (sandbox) -- the bedroom desync was the boot: BizHawk never skips the BIOS intro for a movie

**Root cause, cited.** `MGBAHawk.cs:41` (2.11.1 sources in
`$FRLG_ARTIFACTS/reference/bizhawk-2.11.1/`) constructs the core with
`skipBios: _syncSettings.SkipBios && !lp.DeterministicEmulationRequested`. Movie playback
requests deterministic emulation -- that is the same condition that made line 30 throw
`MissingFirmwareException` on the host until the BIOS existed -- so the template's
`SkipBios: true` is dead on replay, `bizinterface.c:171`'s `GBASkipBIOS` never runs, and EmuHawk
plays every movie through the ~272-frame BIOS boot animation while consuming movie input. Tier 1
booted `opts.skipBios = true`, so on BizHawk the entire log ran ~272 frames early. The failure
shape matches the watched replay exactly: mash segments absorb a constant shift, and the first
frame-exact walking (bedroom -> downstairs) dies. The other suspects were checked against the
sources while getting here and came up equal for retail BPRE: overrides/savetype paths converge
(`GBAOverrideFind` static table, FLASH1M, HW_NONE), idle loop is a no-op on both (REMOVE with
`idleLoop == NONE` vs IGNORE), RTC is inert (no RTC hardware on BPRE), vbaBugCompat only touches
HLE SWIs and GPIO, neither of which this cartridge exercises.

**The fix.** `frlg_core_load_bios` grew a `skip_intro` flag; `boot_with_default_bios` boots
real-BIOS-with-intro and stamps the ledger `bios+intro:<sha1>` -- a new marker on purpose, so
the retired skip-intro `bios:<sha1>` evidence can never be mistaken for the new boot. Route
rebuilt: **12713 frames** (the intro costs ~272 at boot; the battle re-rolled to 4018 frames,
16/16 start delays win, delay 1 kept), tier-1 verified, exported, queued as
`route-12713f-a4ad4280bbdc`. The stale 12222-frame queue entry was withdrawn. Two tests assumed
the old boot (replay-from-HLE-reset, copyright screen at frame 60) and now boot/wait properly.

**Desyncs now come with a frame number (when the Lua cooperates).** `frlg route export` replays
the exported movie once on tier 1 and queues `<id>.trace` beside it: gRngValue after every frame
(the game advances it once per VBlank, `decompiled/src/main.c:412`), u32 LE per frame. The
replay doubles as an export gate -- a movie whose final fingerprint is not the ledger's refuses
to queue. `verify-runner.lua` compares the probe each frame and reports `desync_frame=`;
`verify-runner.sh` forwards it into the result json. The trace sanity-checks itself: its first
273 values are zero (BIOS animation, RNG unseeded), which independently measures the intro
length. Contract updated in `docs/route.md`.

**Unverified.** The fix's *effect*: no tier-2 result exists yet for the rebuilt movie. The
reasoning is cited but BizHawk has not replayed it. The Lua trace compare has never run (the Lua
has still never completed a report of any kind); its read API
(`memory.read_u32_le(offset, domain)`, framecount-1 indexing per `MGBAHawk.IEmulator.cs:83`) is
the least-tested part.

## 2026-08-11 (evening, host) -- tier 2 ran for the first time, and the first watched replay desyncs in the bedroom

**The BIOS exists.** A downloaded `gba_bios.zip` hashed to exactly
`300c20df6731a33952ded8c436f7f186d25d3492` (16384 bytes, the World BIOS) and is installed at
`$BIZHAWK_HOME/Firmware/GBA_bios.rom`. Doctor is green on it. The route was rebuilt booting from
it: **12222 frames** (real-BIOS boot costs 13 over HLE's 12209, spread over segments 01-03/07),
the battle re-rolled again and now **16/16 start delays win** -- delay 0 kept, 3797 frames.
Verified tier 1, exported, ledger says `bios:300c20df…`.

**Two runner bugs stood between the queue and EmuHawk**, both now fixed in
`tools/verify-runner.sh`: `--lua` was passed relative, and `EmuHawkMono.sh` cd's into the BizHawk
directory first, so the script was never found; and `--userdata` is not a data directory at all
-- it is movie key:value metadata whose parser exits 1 on a bare path ("malformed userdata",
found by bisecting the flags against a live EmuHawk). `--config="$USERDATA/config.ini"` is what
keeps the churn out of the deps tree.

**Then the real result: the movie plays and desyncs.** Watched on the GUI: power-on, menu mash,
naming screen, into the bedroom -- and the player never walks downstairs; the run stalls there.
The shape of the failure is informative: mash segments are robust to small input misalignment,
`nav`'s frame-exact walking is not, and walking is exactly where it died. Prime suspects, in
order: input-delivery timing (when BizHawk's mGBA latches a movie frame's keys vs. our
`setKeys`-then-`runFrame`), then the rest of `SyncSettings`/RTC. Both tiers run the same core
commit and BIOS boot now, so the emulator itself is off the suspect list -- which is what this
whole week of pinning bought. The runner's Lua report has still never completed, so there is no
frame number yet; that diagnosis is the top tier-2 item (`docs/route.md`).

**Watching the replay also put three route questions on the record** (now in `docs/route.md`,
"What is not optimised"): the name is a seven-A wall typed one press at a time and re-printed at
every name mention (one-character name and preset name both unmeasured); text speed is never set
to FAST and every message box in the run pays for it; and LeafGreen builds byte-exact but has
never been raced against FireRed. All three must be measured through the battle, and the version
question through a full build-and-tune.

**Unverified.** The desync location (eyeballed, no frame number); everything downstream of it.

## 2026-08-11 (later, host) -- same core on both tiers, a .bk2 writer, and BIOS wiring; the route re-rolled to 12209

Worked the three items `docs/route.md` still listed under tier 2, on the host (network, mono,
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
frames. Every number that predates the pin is labelled as such in `docs/route.md`. The lesson from
2026-08-10 generalises: the battle is a hash of everything upstream, *including the emulator*.

**`frlg route export` writes the `.bk2`** (`crates/frlg-route/src/bk2.rs`). Template entries are
copied verbatim, only `Input Log.txt` is generated; the ledger's digests gate which logs may be
exported; every export decodes its own output back to masks and compares before reporting
success, and deletes the file on mismatch. The button mnemonics (`U D L R S s B A l r P`) came
out of BizHawk's `ControllerDefinition.MnemonicsCache` under mono -- `Bk2MnemonicLookup`, which
older notes named, no longer exists in 2.11.1 -- and were cross-checked by generating a log entry
per button with BizHawk's own `Bk2LogEntryGenerator`. The exported route reads back through
BizHawk's `Bk2Movie.Load`: 12209 frames, header intact. Export queues
`verify/queue/<id>.bk2` + `<id>.json` (the `docs/route.md` contract, plus `bios`), and the
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

## 2026-08-11 -- tier 2 has a format, a runner, and one thing left that money cannot buy

Worked through `docs/sanity-2026-08-11.md` on the host. The route did not move; what moved is how
much of tier 2 is knowable.

**`route/template.bk2` exists, and was not recorded by hand.** The plan was to open BizHawk and
record a one-frame movie. That path dead-ends: loading a ROM into the mGBA core for a movie sets
`DeterministicEmulationRequested`, and `MGBAHawk`'s constructor throws
`MissingFirmwareException("A BIOS is required for deterministic recordings!")` — which EmuHawk
shows as the Firmware Manager dialog rather than returning an exit code. So the template is built
instead: `tools/bk2-template.sh` loads the shipped assemblies under mono, asks
`Bk2LogEntryGenerator.GenerateLogKey(MGBAHawk.GBAController)` and `ConfigService.SaveWithType(new
SyncSettings())`, and writes the file with BizHawk's own `Bk2Movie.Write`. No GUI, no core
instance, no BIOS, and it reads back through BizHawk's own loader. Reproducible beats recorded.

The column order, which two sessions had called underivable:

    #Tilt X|Tilt Y|Tilt Z|Light Sensor|Up|Down|Left|Right|Start|Select|B|A|L|R|Power|

Four analogue columns before any button, and `Power` last. Anyone emitting the ten buttons from
`defctrl.json` in `defctrl.json` order would have produced a file that loads and desyncs.

**Tier 2 needs a real GBA BIOS, and that is now the whole blocker.** Same IL as above. It has two
consequences worth carrying: an unattended runner hangs on a dialog instead of failing (so
`tools/verify-runner.sh` preflights for the file, sha1
`300C20DF6731A33952DED8C436F7F186D25D3492`), and **tier 1 and tier 2 do not boot the same way** —
tier 1 runs mGBA's HLE BIOS and tier 2 cannot. `Emu::load_bios` exists, so closing that is
configuration, not code. Until it is closed, an early desync has an obvious suspect that has
nothing to do with the route.

**The two tiers also run different emulators, and the fix is not one line.** BizHawk 2.11.1's
`[PortedCore]` attribute says mGBA `0.11`; its submodule gitlink is `94b1578f` (2026-03-03), an
untagged master commit. Built it and pointed the workspace at it: `crates/mgba-sys/csrc/shim.c`
does not compile, because 0.11 dropped `getGameTitle`/`getGameCode` from `struct mCore` and moved
`VFileOpen`. So `MGBA_REF` is now an explicit `0.10.5` rather than "newest 0.10.x", both versions
are recorded in the deps `MANIFEST`, and `bin/frlg-doctor` says the delta out loud at every
startup. Porting the shim is real work and is the next tier-2 item after the BIOS.

**Also done, from the same review.** A repo `CLAUDE.md` (the inherited one contradicts this
sandbox on nearly every point); seven new doctor checks, of which "the writable decomp copy still
matches the read-only mount" is the one that protects every citation in these docs; a decomp
revision stamp in the kit spec so a restarted sandbox cannot silently cite a stale tree; the
`decompiled/` symlink made at startup rather than only by doctor; the empty Python wheelhouse and
its `PIP_*` variables removed rather than filled in.

**Unverified.** `tools/verify-runner.sh` and `tools/verify-runner.lua` have never completed a
replay — they cannot until the BIOS exists. Treat the Lua side, especially its memory-domain
names, as the least-tested code here.

## 2026-08-10 -- the battle was luck, and the obvious trim made things worse

**The win was a coin flip.** Delaying the same A mash into the rival battle by one frame flipped it
from a win to a loss, alternating over twelve consecutive delays. So `08-battle-win` now searches:
16 start delays, keep the shortest that wins, print how many did. Same 11873 frames as before -- the
route did not get faster, it got *chosen* instead of lucky, which is what stops it losing the next
time something upstream moves.

**Criticals are off in this battle.** `gBattleTypeFlags` is `0x1C`, which includes
`BATTLE_TYPE_FIRST_BATTLE`, and the crit check is gated on that flag being clear (or on the tutorial
having spoken): `decompiled/src/battle_script_commands.c:1199`. So the spread is damage variance
(85-100%, `:1558`) and accuracy (`:1093`) only. An earlier guess in this journal that a critical was
what made Squirtle's battle short was wrong; it cannot have been.

**Local trims are not free.** `06-starter` held UP for 8 frames to face the ball; 1 is enough.
Trimming it saved 6 frames and cost 391 in the battle, because the shifted `gRngValue` produced a
battle needing two more attacks. Net 385 slower. That is now the reason `Tuning` exists: knobs like
this are route-level, recorded in the ledger, and swept end-to-end by `frlg route tune`, which scores
each variant on total frames to the win rather than on the segment it lives in.

Swept all eight values end-to-end afterwards: 8 (untrimmed) is the best at 11873, and every trim is
163 to 958 frames worse, with no monotonicity at all. So the route is unchanged in frames and much
better understood.

**Carry this forward.** Every future optimisation upstream of a fight has to be measured through the
fight. Anything that reports "saved N frames" without re-running the battle is not evidence.

## 2026-08-10 -- first working route: power-on to a beaten rival

**Done.** 11873 frames, Squirtle, tier-1 verified (`docs/route.md`, `route/ledger.json`). Segment
code in `crates/frlg-route`, logs in `route/logs`, checkpoints in `$FRLG_ARTIFACTS/states/route`
(which do not survive the sandbox -- the logs are the artifact).

**What the route is built on.** Three pieces, in the order they were needed:

- `Recorder` -- one mask per advanced frame, no exceptions. This is what lets a segment be written
  as "wait until X" and still be a frame-exact log.
- `Observer` -- struct offsets transcribed from the decomp with citations, then checked against the
  running game (`tests/observe.rs`). `gMain.callback2` resolved through `pokefirered.sym` turned out
  to be the single most useful probe: it names the screen the game is on, which is what most of the
  intro's segment boundaries are.
- `nav` -- path search inside the emulator. Never reads the collision map. Walked bedroom -> lab in
  895 frames on its first run, which is when it became clear the route did not need any hand-written
  movement at all.

**What went wrong, and is worth not repeating.**

- `gSaveBlock1Ptr->playerPartyCount` is *not* the live party count. It is a copy that
  `SavePlayerParty` makes (`decompiled/src/load_save.c:164`); the live one is `gPlayerPartyCount`.
  The starter segment sat there mashing A at a Squirtle it had already been given, because the probe
  was reading a field that only updates when the game saves. Cost a debugging round trip; the
  screenshot is what settled it, not the numbers.
- `player_can_step` (runningState/tileTransitionState/preventStep all clear) is never true while a
  direction is held, because the game just keeps walking. The nav edge therefore ends when
  `gSaveBlock1Ptr->pos` changes -- mid-animation, deliberately -- and chaining edges from there is
  exactly what holding the button does. The first version waited for the player to settle and priced
  every tile as a fresh standing start; it also never terminated an edge, so the search expanded one
  node and gave up.
- Mashing A through the starter dialogue says YES to the nickname prompt and buys a naming screen.
  The tail of that segment is a B mash for that one reason.

**Next, in the order that pays.**

1. The battle, 3461 frames of the 11873. Nothing about it is manipulated yet -- it is a mash, and it
   wins on whatever rolls the RNG happens to hand out. `gRngValue` is the lever; the damage and crit
   path in `decompiled/src/battle_script_commands.c` is the thing to read first.
2. Starter choice by measurement: build all three, compare frames-to-win. The rival always takes the
   counter, so this is not obvious in either direction.
3. Naming: preset name vs. the current mash. ~3300 frames sit in those two segments.
4. Text speed. The route never opens OPTIONS; whether the detour pays for itself over ~40 message
   boxes is arithmetic nobody has done here.

**Unverified.** Everything tier 2. No `.bk2` exists and none can be written until
`route/template.bk2` does. The HLE-BIOS caveat from `docs/harness.md` still stands and is untested.
*(2026-08-11: the template now exists and the BIOS caveat turned out to be load-bearing — see the
entry above.)*
