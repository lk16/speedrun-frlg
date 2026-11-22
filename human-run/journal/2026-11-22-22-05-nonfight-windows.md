# 2026-11-22: the non-fight time per WR window, measured off the VOD

Branch `human-run-play`, worktree agent. Plans 1440-1447, rows in `scores.jsonl`. Parent: `1423-boss-turns-whole`
(fights priced at the WR's own VOD-measured cost). Task: the predecessor's (c) list -- with fights honest, the model's
non-fight time is wrong per window (Koga -117, Giovanni 3 -109, Misty -93, Bike -71, Sevii -65; Blaine +132,
Erika +53, Snorlax +52). Measure the WR's non-fight pieces off the VOD, fix the guessed constants, separate plan
differences.

## Method

- **Whole-VOD frame classes.** `battle-feat.sh` on the full WR VOD (13 colour cells per frame, 60 fps; ~25 min),
  then `battle-timeline.py` (every battle chain: the 67 timed trainer fights reproduce, 64 within 2 s, plus wild
  battles, the two untabled trainers and the evolution scenes, which use the battle text box) ->
  `video-analysis/data/wr-battle-chains.jsonl`. New `video-analysis/tools/nonfight/timeline.py FEAT A B` prints the
  stretch between black runs (doors, warps, fades) with its share of overworld text box (`W`) and battle frames;
  `nonfight/sheet.sh START IVL OUT` makes numbered contact sheets (1.5-8 s) to label them. Video = run + 35 s.
- **Model side.** `frlg-human segs <plan> --all --floor` (new flag) prints each segment's kind and base frames;
  `nonfight/windows.py` splits each WR window into trainer fights (the 67 VOD episodes / the model's fight segments),
  wild-styled battles (WR: every other chain incl. evolutions; model: hunts, wanders, walks' encounter time,
  evolutions) and field (walking floors, text, menus, trades, gifts).
- **What is inside the fight price.** The fitted `trainer_cost` is priced on played *episodes* (losing control to
  regaining it), so a leader's or rival's post-battle speech (TM text) is already in the fight (played overheads:
  Misty 15.8 s, Koga 15.5, S.S. Anne rival 14.6, Cerulean rival 27.5). The first pass (1440, 1441, 1443) counted
  those speeches again in the badge lumps; the corrected values (1444+) keep only what follows the speech.

## 1423's non-fight time against the VOD (s; field = walking, text, menus)

| window | WR | model | WR trainer / model | WR wild+evo / model | WR field / model | field err |
| --- | --- | --- | --- | --- | --- | --- |
| Brock | 880 | 945 | 213 / 275 | 246 / 133 | 421 / 537 | +116 |
| Misty | 801 | 766 | 371 / 429 | 92 / 139 | 338 / 198 | -140 |
| Bill | 609 | 597 | 516 / 518 | 0 / 3 | 93 / 76 | -16 |
| Bike | 484 | 409 | 186 / 183 | 17 / 10 | 280 / 216 | -64 |
| Rock Tunnel | 565 | 558 | 503 / 520 | 0 / 16 | 62 / 22 | -39 |
| Giovanni 1 | 557 | 572 | 255 / 299 | 26 / 11 | 277 / 262 | -14 |
| Snorlax | 891 | 954 | 417 / 428 | 88 / 172 | 386 / 354 | -32 |
| Koga | 715 | 617 | 158 / 176 | 272 / 280 | 285 / 161 | -124 |
| Blaine | 672 | 791 | 92 / 79 | 267 / 414 | 313 / 298 | -16 |
| Sevii | 913 | 864 | 194 / 211 | 300 / 281 | 419 / 372 | -47 |
| Sabrina | 827 | 889 | 272 / 336 | 185 / 171 | 370 / 382 | +12 |
| Erika | 176 | 241 | 117 / 129 | 0 / 2 | 59 / 110 | +51 |
| Giovanni 3 | 543 | 387 | 257 / 209 | 108 / 90 | 179 / 88 | -91 |

(Brock: the lab rival is in the WR's "wild" column, the prologue is ledger text; not audited. Sabrina: Silph Giovanni
likewise. Model columns from per-segment means, `splits --all`, R 30k, seed 7401.)

## Wrong guessed constants, measured (MODEL CHANGE, default-off keys)

New keys (`params.rs`): **`vod_frames`** {segment: frames} replaces the base frames of a named `Text` / `Walk` lump
(and the walk branch of `teleport-cerulean-2`) with the VOD value (human speed, no `text_factor` on top; jitter,
mistakes and a walk's encounters stay); **`evo_stone_frames`**, **`evo_candy_frames`** replace the guesses
`EVO_STONE` 1100 / `EVO_CANDY` 700. All default off; with them off the model is bit-identical (1423 splits
re-run: identical output). Values (video seconds; `tools/nonfight/mkplans.py` holds them with their anchors):

| segment | model (1423) | WR VOD | 1444 frames | anchor |
| --- | --- | --- | --- | --- |
| `misty-walk` (26 tiles) | 3.5 s | 18.0 s | 1075 | Cerulean center door 1603.0 -> Misty's speech ~1621 |
| `misty-badge` | 10.2 | 5.8 | 345 | Misty's TM speech ends -> gym door 1701.0 (14.2 s, 8.4 s of it text) |
| `captain-hm01` | 25.6 | 56.1 | 3350 | rival's speech done 2592.6 -> Vermilion control 2648.6 (HM01, walk off, the 23 s departure scene) |
| `teleport-cerulean-2` (walk, 260 tiles) | 35.5 | 54.0 | 3225 | fan club 2721.0 -> bike shop door 2775.0 (Route 6, Underground Path, Route 5) |
| `koga-badge` | 10.2 | 14.7 | 880 | 22.2 s after the battle, 7.5 s of it text: the walk out of the invisible-wall gym |
| `blaine-quiz-text` (+ `blaine-walk` 5 s) | 45.9 | 71 | 4240 | gym door 6010.1 -> Blaine's speech ~6086 (doors, quiz text, no quiz fights) |
| `blaine-badge` | 10.2 | 21.5 | 1285 | 28.3 s after the battle (6.8 s text): gym -> Pokemon Center where Bill waits |
| `sevii-bill-boat` | 44.4 | 32.2 | 1925 | Bill 6208.5 -> One Island PC 6240.7 |
| `celio-tri-pass` | 44.4 | 41.4 | 2475 | Celio 6241.3 -> out 6282.7 |
| `lostelle-hypno` | 16.2 | 7.5 | 450 | Lostelle's text before the static Hypno; the catch (`static-hypno`) is the battle itself, the old lump double-counted a flee |
| `blaine-badge-and-sevii-scene` | 47.6 | 33.1 | 1980 | Celio / Bill 7069.9 -> Cinnabar arrival text done 7103.0 |
| `erika-badge` | 10.2 | 6.8 | 405 | 15.2 s after the battle, 8.4 s text |
| `viridian-gym-walk` (55 tiles at bike speed) | 5.6 | 21.5 | 1285 | door -> Takashi 4.5 s, -> Warren 7 s, -> Giovanni 10 s (spin tiles, one warp) |
| `giovanni-badge` | 10.2 | 9.8 | 585 | 19.9 s after the battle, 10.1 s text; exit at 8658.0 |
| `mt-moon-1f` (90 tiles, guess) | 12.3 | 23.5 | 1405 | field time 1170.9-1182.4 + 1211.5-1226.5 minus Robby's approach |
| `mt-moon-b1f` (100, guess) | 13.7 | 16.7 | 1000 | 1293.6-1318.4 minus the move-learn text |
| `mt-moon-b2f` (100, guess) | 13.7 | 38.2 | 2280 | 1350.3-1391.6 minus the grunt's approach |
| `route4` (+ fossil text) | 11.2 | 23.9 | 1430 | the fossil choice after Miguel (1488.8-1501.9, no model segment) + the Route 4 floor |
| stone evolution (`EVO_STONE`) | 18.7 | 26-28 | 1600 | Nidoking 8176-8202, Nidoqueen 8202-8230, Clefable 8230-8256 (bag to bag), Flareon ~30 |
| candy evolution (`EVO_CANDY` + 300) | 16.7 | 32 | 1620 + 300 | Nidoran M L30 -> Nidorino, bag 8142 -> 8174 (one sample) |

Checked and left alone (within +-5 s of the model): fly (WR ~11 s vs 11.7), `sabrina-walk` (19 s vs 14),
`sabrina-badge`, `surge-badge` (12 vs 10), `route5-6` (36 vs 40), `ss-anne` walk (32 vs 29), `bike-voucher` + fan club
(26.6 vs 24), `bike-shop` + `bike-redeem`, `corner-lostelle-scene` (22 vs 20), `two-island-in/out`, the sails (13-19 vs
15), `deliver-meteorite` (24.1 vs 23.9), `tm-water_pulse` (12 vs 10), the Fuchsia Safari gate (11.7 s).

## Plan differences (the plan does it, the WR does not, or elsewhere)

- **Misty:** the WR catches Spearow, Zubat, Paras (92 s) and buys Awakenings in Pewter (16 s); the model flees
  ~110 s of Mt. Moon B2F encounters (`repel_on` covers 1F only; the WR has no wild battle in Mt. Moon besides its
  catches). Plan idea 1446 / 1447 below.
- **Bike:** the WR trades for Farfetch'd (Spearow, 37 s, 2656.8-2693.8); the model almost never does (no Spearow).
- **Koga:** Safari North + West, 7 species (WR) vs East + West, `safari_stop` 5 (known, journal wr-faithful).
- **Blaine <-> Giovanni 3:** the model does the Warden (33 s) and four evolutions after Koga in the Blaine window
  (95 s, more at the VOD evolution price); the WR does the Warden, Nidorino / Nidoking / Nidoqueen / Clefable and the
  Strength teach after Erika (8125-8270, ~145 s), evolves Eevee -> Flareon before Blaine (32 s), and levels Caterpie ->
  Metapod -> Butterfree inside the Viridian Gym (two post-battle evolutions, 51 s). The model also does the Mr. Mime
  trade (58 s) and a Diglett's Cave hunt (40 s) in the Giovanni 3 window. Blaine's +119 and Giovanni 3's -156 are
  mostly this placement.
- **Erika:** the Hitmonlee gift (70 s) the WR does not take.
- **Sevii:** the WR catches ~8 species there (Ponyta, Pidgey, Gloom, Pidgeotto, Drowzee, Hypno, Meowth, Golduck),
  buys Ultra Balls at Three Island and boxes at One Island's PC; the model hunts Bond Bridge / Berry Forest.
- **Snorlax:** the WR found Surge's cans in two checks (after saving in the gym); the model's can search is the decomp
  process (`trash_can_checks`, mean ~25 checks x 170 f). One lucky sample, not a constant.
- **Pre-fight bag trips (84 s):** fight-side (Elixirs before Silph / Route 22, pre-heals). Not touched (fight keys).
- Residual walking between trainers (Rock Tunnel -39, Bill -16, Giovanni 1 -14) is BFS-tile floors vs a human path,
  spread over many maps; not a single constant, not changed.

## Scores (WR+2m, seed 7401, R 800k, paired vs 1423 = 455.5 h)

| plan | what | kind | EHT | vs parent [95% CI] |
| --- | --- | --- | --- | --- |
| 1440 | first pass: badge lumps incl. the leader's speech (double count) | superseded | 3055.7 h | +2600 [2020, 3557] |
| 1441 | 1440's `vod_frames` only | superseded | 2064.7 h | +1609 [1241, 2115] |
| 1443 | 1440 + Mt. Moon repel | superseded | 1200.5 h | +745 [600, 955] vs 1423 |
| 1442-nonfight-vod-evos | evolution keys only | MODEL CHANGE (half) | 617.1 h | +162 [127, 203] |
| 1445-nonfight-vod-scenes | corrected `vod_frames` only | MODEL CHANGE (half) | 1104.1 h | +649 [512, 821] |
| **1444-nonfight-vod** | corrected `vod_frames` + evolution keys | **MODEL CHANGE** | **1704.1 h** | **+1249 [963, 1595]** |
| 1446-mtmoon-repel | 1444 + `repel_on` Mt. Moon B1F / B2F | **PLAN** vs 1444 | **740.7 h** | **-963 [-1280, -686]** |
| 1447-mtmoon-repel-1423 | 1423 + the same repels | **PLAN** vs 1423 | **293.1 h** | **-162 [-199, -128]** |

## Per-window check (no-reset, R 30k, seed 7401; window error / cumulative error, s)

| window | WR s | 1423 | 1444 | 1446 |
| --- | --- | --- | --- | --- |
| Brock | 880 | +65 / +65 | +65 / +65 | +65 / +65 |
| Misty | 801 | -34 / +31 | +27 / +92 | -86 / -20 |
| Bill | 609 | -13 / +18 | -12 / +80 | -12 / -32 |
| Bike | 484 | -74 / -56 | -24 / +56 | -25 / -57 |
| Rock Tunnel | 565 | -8 / -64 | -8 / +48 | -7 / -64 |
| Giovanni 1 | 557 | +15 / -48 | +15 / +63 | +15 / -48 |
| Snorlax | 891 | +63 / +15 | +61 / +124 | +61 / +13 |
| Koga | 715 | -98 / -84 | -94 / +30 | -91 / -78 |
| Blaine | 672 | +119 / +35 | +165 / +195 | +163 / +85 |
| Sevii | 913 | -48 / -13 | -76 / +119 | -75 / +10 |
| Sabrina | 827 | +62 / +49 | +76 / +196 | +80 / +90 |
| Erika | 176 | +65 / +113 | +60 / +256 | +61 / +151 |
| Giovanni 3 | 543 | -156 / -43 | -131 / +125 | -132 / +19 |
| Start E4R1 | 732 | +175 / +132 | +202 / +326 | +184 / +203 |

Windows outside +-60 s (Brock..Giovanni 3): 1423 8, 1444 8, 1446 9. The constants move the Bike window into
range (-74 -> -24) and add ~168 s by Giovanni 3 (no-reset finish p50 3:30:29 -> 3:33:26); what stays out is plan
placement (Blaine / Giovanni 3, Erika's Hitmonlee, Koga's Safari, Misty's catches and repels) and the fight side
(Snorlax's cans, Sabrina's Silph Giovanni at boss power).

## Verdict

- **1444-nonfight-vod is the honest non-fight model** (MODEL CHANGE, measured: 18 lumps timed on the WR VOD, the
  evolution scenes from 4 + 1 VOD samples), +1249 h [963, 1595] vs 1423. It is pessimistic in sum against the WR to
  Giovanni 3 (+125 s cumulative), which is now explained by plan structure rather than constants: the WR's route
  saves time on plan choices the model's plan does not make (Mt. Moon repels, Safari North, no Hitmonlee, evolutions
  and Warden batched after Erika). All keys are default-off and do not touch fight or League keys, so the combined
  model of record (1423 fights + 1413 shops + League refit) can add `vod_frames`, `evo_stone_frames`,
  `evo_candy_frames` from `plans/1444-nonfight-vod.md` as one block.
- The predecessor's premise ("non-fight ~150 s under") holds only for constants; after measuring them the model's
  non-fight time is over, not under, the WR's on the same route, and the remaining gaps are plan choices.
- **Mt. Moon repels (1446 / 1447): ACCEPT as a plan change on either model** -- `repel_on` + `mt-moon-b1f`,
  `mt-moon-b2f` (Repels are bought and used through the money model, `use_repels`): -963 h [-1280, -686] on 1444,
  -162 h [-199, -128] on 1423. The WR walks Mt. Moon this way; it is the cheapest of the plan differences found.
  On the combined model it should be re-scored (it interacts with money: 1413's shop stops).

## Not done / next

- The plan differences as ideas: Safari North (journal wr-faithful), dropping Hitmonlee for the WR's Sevii catches,
  batching the Warden + Nido evolutions after Erika (placement only moves time between windows unless it saves a
  Fly), the Farfetch'd trade via a Route 3 Spearow.
- Koga's gym walk-in and the Safari's Secret House text were not timed; Rock Tunnel / Route 24-25 walking between
  trainers is tile-floor optimistic but spread thin.
- The trash-can save trick (the WR saves before the cans; luck or a manipulation) is unexplained.
