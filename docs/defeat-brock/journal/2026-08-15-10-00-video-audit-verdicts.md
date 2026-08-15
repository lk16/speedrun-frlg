# Video audit: eleven viewer observations, verified against sources

The 38862 video was watched back and eleven inefficiency candidates flagged
(timestamps at 59.7275 Hz map to the frames below). Each was checked against
`decompiled/` and fresh emulator/planner probes — not against the previous
discussion's conclusions. New read-only tooling this session:
`examples/dodge-probe.rs` (replays the committed run, then re-plans every
wild-map trail from its real entry state at the honest encounter cost *and*
with encounters forbidden), `examples/replan-probe.rs` (re-plan one leg from
any mid-run frame), `examples/trace-steps.rs` (step trace in a frame
window), plus two new planner knobs (`PlanRequest::encounter_cost`,
`PlanRequest::test_bias`).

## Verdicts

1. **Rival trigger tile (1:57, ~f6910) — CONFIRMED, ~48 frames.**
   The committed run triggers the lab battle from the middle tile (6,8)
   (trace: f6830..f6910 walks (9,5)→(6,8), six steps). The trigger row has
   three tiles (5..7,8) (`data/maps/PalletTown_ProfessorOaksLab/map.json`
   coord_events), and with Squirtle chosen the rival's scripted movements
   are: approach Left/Mid/Right = 5/4/3 steps
   (`Movement_RivalApproachForBattleBulbasaur*`), exit Left/Mid/Right =
   6/**7**/6 steps (`Movement_RivalExitAfterBattle*`), all in
   `data/maps/PalletTown_ProfessorOaksLab/scripts.inc`. The door spans
   (5..7,12) (three warps), and the row-8 corridor is only those three
   tiles wide, so from the Squirtle ball at (9,4) the right tile (7,8) is
   also one *player* step closer. Net: right beats mid by 3 scripted walk
   steps ≈ 48 frames, before any stream effects.

2. **Rival turn 1 miss vs turn 2 hit (2:24) — REFUTED as a free choice.**
   The observation is real (audit HP events: our tackles −4/−10crit/−5;
   the rival's only landed hit is −3 at f9067). But miss/hit is not a
   per-turn dial — it is a property of the RNG line. The fitted pacing
   already prices a rival miss *slower* than a small hit
   (`racc_miss_to_turnend` 166 vs 29+3+82+10 ≈ 124,
   `frlg-battle/src/pacing.rs::SQUIRTLE_LAB`), so the solver prefers
   hit-lines; the committed line won the arbitration over the full
   enumerated start-state space for this arrival (floor 2376 modeled, 2445
   realized, `journal/2026-08-14-16-00`). A both-hit line that beats it is
   not reachable *from this arrival*; the fight is re-solved from scratch
   on the re-routed prefix anyway.

3. **Wild encounter 3:12 (f11196, Route 1 Pidgey) — REFUTED (fated).**
4. **Wild encounter 3:29 (f12315, Route 1 Rattata) — REFUTED (fated).**
   dodge-probe, from the run's real Route-1 entry state (f10914,
   entry (12,39)): with encounters priced at 1,000,000 the best plan
   *still* contains 2 fated passes — no clean path across Route 1 exists
   on this wild stream at any detour length. The rate-test sequence is
   step-indexed, not frame-indexed (`src/wild_encounter.c:667-671` own
   LCG; seeded once, `src/new_game.c:103`), so no timing dodges them
   either. Cheaper-to-take-than-dodge is not the claim — *undodgeable* is.

5. **Zigzag 3:35→3:37 (f12862, (16,15)→(17,·)→(16,12)) — CONFIRMED,
   ~32 frames.** replan-probe from the exact post-Rattata state: the model
   optimum is the straight column-16 path (18 steps, one fated-fail test);
   the committed run walked 20 via column 17. Cause: the executor's
   stuck/carry handling (a wandering NPC occupies (19±3,16±1); a blocked
   edge or a 60%-boundary-roll mismatch forces a replan around,
   `brock.rs::walk_planned`). Real cost, re-rolled on any rebuild.

6. **Grass on the deliver leg (4:12) — CONFIRMED geometrically, but not
   free.** The viewer's count is exactly right: the minimum is 5 grass
   tiles (the (12..13, 36..39) chute into Pallet plus its entry tile —
   Route 1 map, `frlg map 3.19`). Priced with the new `test_bias` knob:
   frames-optimal is 41 steps / cost 706 with 14 rate tests consumed; the
   5-grass zero-test line is 54 steps / 984 — **+278 model frames**. The
   committed 20-grass line is the frame-optimal one; low-grass is now a
   dial for re-luck robustness, not a straight win.

7. **Wild encounter 5:35 (f19962, Route 1 Rattata) — REFUTED (fated).**
   Same probe, tutorial's Route-1 crossing: clean-only still holds 1 pass.

8. **Forest encounter 7:52 (f27855, Caterpie) — CONFIRMED as required.**
   Clean-only from the forest entry state still holds 1 pass: on this
   stream the forest cannot be crossed without it.

9. **Forest zigzag 7:55→7:59 (f28401) — CONFIRMED, ~16-32 frames.**
   replan-probe from the post-Caterpie state: optimum is 37 steps via
   column 11/row 27; the committed run went column 12/row 28 (+1-2
   steps). Same executor-realignment cause as (5).

10. **Sammy in 3 turns, all crits (8:14) — numerically feasible, not
    reachable from this arrival.** Weedle L9 has 26 HP (audit); crit
    tackles do 9 (observed twice), so 3 crits ≥ 26 works on paper. Crit
    odds are 1/16 (`src/battle_script_commands.c:588,1199`), Tackle 95
    acc (`src/data/battle_moves.h:437`). The ~26k-line mega-sweep on this
    arrival found no 3-crit line (`journal/2026-08-14-21-45`); the
    reachable-line set is arrival-specific and is re-swept after any
    upstream change.

11. **Grass at 9:07 (Route 2 north stub) — CONFIRMED, cheap to avoid.**
    The 7-grass crossing (6 cooldown-gate rolls + 1 rate test) is on the
    frame-optimal 16-step path (cost 256); the grass-free line is 18
    steps / 288 — **+32 frames** buys an empty constraint surface for the
    Brock-arrival dial sweeps (which found only 7 winners in 19.6k lines).
    Worth taking iff the freed dial space repays 32 frames; wired as the
    per-map `test_bias` knob.

## Net

Direct, certain waste identified: ~48 (trigger tile) + ~32 (Route 1
dogleg) + ~16-32 (forest dogleg) ≈ **96-112 frames**, of which only the
48 are deterministically bankable (the doglegs are realized NPC/roll luck
that a rebuild re-rolls). The four flees and the rival-miss/Sammy-crit
wishes are stream properties, already at their floors *for the committed
arrival* — the trigger-tile change moves every arrival, so all fights and
crossings get re-solved and re-swept on the new streams.
