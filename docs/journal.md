# Session journal

Newest first. Continuity is something you write down; a sandbox ends mid-thought. Anything
unverified says so.

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
