# 2026-08-12 (sandbox, night, addendum) -- "delay the first press, start later in the stream": tested, and it does not work that way

Question asked: the RNG moves before the first button press, so could the first press be
delayed to start later in the stream for free? Three-part answer, all measured
(`seed-probe` example):

- **The stream does move before any input** -- the VBlank `Random()` runs from boot
  (`decompiled/src/main.c:412`; 274 zeroed BIOS frames, then one step per frame from
  state 0) -- but both `SeedRng` calls overwrite the state, so nothing of the pre-press
  stream survives to the battle.
- **A delayed press re-rolls the seed; it does not slide the stream.** The seed is timer
  1 read at the title and naming screens' exit presses, and the timer starts *inside
  those screens* (`title_screen.c:351`, `naming_screen.c:428`, seed read at `:735`/`:722`
  via `main.c:264-269`). Measured stride 18753 ticks per frame: prepending 1 and 2 idle
  frames to the committed movie moved the naming seed `0xdf93 -> 0x4d7b -> 0x4cd2`, and
  the committed battle inputs lose on both. Unrelated seeds, not shifted ones.
- **And the delay is not free.** The run is scored in frames from power-on (README,
  `docs/rival-1/route.md`, the ledger's `total_frames`; the tier-2 `.bk2` contains every frame
  from power-on). "TAS time starts at the first press" is not this project's rule, and --
  labelled as uncitable pretraining knowledge -- not how published TASes are timed either.

The constructive residue is a new dial in `docs/rival-1/route.md`'s "What is not optimised":
idling N frames before the *naming-screen exit* press samples N fresh battle seeds at 1
frame each, which the sweeps only ever hit incidentally. A sampled seed wins if its
searched battle beats 2409 by more than N.

**And the dial was then sampled, N = 1..24 (`seed-sample` example): no winner.** Each
variant replays the committed logs with N idles inserted at the start of `03-names` (all
24 replay clean to the battle -- the downstream logs survive every stream) and runs the
full two-stage battle search on the real stream, anchored at N=0 reproducing the
committed 2409/`[4, 3, 3, 3]` bit-for-bit. Results: best is N=10, battle 2407 -- the
first battle seen under 2409 -- but total 9666 (+8) against its bar of 2399; N=14 lands
+20; everything else is worse, and the seeds at N=11, 18, 20, 23 cannot win at all
(13/64 to 64/64 stage-1 win rates across the wave says how wild the per-seed variance
is). The committed route stands. Along the way the search got 1.8x faster (turn-menu
checkpoint savestates + abort-at-current-best, adopted after the checkpointed anchor
reproduced the committed battle exactly), and `battle-truth` now extracts the committed
battle as a roll-by-roll dataset -- our Bulbasaur outspeeds the rival's Charmander (11
vs 9, we act first), each turn shows a 6-roll AI block, our Tackle burns 2 rolls, the
rival's move 4 -- which is the validation target for a pure-Rust battle model, the next
distillation step.
