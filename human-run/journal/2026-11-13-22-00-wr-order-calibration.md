# 2026-11-13 (night): the WR route order, and calibrating the model to the real splits

Task: the model's no-reset mean for `372-tactic181` was ~5:36 against the WR's 3:25:25 and its route order was
not the WR's. This session built the WR order (`route_style = "wr"`), added per-split output, found where the
time went, calibrated, and wrote `plans/373-wr-order.md`. Branch work only; `CURRENT` (the old-order chain) is
untouched, `plans/CURRENT-WR` points at 373.

## What was built

* `route.rs`: the old order's sections became `leg_*` functions (the old order builds the identical segment list,
  checked: same scores); `route_wr.rs` composes them in the WR order and adds what the WR has and the old order did
  not: Camper Liam in Pewter's gym, the Power Plant detour, Icefall Cave (the Lorelei scene and a forced grunt,
  `decompiled/data/maps/FourIsland_IcefallCave_Back/scripts.inc`), stones after the Sevii trip, the dex fill
  **before** Indigo and no post-Hall-of-Fame fill.
* `frlg-human splits <plan> [--R n] [--all]`: model no-reset cumulative clock at each of the 27 WR splits next to
  the real time (mean, p10, p90, error), species caught and carry level at each split next to the WR's, the time
  per WR window split into fights / walking+wild / text, and model player actions and seconds per fight next to the
  16 fights measured on the video. `--all` lists every segment. A `JSON` line is machine-readable.
* New plan knobs, all default-off: `route_style`, `calib_wr`, `carry_power`, `mewtwo_power`, `candy_early`,
  `candy_early_n`, `preheal_below`, `no_dig`, `Hunt.on_route`, and `group_n` now also caps the cone-forced lists.
  `frlg-human dist` additionally prints which species are missing when the fill starts.

## Where the 2 hours went (372's model, old order, 5:36)

1. **Fights were ~1.5-5x the real ones** and the single biggest cost (54% of the model's run before Giovanni 3 was
   fighting; the WR's windows leave room for ~40%: an estimate). Per-mon and per-action overhead was ~15% high (fit below), but the main
   term was the *number of actions*: the model needed 26 actions for Misty (WR 6.5), 18 for the S.S. Anne rival
   (6), 17-26 for Agatha/Lance (6/8).
2. **Why:** the WR carry out-damages the engine's by ~2x. It keeps Torrent active (it walks into most fights at
   <= 1/3 HP: Koga 33/120, Giovanni 35/143, Sabrina 43/135, Agatha 51/159 ...) and is a 30-IV-SpA Rash Squirtle with
   X Specials in the bag. A `carry_power` of 1.9 (Attack and SpA) brings the model's actions to the video's: mean
   |log(model/WR)| over 16 fights 0.58 -> 0.28. Mewtwo needs 1.5 for the round-2 fights. Fitted stand-ins, not
   mechanisms; sensitivity rows `w373-005/006` show how hard the EHT depends on them (1.6: +46 h, 2.2: -7 h at WR+20m).
3. **Levels:** the model sat 2-3 levels under the WR early (Lv17 at Misty against Lv20) and therefore lost or
   stalled the early fights (Misty 29 actions). The WR is Lv50 at the League, the model Lv59+. Eating the first two
   Rare Candies on the spot (`candy_early_n`) and keeping the rest for the League matches the WR's levels at every
   split it was observed at (table).
4. **Dex before the League:** the old order fills after Hall of Fame 1; in the WR's order 60 must be done before
   Indigo. The cheap non-catch entries (Abra and Clefairy bought at the Game Corner, Magikarp from Route 4's
   salesman, Omanyte, candy evolutions) moved before the League. Moving the two purchases and adding Magikarp cut
   the Giovanni 3 -> Indigo window from 38.5 to 30.7 minutes ("Start E4R1" 3:03:31 -> 2:57:58).
5. **Smaller:** Viridian Forest encounters (the model averages ~3 minutes; the WR's one sample had none, so Brock is
   +7.7% and the WR sits below the model's p10); Dig and the grind segments (not in the WR's route) removed; the
   Hall of Fame text 5400 -> 3000 frames (WR: ~50 s from the Champion's defeat to the save).

## Calibration result (`373-wr-order`, R=100000, no resets, finishers only)

| split | WR | model mean | err | p10 | p90 | species model / WR | level model / WR |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Brock | 0:14:40 | 0:15:47 | +7.7% | 0:14:44 | 0:16:50 | 5 / 3 | 13 / 13 |
| Misty | 0:28:01 | 0:27:50 | -0.6% | 0:26:17 | 0:29:26 | 10 / 8 | 20 / 21 |
| Bill | 0:38:10 | 0:39:37 | +3.8% | 0:37:56 | 0:41:23 | 10 / - | 24 / - |
| Bike | 0:46:14 | 0:45:54 | -0.7% | 0:44:09 | 0:47:45 | 10 / 9 | 26 / 27 |
| Rock Tunnel | 0:55:39 | 0:56:21 | +1.3% | 0:54:28 | 0:58:20 | 10 / - | 30 / - |
| Giovanni 1 | 1:04:56 | 1:06:15 | +2.0% | 1:04:17 | 1:08:18 | 11 / 11 | 33 / - |
| Snorlax | 1:19:47 | 1:20:25 | +0.8% | 1:18:23 | 1:22:35 | 13 / 14 | 36 / - |
| Koga | 1:31:42 | 1:35:35 | +4.2% | 1:33:00 | 1:38:20 | 20 / 19 | 38 / 41 |
| Blaine | 1:42:54 | 1:48:42 | +5.6% | 1:45:55 | 1:51:37 | 27 / 27 | 40 / 42 |
| Sevii | 1:58:07 | 2:01:16 | +2.7% | 1:58:21 | 2:04:19 | 31 / 35 | 42 / - |
| Sabrina | 2:11:54 | 2:18:02 | +4.7% | 2:14:54 | 2:21:20 | 41 / 45 | 46 / 46 |
| Erika | 2:14:50 | 2:22:25 | +5.6% | 2:19:18 | 2:25:42 | 42 / 46 | 47 / - |
| Giovanni 3 | 2:23:53 | 2:27:13 | +2.3% | 2:24:05 | 2:30:31 | 43 / 52 | 48 / 49 |
| Start E4R1 | 2:36:05 | 2:57:58 | +14.0% | 2:50:56 | 3:04:55 | 61 / 60 | 54 / 50 |
| Lorelei 1 | 2:38:58 | 3:00:29 | +13.5% | 2:53:21 | 3:07:39 | 61 / - | 55 / - |
| Bruno 1 | 2:41:03 | 3:02:28 | +13.3% | 2:55:20 | 3:09:37 | 61 / - | 55 / - |
| Agatha 1 | 2:42:53 | 3:04:44 | +13.4% | 2:57:34 | 3:11:57 | 61 / - | 56 / - |
| Lance 1 | 2:45:39 | 3:07:48 | +13.4% | 3:00:36 | 3:15:09 | 61 / - | 58 / - |
| Champ 1 | 2:48:50 | 3:12:02 | +13.7% | 3:04:46 | 3:19:33 | 61 / - | 59 / 56 |
| Icefall Cave | 2:58:24 | 3:19:31 | +11.8% | 3:12:14 | 3:27:03 | 61 / - | 59 / - |
| Rocket Warehouse | 3:07:48 | 3:26:00 | +9.7% | 3:18:37 | 3:33:32 | 61 / - | 61 / - |
| Lorelei 2 | 3:13:59 | 3:32:01 | +9.3% | 3:24:40 | 3:39:36 | 62 / - | 61 / - |
| Bruno 2 | 3:16:03 | 3:33:43 | +9.0% | 3:26:22 | 3:41:17 | 62 / - | 61 / - |
| Agatha 2 | 3:17:56 | 3:35:23 | +8.8% | 3:28:02 | 3:42:58 | 62 / - | 61 / - |
| Lance 2 | 3:20:30 | 3:37:52 | +8.7% | 3:30:34 | 3:45:29 | 62 / - | 61 / - |
| Champ 2 | 3:24:12 | 3:40:50 | +8.1% | 3:33:19 | 3:48:44 | 62 / - | 61 / - |
| End | 3:25:25 | 3:41:33 | +7.9% | 3:34:02 | 3:49:27 | 62 / - | 61 / - |

Species counts: the WR's are the pair shown for the *comparison* routes on its overlay, so they are targets, not
the live run's count. Levels are the carry's level after the split, from the fight analysis where it was visible.

No-reset distribution of the finishers (3074 of 100000 attempts; 14% pass the starter gate, a fifth of those
finish): **min 3:24:56, p10 3:34:02, p50 3:41:07, p90 3:49:27**, mean 3:41:42. The model's old figure was 5:36 and
the WR 3:25:25.

Fights (model actions and seconds against the video; seconds include the outro and level-up text the video's
intro-to-"defeated" durations do not):

| fight | model actions | model s | win | WR actions | WR s |
| --- | --- | --- | --- | --- | --- |
| brock | 2.3 | 56 | 99% | 2.0 | 31 |
| misty | 9.5 | 105 | 94% | 6.5 | 54 |
| rival-ss-anne | 5.1 | 93 | 98% | 6.0 | 76 |
| koga | 7.0 | 107 | 95% | 8.0 | 72 |
| blaine | 4.5 | 86 | 100% | 6.0 | 80 |
| sabrina | 5.0 | 96 | 93% | 9.0 | 80 |
| giovanni | 5.0 | 99 | 100% | 6.0 | 78 |
| rival-route22 | 10.9 | 178 | 85% | 12.0 | 125 |
| lorelei | 12.7 | 151 | 100% | 11.0 | 148 |
| bruno | 8.8 | 122 | 100% | 7.0 | 104 |
| agatha | 10.4 | 138 | 100% | 6.0 | 88 |
| lance | 17.2 | 182 | 100% | 8.0 | 120 |
| champion | 14.6 | 217 | 100% | 10.0 | 154 |
| bruno-2 | 5.6 | 102 | 100% | 5.0 | 92 |
| agatha-2 | 5.1 | 99 | 100% | 6.5 | 98 |
| lance-2 | 13.8 | 151 | 100% | 7.5 | 110 |
| champion-2 | 17.6 | 184 | 100% | 11.5 | 196 |

Ladder: WR+20m, **EHT 22.4 h** (4605 finishes in 200000); WR+10m has 16 finishes, WR+40m and looser 17.0 h. The
plan's reset thresholds are 372's and are not tuned for this order.

## What is still off

* **The pre-League fill (30 minutes) is the biggest residual: +14% at "Start E4R1".** The WR gets 52 species by
  Giovanni 3 and 60 by Indigo in a 12-minute window; the model has 43 at Giovanni 3 and needs ~30 minutes of
  hunts (~85 s/species at the fill sites against the WR's ~50 s/entry blended). Every Mon-by-Mon source was already
  in (gifts, trades, stones, candy evolutions). What is missing is mostly evolutions of caught species the WR
  levels (Golbat, Sandslash, Machoke, Muk, Marowak, Parasect, Venomoth) and Safari species (Tauros, Kangaskhan,
  Chansey, Pinsir). The remaining in-game trades (Seel, Nidoran/Nidorina swaps, Jynx, Lickitung) are not modelled.
* **Early fights are still long in seconds:** Brock 56 s (WR 31), Misty 105 s (54), Lance 182 s (120). The video's
  4-5 s sampling makes the early ones unreliable (a one-turn Brock against two Geodude/Onix hits is not
  plausible), and Lance's Gyarados/Dragonite fights are slow for the engine's carry.
* **Brock +7.7%, Koga/Blaine +4-6%:** forest encounters, Safari at 68 s/species against the WR's ~47, and levels
  38/40 at Koga/Blaine against 41/42.
* **E4 win rates:** of the attempts that reach them, the model loses ~30% at the round-1 Champion and ~55% at
  round 2's Champion (21% of starter-passers finish) even with the calibrated power; the WR passes both. The plan pays 4 sacrifices at the Champion for it (`w373-004`: removing
  them costs +5.7 h).
* **Not measured, still guesses (68% of the run's time by source):** all the scene-text frame counts added by
  `route_wr.rs` (fly 700, boats, Celio, Icefall), the Power Plant walk (260 tiles), Safari entry, the Magikarp,
  Abra and Clefairy purchases' frames. Mistakes (0.3/100 tiles) and the 1.02 factors are "a top runner's typical
  attempt", not measured.
* `human-run/video-analysis/STRATEGY.md` named in the task does not exist in this checkout; the split list used is
  `wr-3.25.25-main97.md`'s table.

## Next

1. Descend 373 on the calibrated model (`descend.py --pointer CURRENT-WR`): reset thresholds, hunts (more on-route
   Safari/Victory Road catches), fill order, the bruno sacrifice (`w373-009`, -0.26 h, 95% CI excludes 0).
2. Model the remaining in-game trades and level evolutions; re-check species counts against the video's own tracker.
3. Tier-1: a one-turn trainer intro and the Route 4 / Game Corner purchase frames.
