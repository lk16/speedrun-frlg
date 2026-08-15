# External RNG notes from MKDasher's FireRed any% TAS, cross-checked

## Provenance

Submission comments of MKDasher's published FireRed any% TAS (1:44:17.34, VBA-RR v23.6),
supplied externally on 2026-08-15 and **partially truncated in transit** — the middle of
the text (including the body of the "Pokémon Center's extra RNG cycle" section) is missing.
The closed network means the original cannot be fetched. Only the claims relevant to
rival-1, defeat-brock, and future targets are kept; his route specifics (party choices,
Elite Four move choices) are deliberately not documented here.

Each claim below is checked against `decompiled/` and against this project's own measured
research. Where his framing and ours disagree, ours is the cited one. Second pass
2026-08-15: every citation below re-verified line by line against `decompiled/` (constants
`0x41C64E6D` = 1103515245 and `0x3039` = 12345 arithmetic included) and every internal
cross-reference checked to exist.

## Claim-by-claim

1. **"This game uses two RNGs."** — Confirmed and already documented:
   `docs/defeat-brock/research/wild-encounters.md` ("Two RNGs, not one"). Main stream:
   `Random()`, `gRngValue = 1103515245·x + 24691`, top 16 bits
   (`decompiled/src/random.c:9-13`, `include/random.h:18-20`).

2. **"Second RNG formula: `val ← val·0x41C64E6D + 0x3039`."** — Confirmed:
   `0x41C64E6D = 1103515245`, `0x3039 = 12345`, which is exactly
   `WildEncounterRandom()`'s private LCG over `sWildEncounterData.rngState`
   (`decompiled/src/wild_encounter.c:667-671`, `include/random.h:20`).

3. **"It only advances if you step in grass, cave, water, etc."** — Directionally right,
   but the precise trigger matters for step-count planning: it advances **only on the
   encounter-rate dice roll** (`DoWildEncounterRateDiceRoll`,
   `decompiled/src/wild_encounter.c:302-307`), i.e. on an encounter-eligible step that
   *survives the cooldown and behavior gates first*. Steps swallowed by the per-map
   cooldown (first 6–7 steps after a map load or battle) do not touch it. Full pipeline
   with citations: `docs/defeat-brock/research/wild-encounters.md` §"Per-step decision
   procedure".

4. **"So it's not possible to avoid wild encounters during the whole run."** — His
   conclusion, our project disagrees as a general rule: the wild LCG's live seed is fixed
   at the title-screen exit (`SeedWildEncounterRng(Random())` in `ResetMenuAndMonGlobals`,
   `decompiled/src/new_game.c:103`, reached from `src/title_screen.c:737` — a boot seeds
   it twice, but the earlier copyright-screen call at `src/intro.c:1004` is overwritten;
   measured in `docs/defeat-brock/research/wild-encounters.md`), so its entire pass/fail
   sequence is precomputable, and encounter dodging becomes
   path/step-count shaping plus the title-exit seed dial — measured and used by the
   defeat-brock route (`docs/defeat-brock/research/wild-encounters.md` §"Consequence",
   `route/defeat-brock/ledger.json`). What *is* true: the state cannot be advanced without
   taking eligible steps, so a route through long mandatory grass has only as many
   pass/fail indices to spend as it takes steps.

5. **"RNG goes forward at two per frame in battle, and one per frame outside."** —
   Confirmed and measured here: once per VBlank always (`decompiled/src/main.c:412`), one
   extra in battle (`VBlankCB_Battle`, `decompiled/src/battle_main.c:1647-1650`; measured
   across all 2409 battle frames of the rival-1 route,
   `docs/rival-1/journal/2026-08-12-13-10-rng-model-and-consumers.md`). On top of the
   per-frame base, event-driven consumers (wandering NPCs, map loads, ambient cries,
   fishing) are itemized with citations in that same journal entry.

6. **"Pokémon Center's extra RNG cycle."** — **UNVERIFIED, content lost to truncation.**
   Only the section title survived; the actual claim (an extra advance per frame inside
   Pokémon Centers? one extra advance per heal?) is unknown. A scan of `src/` on
   2026-08-15 found **no Pokémon-Center-specific per-frame `Random()` consumer**; the
   plausible confound is that Center NPCs include wanderers, whose event-driven rolls
   (`decompiled/src/event_object_movement.c:2716-3110`) look like a higher ambient rate.
   *Needs-emulator:* when a route first enters a Pokémon Center (glitchless-run does),
   diff `gRngValue` per frame while idling inside vs. outside — one `rng-trace` run
   answers it. Until then, do not plan manipulation around any "extra cycle".

## Not carried over

His frame-timing numbers, luck-manipulation waits, and anything downstream of VBA-RR
emulation timing are not transferable (different emulator core, different route). The
Clemi skip, supplied in the same message, is documented separately with a full source
derivation: `docs/tricks/clemi-skip.md`.
