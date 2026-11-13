# Engine audit: why simulated fights run long

Scope: `crates/frlg-human/src/battle.rs` (engine + AI port + `fight` policy), `sim.rs` (glue), against
`decompiled/`. Nothing was edited. All numbers: `target/release/frlg-human battle ... --n 400-1000`, carry IV 20,
no EVs, `--heal-below 0.4` unless stated. Plan-sim numbers come from
`FRLG_SIM_TRACE=1 frlg-human eval human-run/plans/370-clemi-skip.md --rung WR --R 1` (two sampled runs).

## Headline

The engine's *mechanics* check out against the decomp (damage pipeline, crit table, STAB, badge boosts, accuracy
roll, AI flag bits, tie-break). The "stall-move spam" hypothesis is mostly a red herring: enemy stall moves only
cost the player turns through Sand-Attack misses and Withdraw vs physical moves (< 1 turn per fight). Fight
length is set by the player's kill speed (damage per hit vs enemy HP), and the long outliers come from
(1) PP starvation of the carry's good moves, (2) a weak, level-limited carry offence with no Water Pulse, and
(3) healing turns. None of these is an AI-port bug.

## 1. Turn table

### a) `battle` CLI at the "default" setup (last four learnset moves, no tactics, full PP)

| Fight | carry | moves | turns | win |
| --- | --- | --- | --- | --- |
| RIVAL_CERULEAN_SQUIRTLE | L19 | Tackle/Bubble/WaterGun/Bite | 14.2 | 1.00 |
| same, `--heal-below 0` | L19 | same | 13.1 | 0.92 |
| same | L19 | MegaPunch/WaterGun/Bite | 13.5 | 1.00 |
| same | L19 | WaterPulse/WaterGun/Bite | 11.7 (heal 0) | 0.97 |
| same | L22 | WaterPulse/WaterGun/Bite | 7.7 (heal 0) | 1.00 |
| same | L19, IV 31 | WaterPulse/WaterGun/Bite | 10.2 | 0.995 |
| RIVAL_SS_ANNE_SQUIRTLE | L22 | default | 18.0 | 0.95 |
| RIVAL_POKEMON_TOWER_SQUIRTLE | L25 | default | 42.8 | 0.75 (under-levelled probe) |
| RIVAL_SILPH_SQUIRTLE | L30 | default | 40.9 | 0.23 (under-levelled probe) |
| LEADER_MISTY | L19 | default | 10.8 | 0.99 |
| LEADER_LT_SURGE | L24 | default | 27.6 | 0.28 (under-levelled probe) |
| LEADER_ERIKA | L30 | Dig/Blizzard/WaterGun/Bite | 24.2 | 0.55 (under-levelled probe) |
| E4 Lorelei/Bruno/Agatha/Lance | L55, EQ/Blizzard/Surf/Bite, `--league`, no X | 40.6 / 17.0 / 41.8 / 50.8 | 0.998/0.64/0.75/0.71 |
| Champion first | L55 | same | 35.0 | 0.40 |

### b) Same fights at the levels/moves/tactics the plan actually reaches (full PP)

| Fight | level | tactics | turns | win |
| --- | --- | --- | --- | --- |
| Rival Cerulean | 18 | Tackle/WG/Dig/MegaPunch | 16.7 | 1.00 |
| Misty | 23 | Bite/WG/Dig/MP | 6.7 | 1.00 |
| Rival SS Anne | 25 | same | 11.9 | 1.00 |
| Surge | 28 | same | 14.9 | 0.72 |
| Rival Tower | 34 | same | 11.7 | 0.99 |
| Erika | 37 | +3 X Atk | 9.2 | 0.91 |
| Koga | 39 | Surf/WG/Dig/MP | 11.2 | 0.69 |
| Rival Silph | 40 | +3 X Spc | 18.5 | 0.98 |
| Sabrina | 43 | +3 X Spc | 7.4 | 0.96 |
| Blaine | 45 | Surf/Blizzard/Dig/MP | 6.1 | 1.00 |
| Giovanni | 48 | Surf/Blz/Dig/EQ | 5.0 | 1.00 |
| Rival Route22 late | 49 | +3 X Spc | 12.3 | 0.83 |
| Lorelei / Bruno / Agatha / Lance | 56 | +5 X Spc, `--league` | 18.9 / 11.2 / 12.1 / 19.0 | 1.0/0.97/0.98/0.92 |
| Champion first | 58 | +5 X Spc | 25.3 | 0.56 |

Reading: at plan-realistic levels with X items the fights are 5-19 turns. Turn counts of 35-50 (E4 without X
items, Tower/Silph/Surge/Erika at the wrong level) appear only when a probe under-levels the carry or omits the
tactics the plan uses.

### c) Outliers inside the plan sim (single-carry run, real PP state)

| Fight (plan trace) | level | PP state (slots 1-4) | turns | same fight at full PP (b) |
| --- | --- | --- | --- | --- |
| rival-ss-anne (attempt 1) | 25 | [0, 6, 8, 0] (Bite, MP empty) | 15-17 (lost) | 11.9 |
| rival-tower | 34 | [9,10,10,9] / [18,23,10,9] | 11, 11, **25** (lost twice) | 11.7 |
| blaine | 45 | [0, 0, 6, 0] (Surf, Blizz, MP empty; Dig only) | **20** | 6.1 (Dig-only probe: 26.8) |
| agatha | 58 | [0, 3, 10, 0] | 16 (lost) | n/a |
| rival-route22 | 49 | [0,0,10,4] / [9,0,10,10] | 18-20 (lost x2) | 12.3 |
| erika | 37 | [1, 23, 10, 4] | 9-12 | 9.2 |

These are the "2-3x" cases. They are not mechanics errors; they come from the PP state (root cause A).

## 2. Enemy AI port vs `battle_ai_script_commands.c` / `battle_ai_scripts.s`

Verified faithful:
* Flag bits: decomp `AI_SCRIPT_CHECK_VIABILITY = 1<<1`, `TRY_TO_FAINT = 1<<2`
  (`decompiled/include/constants/battle_ai.h:38-39`); engine tests `& 2` for viability and `& 4` for faint
  (`battle.rs:943`, `:1201`). Trainer ai 7 therefore runs all three.
* Scores start at 100, unusable moves 0 (`battle_ai_script_commands.c:299-306`); TRY_TO_FAINT +4 (+6 Quick
  Attack), `-1` for damaging non-strongest moves (`battle_ai_scripts.s:2767-2788`); `simulatedRNG` 85-100 per move
  (`:310`). Engine: `battle.rs:1201-1208`, `ai_damage` `:767`.
* Tie-break is `Random() % numOfBestMoves` over all max-score moves (`battle_ai_script_commands.c:389-408`); engine
  `battle.rs:1210-1214` is the same uniform pick.

Deviations found (none of them lengthens fights; most make *real* trainers stall slightly more):

1. Tie-break quirk not ported (`battle_ai_script_commands.c:393-406`). When a later move strictly beats the running
   best, the second `if` also fires, so the new best is stored twice; with a later tie it gets 2/3 instead of 1/2
   weight. Engine picks uniformly (`battle.rs:1213`). Effect: tiny shift toward later move slots.
2. `AI_CV_DefenseUp` (Withdraw/Harden/Defense Curl/Barrier/Acid Armor) gives +2 at 100% HP with p=1/2 when
   stat level < 9 (`battle_ai_scripts.s:927-936`). Engine's DefenseUp arm only applies `-2 at HP<=40`
   (`battle.rs:1052-1058`). Real enemies pick Withdraw more often (about 50% of the time at full HP, always
   ahead of damaging moves when the +2 lands). Engine sample: Withdraw 35% of enemy-Squirtle turns even without
   the +2. Not length-increasing for special-attack carries; matters for physical carries (Mega Punch, Tackle).
3. Stat-down family is lumped under `AI_CV_AttackDown` (`battle.rs:1059-1075`: -1 base, -1 more at HP<=90).
   Real `AI_CV_DefenseDown` (Tail Whip, Leer) has no base penalty (`battle_ai_scripts.s:1165-1178`): score stays
   100 unless target HP <= 70, so Tail Whip ties with attacks. Engine never picks Tail Whip (0 uses in 400
   Cerulean fights with Rattata and Squirtle holding it); the real game does. Lowers the carry's Def, which only
   hurts the carry; it does not lengthen the carry's kill time.
4. `ai_damage` returns 0 for Charge/Dig/Fly/Recharge/DreamEater (`battle.rs:775-785`), but the decomp
   `Cmd_if_can_faint` (`battle_ai_script_commands.c:1475-1500`) has no effect filter: only
   `get_how_powerful_move_is` filters. Real Hyper Beam/Solar Beam/Fly can earn the +4 can-faint bonus; engine
   never gives it. Direction: real E4 (Lance Hyper Beam, 3/fight) would kill more reliably.
5. Status-move `get_how_powerful_move_is` returns POWER_DISCOURAGED (no -1), so a status move at 100 ties with
   the strongest attack (also at 100) and wins against weaker attacks (99). Engine reproduces this (only damaging
   moves get -1, `battle.rs:1205`).

Quantified effect of enemy stalling on Cerulean (400 runs, `FRLG_BATTLE_TRACE`): enemy actions are Teleport 2.4/
fight (Abra has only Teleport, `trainer_parties.h:3800`, faithful), Withdraw 1.8, Sand Attack 0.6, the rest damage.
Only Sand Attack (-1 acc, 0.6 uses) and Withdraw cost the carry anything; together < 1 turn per fight.

Conclusion for task 2: tie-breaking is faithful in structure and, if anything, *under*-stalls relative to real.
It cannot explain a 2-3x turn excess.

## 3. Player policy and damage

* The player is a fixed, full-knowledge policy (`fight`, `battle.rs:1842` ff.): heal rules, X items in order, sac, else
  `killer.or(best)`. `killer` = a move whose 85%-roll kills, accuracy >= 95 or AlwaysHit, preferring higher priority
  (`battle.rs:1921-1960`); `best` = max of `calc_damage * 0.925 * accuracy` (`player_score`, `battle.rs:725-763`).
  No damage-roll pessimism that lengthens fights: the 85% test only decides whether a "sure kill" exists.
* Damage formula matches `CalculateBaseDamage`/`Cmd_damagecalc`: `((2L/5+2)*P*A/D)/50`, burn/screens, `+2`, crit x2
  after the +2 (`battle_script_commands.c:1215`), STAB x1.5, type chart, 85-100 roll
  (`battle.rs:484-595`). Badge boosts: Boulder (Atk), Soul (Def), Volcano (SpA+SpD) match `pokemon.c:2440-2446`.
  Note Bite and Water Gun are special in Gen 3 (type-based split), so the Boulder badge does nothing for a Squirtle
  line that uses Water/Dark moves (explains `--badges 1` not changing Cerulean).
* Crit table {16,8,4,3,2} matches `battle_script_commands.c:588`. The model has no deliberate crit loss.
* Enemy stats: `[m.iv;6]` with the party's IV scaled (Pidgeotto L17 HP = 49, consistent with IV 50*31/255).
  Enemy natures neutral, enemy EVs 0 (real trainer mons: same, nature is personality-derived: unmodelled, +-10%).
* Where the policy costs turns:
  - `Action::Heal` fires at HP < `heal_below` (0.4) whenever `heal + hp > danger` (`battle.rs:1912`), with no check
    that the foe's best hit actually threatens `me.hp`. In the Cerulean trace the carry drinks a Super Potion at
    20/55 HP against a 7-damage foe (danger 7). Cost: Heal 0.85 turns/fight on Cerulean (6%); `--heal-below 0`
    gives 13.1 vs 14.2 but loses 8 points of win rate, so the policy is not simply wrong, just crude.
  - League `FullRestore` is unlimited (`full_restore: bool`, no count) and fires on sleep/freeze/confusion
    as well as danger: 6.3 (Lorelei), 7.8 (Agatha), 10.8 (Lance) uses per fight, 16-30% of turns. Lowering
    `heal_below` to 0.25/0.1 barely moves E4 turns (39.2 -> 38.1 -> 37.5), because the `in_danger` and status
    triggers dominate.
  - The carry's `Mega Punch` (85%) is picked over Bite/Water Gun on expected damage; the 15% misses add ~0.3 turns
    per fight but win it back in damage. Not an error.

## 4. Candidate root causes

### A. PP starvation of the carry's strong moves (largest single outlier source)

Evidence:
* Plan trace: Blaine at L45 with `pp[0,0,6,0]` takes 20 turns; same fight at full PP, 6.1 (`battle` CLI); Dig-only
  probe 26.8, so the trace matches "forced down to the one move with PP left". Rival SS Anne 15-17 turns with
  Bite and Mega Punch at 0 (11.9 at full PP). Rival Tower 25 turns. Agatha `pp[0,3,10,0]` lost.
* Cause in engine: `fight` always picks the highest expected-damage move (`battle.rs:1921-1960`), even against
  trash trainers where Water Gun would do. Rock Tunnel/Nugget Bridge/'group' fights (4-8 turns each, 1 slot spent
  per turn) drain Bite (25 PP), Surf (15), Blizzard (5) before the fights that matter.
* Refill rule in `sim.rs:986`: `pp_low = sum(slot_pp) < 12 || (league && any slot == 0)`. A carry with two empty
  slots and Dig at 10 PP is "not low" (sum 20) before a non-league leader/rival fight, so no Elixir is used.
* Real game: PP drains too, but a human saves the best move for the named fights and carries Elixirs/Ethers.

Expected magnitude: in the affected fights 1.5-3.3x turns (examples above); across the whole run roughly 10-20% of
total fight turns and a visible share of the model's "lost fight, retry" cost. This is the best explanation of
"2-3x longer" for named fights; it does not explain the Cerulean example (PP is full there).

Suggested change: in `fight`, when the foe is a non-named trainer (or whenever a weaker move still kills / is within
10% of `best`), pick the move with the lowest PP value; and make `sim.rs:986` `pp_low` trigger on "best two slots
below their per-fight need" (e.g. `slot_pp[best_k] < 6`), not on the sum.

### B. Weak, level-limited carry offence (Cerulean specifically)

Evidence: turns are `sum(ceil(foeHP / hit))`. Wartortle L19 Water Gun does ~15 against 34-49 HP foes
(Pidgeotto 49 HP: 3.9 turns; Squirtle 46 HP: 5.3 turns; Abra 34 HP, Teleport only, 2.4 turns: all derived from
the decomp party at `trainer_parties.h:3789-3814`). Sensitivity is steep: L19 -> L22 = 11.7 -> 7.7 turns (-34%);
Water Pulse instead of Tackle/Bubble = 13.1 -> 11.7 (-10%); IV 20 -> 31 = -13%.
The plan teaches Water Pulse only to Mewtwo (`370-clemi-skip.md`, `"mv": "MOVE_WATER_PULSE", "mewtwo": true`), while
the WR run has it on the Wartortle from Misty on (`video-analysis/wr-3.25.25-main97.md:~107`).
The plan also reaches Cerulean at L18 by design (`fight_until_level` 15); the sim's per-fight level is a plan
parameter, not an engine defect.

Expected magnitude: +3 carry levels = -1/3 of turns in early rival/leader fights; a carry-held Water Pulse = -10%
from Misty onward. Combined, early fights drop to roughly 8-10 turns, which is the WR's range.

Suggested change: let the plan teach TM03 Water Pulse to the carry (Misty gift) and offer `Mewtwo` its own copy
only if a second TM source exists; and calibrate `fight_until_level`/rare-candy timing so Cerulean is L20+.

### C. Crude heal rule (small, broad)

Evidence: `battle.rs:1912` (`my_frac < heal_below && heal + hp > danger`) heals when the foe's best move is far
below current HP; `battle.rs:1890-1893` has the correct `in_danger` guard but the first arm doesn't use it.
Impact: 0.85 heal turns/fight on Cerulean (~6%), more on long fights; League FR is unlimited and also status-
triggered (16-30% of E4 turns).

Expected magnitude: -5% on early fights, -10% on long ones; E4 unchanged without capped FR counts.

Suggested change: require `me.hp <= danger * 2` (or `in_danger`) for the `heal_below` arm; give `Tactics` a
`full_restores: u32` budget so League fights price their real supply.

### D. Faithfulness gaps (do not shorten fights)

* Missing tie-break double weighting (`battle.rs:1213` vs `battle_ai_script_commands.c:393-406`).
* DefenseUp full-HP +2 missing (`battle.rs:1052` vs `battle_ai_scripts.s:927-936`).
* Tail Whip/Leer lumped under AttackDown (`battle.rs:1059` vs `battle_ai_scripts.s:1165-1178`).
* Charge/recharge moves excluded from the can-faint bonus (`battle.rs:775-785` vs
  `battle_ai_script_commands.c:1475`).
Fixing these makes the AI *more* faithful and the E4 slightly harder; it does not cut turn counts.

### E. Measurement note

The `battle` CLI defaults (last four learnset moves, no X items, full Full-Restore supply) overstate turns for
late fights: E4 at L55 without X items is 35-50 turns, with 5 X Specials and Surf/EQ/Blizzard at L56 it is 11-19
(table b). The real WR uses 20 X Specials and Mewtwo. Compare model and WR on the same tactics.

## Answers to the task's specific questions

* Real tie-break: uniform random among max scores with the double-count quirk; engine: uniform. Faithful enough.
* Growl/Tail Whip/Harden spam: real AI spams Withdraw/Harden *more* (+2 at full HP), Tail Whip is unpenalised;
  engine spams less. Not a length driver for a special-move carry.
* Expected vs rolled damage: AI's `ai_damage` rolls 85-100 per move per turn exactly like `simulatedRNG`; the player
  uses 0.925 expected for choice and a 0.85 roll for "sure kill". No pessimism beyond that.
* Fixed policy: yes, one deterministic policy per fight (`Tactics` + heal thresholds), not an optimiser per turn.
* STAB/badge/crit: present and decomp-correct.

Key files: `/home/luuk/projects/speedrun-frlg/crates/frlg-human/src/battle.rs`,
`/home/luuk/projects/speedrun-frlg/crates/frlg-human/src/sim.rs`,
`/home/luuk/projects/speedrun-frlg/decompiled/data/battle_ai_scripts.s`,
`/home/luuk/projects/speedrun-frlg/decompiled/src/battle_ai_script_commands.c`.
