# Handoff for the next session (written at the end of the 2026-11-17/18 session)

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
