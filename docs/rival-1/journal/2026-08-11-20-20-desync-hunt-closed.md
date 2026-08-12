# 2026-08-11 (sandbox, late) -- hunted for a second desync cause and found none; hardened the thing that names the frame

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
