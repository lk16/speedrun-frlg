# 2026-11-26: optimal-stopping reset table (draft, interim)

Base `1029-refit-lane`. Code: `crates/frlg-human/src/optstop.rs`, `frlg-human optstop`, plan key `reset_table` ({rung: {split: seconds}}), `tools/optstop_plan.py`.

## Policy
The existing split rule is already a per-split elapsed cutoff: reset if `clock + floor + aggr*(mean_finished_remaining - floor) > T`, with the mean from a no-reset pilot of *finished* attempts. At a slower model (trainer_fit .86) that mean is optimistic and the cutoffs come out 100-300 s too tight at Brock .. Surge, which rejects almost everything (the cliff).
Replacement: at each split, continue iff V(split, elapsed) > 0, V = E[lambda * 1(finish <= T) - wall time still spent | elapsed at that split], lambda = EHT of the policy itself (renewal-reward fixed point). V is fitted backward over splits from simulated attempts that play on (training cutoffs = current table + slack): target = wall to next split + max(0, V_next(elapsed_next)), regressed on elapsed at this split (binned means, monotone isotonic fit); last split uses the realised outcome. Cutoff = zero crossing of V, floored to whole seconds and capped by the best-case floor. Iterate with lambda <- EHT of the table on fresh seeds (6 iterations, R 2M training, 3.2M eval). No hindsight: V depends only on elapsed times already passed. Salvage rules, `no_salvage_until`, evidence guards, Rash/Mild are untouched.

## Result so far (paired seed 91, R 3.2M, table fitted at each cell's own constants)
WR+2m EHT optstop vs 1029 formula, h:
| trainer_fit / mistake | .3 | .16 |
|---|---|---|
| .76 | 330.6 vs 332.3 | 312.1 vs 313.7 |
| .81 | 409.4 vs 410.9 (-1.6, CI -2.5..-0.9) | 377.1 vs 378.9 |
| .85 | 546.9 vs 595.2 (-48) | 474.5 vs 524.5 (-50) |
| .86 | **582.4 vs 51 059** | 510.4 vs 2 074 |
The cliff is gone: EHT is smooth in trainer_fit. At the plan constants the hand-tuned thresholds were already within 0.4% of the optimum. At WR (tf .81): 565.1 vs 582.2 (-17.2, CI -26.9..-8.2) mr .3; 481.6 vs 506.3 mr .16.
The fitted table is almost independent of trainer_fit (Brock 1057-1063 s at tf .81 and .86), while the formula's Brock cutoff moves 1067 -> 758 s: the formula, not the optimum, is what is fragile.

## Final
Tables (reset if the run clock at the split is above, seconds; fitted at tf .81 / mistake .3; `reset_table` in plan 1040):
see `human-run/plans/1040-optstop.md` (WR+2m and WR). WR+2m: brock 1057, misty 1915, bill-cottage 2586, bike-redeem 2926, scope-and-dig 4103, surge 4842, super-rod 4970, koga 5726, blaine 6504, blaine+sevii 7464, sabrina 8402, erika 8614, giovanni 8873, indigo-heal 9445, HoF-1 10327, celio-network 11533 (first seven were still censored, i.e. lower bounds).
WR grid (table refit per cell vs formula): .76 398.3/402.6, .81 565.1/582.2, .85 852/5576, .86 975/no finish; mistake .16 .76 363/367, .81 482/506, .85 689/4755, .86 772/no finish.
Fixed table (fitted at .81) at .85 / .86, WR+2m: 548.4 / 583.7 h (formula 595 / 51 059); WR .85 853.8 (5576), .86 977.7 (no finish).
Plan 1040 vs 1029 at the plan constants, R 3.2M, fresh seeds: WR+2m -1.39 (92) -0.69 (93) h, WR -14.2 / -18.2 h (CIs exclude 0 at WR). Accepted as CURRENT on the WR rung.
Caveats: eval-seed scatter of the iteration EHT is about +-3%; the early-split cutoffs were censored at the training ceiling (cutoff+600 s slack) and may be higher at the optimum (a longer run would loosen them; at the plan constants the gain is already within 1%). The decision uses elapsed time only. Next: iterate to uncensored early cutoffs, condition on a second state variable (dex count / level) for a 2-number rule, re-fit the table whenever the window refit changes the model.
