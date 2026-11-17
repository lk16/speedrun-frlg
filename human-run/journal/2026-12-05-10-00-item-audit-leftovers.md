# 2026-12-05: item audit continued (every ball / hidden item), vitamin and sale modelling, Leftovers needs the Itemfinder

CURRENT stays `1058-hidden-candy-calcium-nugget` (re-checked first, merged `human-run` first): WR+2m 116.8 h, WR 162.9 h (R 1.6M, seed 5601; the plan file says 117.7 / 166.0).
Screens: R 800k, WR+2m, seed 4501, paired against CURRENT (parent 124.3 h on that seed), + is worse; two-rung checks R 1.6M seed 5601. Scratch prefix `it-`; rows in `scores.jsonl` (`it-*`), list in `data/item-audit-maps.json`
(349 item balls and hidden items from `decompiled/data/maps/*/map.json`, with the `underfoot` flag).

## Enumeration (what each class is worth to the run)

| class | where (on or near the route) | verdict |
| --- | --- | --- |
| Rare Candy | taken: Mt Moon 1F, Hideout B3F, Tower 6F, Warden, Silph 10F, Victory Road, Route 9 / 17 hidden. Not taken: Route 6 hidden (19,5), Mansion 3F hidden (36,13), Cape Brink hidden (15,13) (Two Island, 85 tiles from the harbour, ~110 tiles detour), Cerulean (18,7) (behind the robbed house), Route 12 (9,116), Lost Cave | free (1 frame) each -3.5 h (-3%), they do not stack (saturated: the carry's level is not the limit once 8 are eaten); at a realistic 400 f -1.8 [-4.5, +0.5], bundle with Calcium / HP Up -0.7 at x1, **+9.1 [+4.4, +13.1] at x2** |
| Vitamins (+10 EV, 100 per stat cap `pokemon.c:4235-4246`, `EV_ITEM_RAISE_LIMIT 100`, amount `ITEM6_ADD_EV 10`) | Calcium: Hideout B4F, Silph 7F (both in the plan), Mansion 2F ball, Silph 9F hidden; HP Up Mansion 2F, Silph 6F ball / 10F hidden; Iron / Protein / Carbos / Zinc many | free: Mansion Calcium -3.2, three Calciums at once -6.7, HP Up -3.2, Silph 9F Calcium -1.2, Iron -0.7, Protein 0.0; at 400-450 f: Mansion Calcium -1.8 [-3.6, 0.1], HP Up -0.8, three Calciums -0.3: n.s., saturated |
| PP Up / PP Max | plan has 3 PP Ups (Hideout B1F, Route 17, Celadon). Silph 5F hidden, Three Island hidden: free 0.0 / +0.1 (the Mewtwo slots are full: Psychic, Fire Blast, Earthquake x3 each, `apply_pp_ups`). PP Max Cape Brink: underfoot, needs the Itemfinder anyway, +0.1 | refuted |
| Healing items (Full Restore, Max Potion, Max Revive...) | Route 17 / 23, Victory Road hidden Full Restores, Safari, Power Plant, Silph, Mansion, Cerulean Cave | money is not binding (17-30 k mid-run, the Indigo mart buys 6 Max Potions at 2500, `IndigoPlateau_PokemonCenter_1F_Items`): 3 free heal items +3.6 [-0.2, +7.6], 3 free Revives +0.2: nothing to gain |
| Sellables (Nugget 10 000 / 2, `item_menu.c:1933`; Pearl, Big Pearl, Star Piece, Heart Scale) | Route 24 Nugget (plan); Tower, Safari Center, Seafoam B3F, Silph 8F, Cerulean Cave Nuggets, Rocket Hideout B3F hidden Nugget (1,3); Pearls in Rock Tunnel and the Warehouse | saturated after the first Nugget (second +3.1, journal 12-04); the Warehouse valuables come after the last purchase |
| Held items | Never-Melt Ice (Icefall B1F (21,7), 8 tiles detour, `HOLD_EFFECT_ICE_POWER`, Ice moves x1.1): free +0.5, 450 f **+2.8 [+1.9, +3.7]** (it replaces the Black Glasses' Bite boost and only reaches round 2): refuted. **Leftovers**: see below. Quick Claw (Safari North), Sitrus / Leppa / Lum berries: not modelled for the carry | |
| TMs, stones, X items | TM teaches are a descent knob (`[tm]` rows in `scores.jsonl`, e.g. Thunderbolt@mewtwo); stones (Leaf / Thunder / Moon x3) already taken, Water / Fire stones come from the Celadon store; X Special ball (Silph 6F) saves 350 yen, money not binding | nothing to add |

## Leftovers: the one big item, and it is gated

Route 16 hidden Leftovers (31,13) is **0 tiles off the path** from Celadon (`it-dist.py`: detour 0, 5 tiles from the gate door), but the hidden item is `underfoot: true`: `field_control_avatar.c:497-500` returns NULL for
underfoot hidden items when pressing A, they are only found with the Itemfinder (`itemfinder.c:241-246`), which the Route 11 east-gate aide (`Route11_EastEntrance_2F/scripts.inc:56-73`) gives at **30 Kanto species caught**
(`prof_pc.c:23-29`, `REQUIRED_CAUGHT_MONS 30`). The sim has 30 species from the Three Island Berry Forest on (31.9), so the earliest sensible trip is the fly to Celadon after the Sevii stones (`fly-celadon-2`):
fly Vermilion instead, ~125 tiles east on Route 11 (10 trainers, 1-3 on the line), the aide (Yes/No, count, receive, explanation), ~125 tiles back, fly Celadon, Route 16 (31,13) with the Itemfinder, give Leftovers.
My estimate: **4 000-6 000 frames** (bike 6 f/tile: 1 500; 1-2 trainer fights ~1 500; dialogue ~900; Itemfinder and give ~1 000). Engine: new `Mon.leftovers` (`battle_util.c:2626-2639`: maxHP/16 at turn end while below max, replaces the Black Glasses boost),
pickup kind `leftovers` in `elixirs`.

| frames at `fly-celadon-2` | WR+2m (R 1.6M, seed 5601) | WR |
| --- | --- | --- |
| 1 (free) | -25.7 [-30.1, -22.0] (R 800k) | |
| 2 350 | -13.5 [-16.3, -10.6] | -16.1 [-20.7, -11.5] |
| 3 000 | -14.0 [-18.6, -10.0] (R 800k) | |
| 4 700 | -2.2 [-5.0, +1.0] | **+8.9 [+3.6, +14.6]** |
| 6 000 | +1.3 [-3.7, +6.0] (R 800k) | |
| 9 400 | **+30.7 [+26.9, +35.1]** | **+85.6 [+76.2, +96.2]** |

(free at 'cycling-road' 500 f: -36.3; free after Victory Road only: -11.2; free after Berry Forest: same as `fly-celadon-2`.) Break-even is at about 4 000 f (WR+2m) and 3 500 f (WR); my central cost is at or above it and x2 loses 30-85 h.
**Not adopted** (rule: robust to pickup cost x2). It is the only item the audit found that is worth more than the existing pickups (a 25 h free gain, healing 6% per turn in every fight), so a measured cost matters:
if the Route 11 trip can be done in under ~3 000 f (two fights or fewer, short menus), it would be worth -14 h (WR+2m). Needs tier-1 / video evidence on the aide trip. Also: with the Itemfinder, Route 12 Leftovers (Snorlax road) and Cape Brink PP Max are the only other underfoot items; neither helps.

## Modelling checks

* Vitamins: `pokemon.c:4235-4246` (+10 per use, refused at 100 EVs in the stat, total 510 cap, `CalculateMonStats` on use) is what `elixirs` kinds `calcium` etc. do (`sim.rs`): correct, including the side effect that a vitamin flushes the carry's pending battle EVs (`ev_lag`).
  Per-point value is small (+10 EVs = +2.5 EV bonus -> +1 stat at Lv 45), so the -12 h of Calcium in 1058 is the Sabrina damage breakpoint plus the flush, as the 12-04 attribution says.
* **Fixed**: the Rare Candies eaten at the Indigo heal bumped the level but did not run `CalculateMonStats`, so the carry's cached stats missed the EVs gained since the last level-up (`ev_lag` on in the plan). Now `st.evs = st.evs_real` there. Effect on CURRENT: -0.08 h (WR+2m, seed 4501), i.e. nothing; kept because it is correct.
* Selling: `ItemId_GetPrice / 2` (`item_menu.c:1933`), Nugget 10 000 (`items.json`): correct. Indigo mart sells Full Restore / Max Potion; `league_heal_price` 2500 is Max Potion: correct.
* Bag limits: the Items pocket has 42 slots (`BAG_ITEMS_COUNT`); the plan holds about 25 distinct kinds, X stacks are far below 99: not binding, not modelled.
* Pickup frames of the adopted four, from tile counts (`bin/frlg-mapgraph`, `tools`-style helper `it-dist.py` in the session scratch): Silph 7F Calcium (0,12) is 5 tiles from the 7F west-corridor trainer Joshua (3,13); if the route passes him (card doors are not modelled by the tool, so this is a lower bound) the detour is ~10 tiles (~200 f, plan 450);
  Route 9 candy 7 tiles off the road (~150-300 f, plan 300); Hideout B4F (6,23) is not path-checkable (the tool returns unreachable for B4F), ~13 tiles from the stairs (~26 round trip, ~300-450 f). The plan's frames are, if anything, high, which supports the x2-x4 robustness already measured.

## Structural dimension (4)

Items do not unlock a bigger restructuring: money is not binding (more heal items and Revives are +3.6 / +0.2 at zero cost, `league_heals` 14 is 0% in ROBUSTNESS), candies and vitamins saturate at about -3 h per free pickup, PP Ups are capped by Mewtwo's four slots,
and the only large lever (Leftovers) is gated behind a trip that costs about what it returns.

## Next
* Measure the Itemfinder trip (Vermilion -> Route 11 aide -> Celadon -> Route 16 (31,13)) in frames; if < 3 000 f it is a -14 h plan change (it needs a segment for the fly to Vermilion, the aide, Route 11 trainers in the walk, and `elixirs` `leftovers` after `fly-celadon-2`).
* Leftovers makes in-fight healing valuable: a `league_preheal` / `heal_below` refit would be the follow-up if the trip is adopted.
