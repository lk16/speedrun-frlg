# Tier-1 calibration of the human-run model (libmgba)

Where a modelled cost matters to a decision, `human-run/PROMPT.md` asks for a measurement on tier 1.
`crates/frlg-calib` measures wild-encounter pieces on libmgba; this file records what it found and
which constants moved. Everything below is a **floor** (A mashed on alternate frames, no human delay);
the plans multiply by 1.2-1.25 for a human.

## Method

`frlg-calib <state> <trials>` loads `route-defeat-brock-r78/to-forest.state` (Viridian Forest, south
entrance; a TAS state with five Poké Balls and the starter at level ~6), idles `7*trial` frames so each
trial sees different RNG, walks along the tall grass until `gMain.inBattle` is set, then:

* **intro**: frames from `inBattle` to `HandleTurnActionSelectionState` (the first action menu).
* **catch**: writes the wild mon's HP to 1 (a 255-rate forest bug is then caught with certainty), taps
  RIGHT (Bag), mashes A through pocket, item and USE until `inBattle` clears; reports frames from the
  menu to the end of the battle (throw animation, shakes, Gotcha text, dex page, nickname prompt),
  then B-mashes to the first free step (4-5 frames).
* **run**: taps DOWN, RIGHT (Run), mashes A; counts each attempt that returns to the menu as a failure.

## Results (80 trials; see the output block at the end)

| Piece | Measured floor | Old model | New model (`calib_enc`) |
| --- | --- | --- | --- |
| battle start to first menu | 358 frames (358-360) | `ENC_INTRO` 220 (+ reaction 15-30) | 450 (+ reaction) |
| one ball, result back at the menu (fail) | 397-416 | `THROW` 420 | 480 |
| throw to battle over, caught | 409-771, mean 549 | `THROW` 420 + `CATCH_END` 380 x 1.25 | 480 + 150 |
| Run, first try succeeding | 400-410 | `FLEE` 150 | 500 |
| Run, failed attempt | 277-585, mean 374 | (none) | 470 each |
| Run success rate | 10 of 54 attempts at level ~6 | 100% | 20% below carry level 15, else 100% |

Findings: an encounter costs about twice what the model assumed (intro alone is 358 frames), a
successful catch is cheaper than modelled once the throw is counted, and running away at low level
mostly fails. The escape rule is `TryRunFromBattle` in `decompiled/src/battle_main.c`; the 15 and the
20% are the guesses (one sample of 22 encounters).

## Not measured (still guesses)

`TURN`, `MON_OVERHEAD`, `LEVEL_UP`, `TRAINER_OUTRO` (their ratio to the TAS ledger is `battle_scale`
1.75, `human-run/plans/022-calib.md`), mart visits (900 frames), Safari entry and exit text, nickname
declines for fishing, bag navigation for ball types other than the first, walking (decomp: 16/8/6
frames per tile, `path_overhead` 1.12 is a guess).
