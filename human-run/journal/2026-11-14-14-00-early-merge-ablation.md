# 2026-11-14: ablating plan 383's early-game params on the audited engine; plans 384-386

Question: why is 383 (382 + 377's early params) +2.2 h worse than 382 on the audited League engine?
Paired ablations, each param of 383 reverted alone (WR+10m, R = 40 000, seed 1; positive = the param helps):

| param reverted | a - b (h) | CI | reading |
| --- | --- | --- | --- |
| `no_salvage_until hall-of-fame-2` | -3.41 | [-4.18, -2.71] | **hurts**: every whiteout up to HoF-2 abandons the attempt; 383 reaches HoF-2 on 4.2% of attempts vs 6.4% (382) and 7.2% (384) |
| `poison_frames_per_hp 70` | -2.81 | [-3.34, -2.27] | removing it only makes field poison free (decomp drain, `field_poison.c`); kept, it is fidelity, not a knob |
| manip `quiet_*` | +3.43 | [+2.38, +4.43] | helps |
| `mt-moon-extra` fights | +1.94 | [+0.76, +3.03] | helps |
| `torrent_*` | +1.11 | [+0.03, +2.08] | helps |
| Misty `power 0.75` | +0.38 | [+0.01, +0.73] | slight help |
| Brock `entry` | -0.09 | ns | |

So no engine bug: the one harmful param is a policy (never salvage an early death). `no_salvage_until` over the whole
pre-HoF-2 run is wrong on this engine because League-level deaths are now common and a reset throws away the whole run;
for the early game it helps slightly (below).

Plan 384 = 383 without `no_salvage_until`: 16.04 h vs 17.23 h (382), diff -1.20 h CI [-1.83, -0.50] (R 40 000;
seed 7: -1.12 [-1.74, -0.57]).

## Engine fix

`manip.quiet_window` above 60 was a free lunch: the quiet-seed density table (`ManipParams::quiet_p`, doc 3.4) is for
hits within 60 rate tests and does not depend on the window, so window 150 "won" 1.7 h. `sim.rs` now caps the window at
60 (no effect on any existing plan, which all use 60).

## Descent on the early game (seed 1, then seed 2 for every accept)

Accepted on both seeds: `no_salvage_until rival-ss-anne` (-0.41 / -0.71; later values are worse: koga +0.45,
sabrina +1.4, indigo-heal +2.5), `torrent_until hall-of-fame-1` (-0.50 / -0.78), `torrent_hits 1` (-0.33 / -0.28),
Misty power 0.9 (-0.13 / -0.07). Combined = plan 385: -1.05 / -0.72 h vs 384 (R 40 000, seeds 1 and 2).
Then `torrent_hits 1.25`: -0.70 / -0.77 / -0.58 h at fixed R = 160 000 on seeds 1, 2, 4 = plan 386. The knob is
non-monotone (1.4 -0.55, 2.0 worse, 3 much worse).

Refuted (worse or ns): torrent_until earlier than HoF-1 (+1.5 to +5 h), torrent_heal_below 0.2/0.25/0.4/0.5,
torrent_hits 3, quiet_max_hits 2/3/5/6/8, quiet_window 30/45, manip min_spa 25/28/32, min_spe 15/25, min_hp 8/16,
natures Rash-only / Mild-only / +Quiet / +Hasty, Misty power 0.5-0.8/0.95/1.0, Brock without entry, Misty entry,
Brock/Misty `x special`, fight_until_level 12-18 (<=16 identical; 17+ +2.6 h), reset aggr at Brock (no effect) and
Misty badge (1.0 is catastrophic), extra heals at Brock/Misty/Cerulean/Rock Tunnel, version FR (+8.5 h), dropping
Miriam from mt-moon-extra (+1.8 h). Not accepted although "Modest" in the nature list scored -0.77 at seed 1: the
user's rule is Rash/Mild only. Pending-nothing: `drop extra_heal nugget-bridge-trainers`, `no_salvage rival-cerulean`
(seed 2 ns), `min_spa 28` (seed 2 ns).

Method note: `try.py` stops at the first R whose CI excludes 0 (10 000 attempts), which overstates large effects
(torrent_hits 1.25: -2.1 h at R=10 000, -0.7 at R=160 000). Confirm at fixed large R before promoting.

## Score

`386-hits125`, WR+10m: **14.60 h** (6940 finishes of R=160 000) vs 382 16.91 h, paired diff **-2.31 h CI [-2.67, -1.99]**
(seed 5, R=160 000). No-reset finishers (20 000 attempts): min 3:23:23, p50 3:36:26.

## Next

* `no_salvage_until` between Misty and Rival Silph is only a coarse sweep; a per-segment salvage rule (salvage when
  the whiteout costs < X min of run clock) is untried.
* The Brock/Misty fight tactics (x lists, sacs) and the torrent threshold are now flat; remaining early-game loss is in
  the starter reset phase (66% of attempts end there; 11.9 min mean wall to a good starter).
