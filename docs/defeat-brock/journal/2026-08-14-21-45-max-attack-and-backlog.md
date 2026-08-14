# 2026-08-14 21:45 — the max-attack order, and the backlog worked end to end

Task (user): force a Squirtle with **attack stat 15**, full search, accept
frames lost at the ball; then do everything the docs list as not
optimised.

## Attack 15 does not exist at these levels — the arithmetic, cited

`CALC_STAT` (`decompiled/src/pokemon.c:2093-2096`) with Squirtle's base
attack 48 (`src/data/pokemon/species_info.h:212-239`) and
`ModifyStatByNature` (`:5404`):

| level | neutral max (IV 31) | Adamant max |
| --- | ---: | ---: |
| 5 (rival) | 11 | 12 |
| 6 (Sammy) | 12 | 13 |
| 7 | 13 | 14 |
| 8 | **15** | 16 |

Attack 15 first exists at **L8**, and L8 before Sammy needs 314 exp
against the ~203 the route has there — no trainer source exists, so it
would take ~5 deliberate wild kills at ~700+ frames each. Denied by
arithmetic, not by preference.

What the reachable maximum (Adamant, Atk IV >= 24 → atk 12 at L5, 13 at
L6) actually moves, through the gen-3 damage floors
(`CalculateBaseDamage` + `*gCritMultiplier`,
`decompiled/src/battle_script_commands.c` damagecalc; variance `:1558`):

- **Sammy, normal hits**: base 5 at atk 11, 12 *and* 13 (the /50 floor;
  base 6 needs atk 15). Unchanged.
- **Sammy, crit hits**: crit = 2 × (base incl. +2) → 10 at atk 11-13,
  variance 8/9/10. **Unchanged — a higher reachable attack does not make
  the 3-crit kill more likely or more lethal.** The 3-crit line stays a
  (1/16)^3 · 0.79 ≈ 1.9e-4 lottery at any reachable attack.
- **Rival, ungrowled**: base 5 at atk 10-12 (needs 13 for 6). Unchanged
  vs the committed atk-10... wait — atk 10 gives floor(155/50)=3+2=5 and
  atk 11/12 give the same 5; the gain over the *committed Docile* is
  only on **growled** turns (stage -1, ×2/3): eff 6 → base 3 (hits 3-4)
  at atk 10 vs eff 8 → base 4, stab 6 (hits 5-6) at atk 12. A 2-turn
  rival kill (turn-1 normal 7 max + turn-2 crit 14 max ≥ 19) needs the
  *ungrowled* base 5→stab 7 — open at atk >= 11, closed at 10 (max
  6+12 = 18 < 19).

So the executable version of the order is **Adamant / Atk IV >= 24 with
Spe IV >= 14** (spe 10, keeping move-first vs the rival's spe 9). The
extended ball-scan (0..256) has exactly three such rows with safe speed:
delay 44 (Atk IV 30), delay 142 (27), delay 218 (28). Wave 2 builds 44
and 142 with the committed wait dials and `FRLG_PRE_SWEEP=12,48` on
every fight, from the committed prefix.

## The symbol-pairing bug the session flushed out

`default_sym_path()` is FireRed's table, and the audit examples used it
for *any* ledger — on rival-1 (LeafGreen) every probe resolved to a
plausible wrong address, which is why the first rival-1 audit printed
zero battles. `frlg_emu::sym_path_for_rom` now pairs the `.sym` sibling
with the ROM (the CLI already did), the examples use it, and the
re-audited rival-1 is genuinely clean: no reversals, no idle runs, one
dogleg = Oak's scripted escort.

## Backlog items closed by measurement/inspection this session

- **rival-1 `02-intro-oak` (input vs timer)**: `examples/intro-slack.rs`
  injects one idle frame per 8-frame stride and replays the rest —
  191/196 probe points shift the segment end 1:1; only the final fade
  (~36 frames) absorbs. The intro is input-paced wall to wall; 1565 is
  this drive shape's measured floor.
- **rival-1 player-name letter**: the drive already takes the
  cursor-start letter with zero moves and all 1-char names print
  identically — no letter can be cheaper; the residual (exit-frame
  shift) is the closed seed dial.
- **Per-segment text_hold**: `FRLG_TEXT_HOLD_<SEGMENT>` now overrides
  the global knob for one segment (recorded in the log, no schema
  change). Sweep results below.

## Wave 2 and the remaining sweeps

(results below as they land)
