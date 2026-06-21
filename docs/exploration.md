# Exploration: pokefirered decompilation

A map of where route-relevant data lives in `decompiled/`. This is the pret
**Pokémon FireRed/LeafGreen** decompilation — it builds the real FRLG ROMs, so
the values here are exactly what the game uses. All paths are relative to
`decompiled/`.

## Repo layout (top level)

| Path | Contents |
|------|----------|
| `src/` | C source (289 files): game logic, battle system, menus, field. |
| `src/data/` | Static game data as C headers + JSON (species, trainers, items, encounters). |
| `data/` | Assembly-side data: maps, layouts, scripts, text, tilesets, event scripts. |
| `include/` | Headers; `include/constants/` holds the enums/macros (species, items, maps, moves, opponents…). |
| `constants/` | Assembly `.inc` constants. |
| `graphics/`, `sound/`, `asm/`, `tools/` | Assets, raw asm, and build tools (incl. `tools/mapjson`). |
| `docs/` | Upstream docs: `bugs_and_glitches.md` (useful for glitch routing), WSL install. |

Build SHAs and the build entrypoint are in `README.md` / `Makefile`. Known
vanilla bugs/glitches (relevant for glitch-route reference) are in
`decompiled/docs/bugs_and_glitches.md`.

---

## 1. Wild encounter tables ⭐

**Where:** `src/data/wild_encounters.json` (706 KB) — master per-map encounter
data. Template `src/data/wild_encounters.json.txt` generates slot-probability
`#define`s at build time, consumed by `src/wild_encounter.c` via the generated
`data/wild_encounters.h`.

**Structure** (`include/wild_encounter.h`):
- `WildPokemon` — `{ minLevel, maxLevel, species }`
- `WildPokemonInfo` — `{ encounterRate, wildPokemon[] }`
- `WildPokemonHeader` — `{ mapGroup, mapNum, land, water, rockSmash, fishing }`
- Slot counts: land `12`, water `5`, rock smash `5`, fishing `10`.

A single encounter entry:
```json
{ "min_level": 25, "max_level": 25, "species": "SPECIES_UNOWN" }
```

**Slot probabilities** (per-slot, cumulative in `_SLOT_n` defines):
- Land: `[20,20,10,10,10,10,5,5,4,4,1,1]` (=100)
- Water / Rock Smash: `[60,30,5,4,1]` (=100)
- Fishing (grouped by rod): `[70,30, 60,20,20, 40,40,15,4,1]`
  - Old Rod = slots 0–1, Good Rod = 2–4, Super Rod = 5–9.

**Selection logic** (`src/wild_encounter.c`): `ChooseWildMonIndex_Land`,
`ChooseWildMonIndex_WaterRock`, `ChooseWildMonIndex_Fishing`,
`ChooseWildMonLevel`, `GetCurrentMapWildMonHeaderId`.

Also: `src/wild_pokemon_area.c` / `src/pokedex_area_markers.c` map species →
areas (Pokédex "Area" screen) — useful for "where can I catch X" lookups.

---

## 2. Maps / areas ⭐

**Per-map data:** `data/maps/<MapName>/` — 425 map folders. Each has:
- `map.json` — id, name, layout, music, weather, `map_type`, `requires_flash`,
  `allow_cycling/running/escaping`, plus event arrays:
  - `connections` — overworld adjacency: `{ map, offset, direction }`
  - `warp_events` — doorways: `{ x, y, elevation, dest_map, dest_warp_id }`
  - `object_events` (NPCs/items), `coord_events`, `bg_events` (signs, hidden items)
- `scripts.inc`, `text.inc` — event scripts + dialogue (assembly).

**Master indexes:**
- `data/maps/map_groups.json` — all map names, grouped into 43 groups
  (`Dungeons`, `SpecialArea`, `TownsAndRoutes`, indoor groups per town/route).
- `data/layouts/layouts.json` — layout metadata: `id`, `width`/`height` (blocks),
  border dims, primary/secondary tileset, paths to `map.bin` (blockdata/collision)
  and `border.bin`.
- `data/layouts/<LayoutName>/` — binary `map.bin` + `border.bin`.

**Region map / location names:** `src/data/region_map/region_map_sections.json`
(`MAPSEC_*`), plus `region_map_layout_kanto.h` and Sevii layouts.

**Constants:** `include/constants/maps.h` — `MAP_GROUP(map)`, `MAP_NUM(map)`,
`MAP(map)`. Build glue: `data/maps.s` (includes generated headers via `tools/mapjson`).

For routing, `connections` + `warp_events` across all `map.json` files give the
full traversal graph (overworld tiling + indoor/door transitions).

---

## 3. Catch rates & catch formula ⭐

**Per-species catch rate:** `src/data/pokemon/species_info.h` — `gSpeciesInfo[]`,
each entry has a `.catchRate` field (1–255) alongside base stats/types/exp yield.
Struct is `struct SpeciesInfo` in `include/pokemon.h`.

```c
[SPECIES_BULBASAUR] = {
    .baseHP = 45, .baseAttack = 49, /* ... */
    .types = {TYPE_GRASS, TYPE_POISON},
    .catchRate = 45,
    .expYield = 64,
    /* ... */
}
```

**Capture formula:** `src/battle_script_commands.c`, `Cmd_handleballthrow`
(~lines 9490–9615):
```c
odds = (catchRate * ballMultiplier / 10)
     * (3*maxHP - 2*hp) / (3*maxHP);
if (status & (SLEEP|FREEZE))            odds *= 2;     // ×2
if (status & (POISON|BURN|PARALYSIS|TOX)) odds = odds*15/10; // ×1.5
// shake check:
odds = Sqrt(Sqrt(16711680 / odds)); odds = 1048560 / odds;
for (shakes = 0; shakes < 3 && Random() < odds; shakes++);
```

**Ball multipliers:**
- Base table `sBallCatchBonuses` (~line 808): Ultra ×2.0, Great ×1.5, Poké ×1.0,
  Safari ×1.5 (values stored ×10).
- Conditional (~9503–9544): Net ×3 (water/bug), Dive ×3.5 (underwater),
  Nest `(40-level)`, Repeat ×3 (if already caught), Timer `turns+10` capped ×4,
  Master = guaranteed.
- Enums: `include/pokeball.h`, item IDs `include/constants/items.h`.

---

## 4. Trainer battles ⭐

**Trainer DB:** `src/data/trainers.h` — `const struct Trainer gTrainers[]`
(854 entries). Struct (`include/battle.h`):
```c
struct Trainer {
    u8 partyFlags;            // F_TRAINER_PARTY_CUSTOM_MOVESET / HELD_ITEM
    u8 trainerClass;
    u8 encounterMusic_gender;
    u8 trainerPic;
    u8 trainerName[12];
    u16 items[4];             // in-battle items the trainer uses
    bool8 doubleBattle;
    u32 aiFlags;
    u8 partySize;
    union TrainerMonPtr party;
};
```

**Parties:** `src/data/trainer_parties.h` — named arrays (`sParty_LeaderBrock`,
`sParty_EliteFourLorelei`, …). Four mon-entry shapes depending on flags:
`{No,}Item × {Default,Custom}Moves`, each with `iv`, `lvl`, `species`, and
optionally `heldItem` / `moves[4]`.

```c
static const struct TrainerMonNoItemCustomMoves sParty_LeaderBrock[] = {
    { .iv=0, .lvl=12, .species=SPECIES_GEODUDE, .moves={MOVE_TACKLE,MOVE_DEFENSE_CURL} },
    { .iv=0, .lvl=14, .species=SPECIES_ONIX,    .moves={MOVE_TACKLE,MOVE_BIND,MOVE_ROCK_TOMB} },
};
```

**IDs / classes:**
- `include/constants/opponents.h` — 743 `TRAINER_*` IDs. Gym leaders
  (`TRAINER_LEADER_BROCK`…), Elite Four (`TRAINER_ELITE_FOUR_*`), rivals
  (`TRAINER_RIVAL_*`) are distinct constants → easy to pick out boss fights.
- `include/constants/trainers.h` — classes (`TRAINER_CLASS_LEADER`,
  `_ELITE_FOUR`, `_RIVAL_EARLY`…), pics, encounter music, party flags.
- `include/constants/battle_ai.h` — AI flags (`AI_SCRIPT_CHECK_BAD_MOVE`,
  `TRY_TO_FAINT`, `CHECK_VIABILITY`, …).

**Triggering a fight:**
- `src/trainer_see.c` — `CheckForTrainersWantingBattle`, sight-range/approach
  detection (`ObjectEvent.trainerRange_berryTreeId` holds sight range).
- `src/battle_setup.c` — `BattleSetup_ConfigureTrainerBattle`, `StartTrainerBattle`,
  `SetBattledTrainerFlag`, `GetTrainerBattleMode`.
- Per-map `data/maps/<Map>/scripts.inc` uses `trainerbattle_single/double` with
  a `TRAINER_*` id; generic handlers in `data/scripts/trainer_battle.inc`.

---

## 5. Other route-relevant data

In `src/data/pokemon/`:
- `experience_tables.h` — XP-curve tables (level-up timing / grind cost).
- `level_up_learnsets.h` (+ `_pointers.h`) — moves learned by level.
- `tmhm_learnsets.h`, `tutor_learnsets.h`, `egg_moves.h` — other move sources.
- `evolution.h` — evolution methods & levels.
- `pokedex_entries.h`, `pokedex_categories.h`, `pokedex_orders.h` — dex data.

Elsewhere in `src/data/`:
- `items.json` — item definitions (prices, effects, pockets).
- `ingame_trades.h` — in-game NPC trades (can be route shortcuts).
- `heal_locations.json` — respawn/heal points (Poké Center / map heal spots).
- `trainer_class_lookups.h` — class → name/AI mappings.

Relevant source:
- `src/pokemon.c` — core stat/exp/catch helpers; `src/battle_main.c` battle core.
- `src/daycare.c`, `src/learn_move.c`, `src/tm_case.c`, `src/item_use.c`.
- `src/script_pokemon_util.c` — gift/script-given Pokémon.

---

## Quick reference: "where do I look for…"

| Question | File(s) |
|----------|---------|
| What spawns on Route N, and at what rate? | `src/data/wild_encounters.json` + slot tables above |
| How do maps connect / where do warps go? | `data/maps/<Map>/map.json` (`connections`, `warp_events`) |
| Catch chance for species X with ball Y? | `species_info.h` `.catchRate` + formula in `battle_script_commands.c` |
| What's in a trainer's party? | `src/data/trainers.h` → `sParty_*` in `src/data/trainer_parties.h` |
| When does my mon learn move M / evolve? | `level_up_learnsets.h`, `evolution.h` |
| How much XP to reach level L? | `experience_tables.h` |
| Where can I heal / respawn? | `src/data/heal_locations.json` |
| Known glitches to exploit? | `docs/bugs_and_glitches.md` |
