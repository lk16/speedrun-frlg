# 2026-11-13: WR + stream video analysis; Sevii-trip model audit; plans 367-370

## What happened

Two videos landed on the host (`ignored/downloaded/`, visible through the read-only mount):
the 3:25:25 WR run and an 8.4 h stream from the WR holder (2026-11-12) grinding a newer
LeafGreen route. Built a reusable extraction pipeline (`human-run/video-analysis/METHOD.md`
+ `tools/make-sheets.sh`: keyframe-only ffmpeg contact sheets, Haiku subagents transcribing
to JSONL — the whole 22 GB stream costs ~3 min per extraction pass and a few hundred k Haiku
tokens). Findings and raw transcriptions are committed under `human-run/video-analysis/`;
neither video should ever need re-opening for a question those files answer.

## Route evidence -> model tests (paired, seeds 1 and 2, fixed rung WR+160m)

Accepted, promoted as plans 367-370:

1. **367 Fire Blast on Mewtwo** (WR video: Mewtwo fights the rematch E4 with Fire Blast;
   TM38 is free from Blaine): -0.47/-0.67 h. The earlier d367 Mewtwo-TM probes that scored
   exactly 0.0 were silently skipped by the money model (80k cost); free TMs were never tried.
2. **368 Fighting Dojo Hitmonlee** (stream's new route): gift after beating only Koichi
   (`SaffronCity_Dojo/scripts.inc`), 4000-frame guess: -0.15/-0.12 h.
3. **369 catch Lostelle's forced Hypno** (enabled by the Sevii audit below): -0.09/-0.16 h.
4. **370 Clemi skip** on every successful catch: tier-1 measured this session (new
   `frlg-calib --clemi` mode): control catches mean 668 floor frames, good L presses land
   ~435-455 → 170-230 frames saved per catch; model credits 140. -0.67/-0.54 h.
   **Legality on the console glitchless boards is unconfirmed — human to check.**
   Also decomp-derived (not yet measured or modelled): the level-up jingle uses the same
   `{WAIT_SE}` gate and should be skippable the same way (`docs/tricks/clemi-skip.md`).

Refuted: stones MOON+FIRE+THUNDER (+0.16 h), MOON alone (+0.03 h) — the WR routes' late
evolution banking does not pay under our fill economy; Berry Forest extra hunt steps
(+0.91 h); the deferred-catch *structure* itself needs no transplant — our plan already
defers more than his (fills post-HoF vs his pre-E4 60/60).

## The big model correction: the forced first Sevii trip

Our route modelled Blaine->back-in-Kanto as one 5000-frame guess. The decomp audit
(gating chain: `seagallop.inc:4` Vermilion needs CINNABAR scene >= 4 <- Bill sail-home scene
<- PC scene 2 <- meteorite delivery `TwoIsland_JoyfulGameCorner/scripts.inc:149-152` <-
Lostelle rescue `ThreeIsland_BerryForest/scripts.inc:24-39`) shows the trip is ~10 minutes
of forced content: 4 Three Island biker fights (a physical NPC wall), 2 unavoidable Bond
Bridge trainers, a forced wild Hypno Lv30 (catchable — plan 369), ~545 tiles, ~70-90 grass
steps on Bond Bridge/Berry Forest (their LG tables carry Weepinbell/Slowpoke/Drowzee/
Exeggcute/Meowth — the stream route's Berry Forest catches now emerge naturally from the
pass-catch rule), and a free Moon Stone. `route.rs` now models the chain segment by segment
(scene texts still labelled guesses). EHT(WR+160m) of the best plan honestly rose ~2 h;
**headline after the whole session: `EHT(WR+160m)` 13.40 h at R=20k, seed 1 (plan 370)** —
worse than yesterday's 12.58 h *number*, but yesterday's number was wrong.

The WR video corroborates the magnitude: his Blaine->Sevii split is 15:13 (LG PB 14:23).

## Encounter-seed manipulation (user-supplied observation, quantified)

The WR holder resets for title seeds that give few encounters pre-Brock. Mechanism was
already documented (`docs/tricks/mkdasher-rng-notes.md`, defeat-brock research): the
encounter dice stream is a private call-locked LCG seeded once from the title-screen press
timing; hit/miss is seed-determined, but the *roll index* drifts via two frame-locked gates
(5% cooldown pass, 60% tile-change gate), so alignment decays with distance. New exact
scan of all 65536 seeds (pure LCG math, `wild_encounter.c:302-307,667-671`):

- initial clean-miss run at Viridian Forest's 14%/roll: median 4 rolls, p90 15, p99 30, max 76;
- hits in the first 60 forest-rate rolls: median 8, best seeds 1-2, P(<=2 hits) = 0.7%,
  P(<=4) = 6.3%.

So "quiet early game" seeds exist and resetting can select for them (worth modelling as a
starter-reset side benefit — not yet in the engine). **Extending the manipulation to save
the Mt. Moon Repel: refuted.** ~25 minutes of human play cannot hold the roll index, and
even the best seed's miss window cannot be aimed at Mt. Moon's rolls (`main.c:412`
frame-locked advancement; gates at `wild_encounter.c:350,752`). The Repel stays.

## Also this session

- Clemi-skip "Switch-only" web claim refuted by measurement on the GBA game itself.
- Web intel on top runners is thin (every primary fetch 403s; snippets archived in
  `video-analysis/data/runner-intel-web-2026-11-13.md`): WR was a $5k bounty run
  (PacePals 2026-08-23); Gunnermaniac is the previous WR holder; no public route docs found.
- His `!deaths` nicknames map to forced trainers we model: "sammy" =
  `ViridianForest_EventScript_Sammy` (our forced-audit name), "liam" = the Pewter gym camper.
- Audio transcription of the stream is not possible in-sandbox and the Anthropic API has no
  audio input; if the commentary matters, the host should run whisper on the mp4 and drop
  the transcript next to the videos.

## Next

1. Model the seed-reset side benefit (starter resets also draw a fresh wild seed; condition
   pre-Brock encounter counts on the seed distribution above) and re-tune brock-split aggr.
2. Tier-1 measure the level-up jingle skip; if real, credit it (many level-ups per run).
3. Emulator partial-run program: build savestates at route checkpoints (E4 entry, Sevii
   start) and replay the new Sevii chain headless to validate the segment-by-segment guesses;
   same harness then measures one real rematch-E4 fight (the model's most suspect area).
4. Re-run the WR-technique battery (journal 2026-11-13-04-30) on the corrected model when
   convenient; the paired verdicts predate the Sevii fix.
5. Human to confirm: Clemi-skip legality on the console boards; whisper transcript if wanted.
