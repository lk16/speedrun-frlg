# 2026-11-13: manipulated starter + quiet wild seed in the simulator (plan 374)

## What was built

`starter_model = "manip"` (old `"random"` stays the default), `crates/frlg-human/src/manip.rs` plus small hooks in
`sim.rs` (attempt start, the `starter` check, `run_encounters`) and `eval.rs`/`main.rs` (`frlg-human starter-phase`,
also printed by `report`). Mechanism and parameter derivation: `docs/tricks/starter-manip.md` (+ new section 6).
Plan 374 is accepted: `EHT(WR+160m)` 11.76 -> 10.59 h (-1.17 h, CI [-1.26, -1.08], R = 640000; seed 2: -1.22 h).
CURRENT = `374-manip-starter`. The headline rung is still WR+160m: the manipulated starter does not reach a tighter
rung at R <= 400000.

## Model

Three presses (title A, naming exit, ball-dismiss A) each hit with `p = P(|N(mu, sigma)| < 0.5)`; any miss gives an
independent random starter (kept only if it happens to meet the tier), the title miss also loses the quiet seed. A
miss is seen at the stats screen and costs `s_reset` off the timer; `s_boot_title` precedes every attempt. On
success (nature, IVs) are drawn from the reachable-good set and used as the carry's real IVs/nature (`battle_sim`
already derives all stats from them). The tier's density `rho` reproduces the doc's table (T1 0.75%, T2 0.0396%),
and the on-timer idle for the best (N, j) pair is Rayleigh (2 dials, doc means 15/65/147 frames) or exponential
(1 dial, `1 / (0.933 rho)`).

## Fit (scan data, `video-analysis/data/starters/`)

- survival past the lab 34% (38/113 over four sessions, range 24-48%) -> `p = 0.692` (sigma 0.49 frames);
- reject at the stats screen/rival fight 108-200 s of timer (model 129 s + idle);
- start-to-start of a rejected attempt 282-335 s median (model 298 s) -> boot 45 s + reset 100 s off the timer;
- result: 3.0 attempts and 13.5 min wall per good starter (p90 28 min); the old random gate needed ~7 attempts, ~17 min.

## Sweeps (R = 160000, rows in `scores.jsonl`)

- **Natures**: Rash+Modest beats Rash+Mild by 0.30 h (CI [0.24, 0.36]); Mild's -Def is a net loss. Rash 10.65,
  Modest 10.64, Rash+Mild 10.93, all three 10.88. Modest is worth adding to the user's Rash/Mild set.
- **IV tier**: SpA >= 30 alone does nothing (11.79 vs 11.83); Spe >= 20 -0.5 h; HP/Def floors are the real lever
  (the model is very sensitive to carry HP/Def at Brock/Surge/E4). With the idle cost on the timer the optimum
  is flat: Rash+Modest, Spe >= 20, HP >= 12 (15 s idle) 10.66, + Def >= 12 (24 s) 10.48, HP >= 25 & Def >= 25 (200 s
  idle) 10.31. With two dials (doc 4.2) Spe/HP/Def >= 25 costs 2.5-3 s and reaches 10.08-10.15. The scan data show
  accepted starters with HP 10-30 and Def 11-31, so the plan assumes the light one-dial tier.
- **Hit rate**: p 0.5 / 0.69 / 0.9 / 1.0 -> 11.91 / 10.93 / 10.56 / 10.49 h: a drifted regime (sigma 1.0 frame,
  p = 0.38) costs +3 h. Off-timer cost x0.5 / x2: -0.1 / +0.25 h (small next to p).
- **Quiet wild seed** (cap on encounters in the first 60 forest-rate rolls, wait for the title frame in the wall
  time): *refuted* at every tier, +0.15 h (30-roll window) to +1.6 h (<= 2 hits). Reason in the model: the 3.8 min
  of forest encounters disappear, but the carry arrives at Brock at 7.4 not 8.2 (whiteouts x2.6) and with 3.3
  instead of 6.0 species, which the later fill pays back. Grinding 2 battles after the forest trainers recovers 0.2 h
  only. This contradicts the runner's own "saves a minute", so it is probably a limit of the model (no route re-tuned
  around the quiet seed, no choice of *which* encounters), not evidence about the real manip.

## Uncalibrated / next

1. One vs two dials: the biggest unknown for the tier (about 0.5-1 h). A tier-1 emulator check of whether a
   naming-idle table is feasible (doc 3.1: the naming seed is chaotic per N) or a look at what his overlay "frame"
   counts would settle it.
2. `p` mixes hits, Context failures and rejects for other reasons; no drift regime; `p_accept` = 1.
3. A route re-tuned for a quiet seed (Route 1 grind/catch policy, Brock tactics) before ruling the manip out; the
   suppression model also counts rolls per step (guess, the real index drifts, journal 2026-11-13-12-30).
4. Off-timer segment costs are fitted to cycle times; the per-segment sums (boot, naming, ball, stats) are not
   measured on hardware.
5. Merge note: the other worker rebuilding `route.rs`/`sim.rs` only meets this work at three hooks in `sim.rs`
   (`State` quiet fields, `quiet_account` in `run_encounters`, the `starter_check` branch) and one `Params` field pair.
