# Fill-economy audit: plan 370-clemi-skip, oak-dex-check phase

Method. Repo read-only. I copied `crates/` + `human-run/data` to `.../scratchpad/calib/ws`, added
counters (thread-local frame accumulators by cost category, per-site and aggregate, plus
caught-set histograms at Oak's check and after the fill), and ran `frlg-human dist
human-run/plans/370-clemi-skip.md --R 20000 --seed 2`. The instrumented build reproduces the
committed binary (same oak mean 35.8, same per-site table). All "per attempt" numbers are means
over the 1208 of 20000 attempts that reach Oak's check; ~24 species are filled per such attempt
(35.8 -> 60). Experiment variants live in the scratchpad as `p370*.md`, `pcomb.md` etc. (plan
names end in `-x` so env switches only affect them); helper `fsum.sh`. Anything not cited to
`decompiled/` below is a measurement of this repo's simulator, not a game fact.

## 0. Corrections to the premise (read first)

1. **`pass_min_catch: 45` is not binding.** `pass_min_catch 0` gives identical output to 45 (oak
   mean 35.8, fill 44.5 min). Only species with catch rate < 45 are excluded (Clefable 25, Chansey
   30, Ditto 35, Snorlax 25, legendaries 3); everything else in the FRLG wild tables is >= 45.
   Raising it to 120 / 190 drops the Oak count to 33.0 / 28.7 and lengthens the fill by +2.9 /
   +10.9 min, so passive catching matters, but the threshold at 45 is not the gap.
2. **The fill is mostly encounter handling, not catching** (sec. 1).
3. **Stones are modelled after the fill, so they can never help** (sec. 3). The journal's
   "stones refuted (+0.16 h)" is an ordering artifact, not evidence against evolution banking.

## 1. Model marginal cost per filled species (from the code)

Total fill 2669 s = 44.5 min per attempt reaching Oak; 24.3 species -> **110 s / species**
(matches the 1:43 estimate). 92.5 wild encounters per attempt, of which only 30.5 are of a wanted
species (21.3 non-Safari catches + ~3 Safari) and **62 are duplicates the model flees from**:
3.8 encounters and 2.55 dup-flees per filled species.

| cost term (code) | s / attempt | % | s / species |
| --- | --- | --- | --- |
| encounter intro: `enc_intro` 450 f + `react` ~22 f, paid on EVERY encounter (`encounter()` sim.rs:604, `calib_enc`) | 732 | 27% | 30 |
| flee of dups: `flee()` 500 f each (p_ok=1 at carry >= 15) | 519 | 19% | 21 |
| waiting: `steps * frames_per_tile` on land, 180 f per cast when fishing (`hunt()` per_step, sim.rs:~1297) | 400 | 15% | 16 |
| throws: `throw_cost` 480 f + react per ball (`throw_balls` sim.rs:564), 40 balls | 337 | 13% | 14 |
| weaken: 1.5 turns x 385 f on every wanted non-Safari encounter (`ctx.p.weaken`) | 244 | 9% | 10 |
| travel: `detour_tiles` x2 ways x 6-8 f/tile x `path_overhead` (`hunt()` walk bump) | 280 | 10% | 12 |
| Safari ball/rock cycles (`safari_catch`) | 121 | 5% | 5 |
| catch_end (Clemi-skipped, 10 f) + other | ~8 | <1% | <1 |

Grouped: **encounter overhead (intro + dup-flee) = 1251 s = 47%, the dominant term** (52 s per
species); catch mechanics (weaken + throws + Safari) = 26%; travel + waiting = 25%. The single
largest line is `enc_intro` x number of encounters. The root cause is encounter *efficiency*: 1
wanted species per 3.8 encounters, because by the time of the fill the easy species are taken
and each site's remaining unfilled slots are thin.

Per-site marginal (instrumented; s per species gained): Route 11 59, Route 21 65, Power Plant 74,
Seafoam B3F 77, Diglett 77, Route 6 94, Vermilion 104, Tower 7F 105, Victory Road 113, Fuchsia 113,
Route 23 123, Safari West 127, Seafoam B4F 165, Safari Center 183, Seafoam B2F 231 (0.29 species
per visit), **Route 13 312** (0.73 per visit; 8.2 dup-flees per visit). The plan's list order is
not by marginal cost, so cheap sites sit behind 230-310 s/species ones.

Two small modelling errors seen in the breakdown:
- Safari dup-flee is charged the generic 500 f `flee()`. Safari "Run" is unconditional and
  instant: `HandleAction_SafariZoneRun` sets `B_OUTCOME_RAN` with no escape roll
  (`decompiled/src/battle_main.c:4414-4420`). Charging 200 f instead: -1.4 min fill.
- Fishing hunts pay 180 f per cast regardless of bite; that is a guess (comment in `hunt()`).

## 2. The WR runner's observed cost per species

Source `human-run/video-analysis/data/wr-scene-*.jsonl` (1-min samples of his tracker, "caught"
column; consistent with the pairs in `wr-3.25.25-main97.md`) and the stream file.

Caveat: his counter updates live for catches (Pikachu 22->23 at 1:34 during the Pikachu battle;
Grimer 24->27 across 1:36-1:38) but jumps in batches around evolution/stone/trade events (38->46
inside the 2:05-2:06 samples; 55->60 inside 2:32-2:33), so per-minute attribution there is soft
(+-1 min, and entry-to-event pairing is inferred).

| block (video time) | entries | what they are | time | s / entry |
| --- | --- | --- | --- | --- |
| Safari, 1:22-1:28 | 14 -> 21 (+7) | all catches (Nidoran F/M, Nidorina, Exeggcute, Rhyhorn...), Safari Balls 29 -> 20 = ~9-10 balls | ~6 min incl. walk, entry, Fly out | **~50** |
| Power Plant + Route 21 + Mansion, 1:32-1:40 | 21 -> 29 (+8) | catches (Pikachu by Eevee mid-paralysis, Raticate, Grimer, Vulpix in LG...) | ~8-9 min incl. Surf | **~60-65** |
| Sevii trip, 1:43-1:57 | 29 -> 37 (+8) | forced content with catches on it (Berry Forest, Drowzee, Meowth...) | 15 min, mostly forced | marginal unknown, low |
| Silph/Cinnabar/Saffron, 1:58-2:06 | 38 -> 46 (+8) | **evolutions / trade / gift**: Omanyte revive 1:59, Pikachu->Raichu 2:00 (Thunder Stone), Venonat->Tangela trade 2:01, Weedle->Kakuna 2:05, Kakuna L9 (party-select menu 2:06 = Rare Candy)->Beedrill 2:07 | ~3 min of dex-specific actions in a 9-min window | **~25-35** |
| Nidoking + Clefable, 2:15-2:17 | 46 -> 50 (+4) | Moon Stone evolutions | ~1.5-2 min | ~25-30 |
| Victory Road + Indigo, 2:28-2:33 | 53 -> 60 (+7) | 2 real catches (Psyduck 2:28, one more ~2:31), then **+5 in ~1 min at the Indigo Center**: party screens (Persian L32, Growlithe L32, Gloom, Exeggcute; "Use on which POKEMON?" 2:34) = stone/candy evolutions | ~4 min | ~35 (catches ~100, evos ~15) |

Evolution entries in his 60 (at least): Wartortle, Blastoise, Raichu, Kakuna, Beedrill, Nidoking,
Clefable, plus the 5 at Indigo. Stream file: "the route banks several dex entries as late
evolutions (Moon Stone Nidoqueen + Clefable, Leaf Stone Victreebel, Metapod->Butterfree chain)"
and "5 evos now / late moon can do that". So **>= ~12-14 of the 60 are not catches.**

Whole-block upper bounds (include non-dex time): WR route Sabrina (46) -> E4 (60): 24:11 for 14
entries = 104 s/entry. Stream LG PB (34 at Sabrina -> 60): 36:11 for 26 entries = 83 s/entry,
including Erika, Viridian gym, rival 7, Victory Road trainers, Indigo. Marginal comparison of the
two routes: the new route spends +12:00 more over Erika..E4 for +12 more entries -> **60
s/entry**, with ~5 of the 26 late entries being evolutions. Subtracting ~12 min of non-dex content
from the stream block gives ~52 s/entry.

Observed: catches ~50-65 s each, evolutions/gifts/trades ~15-35 s each, blended ~50 s, versus the
model's 110 s. About 2x.

## 3. Structural gap

**(a) Evolutions are credited only through `Kind::Stones`, which runs AFTER the fill.**
`route.rs:906` pushes `oak-dex-check`, `route.rs:910` pushes `stones` after it; the fill loop
(`sim.rs:1600+`) ends once `caught.count() >= 60`, so stone entries land on a dex that is already
full. Stones are pure cost. Measured with 7 stones (MOON,MOON,FIRE,THUNDER,LEAF,LEAF,WATER, 2200 f each
as the code charges): as written (after the fill) mean finish +4.5 min and paired EHT **+1.36 /
+1.38 h worse** (seeds 1/2); with the Stones step moved before the check, fill 44.5 -> 37.3 min
(-7.2), mean finish -2.9 min net, paired EHT **-0.63 / -0.79 h**. Also `Kind::Stones` evolves only
the *first* matching species per stone in alphabetical species order (`break 'found`,
sim.rs:~1672): Thunder goes to Eevee->Jolteon (SPECIES_EEVEE sorts before PIKACHU), not Raichu, so
Raichu never exists and plan 370's trade "Electrode requires SPECIES_RAICHU" (special at
"blaine-walk") can never fire (Electrode afterFill = 0.00 in the histogram).

**(b) Level-up evolution of any non-carry mon is not modelled at all.** `grep EVO_` in
`crates/frlg-human/src`: only `eval.rs:58-60` (the carry's evolution level) and `sim.rs:1665`
(`EVO_ITEM` in Stones). Caught mons carry no levels or exp; fodder never evolves. Rare Candies
exist only as 6 pickups (`"kind": "candy"` in plan 370; `sim.rs:1851`), all eaten by the carry at
`indigo-heal` (`sim.rs:1921-1930`, one level each, ~400 f). So the WR's Weedle->Kakuna->Beedrill
candy chain (party-select on a Kakuna L9, then Kakuna evolving, 2:05-2:07) has no model
equivalent. Evolution levels: `decompiled/src/data/pokemon/evolution.h:9-12` Caterpie->Metapod
7, Metapod->Butterfree 10, Weedle->Kakuna 7, Kakuna->Beedrill 10. A wild L5 Weedle/Caterpie needs
5 candies for the whole chain (2 entries; Kakuna and Metapod are already wild-caught 39% / 65% of
the time in the model, Weedle/Caterpie 99%).
Stone sources on the WR route, none credited: Moon Stone x3 (`MtMoon_B2F/map.json:223`,
`PokemonMansion_1F/map.json:176`, Lostelle's dad gift `TwoIsland_JoyfulGameCorner/scripts.inc:165-169`),
Thunder (`PowerPlant/map.json:185`), Leaf (`SafariZone_Center/map.json:149`), Water
(`SeafoamIslands_B4F/map.json:140`), Fire (`MtEmber_Exterior/map.json:380`); Celadon 4F sells
Fire/Thunder/Water/Leaf (`CeladonCity_DepartmentStore_4F/scripts.inc:31-34`; Fire Stone 2100 in
`items.json`). Money for stones is not modelled in `Kind::Stones`.

**(c) Passive catching vs targeted blocks.** The Oak-check dex of 35.8 = 3 starter line + ~9
gifts/trades/statics + ~23 wild. Passive wild catching happens on the model's route grass steps
plus 3 in-route hunts (Route 25 240 steps, Route 3 15, Route 5 120), under Repel on 10 segments.
Removing Repel gives 38.5 at Oak but the no-reset mean finish is +11 min worse and far fewer
attempts reach Oak, so Repel is right. A passive catch costs roughly weaken 10 s + ~1.9 throws
14 s over a flee that happens anyway (~25 s net), against 110 s in the fill. The WR's Safari /
Power Plant / Mansion blocks catch 4-7 species per site with ~1.5-2 encounters per entry; the
model's late fill gets 1 wanted per 3.8 encounters because it comes after the passive pass and
walks one thin site at a time.

**(d) Fill list order and tail sites** (sec. 1): Route 13 and Seafoam B2F/B4F cost 165-312
s/species and sit mid-list. Reordering the 16 hunts by measured marginal cost: mean finish -4.0
min (5:52:29 -> 5:48:30), paired EHT -0.71 / -0.82 h; dropping only Route 13 + Seafoam B2F: -3.6
min, EHT -0.61 / -0.89 h.

## 4. Recommendations, ranked by measured effect

Minutes are per attempt reaching Oak. "Fill gross" = fill-phase length; "net" = change in mean
finish of finishing runs after paying the evolution actions at the model's guessed frames. EHT is
paired at WR+160m, seeds 1/2, R=20000.

1. **Credit evolutions before the check.** Move `stones` ahead of `oak-dex-check`
   (`route.rs:906-910`); let each stone choose a target (Thunder->Raichu unlocks the Electrode
   trade; Moon x3 -> Clefable, Nidorina/o, Jigglypuff); use the free pickups on the route (Power
   Plant, Safari Center, Seafoam B4F, Mt Ember, Mt Moon, Mansion, Lostelle) instead of Celadon
   money; fix `break 'found`. Measured with 7 stones: **-7.2 min fill gross, -2.9 min net, EHT
   -0.63/-0.79 h** (stone cost is a guess: 1500+700 f each). Evidence: WR/stream evolution list
   (sec. 2), decomp stone locations (sec. 3).
2. **Reorder and trim the fill list by marginal s/species** (plan JSON only): Route 11, Route 21,
   Power Plant, Seafoam B3F, Diglett, Route 6, Vermilion, Tower 7F, Victory Road, Fuchsia, Safari
   West, Route 23, then Safari Center / B4F / B2F / Route 13 last. **-4.0 min net, EHT
   -0.71/-0.82 h.** Evidence: instrumented per-site table.
3. **Candy-evolution special** (`kind:"candy_evo"`: species, candies, frames): stop spending the
   6 candies on the carry; Weedle and Caterpie chains (`evolution.h:9-12`). Stand-in measured by
   crediting BEEDRILL + BUTTERFREE at 2500 f each: **-4.4 min fill gross, -3.0 net, EHT
   -0.52/-0.65 h**. Two chains need ~10 candies and the plan holds 6 (plus item balls on the
   route: Hideout B3F, Tower 6F, Mt Moon 1F, Silph 10F, by the `FLAG_HIDE_*_RARE_CANDY` maps);
   one chain alone is about half of that. Evidence: his 2:05-2:07 sequence.
4. Upper bound for 1+3 together: crediting BEEDRILL, BUTTERFREE, RAICHU, CLEFABLE, VICTREEBEL,
   EXEGGUTOR at 2500 f each: **-12.1 min fill gross, -7.9 net, EHT -1.16/-1.29 h**, assuming the
   bodies exist.
5. **Safari dup-flee ~200 f** (`encounter()` else branch when `safari`;
   `battle_main.c:4414-4420`): **-1.4 min**. Safari as one multi-area entry (600 steps / 30 balls
   per entry, `safari_zone.c:32`) rather than West-only 95 steps may approach the WR's ~50 s/entry;
   not tested.
6. **`pass_min_catch` as a per-site threshold: no effect worth chasing** (0 vs 45 identical).
   Raise passive *opportunity* (extra in-route steps at multi-slot sites) instead; the cost
   asymmetry (~25 s passive vs 110 s fill) says each extra passive entry is worth ~1.5 min.
   Weaken is applied to every wanted non-Safari target (9% of the fill); skipping it for catch
   rate >= 190 is worth <= ~2 min and is untested (my `weaken:false` and `ultra` runs were
   confounded by money/reset effects and are not reported).

All of 1, 2, 3 (two entries) and the Safari fix applied together (`pcomb.md`): fill **44.5 -> 28.5
min**, Oak count 35.8 -> 41.4, mean finish **-10.2 min net**, paired EHT **-1.38 / -1.52 h**. That
still leaves a ~28 min fill versus the WR's 10-15 min extra investment. The remaining gap is (i)
late-fill encounter efficiency (3.8 encounters per entry) and (ii) fewer wild catches banked
before the check than his Safari / Power Plant / Mansion blocks (22 -> 29 in ~20 min). Moving real
multi-species blocks (Safari before Koga, Mansion/Power Plant en route) out of the late fill and
into the route is the next structural step; not tested here.

## 5. Evidence index

Model: `crates/frlg-human/src/sim.rs` (`throw_balls` 564, `encounter` 604, `hunt` 1222, OakCheck
1600, Stones 1656, candies 1851/1921), `route.rs:906-910`, `params.rs:256-263`, plan
`human-run/plans/370-clemi-skip.md` (fill list, `stones: []`, candy pickups).
Real run: `human-run/video-analysis/data/wr-scene-{A,B,C}.jsonl`, `wr-3.25.25-main97.md`,
`stream-2026-11-12-main97.md`.
Decomp: `src/data/pokemon/evolution.h`, `src/battle_main.c:4414-4420`, `src/safari_zone.c:32`, the
map item files listed in sec. 3.
Scratch: `.../scratchpad/calib/ws` (instrumented copy), `tgt/release/frlg-human`, `fsum.sh`,
`p370*.md`, `pcomb.md`.
