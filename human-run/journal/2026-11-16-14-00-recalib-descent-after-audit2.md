# 2026-11-16: coordinate descent after the audit2 / mechanics / powers refit

`descend.py --set recalib --notkinds nosalv --rung WR+2m --r1 400000 --r2 800000` from 913 (=910 + quiet_max_hits 6), 2 passes; pass 1 accepted nothing.
`no_salvage_until` stays `rival-ss-anne` (runner habit, STRATEGY s7).

Note only, not a candidate: a first run (without `--notkinds nosalv`) accepted `no_salvage_until ""` as 914 (-21.2 h CI [-26.3,-16.4] seed 1 R=400k; -14.8 h CI
[-17.8,-12.2] seed 2 R=800k) and `candy_early_n 3->1` as 915 on top of it. Both were withdrawn: that is the upside if the human could play on after an early whiteout.
915's candy change was re-screened on 913 (seed 1 -6.9 h) but did not survive seed 2, so it is not in the final plan. Plans 914/915 stay on disk for the record.

Accepted (all confirmed seed 2, R = 800k): 916 reset_aggr_at misty 0.7->0.9 (-0.02 h, noise-level), 917 Rival-Tower +X Special (-3.3 h), 918 Rival-Route22 sacs 1 (-1.0 h),
919 Lorelei-2 sacs 1 (-0.1 h), 920 teach Ice Beam at fly-celadon-2 (-0.003 h), 921 Pidgeotto stone moved to erika-badge (-0.002 h), 922 Koga aggr 0.85 (-0.03 h).
Only 917 and 918 are real; the rest are inside noise but passed the two-seed rule. Final **922-aggr785**, paired seed 3 vs 910:
WR+2m 111.4 h (3556 fin, R=1.6M) vs 115.0 h, -3.6 h CI [-4.7,-2.4]; WR 131.9 h (4489 fin, R=2.4M) vs 138.6 h, -6.7 h CI [-8.3,-5.1].
What mattered: the model is on a plateau after 910; policy knobs (starter thresholds, hunts, dex/fill, xbuys, elixirs, evolutions, heals) refuted everywhere; only the early rival tactic changes helped.
