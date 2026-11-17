# 2026-12-04: structural dimensions the sim never varied; the item audit finds hidden Rare Candy, Calcium and a Nugget sale (plan 1058)

CURRENT on entry `1057-calm-sp9-ppup-redo` (re-checked first, merged `human-run` 0b692c3: WR+2m 155.8 h at R 800k seed 4401, 155.0 / 161.1 h and WR 213.9 h at R 3.2M seeds 5201 / 5202; the new knobs are default off and
reproduce those numbers to the digit). Tools: scratch `st-*` (probe with list-append patches, corner runner, trace), `human-run/tools/fit_lane.py --check`. Screens R 800k WR+2m seed 4401 vs CURRENT unless said;
confirmations R 3.2M on two fresh seeds, WR+2m and WR. Rules kept: Rash/Mild, reset time counts, `no_salvage_until`, `ball_batch` 20, no Brock potions, no glitches, no save-reload.

## Result: plan `1058-hidden-candy-calcium-nugget` (CURRENT)

Three resources, found by listing every item ball / hidden item / script reward on the Kanto and Sevii maps (`decompiled/data/maps/*/map.json`, `scripts.inc`) against what the plan picks up:
* **hidden Rare Candies** on Route 9 (63,2) and Route 17 (18,83) (hidden, press A facing; Route 17 is a Cycling Road pull-down tile, B held stops the slide: `bike.c:56-72`; the plan already takes the Route 17 PP Up and Max Elixir there);
* **Calcium** (+10 SpA EVs, new `elixirs` kinds `calcium`/`hpup`/`protein`/`iron`/`carbos`/`zinc`; Gen 3 vitamins stop at 100 EVs) in the Rocket Hideout B4F ball (6,23) and the Silph Co. 7F ball (0,12);
* **the Route 24 Nugget** (given by the Rocket before the fight, `Route24/scripts.inc:5-51`) **sold** (new `specials` kind `sell`, half of 10000, `item_menu.c:1933`) at the Celadon Dept. Store visit of the first X buy.
The earlier agents had barred the hidden candies by an evidence guard (`descend.py evidence_ok`, journal 2026-11-19-14-00: "in no stream"); that is a statement about what the runner does, not about what the game allows, and the user's
rule list does not contain it. The guard now lets a candidate keep the parent's hidden candies (it still refuses new ones and more than the parent's count).

Paired vs 1057, R 3.2M: the three changes with the parent's windows: WR+2m -46.3 [-49.7, -43.5] / -47.7 [-51.0, -44.5] (seeds 5301 / 5302), WR -59.4 [-64.3, -54.7] / -60.2 [-65.7, -54.6]. With the lane windows refit on the new plan
(`fit_lane.py`, R 400k; powers 1.11 / 2.57 / 1.35 / 0.77 / 1.57 / 2.31 against 0.99 / 2.61 / 1.45 / 0.78 / 1.43 / 2.37) = **the plan file**: WR+2m 159.8 -> 117.7 h (-42.1 [-46.2, -38.6]) and 157.9 -> 118.7 (-39.2 [-43.2, -35.8]);
WR 222.6 -> 166.0 (-56.6 [-63.1, -50.7]) and 221.4 -> 166.8 (-54.6 [-61.5, -49.1]) (seeds 5401 / 5402). `fit_lane.py --check`: start -> door survival .075 (1057 .060, runner .056), all eight windows inside the runner's CIs
(sevii-gio3 .77 vs .67 [.30, .90], rt-snorlax .89 vs 1.00 [.76, 1.00]).
Robustness: the same three changes at stand-in corners (R 800k, WR+2m, each corner on both plans): carry_power 1.7 / 2.3 -235 / -29 h, league_power 1.0 / 1.8 / 2.2 -198 / -9 / -167, mewtwo_power 1.0 / 1.5 -53 / -32, trainer_fit .76 / .86 -26 / -40,
mistake_rate .16 / .4 -26 / -36, level_gap 0 / 12 -53 / -31: never worse. Pickup frames x2 / x3 / x5 (all new pickups and the sale): -37 / -25 / **+15 h**, break-even about 4x (20-30 s per pickup).
Where the hours come from (attr, R 800k): Sabrina deaths 3475 -> 1091 per 800k attempts (Calcium before Sabrina: the engine's Sabrina fight is a damage breakpoint, win .83 -> .96), rival-silph 5509 -> 4732, Bruno / Lance / Champion 2178 / 2235 / 958 ->
2159 / 2646 / 869 attempts and 4.6 / 4.9 / 2.3 -> 3.1 / 3.9 / 1.4 h (the extra candies are +2 levels at R1). Start -> door survival moves only .060 -> .063 (P1) with the hidden candies; the rest is post-door.
Caveats: (1) none of the three is seen in any stream (the Nugget sits in the WR bag at Rock Tunnel, "Sells: none observed"); (2) all pickup frames are guesses (Route 6 candy: 29 tiles each way, 450 f; Route 9: 7 tiles off the road; Hideout B4F ball (6,23)
is not path-checkable with `frlg-mapgraph` (arrow tiles)); (3) the level / SpA sensitivity is the engine's and carries the `league_power` stand-in's uncertainty; (4) `reset_table` and the post-door `fatal` rows are not refit.

## The dimension list (what the sim lacked or no search varied) and what each test said

| dimension | sim | test (vs 1057, R 800k seed 4401 WR+2m, + is worse) |
| --- | --- | --- |
| version FR instead of LG | knob existed, never rerun on this model | **+44.1** [35.8, 52.6] (LG-tuned fills; the earlier partial FR re-descent was n.s.) |
| starter Bulbasaur / Charmander | knob exists, plan is Squirtle-shaped | 0 finishes: Bulbasaur learns Vine Whip at Lv 10 (`level_up_learnsets.h:4-16`), Squirtle Bubble at 7 (`:101-105`), Charmander's Ember is resisted by Brock; the lane dies at the Pewter gym trainer at Lv 7-8; `fight_until_level` 24 / 30 does not help (the forest has too few fights). Needs its own early plan, a Surf holder (the Bulbasaur line cannot learn Surf) and refit windows: not testable by a patch |
| order variants with the lane windows keyed by fight (new `risk_canon`) | windows were keyed by position, which made every reorder an artifact | surge_early **+78**, safari_after_koga **+167**, erika_cycling **+104**, no warden_early +3.3, skip Power Plant +27, no bike +51, no Cerulean Cave rope +11, no Abra teleport +1.2. `erika_pre_silph` **-37.5** (P1: -24.9): Erika's exp / Center stop comes before the Silph rival and Sabrina and Sabrina flips .83 -> .96 at +1 level, but `fit_lane --check` puts its sevii-gio3 window at .80 against the runner's .67 [.30, .90]; with refit windows it was +7.4 n.s. (journal 2026-11-20). Not adopted |
| dex fill deferred to after the Hall of Fame (new `fill_after_hof`; Oak reads the 60 only with `FLAG_SYS_GAME_CLEAR`, `PalletTown_ProfessorOaksLab/scripts.inc` `TryStartNationalDexScene`; `GetKantoPokedexCount`, `pokedex.c:75-80`) | fill was always before Indigo | **-4.1** [-6.1, -2.4] on 1057, +0.3 [-0.6, 1.3] on the new plan: not adopted |
| Rare Candy kept for Mewtwo (new `mewtwo_candies`) | all candies go to the carry at the Indigo heal | +24 / +53 / +246 for 1 / 2 / 4: the carry's League level is worth more |
| candy_early_n 0 / 2 (on the new plan) | tuned | +67 / +1.5 n.s. |
| hidden Rare Candies | forbidden by the guard | Route 6 -18.3, Route 9 -17.3, Route 17 -18.2, Mansion 3F -16.9 each; all five -15.4 (saturates); Route 6 on top of the adopted ones +1.7. Cerulean City (18,7) lies behind the robbed house (39 tiles through `CERULEAN_CITY_HOUSE1`): not taken |
| selling items (new `sell`) | nothing could be sold | Nugget +5000: -11.2 (Cerulean, 600 f) / -12.4 (Celadon, 350 f); +10k -12.8, +20k -16.2; second Nugget (Tower 5F ball) +3.1; 3 unused TMs at 1500 each at fly-celadon-2 (500 f) +2.2 n.s. |
| vitamins found on the route (new kinds) | not modelled | Calcium: Hideout B4F -12.4 [-15.8, -9.5], Silph 7F -9.9, both -17.0, a third / fourth -15.2 / -13.6 (saturates); HP Up +1.3, Zinc +0.1, Carbos -1.0 n.s. |
| Mystic Water at the Game Corner (20000, knob existed) | tested only on old plans | +19.7 (after Erika) .. +26 .. +176 (money before the second X buy) |
| Tower / Silph / Safari statics and legendaries for the dex (Snorlax Lv 30 `Route12/scripts.inc:22`, Zapdos Lv 50 `PowerPlant/scripts.inc:40`, Articuno `SeafoamIslands_B4F/scripts.inc:159`, Moltres `MtEmber_Summit/scripts.inc:29`, Marowak `PokemonTower_6F/scripts.inc:9`) | only Hypno is a `static` special | not run: catch rate 3 (the three birds) needs ~20 Ultra Balls at 1 HP asleep; Snorlax (catch rate 25, ~6 Ultra Balls at 20% HP after two Surfs) costs more than the fill's 16 s per species |
| Game Corner prizes (LG: Abra 120 coins, Clefairy 750, Pinsir 2500, Dratini 4600, Porygon 6500; TMs 3500-4500 coins `CeladonCity_GameCorner_PrizeRoom/scripts.inc`) | Abra buy only | Pinsir / Dratini / Porygon and Flamethrower / Thunderbolt / Ice Beam (4000 coins = 80000 yen) are out of the purse (17-30 k); Flamethrower (15 PP) for Fire Blast (5 PP) is the one PP-useful buy and is unaffordable before the League |
| Leppa / Sitrus / Lum berries held (hidden: Route 8, 23, 6) | player-held berries not modelled except as text | Leppa (`battle_util.c:2573`, +10 PP when a move hits 0) is an Ether without a turn: Ethers on top of three PP Ups were already refuted (+2.6, journal 2026-12-02-14-00); Persim / Lum effects would only re-fit stand-ins the runner already contains |
| sell vitamins / Rare Candy (price 9800 / 4800: sale 4900 / 2400) | not modelled | not run: money saturates after the first Nugget (second Nugget +3.1) |
| Mewtwo alternatives | Master Ball only | Ultra Ball at 1 HP asleep ~2.4% per ball (catch rate 3): no |
| Rare Candy at Cape Brink, Route 12 | | off the route (Route 12 (9,116) is 100 tiles south of the Lavender exit) |
| drop scan of every special / hunt / fill entry on the new plan | | no drop helps beyond noise (Route 23 fill -0.7 [-1.3, -0.1], 0.6% < the 1% bar); Farfetch'd and Nidoran trades never fire (0.0) |

## Not in the sim, not tested (need evidence or new mechanics)
HM05 Flash for Rock Tunnel (Route 2 aide needs 10 species, no trip is charged; the WR's split is named after it); Pokemon Center / Poke Mart walking costs per stop beyond the guessed frames; party-slot PC logistics beyond `pc_frames`;
dynamic trainers' facing ("bag manip" = waiting in the bag menu for a `LOOK_AROUND` trainer to turn away; the audit treats them as free bodies); Persim / Lum / Leppa held-item effects; hidden-item frames on Cycling Road lanes.

## Next
* Refit `reset_table` (optimal stopping) and the post-door `fatal` rows on 1058; rerun the descents from it (they run elsewhere; `evidence_ok` now lets them keep the hidden candies).
* Measure the pickups on tier 1 (frames): Route 9 (63,2), Route 17 (18,83) with B held, Hideout B4F (6,23), Silph 7F (0,12), the Celadon sell menu.
* Erika-first needs a window model that is not a refit to the runner's order (e.g. per-fight pass rates) before it can be adopted or refuted.
