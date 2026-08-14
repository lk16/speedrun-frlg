# Full glitchless run: route sketch (distances, transport, roadblocks)

> **Status: RESEARCH SCAFFOLDING, 2026-08-14.** Companion to `plan.md`. Nothing here is a
> route; these are the measured tile distances, the transport modes the route can buy, and the
> roadblocks it must clear, so segment planning starts from numbers instead of memory. All
> distances come from `bin/frlg-mapgraph` (BFS over the decomp's `map.bin` collision bits,
> warps and map connections — the same model that produced `plan.md` §3's forced-trainer
> verdicts) and are regenerable with `bin/frlg-mapgraph dump`. Raw data:
> [`distances.json`](distances.json).
>
> **Model caveats** (also in the tool header): static NPC bodies block, wanderers don't;
> script-`setmetatile` barriers (gym puzzles, Silph doors) and forced-movement tiles (spin
> tiles, water currents) are not modeled; warps cost 1 tile. Distances through puzzle maps are
> lower bounds. Tile costs ignore battles, dialogue, and menus entirely.

## 1. Cost of a tile, by transport

Frames per tile, from the sprite step tables at
`decompiled/src/event_object_movement.c:8866-8931` and the speed assignments at `:158-162`:

| Mode | Frames/tile | Unlock | Citation |
| --- | --- | --- | --- |
| Walk | 16 | always | `MOVE_SPEED_NORMAL` = 16 × `Step1` |
| Run | 8 | Running Shoes: Oak's Aide cutscene, Pewter east exit — so the whole opening through Brock is walk-speed | `FLAG_SYS_B_DASH` gate `decompiled/src/field_player_avatar.c:516-524`; giver `decompiled/data/maps/PewterCity/scripts.inc:695-724`; per-map `allow_running` |
| Surf | 8 | HM03 + Soul Badge | "Same speed as running", `decompiled/src/field_player_avatar.c:509-514` |
| Bike | 6 | Bike Voucher (Vermilion Fan Club) → free bike, Cerulean shop (`plan.md` §6 — the purchase path is dead code) | `MOVE_SPEED_FAST_2` = "water current / bicycle", `decompiled/src/event_object_movement.c:161`; per-map `allow_cycling` |
| Water current | 6, forced | Seafoam interior only (on the mandatory path) | same speed row; MB `0x50-0x53` |
| Fly | warp | HM02 (`decompiled/data/maps/Route16_House/scripts.inc:7-10`) + Thunder Badge | `plan.md` §7 |
| Teleport | warp | Abra learns it at L1; no badge | `plan.md` §7 |
| Dig / Escape Rope | warp to dungeon entrance | **TM28 is handed over by the mandatory Cerulean Dig grunt** (`plan.md` §3) | `plan.md` §7 |

The run's effective speed therefore has three eras: walk (Pallet→Brock), run (Brock→bike),
bike (everywhere `allow_cycling`, which is most of the overworld). A tile saved before Brock is
worth 16 frames; after the bike, 6.

## 2. Measured legs

Tiles, shortest static path, from `distances.json` (regenerate with `bin/frlg-mapgraph dump`;
ad-hoc queries with `bin/frlg-mapgraph path MAP_A x,y MAP_B x,y [--surf --cut]`).

| Leg | Tiles | Notes |
| --- | --- | --- |
| Pallet house → Oak's Lab | 18 | walk era |
| Oak's Lab → Viridian Mart | 116 | walk era, ×2+ for the parcel round trip |
| Viridian Mart → Forest south | 91 | |
| Forest south → north | 139 | includes the forced Sammy corridor |
| Forest north → Pewter Gym | 84 | |
| Pewter Gym → Cerulean PC | 492 | through all of Mt Moon; assumes the fossil passage is opened (see §3) |
| Cerulean PC → Bill's door | 156 | one way; Nugget Bridge + S.S. Ticket detour, ×2 |
| Cerulean PC → Cerulean Gym | 13 | |
| Cerulean PC → Vermilion PC | 260 | via Route 5 Underground Path (Saffron is Tea-gated) |
| Vermilion PC → S.S. Anne dock | 60 | |
| Vermilion PC → Vermilion Gym | 36 | needs Cut (no cut: unreachable) |
| Cerulean PC → Lavender PC | 218 | via Cut tree + Rock Tunnel. Without Cut the only alternative transits Saffron (303 tiles, Tea-gated); with neither Cut nor Tea, Lavender is unreachable — **Cut precedes Rock Tunnel** |
| Lavender PC → Pokémon Tower | 15 | |
| Lavender PC → Celadon PC | 227 | via Route 8 Underground Path |
| Celadon PC → Game Corner | 26 | Rocket Hideout chain |
| Celadon PC → Condominiums | 20 | Tea pickup |
| Celadon PC → Celadon Gym | 86 | needs Cut |
| Celadon PC → Silph door | 79 | Tea assumed |
| Silph door → Saffron Gym | 43 | post-Silph (roadblock rockets cleared) |
| Lavender PC → Fuchsia PC | 547 | Routes 12-15, Snorlax assumed woken |
| Celadon PC → Fuchsia PC | 374 | Cycling Road, Snorlax assumed woken; **mostly bike-era tiles: ≈374×6 vs ≈547×8 frames — the Celadon side wins even before menus** |
| Fuchsia PC → Safari entrance | 70 | Surf + Gold Teeth are inside |
| Fuchsia PC → Warden's house | 9 | Strength |
| Fuchsia PC → Fuchsia Gym | 19 | |
| Fuchsia PC → Cinnabar PC | 287 | surf; **passes through the Seafoam Islands interior — Route 20 is severed at the islands** (the BFS could not stay on water), and the interior has current/boulder puzzles the model ignores, so 287 is optimistic |
| Pallet → Cinnabar PC | 146 | surf straight down Route 21. With Fly/Teleport to Pallet this dominates the Seafoam crossing |
| Cinnabar PC → Cinnabar Gym | 17 | |
| Viridian Mart → Viridian Gym | 55 | |
| Viridian Mart → Route 22 gate | 113 | |
| Route 22 gate → Victory Road door | 167 | Route 23, badge checks + a surf stretch |
| Victory Road → Indigo PC | 155 | **boulders ignored** — real path must solve Strength puzzles, so this is a floor |

## 3. Roadblocks, in rough story order

Every entry is a hard gate (script or geometry), with its opener. Trainer-shaped blocks are in
`plan.md` §3; this list is the *non-battle* topology plus where each key/item comes from.

1. **Pewter east exit** — gym-guide trigger walks you back until Brock is beaten
   (`decompiled/data/maps/PewterCity/map.json` coord_events (42,21-23),
   `.../scripts.inc:256+`). Running Shoes come from the aide trigger right behind it.
2. **Mt Moon B2F fossil passage** — the exit chamber is only reachable across the two
   fossil-pedestal tiles; they clear only after defeating Super Nerd Miguel and taking a fossil
   (both fossil objects are removed together; Miguel ends at (14,8), leaving (13,7) open) —
   `decompiled/data/maps/MtMoon_B2F/scripts.inc:25-83` + layout BFS. **A fossil pickup is
   mandatory**, a fact the dependency graph previously missed.
3. **Route 9 Cut tree** — Cut (S.S. Anne captain) gates Rock Tunnel and everything east of
   Cerulean until Tea exists (§2 table). S.S. Anne needs the S.S. Ticket from Bill (Route 25).
4. **Snorlax ×2** — Routes 12 and 16, both `FLAG_GOT_POKE_FLUTE`-gated; flute ← Mr. Fuji ←
   Marowak (Silph Scope) ← Hideout Giovanni ← Game Corner poster grunt (`plan.md` §1).
   Only one Snorlax needs waking; the Route 16 side feeds Cycling Road (§2 table favors it).
5. **Cycling Road gate** — needs the Bicycle (`FLAG_GOT_BICYCLE` check,
   `decompiled/data/maps/Route16_NorthEntrance_1F/scripts.inc:7`); bike ← voucher ← Vermilion
   Fan Club chairman.
6. **Saffron gates** — Tea (`FLAG_GOT_TEA`), from the Celadon Condominiums (`plan.md` §1).
   Underground Paths bypass Saffron for transit both ways.
7. **Silph Co.** — Card Key (5F) opens the door grid; the 7F→11F warp pad is the only entry to
   Giovanni's side of 11F; beating him clears the 8 Saffron roadblock rockets, including the
   one on the gym door (`plan.md` §3).
8. **Safari Zone** — 500 entry fee (mandatory money sink, `plan.md` §6); Surf (Secret House)
   and Gold Teeth → Strength (Warden) are inside.
9. **Seafoam Islands sever Route 20** — do not route Fuchsia→Cinnabar directly; go
   Pallet→Route 21 with Fly/Teleport positioning instead (§2 table).
10. **Route 22/23 badge chain + Viridian Gym door** — `plan.md` §1.
11. **Victory Road Strength puzzles** — boulder pushes required; static BFS only gives a floor
    (§2 table); needs its own model or emulator work.

## 4. What this buys the next session

- **Transport plan skeleton**: walk to Brock, shoes; bike from Cerulean (after the Vermilion
  voucher round trip — whether the voucher detour pays for itself is a frames question the §2
  table can now put bounds on); Teleport-Abra as a free "return to last Center" once a Route
  24/25 Abra is caught (both versions, `decompiled/src/data/wild_encounters.json`
  `sRoute24_*`/`sRoute25_*`; per `tmhm_learnsets.h` Abra's only HM is Flash, so it is *not* the
  Cut carrier); Fly after Surge for the endgame star pattern (Cinnabar via Pallet, League via
  Viridian).
- **Gym order seed**: the middle six ordering should be searched over the §2 legs, but the
  hard edges (Cut→Surge, Cut→Erika-door, Tea→Sabrina, Silph→Sabrina's-door,
  flute→Fuchsia→Surf/Strength→Blaine) leave less freedom than "order-free" suggests: Surge and
  Erika float, Koga/Sabrina/Blaine sit in a forced late cluster.
- **Open modeling debts**: Victory Road boulders, Seafoam currents, gym interiors
  (`plan.md` §8.3), and frame costs of menus/doors/dialogue, which tiles don't see.
