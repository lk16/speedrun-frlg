# 2026-08-12 (sandbox) -- the bedroom desync was the boot: BizHawk never skips the BIOS intro for a movie

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
length. Contract updated in `docs/rival-1/route.md`.

**Unverified.** The fix's *effect*: no tier-2 result exists yet for the rebuilt movie. The
reasoning is cited but BizHawk has not replayed it. The Lua trace compare has never run (the Lua
has still never completed a report of any kind); its read API
(`memory.read_u32_le(offset, domain)`, framecount-1 indexing per `MGBAHawk.IEmulator.cs:83`) is
the least-tested part.
