# Research: what stands between the lab battle and Brock

Derived 2026-08-12 from `decompiled/` only. Map-geometry claims (grass counts, "no path
exists") come from decoding the binary `map.bin`/`metatile_attributes.bin` with the formats
the source itself documents (`include/global.fieldmap.h:4-11`, `src/fieldmap.c:61-83`,
`data/layouts/layouts.json`), ledges treated as walls and NPC movement ignored — so grass
counts are lower bounds and the robust conclusions are the no-path ones. All of it gets
re-derived frame-exactly by the emulator once segments exist.

## The mandatory chain

1. **Lab battle aftermath** (`data/maps/PalletTown_ProfessorOaksLab/scripts.inc:467-481`):
   rival walks out, `VAR_MAP_SCENE_..._OAKS_LAB = 4` (`:478`), `FLAG_BEAT_RIVAL_IN_OAKS_LAB`
   (`:479`), control returns. Losing would not white out (`RIVAL_BATTLE_HEAL_AFTER`,
   `src/battle_setup.c:912-924`) but is never the plan. Exit-row triggers are dead at
   scene 4 (`map.json:196-241`); the warps at (5-7,12) lead out.
2. **Oak's Parcel is mandatory.** The Viridian north corridor at y=11 is exactly three
   tiles: the old man object (21,11), the RoadBlocked trigger (22,11)
   (`data/maps/ViridianCity/map.json:203-211`, `scripts.inc:23-27,188-200`), and (20,11)
   whose only approach (20,12) is sealed by the FACE_UP woman (`map.json:92-106`); the
   western bypass is sealed by the Cut tree (18,5) (`map.json:135-149`). No path north —
   decoded-map Dijkstra returns none.
3. **Parcel pickup**: entering Viridian Mart force-plays the clerk scene, `giveitem
   ITEM_OAKS_PARCEL`, mart scene var 1, lab scene var 5
   (`data/maps/ViridianCity_Mart/scripts.inc:15-33`).
4. **Delivery**: talking to Oak at mart var ≥ 1 runs the Pokédex scene
   (`PalletTown_ProfessorOaksLab/scripts.inc:576,598-684`): Pokédex
   (`FLAG_SYS_POKEDEX_GET`, `:656`), 5 Poké Balls (`:660`), and the var fan-out `:678-682`
   — including **`VAR_MAP_SCENE_VIRIDIAN_CITY_OLD_MAN = 1`** (`:680`), the only setter.
5. **The catching tutorial is mandatory.** At old-man var 1 he stands at (21,8)
   (`ViridianCity/scripts.inc:17-21`) flanked by triggers (20,8)/(22,8)
   (`map.json:222-238`) → `DoTutorialBattle` (`scripts.inc:202-237`):
   `special StartOldManTutorialBattle` (`src/battle_setup.c:301`), then var 2 + Teachy TV.
   Row y=9 is passable only at x=20-22, all three entries covered. No decline branch
   (quest-log playback aside, `:227`).
6. **Route 2 south → forest → Route 2 north**: both Route 2 grass blocks have clean
   bypasses (0 forced grass); the forest forces ≥48 grass steps from (29,62) to (5,9),
   51 when also dodging every dodgeable trainer.
7. **Bug Catcher Sammy is forced** (`ViridianForest/map.json:72-86`: (7,22), FACE_LEFT,
   sight 4): his sight tiles (3-6,22) plus his body at (7,22) are the entire width of the
   only corridor to the north-exit pocket. Party: **one Weedle L9, IV 0, default moves**
   (`src/data/trainer_parties.h:304-310`, `trainers.h:1035-1043`, AI CHECK_BAD_MOVE).
   The other four (Rick 5-range, Doug 4, Anthony 1, Charlie 1;
   `map.json:44-156`) are all dodgeable, simultaneously.
8. **Pewter**: south edge → gym warp (15,16) touches neither the gym-guide triggers
   (x=42-43; `PewterCity/map.json:183-218` — they seal Route 3, not the gym) nor any
   grass (Pewter has no wild header at all). In the gym, **Camper Liam ((3,8), FACE_RIGHT,
   sight 4; `PewterCity_Gym/map.json:32-46`) is skippable** along the west wall x=1-2
   through (2,8). **Brock is interaction-only** — no coord events in the gym
   (`map.json:80`), talk from (6,6) facing north (`scripts.inc:4-10`).

Route 22 (early rival, L9 Pidgey + L9 counter-starter) hangs off Viridian's west edge and
is never on the path — optional exp, `data/maps/Route22/scripts.inc:4-47,61-69`.

## Trainer sight, cited once for all of them

`CheckForTrainersWantingBattle` runs each field-input frame *before* movement and encounter
handling (`src/trainer_see.c:88-103`, `src/field_control_avatar.c:209-210`); for
`TRAINER_TYPE_NORMAL` only the current facing direction is tested with
`trainer_sight_or_berry_tree_id` as range (`trainer_see.c:123-146`), same row/column within
range (`:149-194`), path unobstructed (`:198-233`). Beaten trainers are skipped by their
flag (`:105-121`).

## Forced encounter-tile budget (decoded lower bounds)

| Segment | Forced land-encounter steps |
| --- | ---: |
| Pallet, Viridian, Pewter, Route 2 (both halves), gym | 0 |
| Route 1 southbound entry + chokepoints | ≥ 20 |
| Viridian Forest | ≥ 48 (51 dodging all four dodgeable trainers) |

Route 1's very first tile north of Pallet (x=12/13, y=35-39) is tall grass — the corridor
is 2 wide and 5 long. Encounter rates: Route 1/2 = 21, forest = 14
(`src/data/wild_encounters.json`), so cooldown `minSteps` = 6 / 7
(`src/wild_encounter.c:673-699`).

## Exp ledger for the mandatory fights (worked, single participant, trainer ×1.5)

- Lab rival's L5 starter (yield 64-66): 67-70.
- Sammy's Weedle L9 (yield 52): 52·9/7 = 66, ×1.5 = **99**.
- Old-man tutorial: the player's mon is not involved (scripted catch demo) — assumed 0 exp,
  *to be confirmed on the emulator*.

Cumulative for a MEDIUM_SLOW starter from 135 @ L5: mandatory fights end near ~300 exp —
L8. Vine Whip (Bulbasaur, L10, 560) needs ~260 more: Camper Liam (~183 + ~219 = 402,
worked in `starter-and-brock.md`) overshoots comfortably; a couple of forest wilds or one
dodgeable bug catcher are the alternatives. Squirtle's Bubble (L7, 236) is covered by the
rival fight + one wild; Water Gun (L13, 1261) is far. These are the knobs the semi-naive
build will pick between and the sweep will settle.
