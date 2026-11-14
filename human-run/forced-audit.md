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

## Round 2 (2026-11-15): the four skipped groups of plan 803, each checked

Plan 803 (`skip_groups`) skipped four groups outright: Silph grunts (4), Surge gym trainers (2), Mansion
trainers (3), Blaine quiz (2). The other seven names in its `skip_groups` are no-ops under
`audited_forced` (`B::forced` ignores `skip`; those legs are the forced lists above and are fought).
Tool for Silph: `human-run/tools/silph_audit.py` (card-key doors closed until the Card Key is taken,
elevator, trigger tiles counted as fights). Everything below cites `decompiled/`; the WR videos give no
evidence on any of these (scene samples are one per minute, no transcript mentions Silph's grunts), so
nothing here is cross-checked against what the runner fights, and that is stated, not assumed.

| Group (803) | Verdict | Evidence |
| --- | --- | --- |
| Silph grunts (4 skipped) | **1 forced**: 11F Grunt2 (TRAINER_TEAM_ROCKET_GRUNT_41). Rival 7F and Giovanni are fought anyway. The other 3 grunts are avoidable | `SilphCo_11F/map.json`: Grunt2 at (2,19) `FACE_UP` sight 5, cone (2,14..18). The left corridor x=0..2 meets the only way east to the Giovanni trigger (5,15)/(6,15) on row 18, and (2,19) is a body, so (2,18) must be crossed. `trainer_see.c:198-231`. Doors do not change it: `silph_audit.py` minimum = {11F Grunt2, 7F rival trigger, 11F Giovanni trigger} with the doors closed before the key and open after, with and without the elevator |
| Silph card-key doors | Not a fight, but a cost: 3 doors on the shortest key path (5F D2, 3F D2, 11F D), each `playfanfare` + message + `setmetatile` | `data/scripts/silphco_doors.inc:509-535` (`EventScript_OpenDoor`, needs `FLAG_HIDE_SILPH_CO_5F_CARD_KEY`), `SilphCo_<n>F_OnLoad` close them while their flag is unset. Doors are closed in `silph_audit.py` phase 0 (1F to the Card Key ball `SilphCo_5F_EventScript_ItemCardKey`): reachable with no fight. Elevator `SilphCo_Elevator/scripts.inc` has no key check; it saves ~26 steps |
| Silph, rotating trainers | 5F Grunt1 (9,21) `FACE_UP_AND_RIGHT` sight 3 would be a 2nd forced fight if the human cannot time its facing (`--rotate`) | `silph_audit.py --rotate`; plan 901 prices it |
| Surge gym trainers (2 skipped) | **1 forced**: Gentleman Tucker (7,8) `FACE_LEFT` sight 3 covers (4..6,8), the three tiles below the barrier gap (4..6,6..7); no other way up. Baily (2,11) and Dwayne (8,13) are `LOOK_AROUND` sight 3 (rotating) | `VermilionCity_Gym/map.json`; `forced.py MAP_VERMILION_CITY_GYM 5,19 ... 5,5 --open MAP_VERMILION_CITY_GYM:4-6,6-7` gives Tucker in every minimal set. This corrects the Result table above, whose "0 (the rotating trainers excepted)" for the Vermilion gym missed Tucker. Barrier: `VermilionCity_Gym_OnLoad` (`scripts.inc:11-40`), opened by `FLAG_FOUND_BOTH_VERMILION_GYM_SWITCHES` |
| Mansion trainers (3 skipped) | **0 forced**, justified | `tools/mansion.py` (switch state modelled): 0. Ted 1F (22,24) `FACE_LEFT` s4, Braydon 3F `FACE_DOWN` s1 and Ivan B1F (34,13) `FACE_DOWN` s6 are static but off the path; Johnson, Arnie, Simon, Lewis wander or look around at sight 1 (bodies only; a human can keep a tile away) |
| Blaine quiz (2 skipped) | **0 fights, but 6 quiz texts**: the six doors form a chain to Blaine (doors 1..6 at (26-27,8-10), (17-18,8-10), (17-18,15-17), (11,21-23), (5-6,16-18), (5-6,8-10)), every question must be answered; all seven trainers are `FACE_DOWN` sight 0 and only fight after a wrong answer | `CinnabarIsland_Gym/map.json`, `scripts.inc:5-21,656-` (OpenDoor1-6), `Quiz<n>Incorrect` start the fight. The model charged nothing when the group was skipped; plan 900 charges 2700 frames (guess, ~450 per question) |

Other checked (not skipped in 803, listed because the task named them): Rocket Hideout stairs side forced =
{B4F Grunt1, the Lift Key grunt} by `forced.py` (arrow/spin tiles not modelled; the model fights the 3 hardest
of 8 via `group_n`, which exceeds the audit); Warehouse: `forced.py` entrance to Gideon = {Admin1}, the model
fights Grunt1 + both admins + Gideon (more than the audit); Rock Tunnel is the 10-trainer forced list above and
is fought in full.

### Effect on the plan (WR+2m, R=800000, seeds 1 / 2, paired)

* `900-audit2` (`audit2: 1`: Silph 1 grunt, Surge 1 trainer, quiz and door texts charged) vs 803: **+10.4 h CI
  [8.5, 12.7] / +9.3 h CI [7.6, 11.1]**; 112.0 h / 105.7 h absolute (803: 101.5 / 96.3). 803's gain from skipping
  Silph (-13..-16 h) was real against 802 but 3/4 of it is genuinely available: against 802-post-evidence (four
  Silph grunts fought) 900 is -4.4 h CI [-7.3, -2.1] / -2.7 h CI [-4.7, -0.7].
* `901-audit2-rot` (rotating trainers on the path fought too): +7.8 h CI [5.7, 10.1] / +7.9 h CI [5.9, 10.5] vs 900.
* `plans/CURRENT` = 900-audit2. The text segments are Src::Guess; the 1:03 mean of the single Silph grunt segment is
  the battle model's (calibrated) cost, not a measurement of this fight.
