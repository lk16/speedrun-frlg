# 2026-11-13 (later): the mismatch hunt — video ground truth vs the model, in parallel

Question driving the session: do we have a strategy that beats the WR in reasonable time?
Answer so far: **no, and the blocker is model error, not route ideas** — before today the
model's no-reset mean was 5:49:53 against a 3:25:25 human reality. This session attacked
that gap with seven parallel audits (4 on the WR video, 3 on engine/code), then fixes.

## Ground truth extracted from the WR video (reports in scratch `calib/`, summarized here)

Real fight lengths (intro to victory, ±4-5 s; turns ±1):

| Fight | real | turns | model (pre-fix) |
| --- | --- | --- | --- |
| Brock | ~31-51 s | ~2 | 1:36 |
| Misty | ~54 s | 6-7 | 2:09 |
| Rival SS Anne | ~76 s | ~6 | 6:41 |
| Koga | ~72 s | ~8 | 3:19 |
| Blaine | ~80 s | ~6 | 3:19 |
| Sabrina | ~80 s | ~9 | 3:04 |
| Giovanni | ~78 s | ~6 | 5:33 |
| Rival Route 22 | ~125 s | ~12 | 4:14 |
| E4 R1 total (5 fights) | ~614 s + 126 s between | 6-11 each | 19:28 |
| E4 R2 (Mewtwo sweep) | 92-196 s each | mostly 1/mon | — |

Observed tactics: no mid-fight heals outside the League; enters gyms at low HP; X
Accuracy+Special on big fights; ~11 X Specials stocked at E4; two heals + a Max Elixir
*between* E4 fights; league Full Restores used sparingly.

## Root causes found (audits), and what was done

1. **PP starvation by the move policy** (engine audit): always picking the strongest move
   drains it on trash; Blaine at PP [0,0,6,0] took 20 turns vs 6.1 at full PP. Fixed:
   among killing moves and among near-best (>=92%) moves prefer the slot with most PP.
2. **Heals too eager; league Full Restores effectively unlimited.** Fixed: heal/restore
   only when hp <= 2x the foe's best hit (plus the existing in_danger path).
3. **No encounter cooldown** (prebrock audit; `wild_encounter.c:673-755`): the game gates
   the first `8 - rate/10` eligible steps after map entry/battle to a 5% pass; the model
   rolled full rate every step -> ~17 encounters pre-Brock vs the video's 2-4. Fixed in
   `run_encounters` (two-phase geometric). Pre-Brock mean 27:58 -> 20:48.
4. **Sammy double-counted** in the forest ledger segment. Fixed (6708 -> 4002).
5. **Stones evolved AFTER Oak's dex check** (fill audit) — a route-ordering bug that made
   stone evolutions useless forever; every past "stones refuted" line (including this
   morning's) was an artifact. Fixed; stones x5 now accepted at -1.38 h.
6. **Fill economy** (fill audit, instrumented): 44.5 min fill at ~110 s/species; 47% of it
   is encounter intros + fleeing duplicates; the WR runner pays ~50 s/entry blended, and
   12-14 of his 60 entries are not catches at all (evolutions/trades/gifts). Applied:
   candy-evolution chains (Weedle/Caterpie, requires-gated, -1.69 h), drop the two worst
   fill sites (Route 13 at 312 s/species, Seafoam B2F at 231 s/species, -0.68 h),
   safari flee 500 -> 200 frames (always-succeeds Run; guess).
7. **Known artifact in the ledger:** two "[engine-audit] carry TM03 Water Pulse" lines
   (+26 h, +20 h) are invalid — the carry already learns Water Pulse by level-up and the
   patch wasted a slot on a duplicate. Do not re-read them as evidence about Water Pulse.

## Where the numbers stand

- Plan **371-fill-economy** (stones x5 + candy evolutions + fill trim on top of 370):
  **EHT(WR+160m) = 11.77 h** at R=20k seed 1 (370 was 13.40 under the fixed engine).
- Model no-reset mean total: 5:49:53 -> ~5:40 and falling; the remaining known gaps are
  the per-fight overhead constants (~1.5x on trainer groups), the still-~30+ min fill
  (real multi-species blocks not yet moved into the route), and scene-text guesses.
- A full `descend.py --passes 2` is running to retune every knob under the corrected
  engine (hunt steps, aggr, X lines, grinds are all stale). It self-commits accepts.
- **Plans 367-370's acceptances predate the engine fixes** — re-validate the chain
  (re-run the battery) after the descent settles.

## Next

1. Let the descent finish; re-ladder; re-run the WR-technique battery on the fixed engine.
2. Move multi-species catch blocks (Safari, Power Plant, Mansion) from the fill into the
   route proper (the WR structure) — now testable honestly.
3. Trainer-group overhead: measure a multi-mon trainer fight on tier 1 (frlg-calib has
   states for the lab rival) to pin MON_OVERHEAD/intro for trainer fights.
4. The seed-manip knob and the level-up-jingle measurement (carried from this morning).
