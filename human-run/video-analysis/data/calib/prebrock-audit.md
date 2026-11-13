# Pre-Brock segment audit: model (plan 370-clemi-skip) vs TAS ledger vs WR video

Model numbers: `frlg-human report human-run/plans/370-clemi-skip.md --rung WR+40m` (reproduces the brief's 27:58).
Experiments ran on a scratch copy of the crate (`scratchpad/w`, instrumented counters + env switches);
the repo itself was not edited (the `battle.rs` / `scores.jsonl` modifications in `git status` are not mine).
EHT numbers are `eval`/`compare` at WR+160m, R=12000, seed 1; CIs for 0.3-0.5 h differences are wide.

## 0. Headline

* The ledger-sourced "Text" segments already reproduce the WR. Sum of the TAS ledger's pre-Brock segments is
  639 s (10:39, boot excluded). The model scales them by an effective **x1.33** (text_factor 1.25 in
  `params.rs:406` times input jitter), not x1.2: 850 s = 14:10, vs the WR's 14:40. So the whole 13:18 gap
  between model (27:58) and WR sits in the `Src::Guess` segments, not in the ledger rows.
* Two mechanical bugs explain **6:58** of it: the wild-encounter model has no cooldown gate (-6:24 when added),
  and the ledger's `forest` row already contains Sammy's fight, which `forest-trainers` charges again (-1:01).
  Corrected model: **28:00 -> 21:02** to Brock. A further 1:15 (Aerodactyl gift + heal) is a structural
  difference from the WR (shifted, not wasted) and ~0:26 is rival-lab calibration. Total explained about
  **8.6 min of the 13.3 min gap**; the remaining ~4.7 min is catches/fights the WR route simply does not take
  (model catches 5-6 species pre-Brock, WR had 3 total at Brock) plus slow trainer fights.

## 1. Aligned columns (seconds)

TAS = ledger frames / 59.7275 (`route/defeat-brock/ledger.json`). WR column: what the video supports.

| segment | model | TAS | TAS x1.2 | TAS x1.33 | model - x1.2 | WR / note |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| intro-oak | 35 | 26.2 | 31.4 | 34.8 | +3.6 | ok at x1.33 |
| names | 23 | 17.5 | 21.0 | 23.3 | +2.0 | ok |
| options | 4 | 3.3 | 4.0 | 4.4 | 0 | ok |
| house | 9 | 7.2 | 8.7 | 9.6 | +0.3 | ok |
| to-lab | 27 | 20.3 | 24.3 | 27.0 | +2.7 | ok |
| starter | 38 | 28.8 | 34.6 | 38.3 | +3.4 | ok |
| rival-lab | 83 | 47.2 (354+2463) | 56.6 | 62.7 | **+26** | calib floor: lab fight 3317 f measured on tier 1 (`calibration.md`) and `trainer_intro` floor 1250 f (`sim.rs:231-237`); ~20 s over x1.33 |
| exit-lab | 19 | 14.5 | 17.4 | 19.2 | +1.6 | ok |
| to-viridian | 59 | 44.7 | 53.7 | 59.5 | +5.3 | ledger row already contains 2 fated flees |
| route1-encounters | **173** | 0 (flees inside neighbours) | - | - | +173 | see 2b |
| parcel | 28 | 21.4 | 25.7 | 28.5 | +2.3 | ok |
| deliver | 99 | 74.0 | 88.8 | 98.5 | +10 | ok |
| tutorial | 124 | 92.9 | 111.5 | 123.6 | +12.5 | contains 1 fated flee |
| to-forest | 25 | 19.4 | 23.3 | 25.9 | +1.7 | ok |
| route2-encounters | **44** | 0 | - | - | +44 | see 2b |
| forest | 150 | 112.3 | 134.8 | 149.4 | +15 | **includes Sammy** (see 2c) |
| forest-encounters | **461** | 0 (1 flee inside `forest`) | - | - | +461 | see 2a |
| to-pewter (+to-gym) | 34 | 25.6 | 30.8 | 34.1 | +3.2 | ok |
| gift-aerodactyl | 60 | 0 | - | - | +60 | structural: WR is (3,3) at Brock, no amber pre-Brock |
| forest-trainers | 74 | (in `forest`) | - | - | +74 | **Sammy counted twice**, WR fought him (+99 exp seen at video 0:11) |
| center-heal | 15 | 0 | - | - | +15 | TAS has no heal (`docs/defeat-brock/route.md`: arrives 20/23) |
| brock | 96 | 83.6 (4991 f) | 100.3 | 111.1 | -4 | ok |
| total | 1680 | 639 | 767 | 850 | +913 | WR split 880 |

Reading: the Text rows are all within 1-12 s of TAS x1.33. Everything that is inflated is Guess-sourced.
The WR video (`wr-scene-A.jsonl`, 1 sample per minute) shows: lab rival (0:02), Route 1 / Rattata battle (0:04),
Viridian mart (0:06-0:07, a Potion item description), a Weedle ball throw at 0:09 (Lv7 squirtle at 18/24),
poisoned + "gained 99 EXP" at 0:11 (= Sammy: `docs/defeat-brock/research/story-gates.md` "Sammy's Weedle L9 ... 99"),
the gym-looking room at 0:12, Lv13 Water Gun at 0:14, Pewter mart at 0:15, Route 3 bug catchers 0:16-0:20.
So the WR walked Route 1/2 and the forest with only 2-4 sampled battles, fought only Sammy in the forest,
and had caught 3 species at Brock. The scene log cannot say whether Route 22's rival was skipped (no rival battle
is sampled between 0:04 and 0:11, and the Lv6 -> Lv7 move is consistent with skipping); and the timestamps
imply Brock was beaten around video 0:13 (run ~12:30), earlier than the 14:40 split, which I could not reconcile
(sampling is 1/min). I did not rely on that.

## 2. The suspects

### 2a. forest-encounters 7:41 (sd 130 s)

Mechanics: `route.rs` `b.wander("forest-encounters","MAP_VIRIDIAN_FOREST", g(81,48,100))` -> `Kind::Wander` ->
`run_encounters` (`sim.rs:~778-835`) draws a geometric gap with a flat p = rate*16/1600 per grass step
(forest 14% = rate 14, Route 1/2 21%: `human-run/data/dex.json`), then `encounter()` (`sim.rs:~598`):
new Kanto species with catch_rate >= 45 are thrown at (`catch_on_pass`, `pass_min_catch` 45); otherwise,
if carry < `fight_until_level` (15 in this plan, `sim.rs:636/655`) the wild is fought with `sim_battle`,
else `flee()` (`sim.rs:~690`).

Instrumented counts per attempt (81 grass steps, plan as committed):

| | encounters | catches | fights (cost) | flees |
| --- | ---: | ---: | ---: | ---: |
| model forest | 11.4 | 3.6 (118 s) | 7.8 (344 s) | 0 |
| model Route 1 (22 steps) | 4.56 | 1.9 (48 s) | 2.6 (126 s) | 0 |
| model Route 2 (5 steps) | 1.04 | 0.2 | 0.8 (40 s) | 0 |
| **model pre-Brock total** | **17.0** | **5.7** | **11.2 (~8:30)** | 0 |
| TAS | 4 fated flees (2 to-viridian, 1 tutorial, 1 forest) ~503 f each = 33.7 s, plus Sammy | 0 | 0 | 4 |
| WR | 2-4 sampled battles incl. a Weedle catch | ~2 | Sammy | ? |

Cause 1 (the big one): the model has **no encounter cooldown**. The game gates the first `minSteps` grass steps
after a map load and after every encounter: `HandleWildEncounterCooldown` passes only with 5% until
`stepsSinceLastEncounter >= minSteps`, `minSteps = 8 - rate/10` (Route 1/2 -> 6, forest -> 7)
(`decompiled/src/wild_encounter.c:673-699, 707-755`; spelled out in `docs/defeat-brock/research/wild-encounters.md`
step 1, and `story-gates.md:70-75`). `grep -i cooldown crates/frlg-human/src` finds nothing. Each encounter therefore
costs 7 + 1/0.14 = 14 steps of forest, not 7; Route 2's 5 steps are entirely inside the gate (6 steps).
With a per-step cooldown patch (scratch copy, env `COOLDOWN=1`; ignores the 60% behaviour-change roll and the
tiny rate buff, resets per wander call):

| | steps | encounters | segment mean |
| --- | ---: | ---: | ---: |
| route1 | 22 | 4.56 -> 1.78 | 2:53 -> 0:54 |
| route2 | 5 | 1.04 -> 0.06 | 0:44 -> 0:02 |
| forest | 81 | 11.4 -> 5.7 | 7:41 -> 3:58, sd 130 s -> 66 s |

That alone is **-6:24** on the mean pre-Brock time. Side effect: the carry arrives short on exp (L8.3 after Sammy
vs 9.8, Brock whiteouts 88 vs 50 per ~870, EHT 13.12 -> 14.40 h), so the plan set must be re-optimised after the
fix; it is a correction to the model, not a free speedup.

Cause 2: the model's fights cost ~44-48 s each (intro 450 + react + turn 385 + overhead 810, `calib_enc`/`calib_battle`)
while the TAS spends ~8.4 s per encounter (flee) and a human with `Run` ~ 500 f (`calibration.md`). A fight gives
~30 exp (Caterpie L4: 53*4/7), i.e. ~0.7 exp/s, vs Sammy's 99 exp in ~74 s (1.3 exp/s) which the WR takes.

Cause 3: flee is mis-modelled. `flee()` (`sim.rs:~690-710`) uses a flat 20% success below carry 15, from 182
attempts of one TAS state. The decomp says: faster-or-equal speed escapes with certainty; otherwise
`speedVar = spd*128/foeSpd + runTries*30 > Random()&0xFF` (`decompiled/src/battle_main.c:4258-4268`). A Squirtle at
L6-7 outspeeds Caterpie/Weedle/Kakuna/Metapod/Pidgey/Rattata at L3-6 almost always, and all four TAS flees
succeeded at ~503 f. Flee at carry<15 is costed at mean ~2370 f in the model (`(1/0.2-1)*470+500`) vs ~500 real.
It only matters for fight_until_level < 15 (see 4).

### 2b. route1-encounters 2:53 and route3-trainers 5:19

* route1: grass steps are 22 *per crossing* (`pathenc.json`, `oaks-lab_to_viridian-mart`: shortest 22, avoid 21/20,
  hard 20). The route crosses Route 1 north twice (`to-viridian`, `tutorial`) and once south via the ledges
  (`docs/defeat-brock/route.md:125-128`), but the model has one wander of 22. With the cooldown fixed this is the
  one place the model *under*-counts steps (about 2 x 22 grass steps, still only ~3.5 encounters at 21%/10.8-step
  cycles, 1 per crossing-ish). Net with cooldown: 0:54 now (mostly catches: 0.7 catches vs 0.2 fights per call
  pair), and doubling the steps would give ~1:45. The TAS paid 3 flees on the two crossings, 25 s.
* route3-trainers (post-Brock, 5:19 at base; 5:55 with cooldown+Sammy fix since the carry is lower): three forced
  fights per `forced-audit.md`: Colton (3 mons L10), James (2 mons L11), Sally (2 mons L10) = 7 mons. 319 s = 106 s
  per trainer / 45 s per mon. The floor per trainer in the model is 1250 f intro (21 s) + 300 outro + per mon one
  turn 385 + 810 overhead (~20 s) + level-ups, so the 7 mons alone are ~140 s and intros ~63 s; the rest is extra
  turns on Metapod/Kakuna-class defences and level-up prompts. Compared with the WR video (0:16 112 EXP, 0:17 102
  EXP, 0:18 "defeated BUG CATCHER", 0:20 next Bug Catcher; ~2 min for three fights incl. Pewter mart) this is about
  2x slow (3 min over). TAS reference is Sammy: 2706 f = 45 s for a 1-mon trainer, vs the model's 74-89 s for the same
  fight. So trainer fights cost ~1.4-1.6x their TAS x1.2, and `intro: 600` -> 1250 f floor (`trainer_intro`) is the
  likely culprit for multi-trainer legs. No sample of a WR trainer-by-trainer timer exists, so this item is lower
  confidence than the cooldown/Sammy ones.
* FOREST3 experiment (scratch env): forcing the three hardest forest trainers instead of only Sammy costs 7:06
  for the group (2:20 each at L6-9) and EHT 13.5-27 h, so the optimizer is right to avoid them *given these costs*.

### 2c. Sammy double count

`route/defeat-brock/ledger.json` `forest` (6708 f, "out the forest's north side") includes the Sammy fight
("`forest` ... Sammy (**2706**, plan [109,15], 4 turns)", `docs/defeat-brock/route.md:130`). `route.rs` pushes
`b.text("forest", 6708, Ledger)` and then `b.forced("forest-trainers", ["ViridianForest_EventScript_Sammy"])` as
a separate Parties fight (74-89 s). `brock` and `rival-lab` are correctly cut out of the ledger (the comment at
`route.rs` "Its battles are re-modelled by the trainer model" covers those two but not Sammy).
Fix: `forest` = 6708 - 2706 = 4002 f. Measured: forest segment 2:30 -> 1:29, **-1:01** mean, EHT 13.12 -> 12.78 h
(single paired run, same seed).

## 3. Grass steps

| leg | route.rs g(short, avoid, guess) | TAS / decomp | verdict |
| --- | --- | --- | --- |
| Route 1 | 22 / 20 / 50 | >= 20 forced per crossing, first tile north of Pallet is grass; `story-gates.md:72`, `pathenc.json` | 22 is one crossing; route has 2 grassy crossings + a ledge crossing south |
| Route 2 | 5 / 0 / 60 | "both Route 2 grass blocks have clean bypasses (0 forced grass)" `story-gates.md:36` | fine as a bounded value; with cooldown it contributes ~0 |
| Forest | 81 / 48 / 100 | >= 48 forced, 51 while dodging trainers; TAS used "one planned A* crossing", 1 flee | 81 is the *shortest* path (human); 48 the floor. Ledger records no per-step count |
| Route 3 | 15 steps in a hunt (g 0/0/50 for walk) | not recorded | unmeasured |

The ledger records no grass-step counts at all (only frames and flee counts in the prose); the only TAS facts are the
four fated flees (2+1+1) and "forest index-walled at >= 2 on every seed". `avoid_grass` (48/20/0 + detours) was
tested: to-Brock sum 28:00 -> 24:19 (-3:41) with EHT 13.4 vs 13.8 h (R=6000) and 13.12 vs 13.14 (R=12000,
`compare` diff +0.02 h, CI +-0.5): the walking detours and lower exp wash it out. With the cooldown fix applied
the step count matters far less (steps are mostly inside gates).

## 4. Recommendations

| # | change | evidence | expected effect on the pre-Brock floor |
| --- | --- | --- | --- |
| 1 | Add the encounter cooldown to `run_encounters`: per grass step, while `since < minSteps` (`8 - rate/10`, 8 for rate < 10, 0 for >= 80) pass the gate with 5%, then the 21%/14% rate test; reset `since` on encounter and map entry; ideally carry `since` across wanders on the same map | `decompiled/src/wild_encounter.c:673-699, 707-755`; `research/wild-encounters.md` step 1 | **-6:24** mean to-Brock (28:00 -> ~21:35); encounter count 17 -> ~7.5; sd of forest-encounters 130 -> 66 s. Re-fit afterwards: carry arrives ~1.5 levels lower |
| 2 | `forest` ledger text: use 6708 - 2706 = 4002 frames (or drop `forest-trainers`) | `ledger.json` + `route.md:130` | **-1:01** (EHT -0.34 h) |
| 3 | Replace the flat 20% flee with the decomp rule (certain when player speed >= foe, else `spd*128/foe + 30*tries > rand&0xFF`), cost ~500 f | `battle_main.c:4258-4268`; TAS 4/4 flees ok at ~503 f | none at fu=15; makes fu<15 honest: removes ~1:50 per forest pass from the non-fighting branch |
| 4 | Keep `fight_until_level` 15. Tested at WR+160m R=12000: fu15 13.12 h; fu12 13.61 (+0.49, CI [0.02,0.93]); fu10 +0.40 (n.s.); fu6 +2.15; fu0 with flee-fixed ~+1.2; fu20 +5.1. With cooldown+Sammy fixes: fu15 13.97, fu12 15.21, fu20 13.98, fu0 16.16 | `eval`/`compare` runs | The optimizer does benefit from fighting low-level wilds, because in the model it is the only exp source besides Sammy and Brock needs L>=8-9 here (Brock whiteouts 5.7% -> 39% at fu0). That is a symptom: WR/TAS reach Brock at L7-10 (TAS L7 `route.md`; WR L7 at Sammy, 99 exp) so Brock's difficulty (tactics/luck) is the real lever, not the knob |
| 5 | Re-fit multi-trainer legs: `trainer_intro` max(600*1.25, 1250) f and per-mon 385+810 give 45-106 s per Route-3 trainer; calibrate to one WR trainer fight (video 0:16-0:18 is ~40 s each) | `sim.rs:231-237`, WR scene log; low confidence | Route 3: up to -2 to -3 min (post-Brock); Sammy 74 -> ~55 s |
| 6 | Treat `gift-aerodactyl@to-pewter` (60 s) and `center-heal` (15 s) as moved, not saved: the WR is (3,3) at Brock and the TAS has neither | WR split table; ledger | 1:15 off the Brock split, same total elsewhere |
| 7 | Report `text_factor` as 1.33 effective (1.25 x jitter), not 1.2; it is not an inflation (TAS x1.33 = 850 s vs WR 880 s) | table in section 1 | 0 |
| 8 | Route 1: model two grassy crossings (2 x 22) once cooldown exists; ensures the 1:45 is not under-counted | `route.md:125-128`, `pathenc.json` | +~0:50 (a correction in the other direction) |

## 5. Total explained inflation

| item | minutes |
| --- | ---: |
| encounter cooldown missing (route1+route2+forest) | 6.4 |
| Sammy in both `forest` and `forest-trainers` | 1.0 |
| **mechanical subtotal (28:00 -> 21:02 measured together)** | **7.0** |
| Aerodactyl gift + heal (structural, off the WR's Brock split) | 1.25 |
| rival-lab over TAS x1.33 | 0.35 |
| **explained of the 13.3 min gap** | **~8.6** |
| remaining: catches/hunts the WR does not do pre-Brock (5.7 vs ~2), slow fights (forest trainers 1:29 at L7, Brock 1:48), trainer-speed constants | ~4.7 |

Caveats: the cooldown patch is approximate (no behaviour-change roll, per-call reset, repel split); counts are
per attempt that reaches the segment, normalised by 2 calls per wander (repel / non-repel); `COOLDOWN` and the
other env switches live only in `scratchpad/w`. The retuned plan (rung ladder, Brock tactics) was not
re-optimised, so EHT after the fixes is *higher* (13.97 vs 13.12 h) until the optimizer re-fits.
