# 2026-12-02: Mewtwo PP on, boost / set-up queues in the R2 fights

Tools: scratch `bm-cmp.py` (paired `frlg-human compare`, optional extra Champion-2 `fatal` added to BOTH sides, so "x0.15" is a 1051/1053 with 0.15 extra Champion-2 death vs the candidate with the same), `bm-run.py`. Screens R 800k WR+2m; confirmations R 3.2M seeds 71/72.

## 1. `mewtwo_pp` on: 1051 does not hold up
`mewtwo_pp` alone (Psychic 10 PP, Fire Blast 5, Earthquake 10, Calm Mind 20, no Max Elixir in the bag, plain Elixirs 2): 1051 collapses to 7 finishes in 200k (EHT x40). The old model dropped the Max Elixirs. Adding the two on-route Max Elixirs
(Route 17 `decompiled/data/maps/Route17/map.json:266`, Route 23 `Route23/map.json:621`; `elixirs` `cycling-road` max 500 f, `route23` max 400 f) and `mewtwo_pp_floor` 4..5 (top Mewtwo up when Psychic < floor before a League fight; 3: +87 h, 6: +147 h, 7-8: more; floor and `league_elixir_reserve` 1-3 no help;
a third Max Elixir at Power Plant `PowerPlant/map.json:175` adds nothing) gives the honest number. Paired vs 1051 (R 3.2M, seeds 71/72, the PP-off 1051 as parent):
WR+2m 254.8 / 256.4 h vs 160.9 / 165.0 (+93.9 / +91.4, CI excludes 0), WR 361.0 / 358.1 vs 205.7 / 209.2 (+155 / +149). Floor 5: 255.4 / 254.7, 358.7 / 354.1: same. **So the PP-off numbers (and every earlier absolute EHT) are ~40-70% optimistic in this respect**; Mewtwo needs the Max Elixirs and about one top-up per round.
Not adopted as CURRENT (model change; all other plans share the PP-off model); plan `1054-calm-sp9-mewpp` is 1054 with it on.

## 2. Boost / set-up queues (R2 Mewtwo fights; screen R 800k seed 41, WR+2m, vs 1051 155.3 h)
Who can set up (decomp): Mewtwo learns TM04 Calm Mind (taught at Saffron, `SaffronCity_Gym/scripts.inc:22-26`), TM08 Bulk Up, TM16 Light Screen, TM33 Reflect, TM32 Double Team (`src/data/pokemon/tmhm_learnsets.h:3546-3580`). The carry (Blastoise) has none of TM04/TM08 and no level-up boosting move
(`tmhm_learnsets.h:201-210`), so for the pre-HoF fights (gyms, rivals, Blaine, Giovanni, R1 League) only X items exist (already tuned; Calm Mind is not an option for them). Bulk Up (Atk/Def) does nothing for a Sp. Atk sweeper; Light Screen / Reflect need a Mewtwo move slot (the 4 are Psychic, Fire Blast, Earthquake, Calm Mind) and Celadon-roof drinks: not tried.
Queue tokens tried per fight (`[]`, calm, calm-calm, special, special-special, calm-special, special-calm, speed, calm-speed, calm-calm-speed): 
* bruno-2 and agatha-2: identical to baseline in every queue (set-up never triggers; they die in one hit). agatha-2 calm: -0.9 CI crosses 0.
* lorelei-2 calm-calm -2.1 [-4.1,-0.3]; lance-2 calm-calm -2.0 [-3.8,-0.1], calm-special -1.5; speed anywhere +4..+22 (worse), none at lance-2 +13.
* champion-2 (current calm-calm): calm 1x +52, calm-speed +39, guard-containing +6..+64, speed-containing +6..+7, calm-calm-special +4, calm-special-special +28, special-special +104 (the known calm gain), calm-calm-calm -1.5 [-5.0,+1.9] (nothing: HP gate), defend token no effect. calm-calm stays.
The calm replacements do not change the fights much but free X Specials: with lorelei-2 + lance-2 on calm-calm, the second X buy `special` 13 -> 9 is **-6.6 h [-10.1,-3.3]** (seed 42; 11: -4.5, 10: -6.9, 8: -4.0 [-9.1,+0.3], first-buy special 9/7/6 and speed 0 no better, 7: +8.5, 5: +61; special 9 alone without calm: -0.9 CI crosses 0, 7 alone +26). Plateau 9-11.

Confirmation R 3.2M, seeds 71/72, vs 1051 and again vs the new CURRENT 1053-spe25 (the starter change is orthogonal):
| vs | extra Champ-2 fatal | WR+2m diff | WR diff |
|---|---|---|---|
| 1051 | 0 | -7.9 / -7.6 | -10.5 / -10.7 |
| 1051 | 0.15 | -9.7 / -9.3 | -12.4 / -13.2 |
| 1051 | 0.30 | -11.8 / -11.8 | -14.6 / -16.6 |
| 1053 | 0 | -7.8 / -7.6 | -10.6 / -10.3 |
| 1053 | 0.15 | -9.7 / -9.1 | -12.6 / -13.1 |
| 1053 | 0.30 | -11.8 / -10.9 | -14.7 / -14.9 |
All CIs exclude 0 (about 5-6% of EHT at x0, larger in absolute terms under more misplay; `special` 11 gives about 70% of this). Note the extra fatal is applied to both sides, i.e. the question "calm line stays better than the old line at equal Champion-2 risk"; the Champion-2 calm line itself is unchanged here (break-even .40 from the audit still stands).
Under `mewtwo_pp` on (parent = same PP model, R 800k seed 44 screen, 1051 base 253 h): A+special9 -20.5 [-31.9,-9.7], floor 5 -27.4 [-38.4,-15.3]; R 3.2M seeds 71/72 vs 1051-PP: floor 4 WR+2m -18.0 / -14.8, WR -22.3 / -25.0; floor 5 WR+2m -24.4 / -21.8, WR -32.8 / -37.0; extra .30 floor 5 WR+2m -38.1 / -27.3, WR -54.8 / -48.4. On 1053-PP as parent (R 3.2M seeds 71/72, floor 5): WR+2m 210.4 / 215.9 h vs 234.0 / 238.5 (-23.5 / -22.6), WR 299.0 / 296.4 vs 331.4 / 331.3 (-32.4 / -34.9); extra .30: WR+2m -35.4 / -31.2, WR -51.5 / -47.1. All CIs exclude 0: the gain is larger under the PP model (calm saves X Specials and Psychic PP).

## 3. Adopted
Plan `1054-calm-sp9` = 1053 + calm-calm at Lorelei-2 and Lance-2 + second X-buy special 9; `CURRENT` -> 1054-calm-sp9 (PP-off model, same as 1053). 1054: WR+2m 141.3 / 144.3 h, WR 179.9 / 181.6 h (seeds 71/72 R 3.2M). `1054-calm-sp9-mewpp` carries the PP model (not CURRENT).

## 4. Next
Descend from 1054 with `calm` tokens and the PP model on (the descent's tactic knobs never offered calm/accuracy/guard tokens); the PP model needs its own descent (Elixir pickups, floor, `league_max_reserve`); reset_table not refit.
