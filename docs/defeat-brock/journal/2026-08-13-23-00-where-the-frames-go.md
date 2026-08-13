# 2026-08-13 23:00 — where the 49143 frames actually go, and the seed dial

Measured on the committed run (traces over the concatenated logs; scripts in the
session scratchpad, methods below reproducible from the ledger alone).

## Battle vs field, per segment

`gMain.inBattle` traced per frame: **15 battle episodes, 22,982 frames in battle**
(47% of the run). The named fights: rival 3166, catching-demo 1943 (scripted),
Rick 4327, Sammy 2637, Brock 4011. The other ten episodes are wild flees of
501–506 frames each — ~5.1k frames of pure loss before transition overhead.

A `gBattleMons` trace corrected a build-report error: the two forest trainer
fights are **Rick and Sammy — Doug was dodged**, not fought (route.md fixed).
Exp check against `research/starter-and-brock.md`'s tables: 135 (L5 start) + 68
(rival Bulbasaur) + 66+68 (Rick) + 100 (Sammy) = 437 → L7 at Brock, Bubble online.

## The ten flees are (mostly) fated

`sWildEncounterData` traced per frame and read at each trigger: **9 of 10
encounters fired on cooldown-free steps** — the second-LCG rate test passed, which
no frame timing can change (`research/wild-encounters.md`); only the f35975 one
fired inside the cooldown window through the 5% `gRngValue` gate (timing-dodgeable).
So on this wild seed, delay-search dodging is nearly worthless: the flee count is a
property of the seed's pass/fail stream and the number of eligible grass steps.

Rate tests consumed across the run: ~50 (11 to-viridian, 2 deliver, 11 tutorial,
25 forest, 1 heal-pewter) of which 10 passed.

## Consequence: the seed is the knob

`seed_delay` (committed today) idles N frames on the title screen; each N buys a
distinct wild seed (measured with `examples/seed-scan`: power-on idles shift only
parity; title-screen idles shift the seeding press frame for frame; delays 0–2
cost no boot frames, each further frame costs one). seed_delay 1 reproduces the
committed run's seed `0x7850` under the new two-phase boot mash.

Simulating each candidate seed's pass/fail stream over the run's approximate
grass profile (Route 1 ×3, forest with two trainer resets, Route 2 sliver;
profile varied ±20% for robustness): the committed seed models at **8.8 flees**
(10 measured); the best of 64 scanned delays model at **4.6–6.5**. Six candidates
(delays 27, 6, 28, 13, 9, 38) are building in parallel now, full runs, to be
scored on measured total frames — the simulation only picked who gets a build.

## Corrections landed

- route.md: Doug was never fought (trace evidence, above).
- `Tuning::variants` now carries the base `seed_delay` instead of resetting it,
  so a later `route tune` cannot silently sweep on the wrong wild stream.
