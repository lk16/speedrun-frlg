# Handoff for the next session

## 2026-11-24 R3 round 2: model bugs from R2a / R2b (journal `journal/2026-11-24-22-00-r3b-model-bugs.md`)

**CURRENT stays 1841-r3-repel** (session ended on the usage limit; nothing adopted). New default-off keys: `cut_honest`
(+ hunt / teach / detour knobs) and `mid_bag` (counted potion bag from Pewter). Findings, ranked for the next session:
1. **`manip.dials` 2 (naming-screen dial): -812 h [-1194, -555]; with tier spe 28 / hp 16 / spd 20 -971 h** (one seed,
   8201). Confirm on a second seed and adopt; the controller's two-dial path (R2a) must be played.
2. **`mid_bag` + Potion top-ups 10 / 10 / 10 (Pewter, Cerulean `persim-and-potions`, `bike-redeem`) + Cerulean heal
   before Route 9 (`extra_heals` / `torrent_heals` `bike-redeem`): -180 h n.s. vs 1841; 10 / 6 / 6 + heal -335 [-708, +15] n.s.** = the honest Rock Tunnel bag
   costs nothing with that response. Adopt as the model of record.
3. **Cut (`cut_honest`) +2812 h**, no plan response found (Route 25 hunt 150-600 steps, a Mt. Moon Paras hunt: worse).
   Try balls before the hunt and a priced walk-around.
4. Tier-1 frames (R2a runs): TM39 sale ~470 f, a buy row ~210 f, Route 1 clerk ~400-560 f, forest Potion <= 628 f.
Not model bugs: the bridge heal (skipped in the Torrent phase), the Teleports (walked), the trades (never happen),
the Persim fetch (in `persim-and-potions`).

**Controller (R2) keys, unchanged from 1841** (table in the R3 section below), plus these controller obligations:
pick up the three hidden Rare Candies (Cerulean (18,7), Route 6 (19,5), Route 9 (63,2): skipping them is +157 548 h);
buy 3 Repels at Pewter (one per Mt. Moon floor) and 1-2 at Cerulean (Rock Tunnel); the Route 4 Magikarp; the
Persim pickup and give; until `mid_bag` is adopted, keep healing at Cerulean before Route 9 and topping up Potions
(the honest plan above is what the model will price).

## 2026-11-24 R3 (review round 1, model and plan; journal `journal/2026-11-24-18-00-r3-review-model.md`)

**CURRENT = 1841-r3-repel** (WR+2m 3258 h vs 1507's 2671, +587 [231, 958]; WR 9120 vs 7608, seed 8106 R 3.2M). The rise
is a MODEL change: `early_bag` (no mart on the walked route before Pewter, so the fights up to Brock have only
picked-up Potions; the old model bought them from the purse, +79 382 h without pickups), answered by picking up all
three early Potions (+975 h net) and the Brock Torrent line (**-413 h [-582, -258]**, forest-brock 0.397 in the
runner's CI). Refuted / n.s.: `min_spa` 30 (+404), 5 Pewter balls (+453), TM39 sale (n.s.), one-letter rival (n.s.,
~1.3 s), Mt. Moon heal (n.s., leaning worse), `torrent_heal_below` 0 and Route 3 per-fight thresholds (n.s.). The
Route 4 Magikarp is already in the model (dropping it +1415 h).

**Plan keys the controller (R2) follows, from 1841** (doc: `plans/1800-r3-keys.md`, `params.rs`):

| key | value | meaning |
| --- | --- | --- |
| `rival_name_len` | 0 | preset name (one letter n.s.; if used, rebuild the starter-naming table) |
| `early_potions` | `bedroom_pc`, `route1_clerk`, `forest_hidden` | pick up all three (forest one after Sammy) |
| `early_bag` | true | MODEL: no Potions before Pewter except those |
| `viridian_potions` | 0 | no Viridian mart stop |
| `tm39_sell` | false | keep TM39 (sale n.s.) |
| `pewter_balls` | 0 | no balls at Pewter |
| `route4_magikarp` | true | buy Magikarp at the Route 4 Center on the first pass |
| `mt_moon_heal` | false | no Center heal before Mt. Moon |
| `early_torrent` | `{fight: brock, steer_poison, no_stall, cure: true, withdraw_first: false, tackle_only: [pewter-gym-trainer]}` | no heal before/in the gym; a poisoned carry walked down to floor(max/3) (not below Onix's max roll after Withdraw + 1), cured with the forest Antidote (28,57); Brock without stall turns; Liam with Tackle |
| `repel_on` | without `route3` | Repels only where the path crosses encounter tiles |
| `manip.min_spa` | 29 | unchanged |
| `torrent_heal_below` | 0.5 | unchanged |

Next: tier-1 timings of the pickups and the poison walk (guesses), a SENS on `early_potion_frames`, compare the
lane's Brock pass (0.73) with played Brock attempts once R2 exists.

## 2026-11-24 review round 1 (read this first)

The human review of the v4 video (`video-analysis/1122-lgdescent-human-review.md`, read-only, items 1-20 up to
Mt. Moon) is now the work queue: `PROMPT.md` *Review round 1* maps every item to a step (R1 walker, R2 early
controller stages from power-on, R3 model/plan evaluations, R4 the v5 run and video) and adds standing rules:
**zero bumps, no unexplained detours, skipped trainers never fought, the early game played by the current
controller** (the lost Nov-21 prefix is retired). Items 8 (forest Antidote) and 20 (Persim) need no work; item 2
(text speed) is a re-check only. The ranked optimiser queue below still holds after R3.

## 2026-11-23 09:30 session end (main session; read this first)

Branch `human-run-play` (cut from `human-run` 2026-11-22). The session ran the PROMPT "Next phase" S0-S9 and
then the optimisation loop with parallel agents; all agent branches are merged.

- **Played run of record:** `$FRLG_ARTIFACTS/runs/human-play/1122-lgdescent-s11-v4`, plan 1122-lgdescent, LG,
  seed 11: second-HoF black frame at **20 472 s (5:41:11)**, `play-verify` from boot matches 97 checkpoints;
  14 rollbacks from Giovanni 1. Prefix to Giovanni 1 is the surviving Nov-21 log (`1122-lgdescent-s11`).
  Journals: `2026-11-23-04-00-controller-v4.md` (newest), v3, finish, postgame, controller-2, battle-bridge.
- **Review video:** `$FRLG_ARTIFACTS/videos/1122-lgdescent-s11-v4-full.mp4` (the earlier 6:10:47 run is
  `1122-lgdescent-s11-full.mp4`, journal `2026-11-23-03-05-full-video.md`). Render: `human-run/VIDEO.md`.
- **Model of record / CURRENT: 1507-honest4**, WR+2m ~2800 h, WR ~7500-8000 h (R 3.2M). Much worse than the
  pre-session 1122 (~85 h) because of model-honesty work (emulator- and VOD-measured fights, non-fight costs,
  shop stops, level evolutions, League at the WR level); every plan step adopted this session lowered it.
  The WR fight-speed factor's CI alone moves the score x0.45..x4.7.
- **Controller next:** spend the ~¥15 800 left at Indigo, the round-2 sell soft-lock, Surf before Koga is now in
  (honesty 4), weight the post-HoF fill by catch chance (v4 journal).
- **Optimiser next:** League refit on the honest carry (then retest Lance +X special, fragile across
  league_power), WR and WR+2m reset-table refits on 1507, `--set earlymid --notkinds fights`, `--set dex`.
- **Process:** rebuild `frlg-human` after every merge (a stale binary silently ignored new keys; those rows are
  INVALID in scores.jsonl); never poll with `pgrep -f` (matches itself); the API spend limit stopped all agents
  three times, so agents must commit every step.


## 2026-11-23 optimiser B: dex fill (journal `journal/2026-11-23-07-00-optimisation-b.md`)

Base 1700-opb-base == CURRENT 1507-honest4 (same JSON). `descend.py --set dexfill` (new `tools/dexmartknobs.py`):
**no confirmed improvement.** Ball stops before the pre-League fill are worse (Indigo Great 20 +514 h, 30 +1740;
Viridian Poke +277/+559; Sabrina Great +5/+10 +171/+331): the money matters more at the Indigo heal purchase than the
fill's empty bag. Fill steps / swaps / drops / 26 added sites, Safari stop / steps / area order under the honest
step counter, candy_early_n, Power Plant skip: all refuted or n.s. 1701-fillsteps14 is a -0.03 h artefact (pointer
`CURRENT-OPB` only; do not adopt). Next: `--set earlymid --notkinds fights` (not started), the rest of the dexfill
subset (hunt/ball/passoff/pewter), drop Diglett's Cave from the fill at R 6.4M (-43 [-105, +0.4]). Use
`FRLG_MINGAIN=0.003` in descents to stop near-zero promotions.

## 2026-11-23 optimisation on the honest-3 model: CURRENT = 1504-reset1612 (journal `journal/2026-11-23-02-00-optimisation.md`)
## 2026-11-23 optimisation (op5, wrap-up 09:15): CURRENT = 1507-honest4 (journal `journal/2026-11-23-02-00-optimisation.md`)

| plan | what | WR+2m (R 3.2M unless noted) | WR |
| --- | --- | --- | --- |
| 1316-shops16 | pre-honesty-3 CURRENT | ~816-858 | ~2400 |
| 1612-honest3-cap1 | 1316 + honesty-3 keys + 1 candy per evolution | 3237 / 3296 (R 9.6M) | ~8500-10000 |
| 1504-reset1612 | 1612 + `reset_aggr_at` all -0.2 + WR+2m `reset_table` refit (optstop train 26M) | 3171 / 3146 (R 9.6M; -66 [-125, -9] / -150 [-226, -83] vs 1612) | = 1612 (WR table not refit) |
| **1507-honest4 (CURRENT)** | **1504 + honesty-4 keys (2 Antidotes at Lavender, ball reserve 8, Safari in-zone steps)** | **2790 / 2803 (-812 [-1173, -478] / -437 [-760, -131] vs 1504; seeds 7951/7952)** | **7459 / 7974 (-1500 / -884)** |

1507 corners vs 1504 (R 3.2M): lp1.0 -633 n.s., lp1.4 -544*, mp1.0 -2167*, mp1.3 -579*, door .10 -975*. Spread across
later seeds: WR+2m ~2550-3000 h at R 3.2M (seed scatter about +-8%).

* **Not adopted (fragile): Lance +X Special on 1507** (the 1302 / 1506 step): main -381 / -282 / -260 h (WR -1141 /
  -714), mp / door corners better, but league_power corners lp1.0 +954*, lp1.1 -8, lp1.3 +265*, lp1.4 +327*: the gain
  exists only at the fit's point estimate lp 1.2 and is zero or negative across its interval [1.1, 1.3]. Re-test after
  a League refit on the honest carry. Lavender Great 15 -> 20: -0.77 / -0.74 h (significant, 0.03%; not adopted, nil).
* Refuted on the honest model (1504 / 1507): Lorelei +X Special (+340 / +321); no Mr. Mime trade (+576 / +666 on 1507);
  a mart stop before the dex fill (after giovanni-badge or victory-road, Great 10/20/30, Ultra 10: 0 .. +3463 h: the
  money is worth more than the balls the fill runs out of); shops pass 2 (only Lavender 20, nil); League descent on
  1504/1507 (all lorelei..champion, round-2, xbuys, setup_min_turns, preheal, revives, ~110 of 171 tail candidates
  before the wrap-up stop): nothing else passed; extra X purchases +370..+650, revives +1440, setup_min_turns +13700.
  That is about 9 refuted ideas in a row after 1507 apart from the fragile Lance step: the knob space around the
  honest plan is close to converged; the next gains are model / structure items below.
* **Always rebuild `frlg-human` after merging**: an old binary silently ignores new plan keys (8 rows marked INVALID).
* **Evidence hygiene (controller v4):** bot League / boss death counts before v4 understate survival (bag reader bug;
  Lance 0/30 -> 7/12): discount the League fit's "controller upper bound", Champion-2 0/28, Lance 8 / Champion 7 deaths.
  No refit or plan decision here leans on them.
* Fight-price uncertainty: the WR speed factor's 95% CI (`trainer_cost` x0.904 .. x0.981) spans about x0.45 .. x4.7 on
  the score (measured on 1309); not re-measured on 1507.
* Next, ranked: (1) League refit on the honest carry (Lv 52.4, `tools/league_fit_grid.py`), then re-test Lance +X
  Special across the refit interval; (2) WR `reset_table` refit (~80M training at the WR finish rate) and a WR+2m
  refit on 1507 (the Antidotes changed the finish rate); (3) `--set dex` (candy economy); (4) the League descent tail
  from `lance: last x accuracy->calm` on (`--skip ~95` on 1507's 171-candidate list) and the 45 per-fight heal_below
  candidates; (5) earlymid without fight knobs (`--set earlymid --notkinds fights`).

## 2026-11-23 06:40 model honesty 4 (read first; journal `journal/2026-11-23-06-40-model-honesty-4.md`)

The controller-v4 bugs, measured with v4 code only (pre-battle states cut from the v4 `run.ilog`, `examples/
cut_battle.rs`, `battle-test --plan 1612`). **Koga is a controller bug**: v4 fought Muk with Water Gun (HM03 taught
after the badge), the model with Surf (taught at `safari-walk`). Water Gun: emulator 11/20, engine at the RAM stats
0.58; Surf: emulator 21/24, engine 0.89, model lane 0.85 -- engine and emulator agree, no engine change; Toxic
frequency n.s. Controller now teaches Surf before Koga. **Lance round 1 is not a bug**: 22/24 from the v4 `agatha`
state vs the model's 0.887 per try. Ball reserve (`pass_reserve` 8, MODEL, 1620) -42 h n.s.; Safari in-zone walk
steps (`safari_walk_steps`, MODEL, 1621) exactly 0 at 1612's budgets (caps future descents); Safari drain (1622) and
throw cap (1627) n.s. / 0. New poison cure (`Action::Cure`, `full_heals`, sim + controller): emulator Koga with Surf
24/24 with one Full Heal. **PLAN 1626-koga-antidote2 (2 Antidotes in the Lavender ball stop): WR+2m -783 h
[-1028, -552] vs 1612 (546 vs 417 finishes, seed 7602 R 6.4M); recommended as a plan step** (a Fuchsia Full Heal
mart visit, 1623, is n.s.; a 3rd Antidote adds nothing). The 150-frame purchase cost is a guess. Other finding:
`dex-fill` runs the ball bag dry (866 empties / 300 attempts): add a mart stop before it in the next descent.

## 2026-11-23 03:50 model honesty 3 (read first; journal `journal/2026-11-23-03-50-model-honesty-3.md`)

The controller-v3 / League-2 model bugs, default-off keys (1316 unchanged), paired at WR+2m. **Level evolutions of
mons that never fight** (`level_evo_honest`: Rare Candies, `max(1, level - catch level)`; `Cmd_getexp` pays sent-in
mons only, `battle_script_commands.c:3148-3175`): +1663 h on 1316; the carry reaches E4R1 at Lv 52.4 instead of 55.3
(the WR's ~51), E4R1 clock +396 s; plan response cap 1 candy per evolution (`level_evo_max_candies` 1) -395 h, drop all
+4160. **Risk lane inherited the timing lane's free Torrent entry potions** (`risk_entry_paid`; the model had no Koga
heal at all, Koga's lane always started full): +234 h; Fuchsia Center before Koga +54 n.s., Center after the badge +2170
(refuted: it costs the Torrent window). **League heals act as Full Restores but cost 2500** (`league_heal_price` 3000):
+27 h. Safari East is *not* a duplicate (it is on the walk, `frlg-mapgraph`, and first in plan order; West-only +260).
Champion-2: AI heals and sandstorm faithful; the played deaths are bot state (`mewtwo_power` 1.0 corner +832 h,
Shock Wave, fewer FRs), not a model bug. **1612-honest3-cap1 (1316 + the three keys + cap 1): WR+2m 3025 h vs 816
(+2210 [1819, 2900], seed 7602 R 1.6M), WR 8315 vs 2396: recommended as the honest model of record** (main session
decides CURRENT). Before descending from it retune resets: `reset_aggr_at` brock 1.0 resets every 1612 attempt at
WR+10m (no `reset_table` there; its mean remaining is ~200 s longer). Next: reset retune, `--set dex`/`shops` descent
from 1612, League refit on 1612 (the carry is now at the WR's level by mechanics).

## 2026-11-22 23:00 canonical run finished (read first; journal `journal/2026-11-22-23-00-finish.md`)

**`runs/human-play/pg-canon5`: one pure-input run from boot to the first full black frame of the second Hall of
Fame, 22 247.4 s (6:10:47)**, model p50 12 279 s (+9 968 s). Dex 51 -> 60 by `play/dexfin.rs` (stones, Rare Candy,
trades, Magikarp, fossil: 505 s, 0 rollbacks), National Dex 18 357 s. League 2 made winnable by post-game pickups
(Meadow PP Up / Max Potion, Cerulean Cave + Route 23 Max Elixirs, warehouse pearls), Shock Wave over Fire Blast,
selling valuables, 2 Revives + 12 Full Restores, a 4-Full-Restore reserve for the Champion and a Blastoise prep before
it. Rollbacks after the fill: Lance 2 x2, Champion 2 x4. Biggest losses vs the model: the post-HoF dex fill (~5 400 s,
and it spent the League money) and the post-game lumps (`calibration.md`, last section). Next: move the fill upstream
(route agent), calibrate the post-game lumps, a stronger second League fighter.

## 2026-11-22 22:05 non-fight windows (read first; journal `journal/2026-11-22-22-05-nonfight-windows.md`)

The whole WR VOD classified per frame (`video-analysis/data/wr-battle-chains.jsonl`, `tools/nonfight/`), each WR window
split into trainer fights / wild + evolution battles / field and set against 1423's segments. 18 guessed lumps were
wrong and are now VOD-timed in the default-off key **`vod_frames`** (gym walks: Misty 3.5 -> 18 s, Viridian 5.6 -> 21.5;
walk-outs after Koga / Blaine; S.S. Anne captain + departure 25.6 -> 56 s; Vermilion -> Cerulean walk 35.5 -> 54;
Blaine's quiz gym 46 -> 71; Sevii Bill / Celio / return scenes 136 -> 107; Mt. Moon floors + the fossil text) plus
**`evo_stone_frames` 1600 / `evo_candy_frames` 1620** (VOD evolution scenes, 18.7 -> 26.8 s). A leader's / rival's
post-battle speech is inside the fitted fight price (played episodes), so the badge lumps keep only the walk-out.
**MODEL CHANGE 1444-nonfight-vod: 1704 h, +1249 [963, 1595] vs 1423** (halves: scenes 1445 +649, evolutions 1442
+162). With the constants honest, the model is +125 s over the WR at Giovanni 3 (no-reset): the predecessor's
"non-fight 150 s under" was constants + plan structure; what remains is plan: Mt. Moon without repels (the model
flees ~110 s on B2F), Safari East+West, Hitmonlee, Warden + Nido evolutions placed after Koga instead of after Erika,
Mr. Mime trade + Diglett hunt in the Giovanni 3 window. **PLAN `repel_on` + `mt-moon-b1f` / `mt-moon-b2f`: -963 h
[-1280, -686] on 1444 (1446, 741 h), -162 h [-199, -128] on 1423 (1447, 293 h).** For the combined model of record:
add the three keys from `plans/1444-nonfight-vod.md` as one block (no fight / League key touched) and re-score the
Mt. Moon repels on it (money interacts with 1413's shop stops).

## 2026-11-22 21:35 boss turns (read first; journal `journal/2026-11-22-21-35-boss-turns.md`)

The sim's short boss fights come from `carry_power` 2.0: the timing lane one-shots every rival / leader mon
(Cerulean rival 4 actions for 4 mons, the WR 7), so X items never trigger. Fixed without touching survival:
**`turn_power` 1.4 / `turn_power_boss` 1.0** (the clock pays the actions of a copy of each non-League trainer fight at
that power; outcome stays at `carry_power`; a FIT on the WR's 67 fights: actions 239.7 vs 237, bosses 0.98 [0.88,
1.08] of the WR, 1285: 0.74). Level-ups out of the fit: `trainer_cost` [798, 536, 251] x the recomputed WR factor
0.938 [0.904, 0.981] = **[749, 503, 235]**. A third double count found and removed: **`tcost_whole`** (no jitter,
reactions or risk-lane potion turns on top of the WR-fitted price; 126 s on the list). **MODEL CHANGE
1423-boss-turns-whole: WR+2m 455.5 h, +179 [138, 234] vs 1285** (seed 7401, R 800k); halves: 1420 (level-ups only)
161 h, 1421 (turns only) 4851 h, 1422 (both, extras charged) 864 h; SENS WR factor 0.904 / 0.981: 234 h / 2097 h,
`turn_power_boss` 1.1: 344 h. Core fight price on the WR list 3693 s vs 3684 s; no-reset clock at Giovanni 3 -43 s
(1285: -151 s). **Recommended as model of record** (does not pass the +-60 s window check: 8 windows out vs 1285's 4,
the early-fight overshoot at 1.4 and Silph Giovanni at boss power). The League agent's refit should be recombined with
1423 (no League keys touched). Non-fight time still ~150 s under the WR to Giovanni 3 (Koga -117: Safari plan;
Giovanni 3 -109: gym spin-tile walk 6 s vs the WR's ~35 s, badge text, evolutions/trade/Diglett placed differently;
Misty -93; Bike -71; Sevii -65; vs Blaine +132), offset by ~84 s of pre-fight bag trips the WR does not make. Next:
time the lab rival and Silph Giovanni on the VOD; per-fight look at the earliest fights and the tower rival (why 1.4
vs 1.0); the Viridian Gym walk and badge-text guesses; the pre-fight Elixir / pre-heal policy.

## 2026-11-22 21:00 WR fight timing (read first; journal `journal/2026-11-22-21-00-wr-fight-timing.md`)

67 WR trainer fights (Sammy -> Route 22 rival) frame-timed off the VOD with a colour-cell battle detector
(`video-analysis/tools/battle-timeline.py`, validated against RAM on the played render), 47 of them paired with the
same fight in the played run. Per component: overhead and per-KO equal (pre-battle speech identical to the frame),
the WR is faster in battle text and slower in menus; **at the WR's own decision counts the WR = 0.944 [0.910, 0.987]
x `trainer_cost`**. The x0.9 of 1283 (two fights) is not supported. Two model errors partly cancel: the 1280 sim plays
rival/leader fights in fewer decisions than the WR (202 vs 237 over the 67 fights: X items, switches, extra hits) and
counts level-up text twice (inside the fitted `trainer_cost` and again as `level_up`); with both, x0.944 makes the
model's total fight time equal the WR's (3696 vs 3684 s). **MODEL CHANGE 1285-wr-fight-speed** (`trainer_cost`
[791, 547, 211], `plans/1285-wr-fight-speed.md`): WR+2m **276.2 h, -777 [-945, -654] vs 1280** (seed 7401, R 800k);
the factor's CI ends score 691 h (x0.987, 1286) and 167 h (x0.910, 1287). **Recommended as model of record.** With
fights at the WR's price the no-reset clock runs ~2.5 min ahead of the WR by Giovanni 3 (-151 s; 1280: +38): the
non-fight time is now the optimistic part. Next: (1) fix the sim's boss/rival decision count and the level-up double
count together (refit `trainer_cost` with a level-up term: [798, 536, 251] at 240 f/level); (2) find the ~150 s of
non-fight optimism before Giovanni 3; (3) League fight timing from the VOD with the same tool (League agent).
## 2026-11-22 model honesty 2 (read first; journal `journal/2026-11-22-21-30-model-honesty-2.md`)

The controller-2 model bugs, each a default-off MODEL CHANGE key (1280 scores unchanged) plus a plan response, paired at
WR+2m seed 7401 R 800k. Strength (`strength_carrier`), stones (`stone_shops`), TM13 (`Teach::prize`, dead entry dropped):
0.0 h each -- the sim's carrier rides a PC visit the plan already pays (trade at `stones` / `fly-viridian`), the
`stones` segment uses the Safari Leaf / Power Plant Thunder pickups, TM13 (80000) was never affordable. Balls (`Mart::topup`:
with any stop, no on-demand buying in caves): best 3 stops **Lavender 15 / Cinnabar 30 / Saffron 35 Great** (1409, +340 h).
Heals (`potion_bag_from` koga-badge, `Mart::potions`; centers stay skipped by the Torrent phase): **Cinnabar 5 / Saffron 4
before Silph / Saffron 3 after Sabrina** (1413, +252 h vs 1409). PP: Elixirs are spent, 2 Max Elixirs are held for the
League; PP Ups on the carry (`carry_pp_ups`) wreck round 2 (Mewtwo's Psychic): refuted. **1413-potion-bag-small vs 1280:
+592 h [416, 876]** (109 vs 171 finishes): recommended as the honest model of record; main session decides CURRENT.
**League agent:** nothing in the League mechanics changed (heal items, preheal, Max Elixir reserve, PP rules); the bag is
risk-lane only. But the stops spend money earlier, so money at the Indigo heal purchase falls and Lance/Champion reach
with it -- refit League stand-ins on 1413's money, not 1280's. Controller: withdraw the Strength carrier at the trade's
PC visit; buy balls/potions at the listed stops only.

## 2026-11-22 19:40 main session: CURRENT = 1280-wr-faithful (model of record)

1280 replaces 1301 as the model of record: the WR's own trainer list and Rare Candies (VOD-read), the emulator
`trainer_cost`, windows refit; the carry matches the WR's level at every split. WR+2m ~1053 h (vs 1270's 371 h, same
policy) -- the honest answer under this model. The two biggest open model uncertainties now dominate every plan knob:
(1) per-fight speed of a WR-level human vs the bot's played fights (0.9 factor = 1283, 146 h; grounded only by two
timed WR fights -> time 10-20 more WR route fights from the VOD); (2) the League fit relies on Rare Candies the WR
never eats (WR enters at Lv50; holding them back is +5609 h) -> refit the League stand-ins at the WR's League level.
1301's confirmed tactics (Miguel potions 5, Koga heal_below .55, League heal bundle) should be re-applied on 1280 once
(2) is done.

## 2026-11-22 WR-faithful model (read first; journal `journal/2026-11-22-19-30-wr-faithful.md`)

Every trainer the WR fights from Route 3 to the League read off the VOD (10 s / 2-7 s contact sheets): vs 1270 the WR fights
no Cerulean gym trainer, Greg (not Sally) on Route 3, Robby (not Miriam) in Mt. Moon, Route 25 x4, the Cerulean Rocket after
Bill, Route 6 Jeff only, Route 9/RT x10, Route 8 Rich only, hideout grunts 7/18/16/17 (not 13), no Aroma Lady Nikki, Celadon
Tamia + Lori; everything else matches. Its candies: two eaten in Cerulean (Mt. Moon + hidden Cerulean City), three in the
hideout before Giovanni (Route 9, Route 6 hidden, B3F). **1280-wr-faithful** = 1270 + that list + those candies +
`trainer_cost` (no `trainer_fit` 0.6) + windows refit: the carry has the WR's level at every split Brock -> Giovanni 3, and
scores **WR+2m 1053 h, +682 [547, 868] vs 1270** (seed 7401, R 800k). **Recommended as model of record** (honest, worse).
No candidate passes the +-60 s window check (1280: 7 windows out, total +269 s; on the WR's structure 1281: 4 out, +319 s;
the bot's fights are ~7 % slower than the WR's: x0.9 on `trainer_cost` = 1283, -225 h vs 1270, a FIT on two WR fights).
Safari / Power Plant per-species cost matches the WR (~51 s); the Koga gap is the plan (East+West, stop 5 vs the WR's
North+West, 7 species). League finding (1284): at the WR's League level (no candy dump at Indigo, Lv51 vs 55) finishes fall
171 -> 27: the League fit leans on held candies the WR does not eat. Ranked next: League refit at WR levels; time more WR
route fights for the skill factor; Safari North alone as a plan; Sevii/fill dex split.

## 2026-11-22 route windows (read first; journal `journal/2026-11-22-17-30-route-windows.md`)

Structural causes of 1253's window errors, read off the WR VOD with contact sheets (the 60 s scene log is too coarse; it misled
twice): Warren fought in the Viridian Gym (MODEL CHANGE `viridian_warren`, 1270: +81 h [57, 109] vs 1253); the runner splits
"Erika" at the Warden's door (split mapping fixed, reporting only); the WR gives the Gold Teeth after Erika, fights the
Cerulean Rocket after Bill, catches on Route 3/4, skips Hitmonlee (plan 1271 with all of them: +3443 h, refuted as a plan);
Snorlax split checked (on the Cycling Road: the model was right). **trainer_fit 0.6 conflict resolved**: the played 79 fights
fit `a 838 + 579/action + 223/KO` (MODEL CHANGE `trainer_cost`): one-shot fights are 0.85 of `calib_wr`, multi-turn 1.12, so
the formula's 630/KO overprices route trainers; and 1253's list fights 7 vs the WR's 4 trainers on Route 25 and 16 vs 10 on
Route 9 / Rock Tunnel (video). With the WR's lists Bill/Rock Tunnel match at `trainer_cost` or 0.75, but the carry is then 2
levels short (the WR's exp comes later: 5-6 hideout grunts + two Rare Candies before Giovanni 1), deaths rise (1272 +559 h).
**Recommendation: keep 1253 as model of record for now; adopt `viridian_warren` (1270) as a model change (it is evidence,
not a choice), and replace `trainer_fit` 0.6 only together with video-derived fight lists for Giovanni 1 -> Koga and a risk-
window refit** (next step 1). No candidate passes the +-60 s per-window check (1275: 5 windows out, total +456 s).

Next, ranked: (1) fight lists Giovanni 1 -> Koga from the VOD (Route 8: Rich only; hideout door grunt + 5; two candies),
then `trainer_cost`, refit risk windows, re-score; (2) Safari/Power Plant hunting time (WR ~30 s per Safari species, model
~60-80 s at the margin with matching per-event costs: the search/unwanted-encounter part); (3) dex re-plan toward the WR's
early sites (Route 3/4 done in 1271; costs exp-free time in this model).

## 2026-11-22 16:55 main session: CURRENT = 1253-runner-fights-tf (model of record)

Switched from 1132 because 1253 is the more honest model: the carry fights the trainers the runner
and the played run fight, and matches the WR's levels at every split, and the risk windows refit to
~1.0 (they had been standing in for missing exp). Its score (WR+2m 301 h) is worse, and that is the
honest answer. One open conflict: `trainer_fit` 0.6 is a FIT to the WR splits, while the emulator
measured 52 trainer battles at 0.95-1.02x the `calib_wr` formula (calibration.md). Resolve it in the
route-windows work (more battles in the model than the runner actually fights there? or shorter
battles from a higher level?) before trusting 0.6. All descents from now on start from 1253.

## 2026-11-22 segment-cost calibration: model of record stays 1132 (read first)

Journal `journal/2026-11-22-14-00-calibration.md`, `calibration.md` ("Segment-cost calibration" and
"Reconciliation with the WR windows"). Measured per-event costs from the played run are in as MODEL CHANGE keys
(`catch_end_new`, `throw_frames`, `flee_frames`, `safari_run_frames`, `indoor_speed`, `fly_frames`, all off by
default). With all of them, 1245 overshoots the WR's windows by +669 s (1132: +178 s). No WR-fitted constant
absorbs that cleanly: `trainer_fit` is pinned by the fight-only windows (Bill, Rock Tunnel), `text_factor` /
`path_overhead` would go below measured floors, and a FIT on the guessed pre-League text lumps (`guess_text_scale`)
either fixes the windows 1132 already matched (0.71, plan **1246**: +323 h, total still +451 s) or the pre-League
total (0.42, plan **1247**: -2.1 h [-12.9, 7.6], but Sevii -97 s, Bike -178 s, and 0.42 contradicts the played
text lumps at ~0.9). Neither passes the per-window WR check, so **CURRENT stays 1132-standin-fheal**.

| plan | what | WR+2m vs 1132 (seed 7401, R 800k) | no-reset mean | window total vs WR |
| --- | --- | --- | --- | --- |
| 1132-standin-fheal (CURRENT) | | 173.9 h | 3:28:22 | +178 s |
| 1245-calibrated-model | measured costs | +3461 h [2687, 5005] | 3:35:45 | +669 s |
| 1246-reconciled | 1245 + guess text 0.71 | +323 h [282, 378] | 3:32:07 | +451 s |
| 1247-reconciled-total | 1245 + guess text 0.42 | -2.1 h [-12.9, 7.6] | 3:28:28 | ~+0 pre-League |

Ranked next steps: (1) the structural window errors that 1132 already has and that dominate the check (Start E4R1
+175..+292 s from the back-loaded dex fill, Giovanni 3 -206 s, Bike -144 s, Blaine +115 s, Snorlax +65 s) are
route/plan differences against the WR, not constants: fix those first, then re-run the reconciliation; (2) measure
more guessed text lumps on the played run (`tools/cal_decompose.py` per stage) instead of one global scale;
(3) the controller agent's inefficiency list (journal) before the next played pass.
## 2026-11-22 exp economy and Koga (read first; journal `journal/2026-11-22-16-50-exp-and-koga.md`)

The sim's carry was 4-5 levels short because it fought only the sight-cone minimum; the played s11 run and the runner fight
~20-36 more trainers (exp formula verified exact against the played run). **1253-runner-fights-tf** = 1132 + MODEL CHANGE:
`fight_lists` (runner-level list: the sim now matches the WR's level at every split), `risk_windows` refit
(**2.565 -> 1.04, 1.35 -> 0.99**, .773 -> .674, 1.572 -> 1.06, 2.314 -> 2.17; first window kept 1.111), `trainer_fit`
0.81 -> 0.6 (pre-door splits back to the WR's). **WR+2m 301.0 h / WR 477.4 h** vs 1132's 182.3 / 287.9 under the old model
(seed 7402 / 7401, R 1.6M). Under 1253 dropping the extra fights costs +132 h (1254); the full s11 list has no WR+2m finishes
(1255). Koga: the audit's 35% was a CLI mismatch (setup-min 1); engine 60.5% at the plan's policy vs emulator 73/100; the
residual is Muk's Toxic frequency (open). `ai_cure_items` (Full Heal is no HP heal) is correct and nil.

**Coordination**: everything is behind default-off keys; 1132 scores unchanged. CURRENT not switched (League descents run on
1132). Proposed: descents move to 1253's keys (`fight_lists`, `risk_windows`, `trainer_fit` 0.6); a League refit of
`risk_windows` should start from 1253, not 1132. Open, ranked: optional-trainer descent per segment under 1253; measure
route-trainer fight time on the emulator (replaces the 0.6 fit); first window not identifiable; Koga Toxic AI term.

## 2026-11-22 stand-in audit: descend from 1211 (read first)

Journal `journal/2026-11-22-14-30-standin-audit.md` (per-fight table: sim rate, engine at real stats, emulator).
**1210-standin-audit** (MODEL CHANGE: `persim_held`, Misty and Cerulean rival `risk_power` 1.0) WR+2m 210.6 h /
WR 335.4 h; **1211-cerulean-potions** (plan response: 3 potions, heal below .35 at the Cerulean rival)
**199.4 h / 323.5 h** (seed 7402 / 7401, R 1.6M). Descents start from 1211. Open, ranked:

1. Exp economy: the sim's carry is 4-5 levels behind the played s11 run on the same route (L32/33/37 vs 36/38/42 at
   Giovanni 1 / tower rival / Koga). The 2.565 window stands in for it (emulator 40/40 at s11 levels).
2. Koga's Muk: engine 35% vs emulator 15/20 at identical stats; the 1.35 window stands in for an engine bug.
3. The 0.773 Blaine/Sevii window is deadlier than the honest engine (-21 h at 1.0); refit windows under 1211.

## 2026-11-22 optimisation / combined honest model (journal `journal/2026-11-22-12-00-optimisation.md`): CURRENT = 1316-shops16

**1316 = 1312 + shop-stop bundle; 1312 = 1311 without Hitmonlee; 1311 = 1423 fight clock + 1444 non-fight VOD block + 1413 honest shop stops + the WR-level League refit + candies eaten
at Indigo + tactics + Mt. Moon B1F/B2F Repels.** It is the honest model of record; it is worse than the partial models
because each block made the model more honest, not because a plan step lost.

| plan | what | WR+2m (R 3.2M) | WR |
| --- | --- | --- | --- |
| 1304-e4level-lgfit | 1280 + League at the WR level (candies held) + refit lp 1.2 / mp 1.15 / fatal | 1785 (vs 1280 999) | 5263 |
| 1309-honest-e4fit-candies | 1413 stops + 1423 clock + refit + candies eaten + Miguel 5 / Koga .55 | 715 / 741 (vs 1423 450 / 472) | 1752 / 1716 |
| 1310-honest-vod | 1309 + 1444 VOD block | 3606 / 3337 (vs 1444 2040 / 1795) | 15104 / 13200 |
| 1311-honest-mtmoon-repel | 1310 + Mt. Moon Repels | 1326 / 1269 (seeds 7781/7782; -2280 / -2068 vs 1310) | 4171 / 4015 |
| 1312-honest-nolee | 1311 without the Hitmonlee gift (WR choice) | 1130 / 1104 (seeds 7791/7792; -184 / -154 vs 1311) | 3222 / 3039 |
| **1316-shops16 (CURRENT)** | **1312 + Route 21 Great Balls 30->20, potions 5->7, Saffron Great Balls 35->25** | **862 / 853 (seeds 7801/7802; -325 / -316 vs 1312)** | **2347 / 2513 (-1073 / -924)** |

* **League refit at the WR's League level** (`tools/league_fit_grid.py`, data `data/league-fit-wr-level-2026-11-22.jsonl`):
  at Lv55 (candies eaten) the grid reproduces the old fit (lp 1.0, mp 1.3); at the WR's Lv51 **lp 1.2 [~1.1, 1.3], mp 1.15
  [1.0, 1.3]**, unchanged under 1413's money. Post-door `fatal` refit under 1413 money: engine-only Lance .253, Champion
  .362; fatal Bruno .172, Lance .097; door-to-finish total .254 (posterior median .239 [.08, .48]).
* **Eating the held candies at Indigo is a large plan gain under the WR-level fit** (1309 vs 1308: -606 / -651 h WR+2m,
  -1363 / -1639 h WR; on 1305 -592 / -668). The WR keeps two candies; in this model that costs him League survival.
* **Fight-price sensitivity** (the WR factor's 95% interval, `trainer_cost` 0.904 / 0.981, journal 21-35-boss-turns): on
  1309 (seed 7771, R 1.6M) WR+2m 317 / **718** / 3396 h, WR 547 / **1666** / 13613 h. Report 1311's score with that
  spread: ~1300 h at WR+2m with a roughly x0.45 .. x4.7 range from the fight price alone.
* Attribution on 1309: 1423 + 1413 stops alone is 764 h (WR+2m) vs 1309's 718: the stops cost ~+300 h, the League refit
  + candies + tactics recover ~50 h net.
* Confirmed tactics carried through every model: League heal_below bundle (in 1253+), Mt. Moon Miguel potions 5, Koga
  heal_below .55. Earlier: all optional-trainer drops refuted on 1300 (exp outweighs fight time), reset_table refit
  not adopted (and now below the 2000-finish noise floor: ~550 finishes per 3.2M at WR+2m).
* New descent sets: `--set fights` (fightknobs.py), `--set shops` (shopknobs.py: mart top-up targets, potion bag,
  league_heals). WR structural choices on 1311 (screen R 1.6M / 3.2M): **no Hitmonlee accepted and confirmed (1312)**; refuted:
  Warden after Erika +353 h, Warden + evolutions after it +32 (n.s.), Routes 3/4 catches +3125, Safari North+West
  +744, no Diglett hunt +289, no Mr. Mime -72 [-146, +7] (n.s.; retest on 1312), no Mime + no Diglett +319, all
  together +3834. Post-game bugs: no MODEL CHANGE made (the model already matches the WR's post-game windows within ~1
  min; adding the controller's slower stretch times would double count; the Mewtwo PP items are in the plan and
  reachable, the controller does not collect them; journal round 6). `--set shops` from 1312: 4 promotions
  (1313-1316), bundle confirmed in every cell; refuted: Lavender Great Balls 5 (+166) / Ultra 5 (+2552) / drop (+7792),
  Route 21 potions 3/4 (+117 / +47), Silph potions 6 (+32), league_heals +-2 (0.000), rest n.s.
  **Next, ranked:** (1) `--set league` from 1316 (League knobs under the WR-level fit; on 1301 lance +x special and
  champion -x special screened -12 / -16 h but on the old fit); (2) second `--set shops` pass from 1316 (the optimum
  moved: Route 21 great 20 -> 10?, potions 8?); (3) no-Mr.-Mime retest (-72 n.s. on 1311); (4) `--set earlymid
  --notkinds fights`; (5) a reset_table refit needs >= 2000 finishes per rung: ~900 per 3.2M at WR+2m now, so train
  R >= 8M; (6) the window-neutral post-game restructure (route code, post-game agent).
* Pointer `plans/CURRENT-OPT` is the descent pointer (`descend.py --pointer CURRENT-OPT --num-from 1300`).


## 2026-11-22 model honesty: later descents must use the new model

Journal `journal/2026-11-22-13-00-model-honesty.md`. The battle bridge and the controller's played run
showed stand-ins and route steps the model got wrong. Fixed as labelled MODEL CHANGE keys (all default off
in `params.rs`, so every older plan scores as before; a plan opts in by setting them):

| plan | what | WR+2m | WR |
| --- | --- | --- | --- |
| 1122-lgdescent | old model (CURRENT file) | 83.3-86.2 | 112.6 |
| 1200-model-honesty | 1122 + S.S. Anne rival `risk_power` 1.0, `potion_tiers`, `sleep_no_potion`, `ball_shop_from`/`early_balls` | 134.1-139.1 | 184.8 |
| 1201-awakening | 1200 + 3 Awakenings (`awakenings`, `Action::Awaken`), S.S. Anne potions 9 / heal_below .35, `early_balls` 0 (plan response, -30.4 h) | 106.9-108.7 | 143.4 |
| **1202-controller-model** | 1201 + `fly_badge_gate`, `coin_case_frames`, `trash_cans`, `marowak_fight` (same policy) | **162.2-165.1** | **260.4** |

**1202 is the honest score of the best policy known; descents started from 1122 or earlier score the old,
optimistic model.** `plans/CURRENT` still names 1122 (another agent's descent runs on it): switch CURRENT to
1202 at the next merge and re-run descents from it. Open model questions, ranked:

1. `rival-cerulean` `risk_power` 3.5 is the same stand-in class as the S.S. Anne rival (sim engine 44% win
   at power 1.0 vs 100% at 3.5; what-if +48.9 h). Cut a battle-bridge state before the Cerulean rival at
   the plan party and measure 20+ seeds before changing it. Misty's 2.0 is harmless (0.94 either way; 14/14
   played).
2. The timing lane's pre-fight potions are free and unlimited outside the League (`preheal_below`, entry
   heals): why the controller blacked out at the tower and Koga while the sim's extra heals there are worth
   0.1-0.2 h. A finite bag in the timing lane (as in the risk lane) is the next honesty step.
3. In the League the fighter fainting is a loss in the sim, while Blastoise behind Mewtwo can finish a fight.
4. The plan's sim carry is L25 at the S.S. Anne rival; the played s11 run was L27.
5. Plan responses still open: Awakening count/where, Super Potions at Cerulean vs the WR's Potions, the
   S.S. Anne rival lead/X Defend, levels before the S.S. Anne (the `skip_groups` entries are ignored by
   the `wr` route style, so un-skipping does nothing).

Read `PROMPT.md`, `CLAUDE.md`, this file, then `ROBUSTNESS.md` sections 12-19 and the newest journals.
`plans/CURRENT` names the plan to beat. All EHT figures below are hours of expected human time to a
finish at or below the rung (WR+2m / WR), counting resets and the off-timer starter wait.

## State at the end of the session

| Plan | What it is | WR+2m | WR | Model |
| --- | --- | --- | --- | --- |
| 1040-optstop | start of the session's chain: optimal-stopping reset table | ~410 | ~569 | old (pre-PP) |
| 1054-calm-sp9 | Calm Mind for Mewtwo at Champion-2 etc. | ~141 | ~180 | PP-off (optimistic) |
| 1057-calm-sp9-ppup-redo | honest Mewtwo-PP model, 3 PP Ups | ~160 | ~221 | PP-honest |
| 1058-hidden-candy-calcium-nugget | hidden Rare Candies, Calciums, Nugget sale | ~118 | ~166 | PP-honest |
| 1067 / 1090 / 1092 | descent + geometry pickup frames, Silph Calcium dropped, Elixir skip | ~100 | ~135 | PP-honest |
| 1096 .. 1103 | Jolteon timing, ball rule, Mt Moon pass_off, fill orders | ~88 | ~115 | old League constants |
| 1107-evo10 | + Parasect level evolution | 84.5 | 108.8 | old League constants (R=400k) |
| 1113-lgmodel | 1107 under the fitted League constants (league_power 1.0, mewtwo_power 1.3, scarce_factor 1.0), refit post-door rows and reset table | 123.4 | 166.1 | evidence-fitted (MODEL CHANGE) |
| **1122-lgdescent (CURRENT)** | 1113 + League re-descent: Champion queue guard/speed/special/special (1115), Koga +1 X special (1105), Agatha -x speed (1114), preheal 0.5 (1116), 1096 reverted, 1097-1099 and 1107 kept | **84.3 (R400k) / 84.6-85.1 (R3.2M)** | **113.8 (R400k) / 111.0-112.2 (R3.2M)** | evidence-fitted |

`1122` vs `1113`: -37.8 h at WR+2m and -55 h at WR (paired R 3.2M, seeds 7401/7402). Corners (WR+2m,
R 800k): better by 36 h at mewtwo_power 1.15/1.45, 94 h at door survival 0.10, 205 h at carry_power 1.7,
but **6.7 h worse at the old league_power 1.4**. The Koga +1 X special (1105) is +8 h at the plan constants
but 42 h worse at carry_power 1.7 (CI [-83, +0.9]): a trade-off the next session may veto. 1120 (Lance first
X special -> calm) is screened only, not in 1122. The evidence-weighted interval for 1122 was NOT computed.

Figures at R=400k with the plan's constants; the evidence-weighted interval for 1067 was
median 137 h [68% 67-327] at WR+2m and 166 h [76-429] at WR with the door survival pinned to the
runner's data, or 88/107 h carrying the 1057 hazard residual (ROBUSTNESS 14, 18). Treat the spread, not
the point, as the answer: door-to-finish survival and pre-door survival dominate.

## Model changes made this session (all labelled in scores.jsonl / journals)

1. Mewtwo per-move PP tracked (`mewtwo_pp`), Max Elixir / PP Up / Ether pickups; honest default from 1055.
2. Pickup frames from map geometry (candy Route 9 170, Route 17 1030, Hideout Calcium 400, Silph Calcium
   1170, Nugget sale 300); hidden items vs `underfoot` Itemfinder gating added to the sim (`itemfinder`,
   `leftovers`, `ppmax` kinds). Nothing in the plan needs the Itemfinder.
3. Dex fix: multi-evolution species (Eevee, Gloom, Poliwhirl, Slowpoke) were dropped by `gen_dex.py`.
4. Optstop reset policy (hindsight-free table), common-seed fit and best-iterate guard (refit only
   with >= ~2000 finishes per rung; smaller trainings are noise).
5. League stand-ins refit against the WR VOD and the 2026-11-14 stream (calib data in
   `video-analysis/data/calib/`): league_power 1.0, mewtwo_power 1.3, carry_power <= 1.7 (weak),
   `scarce_factor` (Blizzard rationing) is a plan parameter.
6. Post-door flat `fatal` rows refit to the door evidence (1042-postfit); posterior door-to-finish .23.
7. Starter manip: strict IV tier (spa 29, spe 25, hp 15, spd 15 on one dial) is the safe pick; with a
   two-dial table the optimum is stricter (spe 28, hp 16, spd 20, about -10 h at WR+2m). Unknown which
   table the real setup has.

## Rules and guard rails (unchanged)

Rash/Mild starters only; reset time counts; `no_salvage_until`; `ball_batch >= 20`; no Brock potions;
no glitches; **no save-and-reload** (the `reload_*` knobs are a what-if only, legality unknown, never in
CURRENT); every claim cites `decompiled/`; no network; Python stdlib only; no `rm`; never `pkill -f`
(exact PIDs of your own processes only); never call `descend.py --help`.

## Upcoming searches, most valuable first

1. **Finish the League pass under the fitted model** (`descend.py --set league`, 217 candidates, stopped
   around Lance-2): remaining Lance-2 / Champion-2 / round-2 candidates, xbuys (special/speed/guard/
   accuracy +-), league_heals, setup_min_turns, heal_below for all League fights, more Mewtwo teaches.
   Confirm 1120 (Lance first X special -> calm) at R 3.2M on two seeds both rungs with corners, and 1114
   (Agatha -x speed, WR seed 7202 interval includes 0).
2. **Refit under 1122**: the reset_table in 1122 is still 1113's; re-run `tools/fit_post_evidence.py` (the
   Champion engine death changed: Champion 1 now has a 4-item queue) and `fit_lane.py --check`
   (noise rules: train R >= 2M, >= ~2000 finishes, common seed, best-iterate guard).
3. **Sets not run under the new model**: `--set all`, `earlymid`, `pp`, `dex`, fill/hunt orders (1101-1103,
   1112-evo48 Golbat evolution screened -0.7 h only), Koga/Sabrina/Giovanni X queues. Old-model steps
   1096 (reverted), 1097-1099 and 1107 were re-tested and kept/reverted (see the journal).
4. **Evidence-weighted interval and rank-flip grid for 1122** (`tools/rb_mc.py` with the new League
   stand-in posteriors, ROBUSTNESS 14; `rb_var.py` for 1115, 1105 and 1096-undone at 8-12 corners, R 500k).
   Candidate mechanism for carry_power: the double Torrent at Koga/Sabrina/Giovanni (`free_torrent` x
   `power` .667 x carry_power 2.0); the video shows plain one-Surf KOs there. Test cp 1.7 with the Torrent
   factor removed.
5. **Uncertainty reduction** (these move the answer more than any knob): door-to-finish survival (5
   attempts, 2 finishes), pre-door survival (63 runner attempts), trainer_fit, carry_power (needs a
   mechanism or mid-game per-fight evidence), mewtwo_power (Champion-2 length: runners 11 actions vs
   sim 12-13), pickup frame costs (only the PP Up pickup was measured on video, ~2.5 s).
6. **Starter table**: find out whether the real manipulation has one or two dials (idle-cost model 4.2);
   on-timer reject time is ~15 h of EHT and an earlier visual tell for a bad starter would cut it (needs a
   host check of the stream video, nothing in the committed transcripts supports it).
7. **Structural ideas not yet run**: walking Vermilion -> Saffron instead of the Celadon visit before
   Silph (saves a Fly, moves the X buy); a real FireRed early plan (FR was +44 h at the plan constants;
   Bulbasaur/Charmander give 0 finishes without their own early plan and a Surf holder); Light Screen /
   Bulk Up for Mewtwo; erika_pre_silph (-37 h screen but its sevii window overshoots the runner).
8. **Simulator gaps**: dynamic ("bag manip") trainers are modelled as free, the Flash trip and PC
   logistics beyond `pc_frames` are not modelled; Pressure is ignored (only Lance's Aerodactyl has it).

## Process lessons

- Run descents with an inline confirmation loop (paired R 3.2M, two fresh seeds, both rungs, stress corners
  league_power 1.0/1.4, mewtwo_power 1.15/1.45, carry_power 1.7/2.0, door survival 0.10); screens at
  R 800k, most unconfirmed promotions turned out to be noise (+-1 h).
- Agents share the scratchpad: use unique file prefixes; agent worktrees can be cut from an old base, so
  always `git merge human-run` first; resolve `scores.jsonl` merge conflicts by keeping both sides
  (but after an archive, keep only the branch's own new rows: see CLAUDE.md "Scores archive"). Rows up to
  2026-11-23 18:15 are in `scores-archive/2026-11-23-1815.jsonl.gz` (38 371 rows, 53.6 MB -> 2.2 MB).
- Descents are close to converged on knobs (the last confirmed steps are worth 1-3 h of ~100); the
  big wins this session came from model honesty and from audits (items, PP, fishing, Safari, evidence fits).
