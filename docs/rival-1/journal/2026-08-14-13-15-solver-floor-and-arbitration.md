# 2026-08-14 13:15 — the solver put to work on rival-1: floor 2392, and the committed 2409 survives everything

The constraint solver + battle engine (built yesterday on the defeat-brock
side, validated against this battle) were pointed at the committed rival-1
route to extract frames. Outcome up front: **no frames — but three open
questions closed with hard numbers**, which is what the solver is actually
for. The committed 9658 stands, now with much stronger evidence that its
battle is at a real optimum.

## 1. The global floor: no start state plays this battle below 2392

`examples/global-floor.rs`: sample the 2^32 battle-start space with
`engine::simulate` (2^21 anchors through a bijective bit-mixer, plan grid
d0 × {0,4}-turn-delays — delay 4 unlocks the 8-frame commit gate,
`pacing::commit_durations`, net −1/turn when it cooperates), then hand the
fastest classes to `trace::extract_leaf` + `ConstraintSet::count_all` for
exact counts. Results (best-leaf per anchor, i.e. gates optimistic):

- **Floor: 2392** (plan `[3,4,4,4]`, gates `[8,8,8]`), exact density
  2.17e-4. 2393: 1.10e-4. 2394: 1.54e-4. 2395: 8.83e-4.
- With the wider plan grid, 100% of anchors have *some* winning leaf.
- Committed 2409 sits at roughly the median of the best-leaf distribution.

Two consequences, both permanent for this fight (mons fixed — see §5 for
why creation manipulation cannot move the damage numbers):

- **Nothing can buy more than 2409 − 2392 = 17 battle frames.** Every
  manipulation is now priced against that.
- **The naming-exit seed dial is closed.** A seed at N idle frames must
  beat 2409 by more than N; N ≥ 17 is arithmetically dead, and N = 1..24
  was already emulator-searched to a loss on 2026-08-12 (`seed-sample`,
  best +8). Nothing left to sample.

Measurement lesson recorded in the example: the first cut sampled anchors
as `i * odd_constant`, and that sequence correlates with the constraints'
own affine maps badly enough to bias class densities several-fold. Caught
by cross-checking a sampled bin against `count_all` (9.4e-5 sampled vs
8.6e-4 exact); fixed with a murmur3-style mixer. The solver double-checks
the sampler now — that redundancy stays.

## 2. Arbitration: every model-named candidate below 2409 measured, none real

`examples/arbitrate.rs` + `arbitrate-list.rs`, all on libmgba from the
committed battle-start state (frame 7249, `gRngValue 0xed94271d`, replayed
from reset — no RAM writes, because the commit gates are scene state and a
poked anchor would not arbitrate them faithfully):

- **The dense plan grid** (d0 0..5 × delays 0..12 per turn — supersedes
  pure-search's even-only grid) names 217 plans with a winning leaf below
  2409. The best twenty (all predicted 2405, all riding the 8-frame gate)
  played **2410 or 2688** for real. In 40+ replays across delays 4–11,
  **gate 8 never materialized once**.
- **The low-gate family** (all delays ≤ 3, gates {13,18} only — the regime
  the model demonstrably gets right): best prediction 2406, five plans,
  all with turn-3 delay 0 — which `win_battle`'s `TURN_DELAYS = 1..` has
  *never tried on any target*. Real: **2496**, which is exactly the
  `[13,13,18]` leaf of those plans — so the engine's streams are right and
  the gate simply resolved to 18. `[4,3,3,1]` → 2412, `[4,3,3,2]` → 2413,
  matching leaves likewise.
- **The remaining delay-0 gap** closed by direct measurement: `[4,0,3,3]`
  2697, `[4,3,0,3]` 2687, `[4,0,0,3]` 2887, `[4,1,3,3]` 2789, `[4,3,1,3]`
  2779, `[4,2,3,3]` 2898, `[4,3,2,3]` 2615. The committed `[4,3,3,3]` =
  2409 (re-measured) is a sharp local optimum: every ±1 neighbour is +3 to
  +489.

The headline finding about the model: **the engine's leaf sets are correct
(every measured battle equals one of its leaves), but the unmodelled gates
consistently resolve against the 3–5-frame margins.** On this anchor the
gates happen to favour exactly the committed plan's best leaf. So: the
engine is a truthful *enumerator* and a fine pruner/orderer, but adoption
decisions must stay with the emulator — the planned "collapse win_battle
stages to model scans" (yesterday's backlog item 2) should be a
*pre-ordering plus arbitration*, not a replacement.

## 3. Pre-battle waits: priced, measured, dead

w idle frames physically inserted at the head of `08-battle-start`
(`arbitrate.rs` lever 2), w = 1..16 (17+ dead by the floor):

- The measured anchor is **exactly `anchor.jump(w)` for w ≤ 5**; from
  w = 6 a 4-roll event slides into the pre-battle window
  (`anchor(w) = jump(w)` plus 4 extra rolls, constant through w = 16 —
  consistent with one scheduled NPC/object event crossing the battle-start
  boundary; not attributed further, and doesn't need to be).
- Engine bars (`best_total − w`) left 0 candidates at most w; the
  arbitrated candidates at w ∈ {2,4,6} all played their slow leaves
  (~2690–2900 real). **No wait wins.** Note the engine's `INTRO_PRETURN`
  table was fitted at the unshifted entry only; the wait results are
  consistent with the leaf sets' slow ends, so whether the table transfers
  to shifted entries was not separately established — it didn't matter,
  nothing came close to the bar.

## 4. What was NOT re-searched

The whole-route levers upstream of the seed (naming letter, per-segment
text_hold, the 02-intro-oak wait audit) are untouched — they are not
solver-shaped. The one stream lever left unexplored is the **fat-man
zero-cost shift**: a same-length Pallet Town path variant changes when he
spawns and therefore what he rolls, moving the battle anchor at zero frame
cost. With P(best-leaf < 2409) ≈ 40% per anchor but gates eating small
margins in practice, the expected value is a few frames at the cost of a
path-variant search plus per-variant emulator battle searches. Priced,
not started.

## 5. Starter creation manipulation: closed analytically

Backlog item 4 ("starter manipulation at the ball") cannot shorten this
battle. Tackle's damage is locked at 5/10 (crit) across the entire
reachable stat range: the damage formula (`CalculateBaseDamage`,
`decompiled/src/pokemon.c:2385`, `/50` truncation at level 5) needs
Atk ≥ 13 for a 6-damage Tackle (13·35·4/9/50 = 4, +2 = 6, vs 12·35·4/9/50
= 3, +2 = 5), while the best reachable Atk is 12: base 49
(`decompiled/src/data/pokemon/species_info.h:41`), so
`(2·49+31)·5/100 + 5 = 11` (`CALC_STAT`, `decompiled/src/pokemon.c:
2093-2096`), ×110/100 truncating for a +Atk nature = 12
(`ModifyStatByNature`, `:5404-5427`). A +Def nature/IV mon (Def 12 turns
the rival's Scratch from 5 into 4 base damage) saves only 2–5 frames per
incoming hit in HP-bar drain (`pacing::uhp_drain`), at ~1/80 density on
the creation stream — an ~80-frame expected wait for ≤10 frames. Dead on
arithmetic; no ball_delay knob needed.

## Where this leaves rival-1

9658 total, battle 2409 vs a proven floor of 2392, with every model-visible
path to that floor measured and refused by gate resolution. The honest
next moves, in value order: (1) the 02-intro-oak input-gated-vs-timer-gated
audit (1565 frames of scripted beats nobody has decomposed), (2) per-box
text_hold, (3) the fat-man shift above. All are outside the solver's
current reach; none was made cheaper today, but none is blocked either.
