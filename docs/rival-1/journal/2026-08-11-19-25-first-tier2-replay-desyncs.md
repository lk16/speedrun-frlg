# 2026-08-11 (evening, host) -- tier 2 ran for the first time, and the first watched replay desyncs in the bedroom

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
frame number yet; that diagnosis is the top tier-2 item (`docs/rival-1/route.md`).

**Watching the replay also put three route questions on the record** (now in `docs/rival-1/route.md`,
"What is not optimised"): the name is a seven-A wall typed one press at a time and re-printed at
every name mention (one-character name and preset name both unmeasured); text speed is never set
to FAST and every message box in the run pays for it; and LeafGreen builds byte-exact but has
never been raced against FireRed. All three must be measured through the battle, and the version
question through a full build-and-tune.

**Unverified.** The desync location (eyeballed, no frame number); everything downstream of it.
