# Research: the wild encounter pipeline, roll by roll

Derived 2026-08-12 from `decompiled/` only; citations on every claim. This is the spec the
Rust model implements and the emulator tests validate. Items marked *needs-emulator* are
where the source text alone cannot pin behaviour.

## Two RNGs, not one

- `Random()`: `gRngValue = 1103515245·x + 24691`, top 16 bits (`src/random.c:9-13`,
  `include/random.h:19-20`). Advances once per VBlank always (`src/main.c:412`), twice per
  frame in battle (`src/battle_main.c:1647-1650`).
- **`WildEncounterRandom()`**: its own LCG `x = 1103515245·x + 12345` over
  `sWildEncounterData.rngState` (`src/wild_encounter.c:667-671`, `include/random.h:20`),
  used **only** by the encounter-rate dice roll (`DoWildEncounterRateDiceRoll`,
  `src/wild_encounter.c:302-307`). Seeding: `SeedWildEncounterRng(Random())` runs from
  `ResetMenuAndMonGlobals` (`src/new_game.c:103`), which a boot reaches **twice** — the
  copyright screen (`src/intro.c:1004`) and the title-screen exit (`src/title_screen.c:737`,
  immediately after `SeedRngAndSetTrainerId` at `:735`). *Measured on the committed rival-1
  route (frlg-mon `tests/emulator.rs`): seeds at frames 447 and 594, the second on the same
  frame as the main reseed.* So the live wild seed is one `Random()` output of the
  just-timer-seeded main stream: **the title-exit press timing picks both streams at once**,
  and the naming-screen reseed later does not touch the wild one. The state lives in EWRAM,
  not the save (`src/wild_encounter.c:34`).

**Consequence:** whether an *eligible* grass step passes the 21%/14% rate test depends only
on (seed, number of rate tests so far) — not on frame timing. Frame delays reach only the
`gRngValue`-based gates below. Encounter dodging is therefore mostly a *path/step-count
shaping* problem against a precomputable pass/fail sequence, plus one seed dial (anything
that changes the count of `Random()` calls before `src/new_game.c:103` re-picks the whole
wild stream).

## Per-step decision procedure (land)

The check fires on tile-center frames of a real step, not on forced-movement tiles
(`src/field_control_avatar.c:137-143`, `:98`); step-based scripts run first and can skip it
(`:215-228`). Tile eligibility is the metatile attribute's *encounter type* (bits 24-26 =
`TILE_ENCOUNTER_LAND`), not its behavior bits (`src/fieldmap.c:63-83, 377-383`,
`include/global.fieldmap.h:38-43`).

In order (`TryStandardWildEncounter`, `src/wild_encounter.c:757-776`; `StandardWildEncounter`
`:355-403`):

1. **Cooldown gate** (`HandleWildEncounterCooldown`, `:707-755`): `minSteps` from the map's
   encounter rate — `8 - rate/10` (Route 1/2 rate 21 → 6; Viridian Forest rate 14 → 7;
   `GetMapBaseEncounterCooldown`, `:673-699`). While `stepsSinceLastEncounter < minSteps`:
   increment and pass only if `Random()%100 < 5` (1 `gRngValue` roll, default modifiers).
   At/after `minSteps`: passes with **zero** rolls. `stepsSinceLastEncounter` and the rate
   buff reset on map load/warp (`src/overworld.c:764,799`), battle start
   (`src/battle_setup.c:205`), and successful encounter (`:766-767`).
2. **Behavior-change roll**: only if this tile's behavior ≠ `prevMetatileBehavior`:
   `Random()%100 >= 60` aborts (1 roll, `DoGlobalWildEncounterDiceRoll`, `:348-353`,
   invoked `:370-371`). `prevMetatileBehavior` updates on *every* TryStandardWildEncounter
   call including early-outs (`:761,768,773`).
3. **Rate test** (second LCG only, 0 `gRngValue` rolls): `rate = mapRate·16`, ×0.8 on bike,
   `+ buff·16/200`, flute/cleanse-tag/ability mods, clamp 1600; pass iff
   `WildEncounterRandom()%1600 < rate` (`:302-332`). On fail:
   `encounterRateBuff += mapRate` (`AddToWildEncounterRateBuff`, `:778-784`; forced to 0
   while a repel is active).
4. **Roamer**: 1 `Random()` only if a roamer is on this map (`src/roamer.c:228-239`) — not
   the case anywhere pre-Brock.
5. **Slot**: `Random()%100` against cumulative land thresholds 20/40/50/60/70/80/85/90/94/
   98/99/100 (`ChooseWildMonIndex_Land`, `:71-99`; rates from
   `src/data/wild_encounters.json:7-21`).
6. **Level**: `lo + Random()%(hi-lo+1)` — consumed even when `hi==lo` (`:155-174`), which is
   every slot on our maps.
7. **Repel level filter** — after slot+level are already rolled (`:286-289`).
8. **Mon generation** (`GenerateWildMon`, `:226-241`, non-Unown): nature = `Random()%25`
   (`:233`), then `CreateMonWithNature` rerolls `pid = Random32()` until
   `pid % 25 == nature` (`src/pokemon.c:1864-1875`; 2 rolls per iteration, geometric with
   mean 25 iterations), then 2 IV rolls (`src/pokemon.c:1833-1853`). No extra OT/ability/
   gender rolls (fixed personality path, `src/pokemon.c:1775-1802`). *Needs-emulator:* the
   low/high evaluation order inside `Random32()`'s `(Random() | (Random() << 16))`
   (`include/random.h:14`) is compiler-determined.
9. **Battle entry**: wild transition never rolls (`GetWildBattleTransition`,
   `src/battle_setup.c:612-622`; the only transition with a `Random()` is unreachable for
   wilds); `CB2_InitBattle` consumes the usual 3 (`src/load_save.c:69-83` offset, `:126`
   key×2); `gRandomTurnNumber = Random()` before turn 1 (`src/battle_main.c:2926`) and per
   turn (`:2999`).

Total for a realized encounter: `5 + 2k` `gRngValue` rolls (k = PID iterations) plus the
gates that applied; the rate test itself costs `gRngValue` nothing.

## Encounter tables (all slots single-level)

`src/data/wild_encounters.json`; FR and LG Route 1/2 are identical; the forest differs only
in Metapod↔Kakuna placement.

- **Route 1** (rate 21; FR `:8258-8325`, LG `:20801-20867`): Pidgey/Rattata L2-5 across all
  12 slots.
- **Route 2** (rate 21; FR `:8327-8394`, LG `:20870-20936`): Rattata/Pidgey L2-5 slots 0-7,
  Caterpie/Weedle L4-5 slots 8-11.
- **Viridian Forest** (rate 14; FR `:563-631`, LG `:13106-13172`): Caterpie/Weedle/
  Metapod-or-Kakuna L3-6, Pikachu L3 (slot 9, 4%) and L5 (slot 11, 1%). FR slots 6/7/8/10 =
  Metapod-flavoured Kakuna set; LG swaps Kakuna→Metapod.

## Suppression facts

- No lead-ability encounter early-outs exist in FRLG (`GenerateWildMon` has none; ability
  touches rate only, `:334-346`).
- `sWildEncountersDisabled` is quest-log playback only (`:35,66-69,360`;
  `src/quest_log.c:480,1231`).
- Repel: `VAR_REPEL_STEP_COUNT`, decremented per step before the encounter check
  (`src/wild_encounter.c:576-599`, `src/field_control_avatar.c:628`); filters by the first
  healthy party mon's level *after* slot+level rolls (`:601-622`). With repel active the
  rate buff is zeroed (`:780-783`). (A repel costs a mart purchase; probably not semi-naive
  material, priced later.)

## What this means for routing (plan, to be measured)

- The wild-LCG seed is fixed at the title-exit press; its entire pass/fail sequence is
  computable. The count of eligible grass-step tests before each fated "pass" index is the
  budget the path has to live within — spent tests can be *skipped* only by the 5% cooldown
  gate (first 6-7 steps per map entry, which we *want* to fail and 95% does) or avoided by
  stepping on non-encounter tiles.
- The wild seed's dial is the title-screen exit frame — the same dial that picks the main
  stream's seed and the trainer ID. One frame of title delay buys a completely fresh wild
  pass/fail sequence *and* a fresh intro stream, at 1 frame each, upstream of everything.
- Dials that move a *realized* encounter's mon (if we ever want one): frames before the
  triggering step move nature/PID/IVs at 1-2 rolls per frame.
- A battle resets the cooldown (`src/battle_setup.c:205`), so each trainer fight buys 6-7
  nearly-test-free grass steps after it.
