# 2026-08-11 -- tier 2 has a format, a runner, and one thing left that money cannot buy

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
