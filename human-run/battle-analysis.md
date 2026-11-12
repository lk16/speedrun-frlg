# Battles under the exact engine

Since plan `297-exact-baseline`, `Params::battle_sim` *is* the baseline:
`crates/frlg-human/src/battle.rs` simulates fights with the decomp's damage pipeline, stat
stages, status conditions, move effects, the ported trainer AI (CHECK_BAD_MOVE /
CHECK_VIABILITY / TRY_TO_FAINT, ties broken by `Random() % n`), trainer Full Restores, the
player's X items, potions/Full Restores, sacrificial switches and 4-move-slot movesets, all
rolled on the game's own LCG (`rng.rs::GameRng`). The module doc of `battle.rs` lists every
source line and every remaining approximation (no PP, no enemy switching, genderless
Attract, no-op Future Sight/Roar, weather trimmed to damage/accuracy effects).

`frlg-human battle <TRAINER_ID> <level> [--species S] [--moves A,B,..] [--add M] [--x
attack,special,..] [--sacs n] [--badges mask] [--heal-below f] [--league] [--n N]` probes one
fight; `FRLG_BATTLE_TRACE=1` prints per-turn traces. Badge mask bits: 1 Atk (Boulder),
4 Spe (Thunder), 16 Def (Soul), 64 SpA+SpD (Volcano).

## What the exact numbers changed vs the old level-ratio model (n = 800-1500 per cell)

| Fight | old model said | exact engine says |
| --- | --- | --- |
| Rival Cerulean, Wartortle 18 | 83% win | 34% (Pidgeotto Sand-Attack; Bulbasaur Sleep Powder + Leech Seed); 69% at L20 |
| Lt. Surge | L26: ~5% | L26 Dig-only 2-27%; **L28 + Dig 69%**; L27 + Dig + 2 sacs 59% |
| Erika | Wartortle 34: ~100% | **0%** (Giga Drain 2HKOs and heals back); Blastoise 36 + 3 X Attacks 90% |
| Sabrina | needs L42+ | with X Specials + X Speed comfortable at L40-41 |
| E4 R1 at L51 (+2 SpA +1 Spe, Blizzard) | 92-99% each | Lorelei 97 / Bruno 77 / Agatha 82 / Lance 90% |
| Champion R1 | 68% at L54 | SolarBeam OHKOs below ~L55: 28% at L52 (2 sacs), 43% at L53 |
| E4 R2, Mewtwo 70 (Psychic/Recover + HM Surf) | — | Lorelei 98 / Bruno 91 / Agatha 96 / Lance 81% |
| Champion rematch, Mewtwo 70 | — | lone: **0%** (Heracross Megahorn OHKO; Tyranitar is Psychic-immune); +2 X Special behind 2 sacs + Surf: **90%** |

The level thresholds are sharp (Surge 2% → 69% across two levels; Erika 0% → 90% across the
L36 evolution) because gen-3 damage is integer-coarse and the AI's TryToFaint locks onto
kills. This is what the old model's smooth logistic could not represent, and it moves the
optimum toward: more early exp (fight_until_level, grinds, unskipped trainer groups), TM
coverage (Mega Punch, Dig before the rival via `early_dig`, Blizzard), X-item setup with
sacrificial switches, and fighting Erika after the tower leg as a Blastoise.

## Standing facts for tactics (all decomp-cited in code)

* X items are +1 stage and exist only from Celadon Dept. 5F on — Brock, Misty, the early
  rivals and Surge are fought bare (sacs still work).
* A sacrifice buys exactly one free action; it answers moves that out-damage the fighter's
  maximum HP (Megahorn, SolarBeam) and costs a faint + send-out (~900 frames).
* Full Restores (player: from the Indigo mart; trainers: E4/Champion ×2-4 below 1/4 HP)
  make long fights swingy: the engine prices both.
* HMs can be re-taught freely: Mewtwo takes Surf for the rematch Tyranitar.
* The Thunder Badge's ×1.1 battle speed (player only) decides several speed races.

## Calibration status

The engine's *mechanics* are decomp-exact; its *frame costs* per turn/KO/whiteout and the
human-policy rules (heal thresholds, move choice, sac usage) are the uncalibrated parts.
Absolute EHT numbers remain scores of the model; tier-1 measurements of a few real fights
(a trainer battle's turn cadence with items, a League member end-to-end) are the next
calibration targets before trusting any absolute hour figure.
