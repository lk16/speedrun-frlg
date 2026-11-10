# Which trainers can the route not walk around? (sight-cone audit)

Replaces the guessed trainer groups of plans 001-040 for the legs below. Tool:
`human-run/tools/forced.py` (stdlib; reuses `bin/frlg-mapgraph`'s tile model). Plans from 041 on
read these lists (`audited_forced: true`, `crates/frlg-human/src/route.rs`, `B::forced`).

## Method

* Tiles: collision bits and warps/connections exactly as `bin/frlg-mapgraph` reads them
  (`decompiled/src/fieldmap.c:61-72`); static NPC bodies block.
* A trainer notices the player when the player stands in the trainer's facing line, within its sight
  range, with no collision or object on the tiles between (`CheckPathBetweenTrainerAndPlayer`,
  `decompiled/src/trainer_see.c:198-231`; sight distance checks `trainer_see.c:146-196`).
  Objects come from each `map.json` (`movement_type FACE_*`, `trainer_type`,
  `trainer_sight_or_berry_tree_id`).
* Search: label-setting search over (map, x, y, set of triggered trainers), cheapest set first.
  Entering a cone adds that trainer to the set (he fights, then no longer blocks). The result is the
  smallest set of fights any path needs, and every minimal set, so "in every minimal set" is truly
  forced and the others are interchangeable.
* Where minimal sets differ (e.g. one of two interchangeable trainers) the simulator takes the
  weakest party (`B::forced`).

## Result (start and end tiles are the door/warp tiles in `bin/frlg-mapgraph`'s leg table)

| Leg | Forced | Guessed before |
| --- | --- | --- |
| Viridian Forest | Sammy (1) | 3 |
| Route 3 | Colton, James + one of Greg/Sally (3) | 0 (skipped) |
| Mt Moon | Grunt1 on B2F, Miguel (fossil gate, `plan.md` §3) | 5 skipped + Miguel |
| Route 24 | Ethan, Reli, Timmy, Ali, Cale + the Rocket (coord-event trigger at (10,15)/(11,15), `Route24/map.json`) | 6 hardest of the map |
| Route 25 (Bill) | Haley, Kelsey + 2 of Flint/Franklin/Nob/Wayne (4) | 0 |
| Route 6 | Jeff (1) | 0 |
| Route 8 (to Celadon) | one of Julia/Rich (1) | 0 |
| Route 9 + Rock Tunnel | Alicia, Conner; Ashton, Dana, Dudley, Martha, Sofia, Winston; one of Lenny/Oliver; one of Allen/Eric (10) | 7 |
| Pokémon Tower 3F-6F | Emilia, Jennifer + one of Karina/Tammy (3) | 0 (plan.md called them "believed dodgeable"; refuted) |
| Cerulean / Fuchsia / Viridian gym | Diana / Kirk / Takashi (1 each) | 1 / 3 / 3 |
| Celadon gym (Cut tree) | any 2 of the 7 (ring around Erika) | 2 |
| Saffron gym (teleport pads) | 0 | 2 |
| Rocket Hideout B1F to B4F stairs | 0 on the stairs side (plus the Lift Key grunt and Giovanni, `plan.md` §3) | 3 |
| Silph Co 1F to 11F (doors NOT modelled) | 0 (lower bound) | 4 |
| Victory Road (boulders ignored or not: same) | 0 | 4-8 |
| Mt Ember exterior to the summit entrance | Logan (1) | 3 |
| Pokémon Mansion 1F to the Secret Key (switch state modelled, `tools/mansion.py`) | 0 (static cones; Johnson, Arnie, Simon and Lewis wander or look around and are bodies only) | 3 |
| Saffron Dojo to the Hitmon balls (`forced.py`) | 2 of 4 students plus Master Koichi (Hitmonlee/Hitmonchan L37): ~5 min for one species, not used | n/a |
| Cerulean to Vermilion (Routes 5/6), Celadon to Fuchsia (Cycling Road), Lavender to Fuchsia | Jeff on Route 6 only; others 0 | 0 |
| Vermilion gym (switch barrier opened) | 0 (the rotating trainers excepted) | 2 |

## Caveats (what this does not establish)

* Rotating and wandering trainers (`LOOK_AROUND`, `WANDER_*`) are bodies only; their cones move and a
  human can time them. The Vermilion gym's two are of this kind.
* Script-driven barriers are not modelled unless opened by hand: Silph card-key doors, Cinnabar quiz
  doors, the Mansion's switches, Vermilion's trash-can barrier (opened by `--open`).
* Elevation differences between a trainer and the player are ignored (they can only remove cones, so
  the counts are upper bounds on cone-forced fights, not on script-forced ones).
* `coord_events` that start a fight on their own (the Route 24 Rocket, rivals, the Dig grunt, Silph
  7F rival, Giovanni) are not seen by the cone search; the route lists them explicitly from
  `docs/glitchless-run/plan.md` §3.
* The Mansion is audited with its switch (`tools/mansion.py`: state OFF is `map.bin`, state ON the
  `PressSwitch_<floor>` tile lists, a statue flips it); the Blaine quiz (answer correctly and no trainer
  fights; the human is assumed to know the answers) and the Silph grunts (card-key doors assumed opened as
  needed; no cone on the 1F-to-11F path) rest on the static model. Sensitivity: if those four groups
  (Silph 4, Mansion 3, quiz 2, Surge gym 2 trainers) were forced after all, EHT at WR+80m goes from
  5.5 h to 10.4 h (plan 163).
