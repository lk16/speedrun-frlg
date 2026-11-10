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

## Results (80 trials)

| Piece | Measured floor | Old model | New model (`calib_enc`) |
| --- | --- | --- | --- |
| battle start to first menu | 358 frames (358-360) | `ENC_INTRO` 220 (+ reaction 15-30) | 450 (+ reaction) |
| one ball, result back at the menu (fail) | 397-416 | `THROW` 420 | 480 |
| throw to battle over, caught | 407-1027, mean 564 (n=39) | `THROW` 420 + `CATCH_END` 380 x 1.25 | 480 + 150 |
| Run, a successful attempt | 400-1025, mean 681 incl. earlier failures (n=34) | `FLEE` 150 | 500 |
| Run, failed attempt | 277-596, mean 381 (n=148) | (none) | 470 each |
| Run success rate | 34 of 182 attempts (18.7%) at level ~6 | 100% | 20% below carry level 15, else 100% |

Findings: an encounter costs about twice what the model assumed (intro alone is 358 frames), a
successful catch is cheaper than modelled once the throw is counted, and running away at low level
mostly fails. The escape rule is `TryRunFromBattle` in `decompiled/src/battle_main.c`; the 15 and the
20% are the guesses (one TAS state, 80 encounters).

## Fights (same harness, `--trainer` for the rival)

| Piece | Measured floor | Old model | `calib_battle` |
| --- | --- | --- | --- |
| a wild turn that does not kill (menu to menu) | 282-585, mean 375 (n=6) | `TURN` 330 x `battle_scale` 1.75 = 578 | 385 |
| the killing turn through faint and exp text (menu to battle over) | 889-1177, mean 975 (n=6) | 330 + 450, x 1.75 = 1365 | 385 + 810 = 1195 |
| lab rival (`07-starter` ... `08-battle-start` state): start to first menu | 1040 | `intro` 354 x 1.25 = 442 | max(intro x 1.25, 1250) |
| lab rival: turns with Tackle mashed | 406, 280, 280, 282, (last: 1024 to battle over) | | |
| lab rival: whole fight, battle start to free movement | 3317 (TAS ledger: 354 + 2463 = 2817) | | |

The old `battle_scale` 1.75 was fitted to the TAS ledger (rival-lab 47 s, Brock 83 s) and gives the
modelled human 49 s and 98 s. With `calib_battle` the same two segments come out at 57 s and 94 s, i.e.
1.2x and 1.13x the TAS, which is what the human factor says. The two models agree on short fights
and `calib_battle` is cheaper on long ones (E4, leaders): the old per-turn cost of 578 was about
1.5x the floor. 1.2 is a guess for the human factor.

## Not measured (still guesses)

`TURN`, `MON_OVERHEAD`, `LEVEL_UP`, `TRAINER_OUTRO` (their ratio to the TAS ledger is `battle_scale`
1.75, `human-run/plans/022-calib.md`), mart visits (900 frames), Safari entry and exit text, nickname
declines for fishing, bag navigation for ball types other than the first, walking (decomp: 16/8/6
frames per tile, `path_overhead` 1.12 is a guess).

## Raw output (`frlg-calib <state> 80`)

```
intro        n= 80 mean   358.7 min 356 max 360
throw+catch  n= 39 mean   563.5 min 407 max 1027
catch-end    n= 39 mean     4.6 min 4 max 5
flee-ok      n= 34 mean   681.4 min 400 max 1025
flee-fail    n=148 mean   380.9 min 277 max 596
```
