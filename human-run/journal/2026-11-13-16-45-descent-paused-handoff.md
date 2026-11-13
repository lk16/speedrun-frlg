# 2026-11-13 (evening): post-calibration descent paused mid-pass; next-session handoff

## State at pause

- Current best: **372-tactic181**, EHT(WR+160m) = **11.73 h** (R=20k, seed 1, 1050
  finishes). Chain today: 366 (12.58 on the broken engine) -> Sevii-audit model fix ->
  367 Fire Blast -> 368 Dojo -> 369 Hypno -> 370 Clemi skip -> engine fixes (PP policy,
  heals, encounter cooldown, Sammy, stones ordering, safari flee) -> 371 fill economy ->
  372 (surge sacs 3->4). All committed with two-seed confirmations.
- The full-knob coordinate descent (`descend.py --passes 2 --max-accepts 25`) was
  **stopped on request mid-pass-1** after 208 of ~400 evaluations (~75 min at ~2.7
  evals/min), with 1 accept (372). Kinds already swept this pass, with counts:
  fillsteps 28, tactic 29 (in progress — stopped at "blaine: +x defend"), aggr 26,
  addsite 22, hunt 18, special 17, repel 16, dropfill 14, order 13, starter 10, ball 5,
  toggle 5, fight 2, catch 2, safari 1. Not yet reached (by the usual enumeration):
  tm, grind, xbuy, elixir, heal, skipgroup, evogate.
- **Resuming:** `descend.py` does not consult scores.jsonl to skip re-tests, so a plain
  re-run repeats the ~208 done ideas (~75 min of compute — harmless, just slow). The
  faster resume is:
  `python3 human-run/tools/descend.py --passes 1 --max-accepts 25 --only tactic,tm,grind,xbuy,elixir,heal,skipgroup,evogate,fight`
  then a full `--passes 1` sweep afterwards to close the loop (and only then trust
  "pass yields no accept" as convergence).

## Still owed after the descent settles

1. Re-validate plans 367-371's acceptances on the fixed engine (they predate it) and
   re-run the WR-technique battery (journal 2026-11-13-04-30).
2. Move multi-species catch blocks (Safari / Power Plant / Mansion) from the fill phase
   into the route proper — the WR structure; the fill audit stopped short of testing it.
3. Tier-1 trainer-fight overhead measurement (lab-rival state exists) to pin the
   remaining ~1.5x on trainer groups.
4. Level-up-jingle Clemi variant measurement; encounter-seed-manip knob (both scoped in
   the 2026-11-13 morning journal).

## Host round-trip requested by the user

The user plans to download material on the host and feed the next sandbox session so
analysis runs concurrently. The shopping list and exact commands are in
`human-run/video-analysis/HOST-REQUESTS.md` (committed alongside this entry). The
whisper step in `tools/host-prep.sh` (commit 5415496) needs one host run before
`bin/frlg-transcribe` works in a fresh sandbox.
