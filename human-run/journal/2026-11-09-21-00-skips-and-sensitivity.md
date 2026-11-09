# 2026-11-09 evening: plans 012-015, rule fix, sensitivity

## Accepted
- 012 in-game trades; 013/014 fill step budgets x0.4 then x0.2 (the scores.jsonl idea labels
  "x0.06/0.04/0.15/0.05/0.07" are really 0.6/0.4/1.5/0.5/0.7 -- typo, results are right);
  015 walk around dodgeable trainers (`skip_groups`): rung **WR**, EHT 48.6 h, 495/10000 finishes.
- Model fix between 012 and 014: an exhausted fill list used to *abandon* the attempt, which made
  tiny step budgets look good. It now goes round the list again (<= 6 rounds). 013/014 were re-checked
  with `dist` after the fix (mean 3:49 and 3:46 vs 012's 3:54), so the ordering holds.

## Refuted
- Per-split reset aggressiveness at Hall of Fame / Oak (0.3, 0.8); catch-nothing-while-passing;
  starter totals 74/75 (rung loosens: the starter filter costs more than it saves).

## Sanity alarm: the WR rung is NOT trustworthy
Plan 015 reaches the WR rung in the model with a mean of 3:34 against the 3:25 record and an
EHT of 49 h, which is implausibly low for a category record. Sensitivity of 015's rung:

| model change | rung | EHT |
| --- | --- | --- |
| none | WR | 49 h |
| `mistake_rate` 1 (events per 100 tiles/inputs, 60-300 f each) | WR | 786 h |
| `mistake_rate` 2 | WR+1m | 17 700 h |
| `battle_scale` 1.15 (battle frame constants x1.15) | WR+2m | 15 800 h |
| `battle_scale` 1.3 | WR+20m | 113 h |

So the headline depends on uncalibrated battle constants (TURN 330, MON_OVERHEAD 450, group
trainer overhead) that are plain guesses, and on there being no mistakes. Treat "WR rung" as
"the model's mean is close to the record", nothing more. Calibrating battle frame costs on tier 1
is the single most valuable next step. Also unmodelled: HM slaves, party exp split, healing/item
use between battles, shopping, Safari step/ball limits, money (stones cost 2100 each).

## Next
1. Calibrate a trainer battle and a catch on libmgba (frames per turn, per faint).
2. Variance-aware resets, and a larger-N check of rankings at `mistake_rate` 1.
3. Sevii routing: Mt Ember/Five Island tile counts are guesses; cite and measure them.
4. LeafGreen tables and other starters.

## Update (later the same session)
- **Calibration found.** The ledger has TAS frames for two battles the model also simulates: lab
  rival 354+2463 = 2817 f (47 s) and Brock 4991 f (83 s, includes gym walking). The model said 35 s and
  63 s, i.e. *faster than a TAS*, which is impossible. `battle_scale` 1.75 matches both (48 s / 89 s).
  That is still only a TAS-level floor for a human, but it moves plan 021 from rung WR to **WR+40m**
  (mean 4:30). Plans 016-021 were scored under the uncalibrated constants; 022-calib is the new baseline.
- **Correction:** Cerulean Cave was in the fill list; it is closed until the link flag. Plans 012-019
  were scored with it. 020 removes it.
- Decomp facts added: LeafGreen wild tables (`sites_lg`), `version` param (LG chosen: -0.18 h, noise level).
