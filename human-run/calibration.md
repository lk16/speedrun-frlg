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

## WR fight fit (`calib_wr`, 2026-11-13)

External evidence, not a decomp citation: 16 fight durations read off the WR video
(`human-run/video-analysis/data/calib/v1..v4`, intro to the "defeated" box, 4-5 s sampling, +-5 s).
Duration = a + m * (enemy mons) + t * (player actions): least squares over the 16 fights gives
`m` = 10.5-12.3 s and `t` = 5.7-5.9 s for any assumed intro `a` of 0-14 s (the fit is insensitive to `a`;
residual rms ~75-80 s over 16 fights).

| Constant | `calib_battle` (human 1.2x floor) | `calib_wr` | source |
| --- | --- | --- | --- |
| player action | 385 frames | 345 | fit `t` = 5.74 s |
| enemy mon (send-out, faint, exp text) | 810 | 630 | fit `m` = 10.5 s |
| trainer intro | max(intro x 1.25, 1250) | intro x 0.8 | E4 swirl to first menu 12-16 s |
| trainer outro | 300 | 240 | guess (not in the video durations) |
| level-up | 300 | 240 | Brock: victory +28 s, "grew to" +32 s |
| wild encounter intro | 450 | 380 | libmgba floor 358 x 1.06 |
| ball throw | 480 | 425 | libmgba floor 397-416 x 1.05 |
| catch end | 150 | 130 | libmgba floor |
| Run, ok / fail | 500 / 470 | 430 / 400 | libmgba floors 400-1025 / 277-596 |

The WR-level human factor in these rows is ~1.05 over the libmgba floor; the plan's `text_factor` and
`path_overhead` stay at their own 1.05 / 1.04 for the ledger text and walking. Model fight *lengths in
actions* are a separate matter: `frlg-human splits` prints model vs WR actions and seconds per fight.

## Early fights against the WR video, 2026-11-14 (`frlg-human splits`, plan 377)

The video durations (`video-analysis/data/calib/v1-early-fights.md`) are intro to the EXP/prize text; `splits` now prints the
model's *core* seconds (intro to the last faint) next to them, and the full segment (outro and level-ups) separately. Brock: video
31 s with a ~4 s intro, model core 43 s (intro 8 s: `trainer_intro` x0.8 of 600; the rest within 3 s per mon); Misty: video 54 s,
model 60 s with 5.1 actions (video 6-8). Torrent figures: Brock at 10/32 HP, Lv11, 97% win, ~2 actions per foe (unit test); at full
HP the same fight takes 3.9 actions. Field poison: 1 HP per 5 steps (`field_control_avatar.c:710-733`), the 70 frames per HP is a guess.
Not measured on tier 1: the Torrent-window potion trips (priced as a battle turn each) and the Antidote trip (400 frames).

## Played segments after Giovanni 1, seed 11, LeafGreen, tier 1 (2026-11-22, controller)

Measured on libmgba by the playing controller (`frlg-human play`, `human-run/journal/2026-11-22-12-30-controller.md`),
first-survivor branches of seed 11, human input model of the plan (jitter U{0..4}, reaction U{15..30}). One sample
each; frames between consecutive checkpoints. Model column: mean seconds from `frlg-human report` (WR+160m, R 2000).

| played stage | frames | s | model segments | model s | note |
| --- | --- | --- | --- | --- | --- |
| hideout-out | 6878 | 115 | scope-and-dig | 32 | elevator to B1F; two B1F Rockets fought on the way (68 s) |
| hm02-trip | 2276 | 38 | hm02-trip | 24 | on foot (Bicycle not yet registered), one Cut tree |
| hm02 | 1334 | 22 | hm02-fly | 14 | talk + teach HM02 through the TM case |
| to-vermilion | 6486 | 109 | (fly-vermilion, surge-walk) | 12 + 18 | walk Celadon -> Saffron -> Vermilion: Fly needs the Thunder Badge |
| surge-cans | 11881 | 199 | -- | 0 | 53 trash cans, two gym trainers met on the way |
| surge | 9540 | 160 | surge-gym-trainers, surge, surge-badge | 25 + 54 + 10 | Tucker + Surge (7 turns, one potion) |
| fly-lavender | 1167 | 20 | fly-lavender | 12 | incl. walking out of the gym |
| lavender-heal | 1295 | 22 | -- | 0 | center heal + options (battle style SET); not in the model |
| tower-rival | 7080 | 119 | tower-walk, rival-tower | 24 + 102 | rival 5 mons |

Canonical pass of the same stretch (one pass, verified from boot, `run.ilog` sha1 bd99f705...), seconds per stage:
hideout-out 110, hm02-trip 25, hm02 22, to-vermilion 70, surge-cans 117 (13 cans), surge 182, fly-lavender 20,
lavender-heal 19, tower-rival 121, tower-marowak 362, fuji-flute 178, fly-celadon 15, snorlax 32, cycling-road 69,
safari-trip 25, safari-walk 293 (3 catches), warden 24, koga 322 (with a center heal). Splits: Surge 6058 s,
Snorlax 6874 s, Koga 7538 s (model p50 4364 / 4470 / 5103).

## Fight win rates on the plan's policy, emulator vs engine (2026-11-22, stand-in audit)

`frlg-human battle-test --plan 1202-controller-model --seg S` from states cut out of the canonical s11 `run.ilog`
(~150 frames before `gMain.inBattle`), seeds 1-20 (Misty 21-40, Cerulean 1-22). "Engine" = `frlg-human battle` at the
s11 carry's exact stats and bag. Journal `2026-11-22-14-30-standin-audit.md`.

| fight | s11 carry | emulator won | engine win (power 1.0) |
| --- | --- | --- | --- |
| Misty | Wartortle L20, 5 Potions | 19/20 (+14/14 bridge) | 88%, 94% with a held Persim |
| Rival Cerulean | Wartortle L21, 5 Potions, no Awakening | 15/22 | 60% (69% with Awakenings) |
| Hideout Giovanni | Blastoise L36, 90/115 HP, 12 Super Potions | 20/20 | 100% |
| Rival tower | Blastoise L38, 11 Super Potions | 20/20 | 98.6% |
| Koga | Blastoise L42, 128/133 HP, 11 Super Potions, 3 X Specials | 15/20 | **35%** (engine too deadly at Muk) |
