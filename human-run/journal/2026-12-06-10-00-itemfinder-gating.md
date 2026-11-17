# 2026-12-06: Itemfinder / pickup correctness check of CURRENT 1067 (and 1057, 1058)

CURRENT stays `1067-d57-combo`. Result: **none of the plan's pickups needs the Itemfinder**; the sim could not have let them skip it; the
only pickup that does (Route 16 Leftovers) is not in any plan and, with the trip charged and the sim now gating it, is break-even at best. No model change to any
adopted plan: 1057 / 1058 / 1067 reproduce bit-identically on the new binary (`compare` 1067 vs 1058, WR+2m, R 20 000, seed 4501: same JSON on the old and the new build).
Tools: scratch `if-*` (variants, `if-run.py`, `if-elev.py`), `human-run/tools/pickup_audit.py` (new), `frlg-mapgraph`, `forced.py`. Screens R 800k, seed 4501, paired against 1067 (+ is worse).

## (1) Which pickups are underfoot hidden items

Source of truth `decompiled/data/maps/*/map.json` (`bg_events` `hidden_item`, field `underfoot`; `mapjson.cpp:329` writes it into the item word). **Only six of the 183 hidden items in the game are underfoot:**
Two Island Cape Brink PP Max (16,28), Pokemon Tower 7F Soothe Bell (11,4), Navel Rock Summit Sacred Ash (9,11), Route 12 Leftovers (14,70), Viridian Gym Macho Brace (2,2), Route 16 Leftovers (31,13).
Everything else (including every hidden item in the plan) has `underfoot: false`. `pickup_audit.py` lists each `elixirs` pickup of a plan against its decomp tile and prints the flag; for 1067:

| pickup (anchor) | decomp source | kind | Itemfinder |
| --- | --- | --- | --- |
| candy (route9) | Route 9 (63,2) | hidden, underfoot false | no |
| candy (cycling-road) | Route 17 (18,83) | hidden, false | no |
| max (cycling-road) / ppup (cycling-road) | Route 17 (14,157) / (4,68) | hidden, false | no |
| max (route23) | Route 23 (9,8) | hidden, false | no |
| ppup (hideout-grunts) / ppup (celadon-tea) | Rocket Hideout B1F (16,17) / Celadon (55,20) | hidden, false | no |
| elixir (silph-giovanni) / (mansion), moon (mansion) | Silph 5F (13,16) / Mansion B1F (35,5) / Mansion 1F (2,21) | hidden, false | no |
| calcium (hideout-giovanni), calcium (rival-silph) | Hideout B4F (6,23), Silph 7F (0,12) | item balls | no |
| candy (mt-moon-1f, hideout-grunts, tower-channelers, silph-grunts, victory-road, warden-strength) | Mt Moon 1F, Hideout B3F, Tower 6F, Silph 10F, Victory Road 1F, Warden's house | item balls | no |
| leaf (safari-walk), thunder (powerplant-out) | Safari East ball, Power Plant ball | item balls | no |
| moon (deliver-meteorite) | Two Island Game Corner script gift | script | no |
| Nugget (celadon-tea `sell`) | Route 24 Rocket, `Route24/scripts.inc:5-51` | script gift | no |
| Black Glasses (`black_glasses`) | Hideout B3F ball (14,24), `item_ball_scripts.inc:494` | ball | no |

Mechanics of the non-underfoot hidden items: `GetInteractedBackgroundEventScript` (`field_control_avatar.c:462-507`): for `BG_EVENT_HIDDEN_ITEM` it returns NULL only if `HIDDEN_ITEM_UNDERFOOT` is set (`:497-500`) or the item's flag is already set (`:503-504`); otherwise it runs `EventScript_HiddenItemScript`
(`data/scripts/obtain_item.inc:148-174`: `additem`, "found one X", fanfare, `SetHiddenItemFlag`). The event is looked up at the tile **in front of** the player (`GetInFrontOfPlayerPosition`, `:351-360`; bg-event kind 7 has no facing restriction, unlike kinds 1-4),
with the player's elevation (`GetBackgroundEventAtPosition`, `:1101-1116`: event elevation equals the player's or is 0; all eleven hidden pickups of the plan have an elevation-3 walkable neighbour, `if-elev.py`: the three on wall-like tiles (Hideout B1F, Silph 5F, Mansion 1F) are reachable from a floor neighbour).
So: stand on an adjacent walkable tile, face the item tile, press A. A is accepted at `T_TILE_CENTER` on a non-forced-movement tile or when not moving (`FieldGetPlayerInput`, `:100-120`) and **not at bike speed FASTEST** (`:106`, `GetPlayerSpeed` `bike.c:361-372`: the Mach Bike's third speed step); the Cycling Road pull-down tile (`MB_CYCLING_ROAD_PULL_DOWN` 0xD0)
is not a forced-movement tile (`metatile_behavior.c:266-276`), and `BikeInputHandler_Normal` (`bike.c:56-72`) with B held and no direction stops on it. Item balls are object events, A facing them, no gating. These interaction costs are in the plan's guessed 300-450 f per pickup (pickup frame robustness x2 / x3: ROBUSTNESS section 15).

Underfoot items: only found by standing exactly on the tile and using the Itemfinder (`itemfinder.c:241-246` `dx == 0 && dy == 0`; 3 dings of 25 frames, `:181-198`, then the dig-up script). The Itemfinder comes from the Route 11 east-gate 2F aide
(`Route11_EastEntrance_2F/scripts.inc:56-73`): Yes/No, then `specialvar GetPokedexCount` with `VAR_0x8004 = 0` -> `GetKantoPokedexCount(1)` (caught flags of dex 1-151, `pokedex.c:75-93`, `prof_pc.c:23-29`) in `VAR_0x8006`, `REQUIRED_CAUGHT_MONS 30`.

Route 16 (31,13) has an extra twist: the `Route16` object event at (31,13) is the **Snorlax** (`map.json`; `Route16/scripts.inc:31-54`, `FLAG_HIDE_ROUTE_16_SNORLAX` set before the battle, so it is gone after any outcome but a loss). The Leftovers lie under it, i.e. on the Celadon -> Cycling Road road
(the plan's `snorlax` segment removes it; the Itemfinder is not available then: 12.4 species at that point in the sim, `splits --all`).

## (2) Does CURRENT get the Itemfinder? Does the sim let a plan skip it?

CURRENT has no Itemfinder segment and no underfoot pickup (audit above), so nothing is charged and nothing is skipped: **the 1067 / 1058 / 1057 numbers stand** (no model change for them). The sim, however, *did* have a hole for future plans:
pickup kinds `leftovers` (and `ppmax`, the Cape Brink item) took effect for any `frames` with no Itemfinder and no species condition (the 12-05 table's "free" rows were that). Fixed (`sim.rs`, committed `b30e64a`): new pickup kind `itemfinder` (frames = the whole trip) sets `st.itemfinder` only when
`st.caught.count() >= 30` (Kanto caught flags, which the sim's `Caught` set is: starter, catches, gifts, trades, evolutions); `leftovers` / `ppmax` do nothing without it (their frames are still charged). The existing descent tools only toggle non-underfoot sites (`descend.py` `pick_sites`), so they cannot hit the gate.
The sim has 31.1 species after `berry-forest`, 35.2 at `stones` (mean); the aide would refuse (wasted trip) in the attempts below 30, which the gate now models.

## (3) The Leftovers trip, measured, and the re-optimisation

Geometry (`frlg-mapgraph path`, `forced.py` = 0 forced fights on both legs; static trainers only, wanderers / LOOK_AROUND not modelled):
* Vermilion fly spot (15,7) -> Route 11 gate door (58,10): 105 tiles; gate 1F (door -> stairs) 10; gate 2F (stairs -> (2,7) under the aide at (2,6)) 11: **128 tiles**, the way back 127. Bike is allowed on Route 11 and in the 1F, not in the 2F (`allow_cycling` false, `allow_running` false): 210 tiles x 6 f + 20 x 6 + 22 x 16 + 4 warps ~ 1 900 f (`event_object_movement.c:158-162` frame rates).
* The aide's text: ~17 pages, ~700 characters, the Yes/No, the item fanfare: ~1 000 f (guess; character count at ~1 f/char mashing).
* Extra Fly to Vermilion: 700 f (the plan's own `fly-*` segments, 12 s). The Fly to Celadon after it is the one the plan already makes.
* Celadon fly spot (48,12) -> Route 16 (31,13): 76 tiles each way (`frlg-mapgraph`, objects ignored; the Snorlax is gone): 912 f by bike; use the Itemfinder (bag, 3 dings, dig, text) ~450 f, give Leftovers ~300 f (the Black Glasses swap out).
Total **~5 300 f** (Itemfinder trip 3 600, Route 16 leg 1 700); the specs below use 5 070 (3 410 + 1 660), the scale `s` multiplies both. The earlier journal's 4 000-6 000 f estimate agrees.

Plan 1067 + `[itemfinder @stones, leftovers @fly-celadon-2]`, R 800k, seed 4501, paired vs 1067 (110 h WR+2m / 152 h WR on this seed):

| s (total frames) | WR+2m | WR |
| --- | --- | --- |
| 0.02 (free) | -20.4 [-23.9, -17.4] | -30.9 [-36.9, -25.6] |
| 0.5 (2 500) | -12.1 [-16.0, -9.2] | -11.4 [-17.4, -5.8] |
| 0.75 (3 800) | -5.6 [-9.6, -2.5] | +1.4 [-4.8, +7.7] |
| 0.87 (4 400) | -2.7 [-6.6, +0.8] | |
| **1.0 (5 070)** | **-0.5 [-4.5, +3.2]** | **+17.5 [+10.3, +24.8]** |
| 1.5 (7 600) | +15.9 [+11.4, +20.4] | +53.6 [+43.2, +63.7] |
| 2.0 (10 100) | +37.8 [+32.0, +43.8] | +108.9 [+94.5, +124.6] |
| gate check: `leftovers` 1 660 f, no Itemfinder anywhere | +7.5 [+6.1, +9.2] | +14.5 [+12.0, +17.4] (frames paid, no effect: the gate works) |

Other anchors at the same trip cost (WR+2m only): Sabrina -> Erika (`itemfinder @sabrina-badge`, `leftovers @fly-celadon-3`) s 0.75 +2.7 [-0.7, +6.1], s 1.0 +9.0 [+5.5, +13.3]; after Victory Road s 1.0 +16.1 [+12.8, +19.8]. Earliest is best (`fly-celadon-2`, right after the Sevii stones), and the break-even there is about 5 000 f at WR+2m and about 3 700 f at WR.
Each 1 000 f of extra trip costs about 4.5 h (WR+2m) / 8.7 h (WR) (the 1 660 f no-effect row).
**Verdict: not adopted.** At the measured ~5 300 f it is break-even at WR+2m and +17 h at WR, and x2 costs +38 / +109 h; the rule (robust to pickup cost x2) fails. It would need the trip under ~3 500 f (two fights fewer, nothing to read) and no tier-1 way to get that exists:
the Itemfinder trip cannot be shared with the hidden candies because the candies are not underfoot (they never needed it). Not tested, structural: skipping the Celadon visit before Silph (Vermilion -> Saffron on foot instead of the Celadon Fly) saves the second Fly (700 f) but moves the X buy; the model has no variant for it.
Leftovers on Route 12 (14,70) is the other underfoot item, off the route.

## Next
* Nothing in the pickup set needs changing. If a descent ever proposes `leftovers` / `ppmax` / an Itemfinder segment, the gate and `pickup_audit.py` are there; `ppmax` (Cape Brink) is gated too.
* Measure on tier 1 / video: the aide dialogue length (~1 000 f guess) and the Route 11 walk (128 tiles, 2F on foot) if the trip is ever reconsidered.
