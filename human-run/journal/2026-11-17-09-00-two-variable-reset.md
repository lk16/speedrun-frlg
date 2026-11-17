# Two-variable reset rule (refuted)

Code: `optstop --adj` / `--slope-only` (`optstop.rs`), plan key `reset_adj` {rung:{split:[kind,ref,s_per_unit]}}, kind 1 = species caught, 2 = lead level; reset if clock > cut + slope*(x-ref). Per split the variable with the larger |t| is chosen (|t|>=3), V fitted by back-fitting f(e)+c(x). Fitted slopes: ~50-90 s per species from Koga to Giovanni, ~210-280 s per lead level at indigo-heal, ~160-190 at HoF-1.

Paired vs 1040, R 3.2M, fresh seeds (h, a-b): full refit with slopes +3.4 (701) / +5.7 (702) at WR+2m, +21 / +18 at WR; 1040 cutoffs kept with local slopes (seed 711) +7.0 at WR+2m, +250 at WR (WR slopes unstable). All CIs exclude 0 on the wrong side. A plain refit without slopes is already ~6 h worse than 1040 (table scatter), and slopes add ~10 h at WR (seed 703 check).

Interpretation: linear slopes over-reset at low x (V is not linear in x) and the in-sample fit rewards noise. 1040 stays CURRENT; 1041-optstop2.md is untracked and not adopted.
Next: clamped shift, threshold rule (reset only if dex < k), x-binned V tables.
