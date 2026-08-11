# Session journal

Newest first. Continuity is something you write down; a sandbox ends mid-thought. Anything
unverified says so.

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
