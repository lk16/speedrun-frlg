# Handoff for the next session (written at the end of the 2026-11-17/18 session)

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

## 2026-11-22 optimisation (journal `journal/2026-11-22-12-00-optimisation.md`): CURRENT = 1301-warren-potkoga

| plan | what | WR+2m | WR | model |
| --- | --- | --- | --- | --- |
| 1127-fheal1 | 1122 + League heal_below bundle (bruno .65, champion .35, champion-2 .5, lance-2 0) | 79.3-80.2 | 103.4-104.5 | old (fitted League) |
| 1130-honest-fheal | 1200 + the bundle | 125.3-126.0 | 166.3-169.9 | honest 1200 |
| 1131-honest-fheal | 1202 + the bundle | 152.0 (-9.8 / -12.5 vs 1202) | 241.8-243.7 | controller model |
| 1132-standin-fheal | 1211 + the bundle | 184.1 / 187.2 (-13.0 / -15.6 vs 1211) | 291.3 / 294.7 | stand-in audit model |
| 1300-runner-potkoga | 1253 + Mt. Moon Miguel potions 5 + Koga heal_below .55 | 269.9 / 283.4 (-15.3 / -11.6 vs 1253) | 477.2 / 487.1 | 1253 model |
| **1301-warren-potkoga (CURRENT)** | **1270 (Viridian Warren) + the 1300 steps** | **353.6 / 343.7 (R 3.2M, seeds 7561/7562; -18.9 / -17.3 vs 1270)** | **672.6 / 651.0 (-39.9 / -29.9)** | **1270 model** |

* League pass finished (lance-2 / champion-2 queues, pp floor, every per-fight heal_below); 1120 and 1114 confirmed
  by undoing them on 1127 (R 3.2M two seeds both rungs + corners). Mewtwo teaches stay dropped (0.001 h).
* reset_table refit on 1131: not adopted (WR+2m -0.4 h n.s., WR table unchanged). Re-run only after a big model move.
* Plan responses of 1201 re-screened on 1131 (Awakenings 2/4/5, S.S. Anne potions 7/12, heal_below .2/.45): all refuted.
* `risk_windows` under 1132 (`fit_lane.py`, R 1.6M seed 7651): `--check` has every lane window inside the runner's
  CI (only forest-brock .589 vs [.30, .54] outside, independent of powers). The refit saturates the first window at the
  4.5 cap (it also governs Brock / Mt Moon, standin-audit finding 6) and gives the Blaine/Sevii window 0.766 again (the
  shrunk runner target .724 asks for it). **Not adopted**; capping that window at 1.0 (-21 h) is a modelling call, not a fit.
* Optional-trainer dimension: `tools/fightknobs.py` (`descend.py --set fights`; first in `earlymid`, also in `all`).
  On 1300 every drop was refuted (+29..+163 h; exp outweighs fight time at ~1.0 windows), s11 Route 3 list +206 h;
  additions untried. Re-run after the WR-read fight lists / `trainer_cost` land.
* Pointer `plans/CURRENT-OPT` is the descent pointer (`descend.py --pointer CURRENT-OPT --num-from 1300`). Running:
  `--set league` from 1301. Next: `--set earlymid --notkinds fights` (tactics), then `--set all` / `pp` / `dex`,
  fill/hunt orders; reset_table refit after the model settles (EHT moved 170 -> 350 h).

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
  always `git merge human-run` first; resolve `scores.jsonl` merge conflicts by keeping both sides.
- Descents are close to converged on knobs (the last confirmed steps are worth 1-3 h of ~100); the
  big wins this session came from model honesty and from audits (items, PP, fishing, Safari, evidence fits).
