# 2026-11-26 14:00 optstop follow-up: censoring and trainer_fit .87/.88

Base plan 1040-optstop (stays CURRENT).

## 1. Early cutoffs "censored": it is the best-case floor, not the training ceiling
Refit with slack 900 s (was 240), 6 iterations, R 2M training, 3.2M eval, seed 95, at WR+2m and WR. The first seven cutoffs (brock..super-rod) came out the same as in 1040 to within 5 s (WR+2m: 1056 1916 2583 2925 4107 4839 4967 vs 1057 1915 2586 2926 4103 4842 4970) in spite of 3.75x more slack, and are still flagged CENSORED. They are capped by `target - floor_after` (the best-case remaining floor): V is positive everywhere below it, so the optimum there is "reset only when no finish is possible", and no longer run can loosen them. Later cutoffs moved by <= 20 s at WR+2m. WR table: koga 5576, blaine 6348, +sevii 7282, sabrina 8185, erika 8423, giovanni 8719, indigo-heal 9339, HoF-1 10210, celio 11406 (identical to 1040's to within a few s).
Paired candidate (refit tables) vs 1040, R 3.2M, seeds 92/93: WR+2m EHT 411.4855 vs 411.5102 and 408.5372 vs 408.5589; WR 570.7085 vs 570.7735 and 566.9496 vs 567.0058. Differences 0.01% of score (CI excludes 0 only because the policies are almost identical, same finish counts). Not adopted: no plan 1041. Iteration EHT scatter across fit iterations is +-3% (406-428 h), so more iterating is just noise.

## 2. trainer_fit .87 / .88 (WR+2m, seed 91, R 3.2M)
Fixed 1040 table vs the hand formula: .87 636.2 h (1208 fin) vs 41 003 h (10 fin); .88 751.8 h (1021 fin) vs 12 172 h (34 fin). EHT is smooth in trainer_fit: .81 410, .85 548, .86 584, .87 636, .88 752. Refit at each cell gains only 0.3% (.87 634.4) and 0.5% (.88 748.1). No cliff for the optstop table; the formula's cliff continues (and is non-monotone). Fixed-table robustness is enough.

## Next
Second state variable (dex count / level) in the stopping rule is the remaining policy idea; the rest of the headroom is in the model (trainer_fit sensitivity: +25% EHT per +.01 beyond .86).
