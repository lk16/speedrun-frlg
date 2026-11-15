# 2026-11-23: League retune under the corrected stand-ins: diagnostics so far (descent still running)

Start: CURRENT = 1017-wrfacts, WR+2m 913 / 872 h (R 3.2M seeds 11 / 12, 809 / 846 finishes; `eval` takes 21 s at 3.2M, so descents run at 3.2M with a seed-12 confirmation).

## 1. Where the model loses attempts in the League (1017 with the League `fatal` rows removed, lenient rung WR+160m, R 400k seed 3)

Engine-only survival per fight (what the sim's own whiteouts do, with the plan's tactics): Lorelei 100%, Bruno 92%, Agatha 94%, Lance 82%, **Champion 40%**; round 2: Lorelei 99%, Bruno 100%, Agatha 99%, Lance 97%, **Champion-2 39%**.
(The plan's flat `fatal` hazards come on top of that: .04 / .18 / .05 / .25 / .10 and .07 / .07 / .07 / .14 / .11; they are the calibration residual S of ROBUSTNESS 4, not engine output.)
So the WR rung is starved by the two Champion fights and Lance, not by Lorelei. Per-fight table from a single-threaded `FRLG_SIM_TRACE` run (heal items at fight start `hi0`, Full Restores used `FR`, mean player actions `turns`; WR recount in brackets):
Lorelei 13.8 turns (13), FR 1.4 (WR 1); Bruno 7.4 (9), FR .54 (1); Agatha 6.5 (7), FR .42 (0); Lance 13.0 (10), FR 2.3; Champion 11.8 (12) at a 38% win rate; Champion-2 11.3 (11.5), FR 3.0, 39% win rate.
Heal items are not the limit (hi0 5-10 at R1 Champion): the fights are lost on damage taken while the queue is spent.

## 2. What differs at the Champion: Mist and the AI (engine fix, commit c6d8084)

VOD frames 2:46:23-2:46:40 (video 9975-10040 s, `frame-sheet.sh`): the WR's carry enters the Champion at 165/165 (Lv55), Pidgeot leads, opens with Sand Attack, then Aerial Ace (165 -> 129, i.e. 36 = inside the engine's 36-43 range),
then Sand Attack / Feather Dance into the carry's Mist twice ("A is protected by MIST!" x2) before Surf one-shots Pidgeot at +3 and Venusaur follows. The WR's Guard Spec turn is not wasted: **the trainer AI keeps choosing the stat-drop moves into Mist.**
The engine's `ai_choose` gave every stat drop -10 when the target side had Mist (`target_side.mist > 0` in the CBM block). The decomp has no such test: `AI_CBM_AttackDown` / `AccuracyDown` read the target's stage (0 = -6) and its ability only
(`decompiled/data/battle_ai_scripts.s:278-286`, `CheckIfAbilityBlocksStatChange`); `AI_CBM_Mist` is `Score_Minus8` (`:382`), not -10. Fixed both, test `trainer_ai_ignores_mist` (Sand Attack ties Aerial Ace at 100 and is picked about half the time with or without Mist).
Effect, `battle` Champion, WR bag (Guard Spec, X Speed, 3 X Special) at Lv55 / 160 HP: win rate 0.456 -> **0.654** (the plan's queue without Guard Spec 0.33). With the plan changed to the WR's Champion queue (`guard, speed, special, special, special`, 2 Guard Specs bought) EHT at WR+2m 913 -> 858 h; with only two X Specials 759 h (R 3.2M seed 11). 1017 itself does not change (it never used Guard Spec).
The WR's other Guard Spec use (Champion only, bag 2 -> 1) is the only one in the recount.

## 3. Hypotheses checked and not the cause

* Damage engine: the WR bar readings (frame-facts) agree; Aerial Ace's 36 on the real carry is inside the engine's range, so foe damage is not inflated either.
* HP / Torrent state: the WR enters the Champion at full HP (165/165), as the engine does. `league_torrent` has zero EHT effect (journal 2026-11-16: the window is never reachable after the engine's fights).
* AI items: Champion carries four Full Restores in the engine too (`ShouldUseItem`, hp < max/4, `battle_ai_switch_items.c:592`); the WR frames show no Champion FR in the first 60 s, so nothing is wrong yet, not enough evidence to change.
* Rotating send-out order: reproduced (journal 2026-11-21).

## 4. Descent so far (`lgdesc.py`, League queues / heals / stock / preheal / setup_min / rollouts, 293 candidates, 3% bar plus seed-12 confirmation)

97 of 293 candidates done (Lorelei, Bruno, Agatha, part of Lance): none beats 1017 by 3% yet; adding an X item to Lorelei costs +3% to +30%; removing one costs about the same; `heal_below` per fight is flat.
The only wins are the Guard Spec lines of section 2 (not yet reached by the descent) and Lance +special (892 h, -2%, below the bar). The descent continues; results and accepts go to scores.jsonl with the plan files.
