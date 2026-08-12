# 2026-08-12 (sandbox, night) -- the RNG gets a Rust model, the consumers get names, the free levers get priced

Task: model the RNG outside the emulator, verify it, and use it to ask whether the rival
battle can be had cheaper without paying idle frames. Everything landed in a new
zero-dependency crate, `crates/frlg-rng`, plus four examples; the route itself did not move
(the honest result below says why).

**The model, proven per frame.** `Rng` is the game's LCG
(`x' = 1103515245·x + 24691 mod 2^32`, top 16 bits returned --
`decompiled/include/random.h:18-19`, `src/random.c:11-12`), with an O(log n) `jump` over
precomputed power-of-two affine maps, an inverse step, and `distance_to` -- the discrete
log, total because the LCG is full-period, answering "how many `Random()` calls separate
these two observed states" in ~135 ns. Correctness is a replay, not an argument:
`tests/emulator.rs` replays the committed 9658-frame route and requires the model to
reproduce `gRngValue` exactly on **every** frame, with only the two `SeedRng` events
(title-screen and player-naming-screen exit) allowed to break stride, each verified to
unwind to a 16-bit seed within 3 steps. It passes. `random()` measures <1 ns, so a
million-state stream scan costs what one emulator frame costs.

**The consumers, measured and cited** (`rng-trace` and `field-experiments` examples;
citations in `docs/rival-1/route.md`'s RNG section, which this session rewrote):

- The VBlank interrupt rolls once per frame unconditionally, **twice** in battle
  (`VBlankCB_Battle`, `decompiled/src/battle_main.c:1650` -- measured: all 2409 battle
  frames move the stream by exactly 2).
- **Pressing A never rolls** -- zero `Random()` on the whole text/menu/interaction path,
  and 600 frames idle vs hold-A vs mash-A consume identically. **Player walking never
  rolls** (only fishing does, `src/field_player_avatar.c:1711+`). Two same-length paths to
  the same tile left the stream identical at a common horizon: on open ground, path shape
  is RNG-neutral.
- What does roll in the field: wander/look-around NPCs
  (`src/event_object_movement.c:2716,2737,3037,3061,3090,3110` -- Pallet Town's two
  wanderers, the lab's three aides), 3 rolls per map load (`src/load_save.c:75,126`), the
  outdoor ambient-cry timer (`src/overworld.c:1141-1172`), and one trainer-id roll at new
  game (`src/new_game.c:56`). Idle in the NPC-free bedroom 2F: exactly 0 extra steps in
  600 frames. Two silencing gates matter for manipulation: the despawn window around the
  player (`event_object_movement.c:1798-1801`) and script freezes
  (`lockall`/`lock`, `scrcmd.c:1195-1221`, so the aides are silent through the whole
  scripted rival sequence).

**The free-shift question, asked directly.** If NPC-roll avoidance could shift the
battle-start stream by k at zero frame cost, what would k buy? `battle-scan` loads the
battle-start state (`gRngValue 0xed94271d` at frame 7249), writes `jump(k)` of it, and
races the battle as a pure `text_hold` mash across worker threads; `battle-plan-scan` runs
the route's full two-stage delay search under the same shift, anchored by reproducing the
committed battle bit-for-bit at shift 0 (2409 frames, plan `[4, 3, 3, 3]`, 38/64 start
delays winning -- and the anchor caught a real trap first: with a mash phase that runs
continuously instead of restarting at every advance stage like the route's
`advance_while`, the "same" search only reaches 2492. The drive shape is part of the
stream; the scanner now matches it exactly.)

Results, RNG-write exploration (tier 0, not evidence about any input log):

- Pure mash, shifts -60..60 x start delays {0,1,2,3,4,5,6,8}: **nothing beats 2409**.
  Best is shift 29, delay 3, at 2413 (+4); best pure-mash-no-delay is shift -6 at 2495.
  The top rows come in families of constant `shift + 2·delay`, which is the battle's
  2-rolls-per-frame arithmetic showing through: an idle frame in battle is a +2 shift
  that also costs a frame.
- The full two-stage search, run under shifts {-6, -4, -2, 21, 23, 25, 27, 29} (the
  grid's most promising, plus the realizable negatives): **-2 and -4 tie the committed
  2409 exactly**, with different plans (`[3, 0, 3, 2]` and `[4, 0, 3, 6]` against the
  committed `[4, 3, 3, 3]`); -6 and 29 reach 2410, 27 reaches 2411, and nothing beats
  2409. The committed battle sits on a floor that many neighbouring streams can reach by
  re-spending delays and none undercuts -- so no route change is recorded, and that is
  the result: the existing delay search was already extracting everything this
  neighbourhood of streams has to give.

**Who actually rolls in Pallet Town -- attribution done properly** (`who-rolls` example,
diffing `gObjectEvents` on every consuming frame). The map.json reads two wanderers; the
running game has **one**: the map's on-load script parks the sign lady at (5,15) as
`MOVEMENT_TYPE_FACE_UP` until her scene var moves
(`decompiled/data/maps/PalletTown/scripts.inc:27-28`), so she never rolls on this route.
Every extra field step while idling outside the house is the fat man -- each of his wander
moves pairs a direction roll with a delay roll ~16 frames later, ~13 steps per 600 frames
-- and his spawn is player-position-dependent (despawned at the house door until the
player steps south). That per-position spawn is the one genuinely free stream lever this
route has: a same-length path that changes how long he stays in the spawn window shifts
the battle stream by a few steps at zero frame cost. The scans above say those few steps
buy nothing here; the lever is machinery for future routes, not this one.

**Unverified / open.** The RNG-write scans are what-ifs by construction; any shift worth
realizing must be reproduced by real inputs and re-verified from reset before it touches
the ledger. Move choice in battle remains unsearched. The lab aides' roll pattern during
`07-starter`'s unfrozen stretches was traced but not itemised.
