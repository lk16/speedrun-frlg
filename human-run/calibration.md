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

## Played segments after Koga, seed 11, LeafGreen, tier 1 (2026-11-22, controller 2)

`human-run/journal/2026-11-22-14-00-controller-2.md`. Seconds between checkpoints. Koga -> Sevii from the
canonical-code pass (`runs/human-play/fwd-canon`, dex hooks active); Sevii -> Indigo from the development
run (`runs/human-play/fwd-main`; same code paths, earlier stages without the dex hooks). One sample each.

| played stage | s | model segments | note |
| --- | --- | --- | --- |
| teach-surf | 23 | warden-strength (23 for HM teaches) | HM03 through the TM case + forget screen |
| power-plant | 94 | fly-lavender-2 12 + powerplant-walk | Lavender fly, bike, surf Route 10 |
| power-plant-out | 486 | powerplant-out 7 + hunt-power_plant | Thunder Stone + 200-step hunt with no balls in the bag |
| to-cinnabar | 47 | fly-pallet + surf-cinnabar + surf-route21-* | two swimmers fought |
| mansion | 358 | mansion + secret-key 15 | 4-6 statue presses, Moon Stone, TM14, Elixir, out on foot |
| blaine | 221 | blaine-walk, blaine-quiz-text 45, blaine, blaine-badge | Blizzard taught, center heal (not in model), 6 quizzes, Blaine 4 turns |
| sevii-boat .. sevii | 737 | leg_sevii | 94 + 47 + 214 (bikers) + 317 (Bond Bridge hunt, Berry Forest) + 65 |
| celadon-shopping | 143 | fly-celadon-2 12 + xbuy 30 | 2F balls, 4F stones, 5F X items, Saffron Hyper Potions |
| silph (3 stages) | 423 | silph-walk .. master-ball | Saffron heal (not in model), 3 card-key doors, Giovanni 2, Master Ball, Lapras |
| sabrina | 454 | sabrina-walk, sabrina, badge, heal, Hitmonlee | teleport pads with bumps |
| erika | 377 | fly-celadon-3, erika-walk, gym, erika, badge, heal, Jolteon | |
| giovanni | 521 | fly-viridian, Diglett hunt, gym, giovanni, badge, heal | PC (Doduo out, Lapras in) + HM04: ~45 s not in the model; one gym re-entry |
| route22 | 228 | route22, rival-route22, hunt-route22 | |
| victory-road | 707 | route23 .. victory-road .. indigo-heal | Route 23 hunt ~160 s; boulders 19 + 6 + 33 + 1 + 19 pushes |
| indigo-mart | 12 | (league_heals top-up) | 12 Max Potions |

Splits (run timer vs model p50): Blaine 8767 s (canonical) / 8611 (dev) vs 5890; Sevii 9503 / 9457 vs 6805;
Sabrina 10191 vs 7678; Erika 10783 vs 7913; Giovanni 11342 vs 8198; Victory Road (indigo-heal) 12308 vs 9175.
The gap to the model p50 is ~2.4-3.1 ks from Koga on and holds roughly flat: Blaine -> Sevii (737 s played vs
915 s model) and Sevii -> Sabrina were faster than modelled, Sabrina -> Erika (592 vs 235) and the Victory Road
leg (966 vs ~980 incl. Route 22) as modelled or slower.

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

## Segment-cost calibration from the played run (2026-11-22, journal `2026-11-22-14-00-calibration.md`)

Every played stage of seed 11 (`runs/human-play/1122-lgdescent-s11` to Koga, `runs/human-play/fwd-main` from Koga
to the Indigo mart) was replayed from its checkpoint savestate with `cargo run --release --example cal_trace`
(RAM hash at every stage end matches `checkpoints.jsonl`, 81 stages) and split frame by frame by
`human-run/tools/cal_decompose.py`: **walk** (tileTransitionState set), **bump** (runningState 2 on a settled tile),
**turn**, **idle** (free, standing), and held episodes labelled **trainer** / **wild** (the whole episode from
losing control to regaining it, so sighting text and the return to the field count), **menu**, **warp**,
**text**. "Overhead" below is controller inefficiency only: bump + turn + idle, transport loss (frames above the
fastest legal transport per map: bike where `allow_cycling` and not indoor, run where `allow_running`,
`decompiled/src/bike.c:245-259`), and trainers fought that `human-run/tools/forced.py` shows avoidable (audited
from Giovanni 1 to Koga only). Model = mean seconds of `1202-controller-model` (`report` / `splits`, R 2000;
1132 is within a second on these segments). WR = Main97 split segments; per-stage WR figures come from the 60 s
scene samples (`video-analysis/data/wr-scene-*.jsonl`, +-60 s).

### Per stage, Giovanni 1 -> Koga (seconds)

| stage | played | overhead | played - overhead | model | WR | verdict |
| --- | --- | --- | --- | --- | --- | --- |
| hideout-out | 110 | 2 | 108 | 32 (scope-and-dig) | | model right: 81 s are two B1F grunts the prefix skipped on the way in (7 grunts played vs 8 modelled) |
| hm02-trip | 25 | 4 | 20 | 24 | | model right |
| hm02 | 22 | 0 | 22 | 14 | | model too optimistic: teaching HM02 through the TM case is 870 f of menus (guess 800 for all) |
| to-vermilion | 70 | 13 | 57 | 27 (walk-vermilion) | | model too optimistic: 774 f of gates/doors and 4 Cuts are not in walk segments; bike not registered (9 s) |
| surge-cans | 117 | 97 | 20 | 77 | | model right (13 cans, ~122 f/check vs 170 modelled; one sample of a random count); Baily + Dwayne avoidable (90 s) |
| surge | 182 | 1 | 181 | 89 | ~60 | per-turn costs right; 11-turn Surge (paralysis), RNG not a constant |
| fly-lavender | 20 | 0 | 20 | 37 (walk-lavender) | | different leg (1202 walks; played flew after Surge); Fly 848 f vs 700 guess |
| lavender-heal | 19 | 0 | 18 | 0 | | model bug (no heal before the tower, NEXT item 2) |
| tower-rival | 121 | 1 | 119 | 127 | | model right |
| tower-marowak | 362 | 176 | 186 | 182 | tower ~400 | model right; 5 avoidable channelers (172 s: Patricia, Laurel, Jody, Paula, Angelica; forced.py: 3F/4F 0, 5F 1, 6F Emilia+Jennifer) |
| fuji-flute | 178 | 2 | 177 | 202 | | model right |
| fly-celadon | 15 | 0 | 15 | 12 | | model slightly optimistic (Fly 809 f vs 700) |
| snorlax | 32 | 1 | 31 | 27 | | model right |
| cycling-road | 69 | 42 | 27 | 43 | | model pessimistic (downhill 5.1 f/tile); Biker Lao avoidable per forced.py (40 s; slope not modelled) |
| safari-trip | 25 | 1 | 25 | 23 | | model right |
| safari-walk | 293 | 82 | 211 | 379 | ~360 (7 catches) | not comparable: this pass threw at whatever came (3 catches); see the hunt measurement below |
| warden | 24 | 3 | 21 | 26 | | model right |
| koga | 322 | 75 | 248 | 201 | ~250 | model too optimistic by the Fuchsia heal (moved before the gym) and 2 extra Koga turns; Edgar avoidable (70 s) |
| **total** | **2006** | **501** | **1505** | **1522** | **1606** (split delta) | |

Giovanni 1 -> Koga, the model of record is right in sum once the controller's overhead is removed (1505 vs 1522 s);
the residual errors cancel (Surge long, Safari not comparable, Koga heal).

### Per WR window, New Game -> Victory Road (seconds)

| window | played | overhead | played - overhead | model (1202) | WR | trainer s played / model fight s | verdict |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Brock | 1076 | 156 | 920 | 884 | 880 | 383 / 213 | model right-ish; overhead incl. pre-shoes walking |
| Misty | 1227 | 45 | 1182 | 731 | 801 | 846 / 429 | bot: 14 trainer battles, 57 turns (older stand-in driver) |
| Bill | 817 | 27 | 790 | 604 | 609 | 699 / 526 | bot: battles (14, 39 turns) |
| Bike | 620 | 104 | 516 | 338 | 484 | 279 / 127 | model too optimistic vs WR too (+146 s) |
| Rock Tunnel | 941 | 26 | 915 | 548 | 565 | 739 / 500 | bot: 15 trainer battles |
| Giovanni 1 | 851 | 59 | 792 | 581 | 557 | 544 / 352 | bot: battles |
| Snorlax | 1342 | 340 | 1002 | 952 | 891 | 898 / 437 | model right after overhead (see stage table) |
| Koga | 664 | 160 | 504 | 622 | 715 | 270 / 180 | model right (Safari not comparable) |
| Blaine | 1072 | 26 | 1046 | 793 | 672 | 358 / 84 | bot: Mansion/Cinnabar trainers + 32 wild flees (not audited) |
| Sevii | 846 | 40 | 806 | 903 | 913 | 294 / 242 | model right |
| Sabrina | 1019 | 36 | 983 | 876 | 827 | 626 / 336 | battles (Silph) |
| Erika | 377 | 6 | 371 | 239 | 176 | 266 / ? | bot: gym trainers the WR walks past |
| Giovanni 3 | 521 | 22 | 499 | 325 | 543 | 248 / ? | model too optimistic vs WR (+218 s) |
| Victory Road (to E4) | 934 | 65 | 869 | 1017 | 732 | 313 / ? | model pessimistic vs WR; VR wild 18152 f (304 s) |

Per-battle cost is **right**: 52 trainer battles after Giovanni 1 (bridge driver, parties and turns logged) sum to
0.95x (22 to Koga) and 1.02x (30 after Koga) the `calib_wr` formula (`0.8 x intro + 240 + 345 x turns + 630 x mons`)
evaluated at the played turns and party sizes (`tools/cal_battles.py`). Where the played windows are slow in
battle, it is more battles and more turns, not slower turns.

### Constant by constant

| piece | played (tier 1) | model | verdict |
| --- | --- | --- | --- |
| walk / run / bike per tile | 16.1 / 7.9 / 6.0 (5.1 on Cycling Road) | 16 / 8 / 6 | right (`event_object_movement.c` speeds) |
| indoor walks (gyms, tower) | 15.1-16.1 f/tile: no bike, no running (`map.json`) | bike 6 for every walk after the bike | **too optimistic** -> `indoor_speed` |
| trainer battle | 0.95-1.02 x formula (52 fights) | calib_wr | right |
| wild Run, carry >= L14 | 635-657 f field to field (45 Runs) | 380 + react + 430 = ~832 | **too pessimistic** -> `flee_frames` 240 |
| Safari RUN | ~505 f field to field (11) | 402 + 200 | too pessimistic -> `safari_run_frames` 100 |
| ball, miss | Safari 509-690 (8, mean 576); Great Ball from the bag 687-883 (5) | 425 + react | **too optimistic** -> `throw_frames` 555 (still low for bag balls) |
| catch of a new species, last ball to battle end | 1028-1174 f (8) | 425 + 130 | **too optimistic** (dex page, `battle_scripts_2.s:73-91`) -> `catch_end_new` |
| Fly, menu to control | 809, 848 | 700 | too optimistic -> `fly_frames` 830 |
| doors / stairs | 115-160 / 59 f each, ~90 s Giovanni 1 -> Koga | not in walk segments (path_overhead 1.02) | too optimistic; not changed (needs warp counts per segment) |
| trash cans | ~122 f/check incl. walking (13) | 170 | slightly pessimistic, one sample; not changed |
| hunt rolls | an encounter roll at every T_TILE_CENTER, turns included (`field_control_avatar.c:138-141`): pacing = 2 rolls per tile at 8 f each; tap-turning in place = 1 roll per 10 f, no step (probe `cal_encprobe ... spin`) | 1 roll per step at 8.2 f | per-roll cost right; the model's hunt `steps` are rolls (the controller paced 90 tiles = 180 rolls for a 90 budget) |

### MODEL CHANGEs (scores.jsonl; paired, WR+2m, seed 7401, R 800k)

| key | value | vs 1202 | vs 1132 (CURRENT) | no-reset mean vs 1132's 3:28:22 |
| --- | --- | --- | --- | --- |
| `catch_end_new` | 720 | +747 h [643, 880] | +814 h [690, 961] (1240) | 3:33:38 |
| `throw_frames` + `catch_end_new` | 555 + 590 | +1183 h [1015, 1426] | +1269 h [1038, 1653] (1241) | 3:34:42 |
| `flee_frames` + `safari_run_frames` | 240 + 100 | -34.7 h [-40.6, -29.5] | -32.6 h [-40.0, -26.8] (1242) | 3:27:02 |
| `indoor_speed` | on | +121 h [107, 137] | +128 h [112, 147] (1243) | 3:30:34 |
| `fly_frames` | 830 | +9.9 h [7.8, 12.4] | +11.1 h [8.3, 15.0] (1244) | 3:28:39 |
| all (plans 1235 / **1245-calibrated-model**) | | 2890 h (+2728 [2216, 3598]) | **3635 h (+3461 [2687, 5005]), 50 finishes** | **3:35:45** (p10 3:29:47) |

The calibrated model finishes 7 minutes slower than the old one and the WR (3:25:25) is now about its p3. The reset
table was fitted under the old model (at WR+5m it resets every attempt): refit it before any descent from 1245.

### Reconciliation with the WR windows (coordinator follow-up, 2026-11-22)

Concern: the 1202/1132 model already matched several WR windows (Brock, Bill, Rock Tunnel, Giovanni 1, Sevii)
with constants fitted to the WR, so adding measured costs could double-count. Model window means (`frlg-human
splits`, R 8000, seconds; error vs the WR window in brackets):

| window | WR | 1132 | 1245 | 1246 (guess text 0.71) | 1247 (0.42) |
| --- | --- | --- | --- | --- | --- |
| Brock | 880 | 884 (+4) | 910 (+30) | 910 (+30) | 910 (+30) |
| Misty | 801 | 733 (-68) | 725 (-76) | 715 (-86) | 706 (-95) |
| Bill | 609 | 608 (-1) | 608 (-1) | 597 (-12) | 585 (-24) |
| Bike | 484 | 341 (-143) | 342 (-142) | 324 (-160) | 306 (-178) |
| Rock Tunnel | 565 | 549 (-16) | 550 (-15) | 550 (-15) | 550 (-15) |
| Giovanni 1 | 557 | 584 (+27) | 608 (+51) | 593 (+36) | 579 (+22) |
| Snorlax | 891 | 958 (+67) | 1011 (+120) | 987 (+96) | 963 (+72) |
| Koga | 715 | 608 (-107) | 691 (-24) | 676 (-39) | 661 (-54) |
| Blaine | 672 | 787 (+115) | 848 (+176) | 818 (+146) | 788 (+116) |
| Sevii | 913 | 908 (-5) | 957 (+44) | 886 (-27) | 816 (-97) |
| Sabrina | 827 | 879 (+52) | 919 (+92) | 904 (+77) | 889 (+62) |
| Erika | 176 | 237 (+61) | 246 (+70) | 243 (+67) | 240 (+64) |
| Giovanni 3 | 543 | 328 (-215) | 345 (-198) | 341 (-202) | 338 (-205) |
| Start E4R1 | 732 | 907 (+175) | 1024 (+292) | 1024 (+292) | 1019 (+287) |
| League + post-game (13 windows) | 2960 | +180 | +192 | +192 | ~+190 |
| **total** | **12325** | **+178** | **+669** | **+451** | **~+190** |

Which fitted constant could have absorbed the measured costs:

- `trainer_fit` 0.81 (intro and per-mon overhead of non-League fights, fitted to the WR video's fight durations):
  0.7 / 0.6 / 0.5 take Bill to -39 / -77 / -116 s and Rock Tunnel to -54 / -86 / -123 s, the two fight-only windows
  that match at 0.81. The fight fit does not contain catches or indoor walking, and the played trainer battles
  match the formula at 0.95-1.02 anyway. Not the absorber.
- `text_factor` / `path_overhead` / `mistake_rate` (1.02 / 1.02 / 0.3, the "WR-level skill" set against the WR
  splits in plan 373): closing the gap with either factor needs `path_overhead` ~0.75 or `text_factor` ~0.9, below
  the measured walking speeds (16.1 / 7.9 / 6.0 f/tile) and the TAS-ledger text floors; `mistake_rate` 0 gives
  -141 s only.
- The guessed text lumps (`Src::Guess`, 48% of the model's time): the only free absorber. New FIT key
  `guess_text_scale` (pre-League lumps only, up to `indigo-heal`; League and post-game keep their own window fits).
  0.71 restores the five windows 1132 matched (sum +12 s vs 1132's +9 s): plan 1246. 0.42 restores the pre-League
  total: plan 1247. Played evidence on five lumps (scope-and-dig 1800 vs ~1740 played, warden-strength 1400 vs
  ~800, safari-fee-and-hms 2400 vs ~1900, snorlax 1500 vs ~1400, hm02-fly 800 vs 1334) gives ~0.91, so 0.71 is a
  stretch and 0.42 is not credible.

The added time sits in catch-heavy segments (forest +27 s, Safari hunts +65 s, Power Plant +25, Mansion +27,
Bond Bridge +28, dex fill +45, Route 23 +17, Victory Road +25) and indoor walks (hideout +24, tower +32, Silph +16,
Sabrina +19, gyms +8..11). Those windows are mostly the ones 1132 already overshot (Snorlax, Blaine, Sabrina,
Erika, Start E4R1); 1132 matched the WR total only because Giovanni 3 (-215), Bike (-143), Koga (-107) and Misty
(-68) undershoot by as much. Those are route and dex-order differences against the WR (the model's fill is
back-loaded into Start E4R1: dex 47 -> 61 there vs the WR's 52 -> 60), not constants.

Scores (paired vs 1132, WR+2m, seed 7401, R 800k): 1246 +323 h [282, 378] (377 finishes vs 1090), 1247 -2.1 h
[-12.9, 7.6]. **Per-window WR check** (each window no further from the WR than 1132's error + 30 s, and the window
total within 1132's): 1245 fails (Snorlax, Blaine, Sevii, Sabrina, Start E4R1), 1246 fails (Blaine,
Start E4R1, total), 1247 fails (Sevii, Bike). CURRENT stays 1132.
| Koga | Blastoise L42, 128/133 HP, 11 Super Potions, 3 X Specials | 15/20; **73/100** (seeds 1-100, 2026-11-22 exp-and-koga) | 35% was a harness mismatch (`battle` CLI at `--setup-min 1 --heal-below 0.4`, so the X Specials went on Koffing turn 1); **60.5%** at the plan's `setup_min_turns` 2 / `heal_below` .6 (n 10 000) |

Koga detail (journal `2026-11-22-16-50-exp-and-koga.md`; the controller now logs `foe_last_move`, `my_stages`, `foe_stages`
per turn): per Water Pulse on Muk the hit rate by Muk's evasion stage is the same in both (engine .495 at mean +3.03, emulator
.533 at +3.10, `Cmd_accuracycheck` matches), damage rolls and Sludge damage match, Muk's Minimize / Acid Armor shares match at
full HP. Remaining gap (~12 points): the engine's Muk picks Toxic more often (5.6% of its turns vs 2.9%), so the carry is
poisoned more (31% of Muk turns vs 16%) and more often dies at the second Koffing / Weezing after Muk (12.7% vs 5%). Not
located in the AI port (`AI_CV_Toxic`, `AI_CBM_Poison` checked); open.

## Post-game, played (controller S8, `play/legs3.rs`, 2026-11-22)

Development legs on LeafGreen seed 11, started from the `fwd-canon2` first Hall of Fame; the dex was **poked to 60
in RAM (dev only)** to get past Oak, so the legs after the National Dex are measured segments but not a run. Model
column = difference of the model's no-reset p50 clocks between the named segments (`model-vs-actual` events).

| stretch | played | model | note |
| --- | --- | --- | --- |
| HoF 1 -> continue (soft reset after the HoF save) | 15 s | 12 s | `hall-of-fame-1` -> `fly-pallet-2` |
| continue -> National Dex (Oak's scenes only) | 58 s | 50 s | needs >= 60 caught (`PalletTown/scripts.inc:51-57`) |
| National Dex -> Celio asks for the Ruby | 53 s | 82 s | fly Vermilion + Seagallop (Fly cannot cross regions) |
| Kindle Road + 2 grunts + Ruby Path in | 343 s | 169 s | Strength boulders on B2F/B3F (15 pushes) |
| Ruby Path out + fly + Rainbow Pass | 134 s | 80 s | boulders again on the way up (8 pushes), no Escape Rope |
| Icefall Cave (Seagallop, ropes, HM07, Lorelei) | 335 s | 156 s | HM07 + waterfall + 2 Escape Ropes not in the model |
| Six Island, Dotted Hole, Sapphire scene | 227 s | 0 s | not in the model: the warehouse password |
| Five Island, Meadow, warehouse to Gideon | 660 s | 600 s | 9 fights (model 5) |
| Sapphire to Celio | 71 s | 98 s | |
| Vermilion, Cerulean Cave, Mewtwo, 3 TMs, lead swap | 368 s | 181 s | 52 planner bumps in Cerulean Cave 1F |
| fly Indigo, heal, Full Restores | 40 s | 55 s | |

Dex at the first Hall of Fame: 16 caught (fwd-canon2), 18 (v2) against the plan's 60; the post-HoF fill measured
~5-6 s per frame-budget step, Seafoam B3F 4-5 new species in ~20 000 frames.

## Post-game, canonical run of record (`runs/human-play/pg-canon5`, 2026-11-22, journal `2026-11-22-23-00-finish.md`)

One continuous pure-input log from boot (no RAM pokes; resumed from `pg-canon4` `dex-fill-24`, the post-game legs from
its own `celio-rainbow` checkpoint after the last controller changes). Seconds between checkpoints (frames / 59.7275);
model = difference of the model's p50 clocks where a `model-vs-actual` names one.

| stretch | played | model | note |
| --- | --- | --- | --- |
| dex finish: stones x3, Rare Candy, Magikarp (Route 4 center, 2 PC sessions) | 244 s | - | `dexfin.rs`, not in the model |
| Farfetch'd trade (Vermilion) | 58 s | - | |
| Nidorino trade + Moon Stone -> Nidoking (Route 11 gate) | 136 s | - | |
| Omanyte (Cinnabar lab, Helix Fossil) | 67 s | - | caught 60 at 18 263 s |
| party back (Cinnabar PC) | 42 s | - | |
| National Dex (fly Pallet, Oak) | 62 s | 50 s | split 18 357 s vs p50 10 032 s |
| Celio's request (fly Vermilion + Seagallop) | 52 s | 82 s | |
| Kindle Road + grunts + Ruby Path in | 384 s | 169 s | |
| Ruby Path out + Rainbow Pass | 147 s | 80 s | |
| Icefall Cave | 287 s | 156 s | ice route re-planned once (Entrance row 30) |
| Six Island, Dotted Hole | 243 s | 0 s | |
| Five Island, Meadow pickups, warehouse pearls, Gideon | 906 s | 600 s | pickups ~150 s |
| Sapphire to Celio | 71 s | 98 s | |
| Cerulean (heal, Max Elixir), Mewtwo, TMs, PP Up | 448 s | 181 s | |
| fly Indigo, Route 23 Max Elixir, heal, sell, 2 Revives + 12 Full Restores | 73 s | 55 s | |
| Lorelei 2 / Bruno 2 / Agatha 2 / Lance 2 | 364 / 224 / 150 / 178 s | 167 / 143 / 126 / 173 s | |
| Champion 2 (to the HoF screen) / HoF 2 to the black frame | 332 / 32 s | - / - | |
| **finish** | **22 247 s (6:10:47)** | p50 12 279 s | +9 968 s; WR 12 325 s |

Rollbacks after `dex-fill-24` (first survivor kept): Lance 2 x2 (from the Agatha-2 checkpoint), Champion 2 x4 (from
the Lance-2 checkpoint); none in the dex finish and the Sevii legs. Over the whole log 48 (42 before, mostly
controller soft-locks in the dex fill).

## Controller v3: the dex on the main route (`runs/human-play/1122-lgdescent-s11-v3`, 2026-11-23, journal `2026-11-23-00-30-controller-v3.md`)

Giovanni 1 -> finish re-played with passing catches (the sim's `catch_on_pass` / `pass_min_catch` rule), the plan's
Safari hunts, plan 1413's mart stops and RAM-planned dex errands at the centers the route passes. One continuous
pure-input log from boot (prefix `1122-lgdescent-s11` to `giovanni1`, then `dev-v3a` -> `dev-v3b` -> `v3` resumes at
checkpoints). Seconds on the run timer.

| split | v2 / pg-canon5 | v3 | model p50 | v3 - p50 |
| --- | --- | --- | --- | --- |
| Koga | 7 353 | 7 936 | 5 103 | +2 833 |
| Blaine | 8 515 | 9 673 | 5 890 | +3 783 |
| Sevii | 9 127 | 10 438 | 6 805 | +3 633 |
| Giovanni | 10 992 | 12 452 | 8 198 | +4 254 |
| Victory Road | 11 795 | 13 669 | 9 175 | +4 494 |
| Hall of Fame 1 | 12 874 (18 caught) | 14 716 (**45 caught**) | 9 970 | +4 747 |
| National Dex | 18 357 | 17 123 | 10 032 | +7 091 |
| Sevii done | 20 446 | 19 278 | 11 217 | +8 061 |
| Mewtwo | 20 894 | 19 701 | 11 397 | +8 304 |
| Lance 2 | 21 884 | 20 617 | 12 006 | +8 611 |
| **Finish** | **22 247** | **20 975 (5:49:35)** | 12 279 | **+8 696** (WR +8 650) |

Stretches (v2 -> v3, seconds): Safari 251 -> 530 (+5 species incl. the walk's 2), Koga stage (heal, errands, gym)
338 -> 711 (errands ~85 s; the rest a Kirk sleep line, RNG), Koga -> Power Plant out 474 -> 691 (+8 species
passing / hunt), Power Plant -> Blaine 687 -> 1 012 (Cinnabar mart + 5 errand species ~300 s), HoF -> National
Dex ~5 480 -> 2 407 (15 species: 13 hunted, Ninetales, Magikarp). Per species: main-route passing catch / errand
**~45-70 s**, post-HoF fill **~150 s** (v3) to ~130 s (pg-canon4, from 18). Money at the Indigo mart before round 2:
¥53 678 (pg-canon5 ¥39 673): 2 Revives + 13 Full Restores.

Rollbacks Giovanni 1 -> finish: 40 logged (warden 2, sevii 1, route22 2, lorelei 1, agatha 20 incl. one dead-end
step back to bruno 1, lance 2, dotted-hole 8 (warehouse admins), lance-2 3), plus 2 controller-bug deaths at
Mewtwo (party full) dropped by the resume after the fix.
