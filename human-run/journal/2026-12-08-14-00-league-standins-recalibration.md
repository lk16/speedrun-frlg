# 2026-12-08: League stand-ins re-measured (WR VOD + 2026-11-14 stream), mechanism for league_power, re-evaluated EHT

Evidence (committed): `video-analysis/data/calib/league-{wr,s14}-{r1,r2}.{json,md}` (per-turn actions, HP before/after, foe bars, X items, heals; WR Round 1 / 2 and the A1 run of the
11-14 stream), `mid-s14-{a,b}.{json,md}` (A1 Misty, S.S. Anne rival, Koga, Blaine, Sabrina, Giovanni, Route 22 and Silph rivals). The 11-12 stream has no League (reset at 2:39:37).
Tools: `frlg-human lstate | hit | fightdist` (new), `tools/cal_hits.py cal_lik.py cal_fights.py cal_eht.py cal_cmp.py`.

## 1. Mechanism checks (no hidden multiplier)
* **Damage**: engine vs the foe-bar drops of 7 non-lethal hits (WR + A1 Dewgong / Lapras / Gyarados Bite at +1..+4 SpA, WR Mewtwo Psychic on Dewgong / Lapras): all inside the engine range +-1 px (`cal_hits.py`);
  Aerial Ace 33-39 vs 36 seen, Surf +3 one-shots Pidgeot as seen. Real carry stats (WR Lv51 box 153/108/119/123/111/107) give EVs about HP 36 / Atk 40 / Def 20 / SpA 44 / SpD 14 / Spe 60; the sim's door-time carry
  (`lstate`) has 34 / 45 / 46 / 56 / 14 / 73, level 55-56 against the WR's 50-52, same Rash / SpA IV 30 / Black Glasses. The carry is not weaker than the runner's; `league_power` is not strength.
* **Where league_power matters** (`fightdist`, per-fight actions among won fights, power 1.0): Lorelei, Bruno, Agatha and Round 2 Bruno / Agatha / Lance already fit (P(<= obs) .6-.9). The misfits were **Lance 1 (13.8 vs 10 / 7),
  Champion 1 (20.7 vs 12 / 13) and Champion-2 (17.7 vs 11.5 / 11)**; league_power 1.4 fixed those and broke Bruno / Agatha (6.2 / 5.7 vs 9-10 / 7).
* **Mechanism found for Round 1**: (a) the player's move choice rations scarce-PP moves (`SCARCE_FACTOR` 0.6): the engine Bites Gyarados four times where both runners Blizzard it at once (A1 Lance: X Special, X Accuracy, Blizzard crit);
  now `Params::scarce_factor` (default 0.6 = unchanged; 1.0 = no rationing); (b) the runners' X queues (Champion Guard Spec, X Speed, 3 X Special; Lance 1 X Special + 1 X Accuracy). With both at league_power 1.0
  Lance 13.0 mean / p50 10 (obs 10 and 7), Champion 14.8 / p10 12 (obs 12, 13). Log-likelihood of the 10 R1 counts (`cal_lik.py`, two samples per fight): plan tactics lp 1.0 -33.0, plan tactics lp 1.4 -30.1,
  **runner queues + scarce 1.0 lp 1.0 -18.3** (lp 1.1 -22.9, 1.2 -25.9, 1.3 -21.6, 1.4 -25.8): **league_power 1.0 [1.0, 1.1]**; it is a policy fact (Blizzard not rationed, runner queues), not a stat factor.
* **Round 2** (mp = lp, runner Champion-2 queue special / attack / attack): loglik -18.5 (1.0), -16.3 (1.1), -14.6 (1.2), **-13.7 (1.3)**, -14.4 (1.4): **mewtwo_power 1.3 [1.15, >= 1.4]**, 1.0 excluded by 4.8 units. Driven by Champion-2 (both runners 11 actions,
  engine minimum about 12-13; the plain-Mewtwo Champion-2 win rate in the engine is 7%, both runners survived with 56/258 and 148/241 left) and Lance 2. No mechanism found: Mewtwo's held item and stats were never on screen (stat boxes skipped), A1's Mewtwo has HP IV ~28 (248 at Lv70) against the WR's 5 and both play alike. Stays a stand-in.
* **carry_power** (mid-game, `cal-cp` runs, conditioned on reaching Sabrina): loglik -30.0 (1.7), -33.7 (2.0), -35.8 (2.4); cp 1.0 and 1.4 reach Sabrina in 0 of 300k attempts (the risk lane kills them), so 1.7 is the edge of what the structure can express.
  Prefers <= 1.7 by 3.7 units but the evidence is weak (A1 Misty / S.S. Anne 9 actions vs the engine's 5-6). Torrent is applied twice in the sim at Koga / Sabrina / Giovanni (`free_torrent` x1.5 plus `power` .667 x cp 2.0 = x1.33): the A1 Koga / Blaine / Sabrina fights are all one-Surf KOs at Torrent
  with no extra factor. Not changed.

## 2. EHT under the fitted constants (R 1.6M seed 5301, door-to-finish D pinned to .23 by scaling the flat post-door rows, `cal_eht.py`)
| model | WR+2m | WR |
| --- | --- | --- |
| CURRENT 1103, old constants (lp 1.4, mp 1.2, scarce .6) | 106.6 | 139.6 |
| + scarce_factor 1.0 (1110), lp 1.4, mp 1.2 | 89.7 | 114.4 |
| 1110, lp 1.0, mp 1.3 (fit) | **124.4** | **169.1** |
| 1110, lp 1.0, mp 1.15 / 1.45 | 132.1 / 128.8 | 185.3 / 179.0 |
| 1110, lp 1.1, mp 1.3 | 106.5 | 144.7 |
| 1110, lp 1.0, mp 1.3, cp 1.7 (Q pinned by lane) | 228.0 | 416.1 |
| 1111 (runner queues for Lance / Champion / Champion-2, lp 1.0, mp 1.3) | 225.7 | 294.6 |
So the fit costs about +18 h (WR+2m) / +30 h (WR) against CURRENT's old stand-ins; the cp prior is the large term (cp 1.7: x1.8 / x2.5). The runner queues are NOT a better plan (1111: +100 h): they fit the action counts but cost item turns.

## 3. Rank flips on the last League-related plan changes (paired, R 1.6M, WR+2m, same constants and same flat on both sides; difference base - variant)
| variant of CURRENT | old constants (flat 1.113) | new (lp 1.0, mp 1.3, scarce 1.0, flat .83) |
| --- | --- | --- |
| champion x = guard, speed, special, special | +26.8 h worse [-29.5, -24.6] | **-23.5 h better** [20.7, 26.4]; seed 6103 -25.2 [22.6, 28.0]; WR rung seed 6102 -43.2 [38.5, 47.8] |
| champion x + a third special | +24.1 worse | n.s. (+2.6, [-6.1, +1.0]) |
| lance x = special, accuracy | +5.6 worse | +35.5 worse |
| lorelei calm undone (1094) | n.s. (+1.0) | +26.9 worse |
| champion-2 x = special, attack, attack | +117 worse | +74.6 worse |
| salvage_at 0.8 (1095 undone) | 0.0 | 0.0 (no effect in the model) |
1094 holds under both; **one real flip: the 4-item Champion queue wins under the fitted constants and loses under the old ones.** Not adopted (the rule: better under both). If the fitted constants are adopted as the model, `1103 + champion x [guard, speed, special, special]` is the first descent step.

## Decision
CURRENT stays `1103-dropfill442`. New plans `1110-cal-scarce1` (model change) and `1111-cal-wrq` (refuted as a plan) are files only; no scores.jsonl row. `scarce_factor` is a new Params field (default 0.6, bit-identical for existing plans; 46 tests pass).
Open: adopt the fitted constants as the model (then re-descend League tactics, re-fit `fatal` / reset_table / windows at lp 1.0); a Mewtwo held-item / stat measurement would pin mewtwo_power; the double Torrent at Koga / Sabrina / Giovanni and cp.
