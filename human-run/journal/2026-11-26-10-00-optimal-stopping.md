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

(Rest of the grid, table, plan 1040 and the second seed follow below when finished.)
