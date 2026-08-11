# Session journal

Newest first. Continuity is something you write down; a sandbox ends mid-thought. Anything
unverified says so.

## 2026-08-12 (sandbox) -- 12713 -> 10946: the three cheap inefficiencies are routed out

Task: optimise the route to the rival win. The three items the 2026-08-11 tier-2 viewing put on
the list -- the seven-character names, MID text speed, battle animations -- are now all gone,
one build, measured end-to-end through the battle. **10946 frames, -1767 (-13.9%), tier-1
verified from reset, exported and queued as `route-10946f-b1a0875a77e9`.** Segment numbering
shifted: the new `04-options` pushes everything after it up by one (`09-battle-win` is the
battle now).

**What the 1767 frames are.** `03-names` types one letter, START (a documented cursor shortcut
to OK, `decompiled/src/naming_screen.c:1485`), A -- and takes KAZ off the rival's preset menu
instead of a second naming screen (rows are `sRivalNameChoices`, `oak_speech.c:647`; the menu
wraps, so it is two UPs from the top). 1450 -> 1238. `04-options` opens START -> OPTION in the
bedroom and sets text speed FAST plus battle scene OFF in one 197-frame detour. Everything
downstream got cheaper: `07-starter` -794 (its text at 1 frame/char instead of 4), `06-to-lab`
-150, `08-battle-start` -88, `05-house` -24, and the battle -- fresh stream, re-searched, 8/16
start delays win -- came in at 3322, -696.

**Two wrong assumptions the decomp corrected, worth keeping:**

- **There is no preset menu for the player.** The 2026-08-11 route notes implied preset names
  were "two D-pad presses away" for both names. The flow is asymmetric:
  `Task_OakSpeech_YourNameWhatIsIt` fades the player straight into the naming screen
  (`oak_speech.c:1352-1379`); the player's preset menu exists only on the say-NO re-ask path.
  The rival's menu is real and literal. (Near-miss worth noting: the player's name buffer is
  *prefilled* with `sMaleNameChoices[Random() % 19]` before the naming screen opens
  (`Task_OakSpeech_DoNamingScreen` -> `GetDefaultName`, `oak_speech.c:1444,2146`), so
  START+A on an untouched screen keeps a random 3-6 char preset. Rejected: a searched-delay
  3-char draw is never better than the deterministic 1-char typed name.)
- **Single-frame taps die in the start menu.** The first options attempt tapped UP twice and
  pressed A on EXIT: while the start menu is up, `gMain.newKeys` goes stale in runs of 2-3
  frames -- input reads get skipped -- and the field swallows everything for ~20 frames after
  the walk-in transition (`Task_ExitNonDoor`). The fix is structural, not a longer wait: every
  press in `04-options` is a mash-until-effect against a RAM observable
  (`sStartMenuCursorPos`, the option menu's working values, its `loadState`), which stops on
  the frame the effect lands and cannot overshoot. New observer probes for all of it, each
  checked against the running game in `tests/observe.rs`.

**Also written down while in there:** the run's RNG stream is seeded twice, both from timer 1
-- at title-screen exit and again at *player* naming-screen exit (`SeedRngAndSetTrainerId`,
`title_screen.c:735`, `naming_screen.c:722`, `main.c:264`) -- so manipulation upstream of the
naming exit cannot reach the battle except by moving the exit itself. In `docs/route.md`'s RNG
section now.

**Unverified.** Tier 2 for the new movie: queued, not replayed (the 12713 pass covers the
previous movie only -- same boot, core and format, but plausible is not proven). The
`turn_hold` sweep is still the mGBA-0.10.5 one, now two route generations stale; `frlg route
tune` on the current route has not been re-run. Whether the new battle contains a crit either
way: not checked.

## 2026-08-11 (host, night) -- TIER 2 PASSES: BizHawk replays the whole route frame for frame

**The headline, for whoever picks this up in a sandbox: `route-12713f-a4ad4280bbdc` passed.**
Luuk ran `tools/verify-runner.sh` on the host and the result is in
`$FRLG_ARTIFACTS/verify/results/route-12713f-a4ad4280bbdc.json`:

    "verdict": "pass"
    "ram_hash": "73b329af5d561a864cc4b0d46e8d4c409ce1b6df"   (== expected_ram_hash)
    "notes":   "replayed 12713 frames; fingerprint matches; probe trace matched every frame"

Read the third line twice. The final fingerprints agreeing would have been a pass; the
**`gRngValue` trace matching on all 12713 frames** means the two emulators never diverged for a
single frame anywhere in the run. The boot fix was the whole desync, the 2026-08-12 audit that
found no second cause was right, and the trace machinery that was built to name a desync frame
instead got used to prove there was no desync to name.

What this closes: the ledger's per-segment `tier2` no longer says "not replayed" (it names the
passing run and the `ilog` digest it was built from); `docs/route.md`'s header no longer says
"tier 1 only"; and `tools/verify-runner.lua`, three sessions old and never once having completed
a report, has now completed one end to end -- status file, 288K RAM dump, per-frame compare.
Rebuilding the route resets the tier-2 stamp, and should: a rebuilt movie has not been replayed.

**One inconsistency worth knowing before it wastes someone's afternoon.** Re-exporting the same
route produces the same `ilog_sha1` and a *different* `bk2_sha1` (`f60e7120…` then `4d947b73…`).
The `.bk2` is a zip and its entry timestamps move; the input log is identical, which is why the
re-exported movie replayed to the same fingerprint. The `.ilog` digest is the identity, the
`.bk2` hash is not. Noted in `docs/route.md`.

### Three inefficiencies, now written down with citations

Luuk watched the run and named three. All three are real, and one of them contradicts something
this journal has claimed since 2026-08-10.

**Battle animations are on.** `NewGameInitData` sets `optionsBattleSceneOff = FALSE`
(`decompiled/src/new_game.c:66`) and nothing in the route ever opens OPTIONS, so
`BattleStartClearSetData` never sets `HITMARKER_NO_ANIMATIONS` (`decompiled/src/battle_main.c:2259`
-- and neither of the two battle types that would block it, LINK and POKEDUDE, applies here).
Every attack in the 4018-frame battle plays its animation. The switch is `MENUITEM_BATTLESCENE`
(`decompiled/src/option_menu.c:514`), reached through START -> OPTION
(`decompiled/src/start_menu.c:531`).

**The name is seven characters, and each surplus character has a price now.** The naming screen
mash types A until the name fills up, and every later message box that prints the name pays for
all seven. The price: `sTextSpeedFrameDelays` is `{SLOW: 8, MID: 4, FAST: 1}` frames per
character (`decompiled/src/new_menu_helpers.c:27-32`), and the route runs at the default MID. Six
surplus characters is 24 frames per message box that prints the name. This also prices the text
speed item that has been on the list since 2026-08-10: FAST is a 4x on every character in the
run, and it is the *same* OPTIONS detour as the battle animations, so the two should be priced
together rather than one at a time.

**"Bulbasaur crits us" -- and this journal said that was impossible.** It was wrong, and the
decomp says so plainly. The 2026-08-10 entry (and `docs/route.md`) claimed criticals are off for
the whole first battle because of `BATTLE_TYPE_FIRST_BATTLE`. The actual condition is
`&& (!(gBattleTypeFlags & BATTLE_TYPE_FIRST_BATTLE) || BtlCtrl_OakOldMan_TestState2Flag(1))`
(`decompiled/src/battle_script_commands.c:1200`), and that second clause was never read.
`BtlCtrl_OakOldMan_TestState2Flag(1)` tests `FIRST_BATTLE_MSG_FLAG_INFLICT_DMG`
(`decompiled/src/battle_controller_oak_old_man.c:2228`, `decompiled/include/battle_controllers.h:287`),
which `CompleteOnHealthbarDone` sets the first time an opponent's hit finishes draining the health
bar, on its way to Oak's "inflicting damage is key" line
(`decompiled/src/battle_controller_opponent.c:304-306`). So criticals are suppressed for the
opening exchange only and are live for the rest of the battle, for both sides. Two things follow:
the crit `Random()` call is consumed on every damaging hit regardless, because `&&` short-circuits
left to right and the roll sits *before* the `FIRST_BATTLE` clause; and the rival's crit is a
search target, not a fact of life -- it costs damage (possibly a turn) plus a message box, in a
stream `08-battle-win` already re-searches. Nobody has measured the battle without it yet.

The lesson is the boring one: a citation that stops at the first `&&` is not a citation. This one
survived two sessions because it was *nearly* right.

### The runner: 213s -> 31s, once it stopped talking to a real X server

Replaying at 100% costs exactly what the TAS costs. `tools/verify-runner.sh` now seeds EmuHawk's
`config.ini` (plain JSON) before each launch: `Unthrottled`, no clock/vsync/sound throttle,
`DispSpeedupFeatures: 0` (its `MainForm::Render` returns immediately -- read from EmuHawk.exe IL,
not guessed), sound off, and the dialog suppressors that keep an unattended run from parking on a
modal window. `--realtime` puts the desk settings back, because watching a replay is what
produced three of the findings above and must stay one flag away.

**Determinism is checked, not assumed**: the same movie, replayed fast and silent, produced the
same fingerprint *and* the same 12713-frame probe trace as the 100%-with-sound run. That is the
pass quoted at the top of this entry -- it was replayed twice.

Unthrottling on the desktop bought only 1.6x -- 134s, ~95 fps -- and the shape of the miss was
the clue: 36s of CPU across 134s of wall clock, so the process was *waiting* ~8ms per frame
rather than working. Luuk installed `xvfb`, and `--headless` (`xvfb-run`) answered it:

    stock EmuHawk                      ~213s    59.7 fps, i.e. the movie's own length
    seeded config, on the desktop       134s    ~95 fps
    seeded config, --headless            31s    ~410 fps      <- default
    --headless, DispSpeedupFeatures 1    47s    ~270 fps
    --headless, DispSpeedupFeatures 2    47s    ~270 fps

**6.9x, and the win is the X server rather than the throttle.** Headless is 32s of CPU across
31s of wall -- the replay is finally CPU-bound, nothing waits. So the ~8ms/frame was the desktop
X connection, most likely the per-frame `UpdateWindowTitle()` that `DispSpeedupFeatures == 0`
switches on (`CalcFramerateAndUpdateDisplay`, EmuHawk.exe IL): a round trip per frame, cheap
against a local Xvfb, expensive against a real desktop. The last two rows are the same
experiment run the other way and they justify keeping `DispSpeedupFeatures: 0` -- letting
EmuHawk render costs 16s even with nothing to display it on. All four replays passed with the
identical fingerprint and trace, which is four more independent confirmations of the tier-2
result at the top of this entry.

`FRLG_VERIFY_CONFIG_EXTRA` (a JSON object applied on top of the seeded config) exists so the
next person to doubt one of these settings can measure it instead of arguing with the IL.

Note what headless is and is not: the sandbox still cannot run BizHawk (no mono, no installs),
so this does not move tier 2 into the sandbox. What it buys is `--watch --headless` draining
the queue on the host without taking over a screen.

**Unverified.** Segment-level tier-2 requests, which have never been made -- only the whole
route has ever been replayed. Everything else in this entry was measured.

## 2026-08-11 (sandbox, late) -- hunted for a second desync cause and found none; hardened the thing that names the frame

Task: find the desync and fix it. The desync on record is the tier-2 bedroom stall, root-caused
last session to the skipped BIOS intro and fixed; the rebuilt `route-12713f-a4ad4280bbdc` sits
in the queue unreplayed, and the only tier-2 result is the pre-fix runner error. So this session
did the two things the sandbox can do: audit every remaining divergence axis against the mounted
sources, and make sure the next host run cannot fail to produce a frame number again.

**The audit came back empty -- no second desync cause.** Checked, with citations (BizHawk/mGBA
claims cite the read-only mounts; this is tier-2 material, not routing):

- *Input latch order*: `bizinterface.c:518` (`$FRLG_DEPS/mgba/src.tar.gz`) does
  `core->setKeys(keys)` then runs the frame -- identical to our `frlg_run_frame` (`shim.c:182`).
  The `keyCallback` BizHawk installs (`bizinterface.c:360`) returns the same per-frame mask
  `setKeys` stored, so KEYINPUT reads see equal values on both tiers.
- *Movie latch indexing*: `MovieSession.cs:96,322` latches input-log row `Emulator.Frame`
  before each advance, and `MGBAHawk.IEmulator.cs:83` increments `Frame` after -- so row 0
  drives the first advanced frame, exactly like tier 1's `log[0]`. Playback flips to FINISHED
  at `Frame == FrameCount` (`MovieSession.cs:112`), no off-by-one.
- *Savedata*: BizHawk hands mGBA a 0xFF-memset buffer (`bizinterface.c:347`); tier 1 attaches
  no save VFile, and `GBASavedataInitFlash` (`savedata.c`) memsets the anonymous map to 0xFF in
  that case. Same erased-flash bytes either way.
- *Idle loop*: mGBA's default is `IDLE_LOOP_REMOVE` (`gba.c:120`), but removal needs a known
  address and BPRE's override row says `GBA_IDLE_LOOP_NONE` (`overrides.c:134`), so it equals
  BizHawk's forced `IDLE_LOOP_IGNORE`. Both sides also converge on FLASH1M + HW_NONE for retail
  BPRE (`bizinterface.c:450`, crc `0xDD88761C` in the known-Pokémon table, so no romhack
  compat), confirming last session's shorter check.
- *`.bk2` decode*: `Bk2Controller.SetFromMnemonic` parses rows strictly in `LogKey` order, so
  the template-copied key plus our column table is the whole story.

**The queue entry re-verifies from scratch.** `frlg log cat` of the eight committed logs
reproduces digest `a4ad4280…`; a cold tier-1 replay reproduces the request's
`ram_hash 73b329af…` and matches the queued 12713-frame `gRngValue` trace byte-for-byte; an
independent Python decoder (game key bits, not the exporter's code) decodes the queued `.bk2`
to exactly those masks. Whatever tier 2 says, it will be about the emulators, not the artifact.

**The Lua's assumptions are no longer guesses.** BizHawk ships typed Lua API docs
(`$BIZHAWK_HOME/Lua/_docs_luacats/`) that the desync hunt had never opened:
`memory.read_u32_le(addr, domain)` and `memory.readbyterange(addr, length, domain)` are the
real signatures, `readbyterange` returns a zero-indexed table (its own doc string, extracted
from `BizHawk.Client.Common.dll`), `movie.mode()` returns exactly
`"PLAY"|"RECORD"|"FINISHED"|"INACTIVE"`, the mGBA domains are named `EWRAM`/`IWRAM`
(`BizHawk.Emulation.Cores.dll`), and `event.onframeend`/`event.onexit`/`client.exit` all
exist. Every assumption the script makes checked out as written.

**The fix this session: the runner can no longer lose the frame number.** The watched
2026-08-11 replay ran, desynced, was closed by hand -- and recorded nothing, because the Lua
wrote its status only at a finish it never reached (`EmuHawkMono_last*.txt` in the deps tree
shows it: Lua loaded, no report). Now `verify-runner.lua` writes the status file the moment
the probe first mismatches, every 300 frames as a heartbeat, and from `event.onexit`; the
shell's timeout branch reads the partial status into the result instead of discarding it. The
new Lua is installed at `$FRLG_ARTIFACTS/verify/verify-runner.lua`, the override the runner
prefers, so the next host run uses it even though the host checkout cannot see this commit.
The shell half only lands when the host pulls.

**Unverified.** Still the same one thing: no tier-2 result for `route-12713f-a4ad4280bbdc`.
The audit narrows the space -- if it still desyncs, the trace frame number is the lead and
there is no named suspect left -- but narrowing is not a pass.

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
