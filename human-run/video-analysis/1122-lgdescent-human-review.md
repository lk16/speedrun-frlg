# Human review: `1122-lgdescent-s11-v4-full.mp4`

> **Editor's note:** This file was reformatted and its typos and wording were fixed. The content
> was not materially changed. The one exception is the reasoning for item 9, which said "antidote"
> where it clearly meant "potion".

This file lists problems a human found while reviewing `1122-lgdescent-s11-v4-full.mp4`.

Timestamps match the on-screen **RUN TIMER**. They were entered by hand, so they may be off by a
few seconds.

## Fields

- **Saw:** what the reviewer observed.
- **Expected:** what the reviewer expected to see instead.
- **Reasoning:** why the expected behaviour is better. Notes on how to fix it can also go here.
- **Certainty:** how sure the reviewer is that the change improves the strategy.
- **Claude verdict:** added 2026-11-23 after per-item subagent checks against `decompiled/`, the
  plans and the v4 run's log/savestates: CONFIRMED, REJECTED or UNSURE, with the evidence. Note that
  everything before Giovanni 1 in this run was played by the lost 2026-11-21 controller, so early
  controller fixes go into new early stages of the current controller.

The reviewer lists each issue once, usually at its first occurrence (not guaranteed). The same
issue may also happen at other timestamps.

---

## 1. No encounter RNG seed (before the timer starts)

- **Saw:** A is mashed through the title screen to start the game.
- **Expected:** press A at the right moment during the intro to pick an encounter RNG seed that
  gives as few early encounters as possible, as in the Main97 runs.
- **Reasoning:** keeping encounters to a minimum in the splits up to Brock saves over a minute
  compared with a typical encounter seed.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (about 60-90 s to Brock). The wild-encounter RNG is separate from `gRngValue` (`decompiled/src/wild_encounter.c:661-671`), seeded by `SeedWildEncounterRng(Random())` (`new_game.c:103`). The intro seeds it (`intro.c:1004`), but the title exit reseeds from timer 1 (`title_screen.c:735-737`). That timer starts when the title screen loads (`title_screen.c:351`), so the title-press frame decides the seed. The starter seed is separate: the player naming screen restarts the timer and reseeds on exit (`naming_screen.c:428,722`). v4 mashed A at boot, yet plan `1122-lgdescent` scores the run as having a quiet seed (`quiet_max_hits 6`, `manip.rs:19-21,251-265`). Cost in v4: 4 wild battles near Viridian (about 105 s), a Route 1 Pidgey catch (about 33 s) and 4 Caterpie flees in the Forest (about 40 s). Fix: sweep the LG title delay (`crates/frlg-route/examples/title-seed.rs`, `segments.rs:324-327` `seed_delay`), reset when the seed is not quiet, then pick the starter-naming frame. The Pokédex-check step is not traced in the decomp yet.

## 2. Text speed left at the default (entire run)

- **Saw:** text speed is set to Mid (visible on screen at 00:50).
- **Expected:** text speed is set to Fast. Changing it must not break the RNG seed setup.
- **Reasoning:** a lot of the run is spent waiting for text to print, and faster text cuts that
  time. If possible, set it before Oak's intro speech.
- **Certainty:** 100%
- **Claude verdict:** REJECTED as an observation; needs a recheck against the video. Savestates from the run read `optionsTextSpeed` Mid at `0000498-title` and `0002389-naming`, and Fast from `0006852-ball` onward (later states too). The controller sets Fast in the bedroom right after Oak's speech (`crates/frlg-route/src/segments.rs:467-525`, 197 frames, `route.rs:1359`). Setting it earlier is impossible on a fresh cart: the title menu is only New Game / Continue (`decompiled/src/main_menu.c:23-24`). The Mid seen at 00:50 is probably just before that step; check the timestamp. Fast matters mostly for field text: Mid is 4 frames per character and Fast 1 (`new_menu_helpers.c:27-31`). Held A/B speeds up battle text and Oak's speech (`text.c:639-646`, `battle_message.c:2784`) but not field messages (`field_message_box.c:17`).

## 3. Rival name (around 00:30)

- **Saw:** the preset name RED was selected.
- **Expected:** a one-character name.
- **Reasoning:** less text to print.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED, small (about +1.5 to +4 s). The first pass wrongly priced typing at 2-4 s. With "New name" the naming screen opens empty (`naming_screen.c:494,2114`) with the cursor on "A". The presses are A, START (jumps to OK, `naming_screen.c:1485`), A. The fades add about 85-100 frames, roughly 1.4 s more than the preset (DOWN, A; `oak_speech.c:1413-1440,2142`). The rival's name prints about 85 times: about 30 `{RIVAL}` lines on the route plus battle strings (`battle_message.c:2078-2081`). Each print is 2 characters shorter with a one-letter name. Fix: `crates/frlg-route/src/segments.rs:421-431`. Then rebuild the starter-manip naming table, because the extra frames shift it.

## 4. Poor pathfinding: bumping into walls

- **Saw:** over 5 seconds lost bumping into a wall (00:58–01:02).
- **Expected:** no time lost like this.
- **Reasoning:** this wastes time for no reason, and it happens in many other parts of the run.
  Those cases are not listed here, but all of them must be fixed. In general, the run should never
  lose time bumping into walls or NPCs. A "bumped something at ... re-planning" message points to a
  bug in the pathfinder, a missed NPC, or something similar; no human would lose time like this.
  Every attempted step should either turn the player in place or move them to a different tile.
  If neither happens, the problem is still there.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED. The run log has 85 bump events (50 "bumped into", 35 "bumped something"), spread from t=62 s to t=19106 s. Repeated bumps on the same tiles (e.g. at 122 s and 127 s) point to NPCs or objects missing from the collision grid. The walker replans on every bump (`crates/frlg-human/src/play/walk.rs:1024-1026`, grid from `live_map` at `:879`). The simulator charges nothing for bumps. The 00:58-01:02 loss is in the boot-to-Giovanni-1 prefix, which was played by the lost 2026-11-21 controller (see item 5). It is mostly the roughly 8 s of DOWN held at the house's ground-floor door without warping (frames 4120-4606). The current walker's warp rule should avoid that.

## 5. Poor pathfinding: extra steps without bumping

- **Saw:** after leaving the house, the player walks right, down, right, up, as if going around
  something.
- **Expected:** no extra steps. There are probably more cases like this; they are not listed here,
  but they need to be fixed too.
- **Reasoning:** extra steps lose time.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (the first pass was wrong). Oak's object at (10,8) has `FLAG_HIDE_OAK_IN_PALLET_TOWN` (`decompiled/data/maps/PalletTown/map.json`, `include/constants/flags.h:60`), set at new game (`new_game.c:149`). RAM during frames 4100-4986 shows only two wandering NPCs on the map. Emulated from frame 4753, holding right then up passes (10,8). The direct path reaches the trigger (12,1) 32 frames earlier than the run. The prefix up to Giovanni 1 was played by the uncommitted, now-lost 2026-11-21 controller (`human-run/journal/2026-11-22-10-00-controller-restart.md`). It most likely blocked static `map.json` objects without checking their flags. The current walker does check flags (`walk.rs:409-411,847-858`). `bin/frlg-mapgraph` `blocked_tiles` still ignores them. Fix: give the current controller early stages (`legs::stages()` starts at `giovanni1`) and make `blocked_tiles` respect flags. A detour scan of frames 0-147k found only +1 to +3 step cases. Those are unverified and may be ledges or trainer avoidance.

## 6. Special Attack selection

- **Saw:** a Sp. Atk of 29.
- **Expected:** a Sp. Atk of 30 or more.
- **Reasoning:** the water starter uses special attacks for most of the run.
- **Certainty:** 70%
- **Claude verdict:** UNSURE, needs a simulator test. The first pass misread 29 as a level-5 stat. It is the Sp. Atk IV: the run accepted a Rash Squirtle with IVs hp27 atk31 def15 spe29 spa29 spd21 after 22 resets. The plan's gate is `min_spa: 29` (`plans/1507-honest4.md:765`, `1122-lgdescent.md:763`; IV minimum in `manip.rs:41,160-168`, default 30 at `:91`). Main97 accepted Sp. Atk IV >= 30 in 63 of 64 cases (`journal/2026-11-14-14-00-early-merge-ablation.md:36-37`). An earlier joint grid put the plateau at 28-29, and 31 was worse. 30 alone was never tested. Settle it with a paired compare of `min_spa` 30 vs 29 on 1507-honest4.

## 7. Weaken wild encounters when possible (07:28)

- **Saw:** wild Pokémon are not weakened before throwing balls.
- **Expected:** weaken them first.
- **Reasoning:** this raises the catch rate and avoids losing time and balls to breakouts.
- **Certainty:** 70%
- **Claude verdict:** CONFIRMED (controller bug). The plan sets `weaken: true` and the rule exists (`crates/frlg-human/src/play/battle.rs:1450`). Yet the first weaken in the log is at t=8169 s. At 07:28 (t=447 s) two Poké Balls went at a full-HP Pidgey Lv3 with a logged catch chance of 0.33. Catch odds scale with (3*maxHP-2*hp)/(3*maxHP) (`decompiled/src/battle_script_commands.c`, `Cmd_handleballthrow`). For catch rate 255 that is about 33% at full HP and about 67% at half HP, so weakening pays here. The prefix controller that played this part is lost (item 5), so the fix belongs in the new early stages.

## 8. Antidote not picked up (10:08)

- **Reasoning:** the antidote is needed later.
- **Certainty:** 100%
- **Claude verdict:** REJECTED (about -5 s net). The hidden Antidote is at (28,57) (`decompiled/data/maps/ViridianForest/map.json:279-283`), opposite to the walked path (east along row 61 to x=38). The pickup costs about 8 extra steps plus the found-item text, roughly 4 s, plus grass-step encounter risk. Antidotes are bought for Koga at a Lavender mart stop the plan makes anyway (`plans/1507-honest4.md:3`, `play/legs.rs:401-406`). One pickup only lowers the quantity from 2 to 1 and removes no menu. Note that the reviewer also wants Sammy's poison kept for Torrent (item 11). Main97 does pick it up (`STRATEGY.md:355`).

## 9. Potion next to the Sammy fight not picked up (~10:00)

- **Reasoning:** the potion is needed later.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED, together with items 10 and 12. The hidden Potion is at (3,22) (`decompiled/data/maps/ViridianForest/map.json:272`), 4 tiles west of Sammy (7,22). Picking it up after the fight costs about 4 s. The bigger miss is the free Route 1 mart-clerk Potion (`decompiled/data/maps/Route1/scripts.inc:4-22`), which Main97 takes every run (`STRATEGY.md:356`). v4 also skipped the bedroom PC Potion. It had no healing item until the Pewter mart. On the reviewer's note that Potions are needed later to heal while staying in Torrent range: early on, a Potion (20 HP) overshoots the whole Torrent window (about 8 HP at 25 max HP), so they serve as faint insurance early and as range-keeping heals later.

## 10. Starter allowed to faint (11:50)

- **Reasoning:** the starter needs to take the EXP, and a fainted starter forces a Pokémon Center
  visit later.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (about 100 s together with items 9 and 12). The faint was to Bug Catcher Sammy's Weedle Lv9 (`decompiled/src/data/trainer_parties.h:304-310`), not a wild one. The lead left Oak's lab at 8/25 HP and met Sammy at 5/25 (states `0026126-deliver`, `0033524-tutorial`). Trainer battles cannot be fled. The heal path needs a bag item (`play/battle.rs:3138-3145`), and there was none. The Squirtle fainted and gained no EXP (still 290). The fight took 8809 frames (148 s) against the model's 2706. Plan `forest-trainers: potions 3` (`1122-lgdescent.md:628`) assumed Potions the controller never had. Fix: collect the Route 1 clerk Potion and the Forest Potion before Sammy.

## 11. Water starter not brought into Brock's gym at low HP for Torrent

- **Reasoning:** Torrent makes the fights against Brock's Pokémon faster. The usual setup is to
  leave the Sammy fight with the starter poisoned and let the poison whittle it down to about 5 HP.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED as a strategy; size unmeasured (our notes say 30-60 s on the first split). Torrent is x1.5 Water power at HP <= max/3 (`decompiled/src/pokemon.c:2500`). Field poison costs 1 HP per 5 steps (`field_control_avatar.c:717-719`). In this decomp it can faint (`field_poison.c:104-106`), so the setup is delicate, as the reviewer says. Our notes on Main97 (`human-run/video-analysis/STRATEGY.md:359-370`) describe this exact line: Sammy poison, HP thresholds 5 -> 7 -> 9, Withdraw turn 1 vs Onix (Rock Tomb 8-10 -> 5-7), never heal before or in the gym, Liam with Tackle to keep range. That gives about 75% passes on a good day. Damage calc at v4's Squirtle (L10, SpA 18): Torrent Bubble one-hits Geodude (34-40 vs 31 HP), and Liam drops from 5 turns to about 3. A subagent's "5% win at 1/3 HP" only simulated a naive entry (flat 1/3 HP, no Withdraw, no thresholds), so it does not count against the line. Correction from the reviewer: Main97 usually reaches L13 and learns Water Gun at the end of the Brock fight, so `STRATEGY.md:259` ("Water Gun learned Lv13 pre-Brock") is wrong. Gap: the model cannot express an early-only Torrent window (`STRATEGY.md:682-686`; the global Torrent policy was refuted at +8.7 h), and the controller never steers HP into it. Both need implementing.

## 12. Avoid the Pokémon Center visit in Pewter

- **Reasoning:** the visit costs time. PP should be sufficient, and low HP is fine because of
  Torrent.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED. The plan has no fixed Pewter Center visit (`crates/frlg-human/src/route.rs:428-483`). This one was the forced "Center stop" (frames 51633-53292, about 21 s over walking straight on) caused by the item 10 faint. Fixing items 9 and 10 removes it. The skip also fits item 11's setup: Main97 never heals before the gym (`STRATEGY.md:366-367`). PP is no issue: Bubble has 30 PP (`battle_moves.h:1894`).

## 13. Pewter Mart visit

- **Saw:** 6 Potions and 3 Repels bought.
- **Expected:** buy some Potions and some Repels, but also Poké Balls. The TM from Brock was also
  not sold for extra money.
- **Certainty:** 100%
- **Claude verdict:** UNSURE, leaning CONFIRMED for the TM sale. TM39 is never taught or used: the only `teach_tm` is TM14 (`play/legs2.rs:435`). It sells for 1500 (price 3000 in `decompiled/src/data/items.json`, half at `decompiled/src/item_menu.c:1933`). Pewter is visited anyway after Brock, and `play/shop.rs` already has a sell routine (used at `legs2.rs:1880`). The simulator does not model selling it, so it cannot show the gain. On balls: the plan buys none early (`early_balls: 0`, `1507-honest4.md:1164`), and the Pewter candidate kind was never run. v4 used Oak's 5 balls (`PalletTown_ProfessorOaksLab/scripts.inc:660`) on Pidgey x2, Weedle, Paras and Zubat. It ran out mid-catch on a Zubat it still needs later, wasting about 16 s. A Pewter ball buy is plausible; test it with a descend over +1500 money and a 5-ball Pewter purchase.

## 14. Repel on the route east of Pewter (18:53)

- **Saw:** a Repel is used long before any encounter tiles.
- **Expected:** use it only when entering Mt. Moon. There may be a few grass tiles before that, but
  a Repel there is probably not worth it.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (about 5 s plus fewer Mt. Moon encounters). I replayed v4 tile by tile with `frlg_route::world` `Tile.land`: 101 Route 3 positions and 26 Route 4 positions before Mt. Moon, 0 of them grass. Route 3's grass patches (x about 33-46 and 70-78, y 10-16) are off the path. The first Repel was used at frame 68410, on the first Route 3 tile, and expired at the Route 4 border. Every step was wasted, and Mt. Moon then started with less cover. The simulator already charges nothing there (`sim.rs:3556` only calls `use_repels` when the segment has encounters). Fix: drop `route3` from `repel_on` (`plans/1507-honest4.md:356`), and only use a Repel on a path that crosses grass.

## 15. Trainer fights on the route east of Pewter

- **Saw:** no Torrent.
- **Expected:** Torrent active and Water Gun available here.
- **Reasoning:** Torrent-boosted Water Gun one-hit KOs most Pokémon, which shortens the fights.
- **Certainty:** 100%
- **Claude verdict:** UNSURE (about 0 to +12 s, or about 25 s if Ben is skipped). Water Gun arrives at L13 (`level_up_learnsets.h:106`), mid-Janice fight. Torrent was already on for Greg (all kills were sure kills) and for James's Caterpie. The heal at 10/37 came from `torrent_heal_below 0.5` (`1122-lgdescent.md:787`, `sim.rs:1873-1874`, `battle.rs:3141-3147`), which works against staying in Torrent. Damage calc: Torrent Water Gun one-hits Colton and Greg's bugs (100%) and Pidgey (69-94%), but not Rattata, Ekans or Jigglypuff. Ben's Rattata outspeeds us and makes low HP risky, which is the main cost. Skipping Ben (item 16) and Robin (item 17) changes the balance. Test `torrent_heal_below` 0 and per-fight heal thresholds.

## 16. Youngster Ben fight

- **Saw:** the fight was done.
- **Expected:** skip it.
- **Reasoning:** he can be walked around on his north side. The EXP from this trainer is not
  needed and the fight is very slow.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED that the fight was unintended. I have not checked that the north-side walk-around is free. The plan lists `route3-trainers` in `skip_groups` (`plans/1507-honest4.md:314`), and Ben (Rattata and Ekans L11, `decompiled/src/data/trainer_parties.h:114`; `FACE_DOWN`, sight 2, at (17,4) in `Route3/map.json`) is not in the forced set. Yet v4 fought him (t about 1304-1347 s). The same walker bug as item 17, ignoring on-screen trainers' sight lines (`walk.rs:419-421`), probably caused this and the Janice fight. Not confirmed per trainer.

## 17. Lass Robin fight

- **Saw:** the fight was done.
- **Expected:** skip it.
- **Reasoning:** she can be passed without a fight by timing the walk past her.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (about 38 s). Robin (`MOVEMENT_TYPE_WANDER_UP_AND_DOWN`, (40,11), range_y 1, sight 3) sees only in her facing direction (`decompiled/src/trainer_see.c:130-166`). Her wander loop waits a random 32/64/96/128 frames per facing (`event_object_movement.c:679,3090,3109-3115`), so each facing lasts at least 32 frames. The player is exposed for one 8-frame step. Every path east crosses column 40 within rows 9-16. The cheapest crossing is (40,9), which is safe whenever she faces south. In v4 the player stepped onto (40,9) at frame 89110 while she faced north from (40,11), and the Jigglypuff fight cost 2421 frames (about 40 s). Cause: `crates/frlg-human/src/play/walk.rs:419-421` skips the sight lines of trainers on screen, and `:427-430` draws wanderers' lines from their spawn tile. Fix: draw sight lines from current position and facing (read from RAM), wait at (39,9), and step when she faces away. The expected wait is about 1.5 s.

## 18. Pokémon Center heal at the Mt. Moon entrance

- **Saw:** the party was healed.
- **Expected:** no heal.
- **Reasoning:** not necessary.
- **Certainty:** 100%
- **Claude verdict:** UNSURE (a subagent said REJECTED; I do not trust it). At the `mt-moon-pc` state the carry is Wartortle L16 at 15/49 HP with 5 Potions and 2 Repels. The heal costs 689 frames (about 11.5 s) with the player already at the nurse. The subagent said the heal is needed because HP later fell to 8-17 in Mt. Moon wild fights (Clefairy, Rattata, Zubat). Those fights happened partly because a Repel was wasted on Route 3 (item 14). At L16 a Repel blocks every Mt. Moon wild below that level (`decompiled/src/wild_encounter.c:286`). 15/49 is also already in Torrent range, which item 11's strategy wants. Recheck after fixing item 14 and modelling the early Torrent window. The heal is not a plan knob: the simulator charges nothing for it.

## 19. Magikarp purchase at the Pokémon Center near the Mt. Moon entrance

- **Saw:** Magikarp was not bought.
- **Expected:** buy it.
- **Reasoning:** this is probably the fastest species to pick up in the game, and well worth the
  ₽500.
- **Certainty:** 100%
- **Claude verdict:** CONFIRMED (saves a late trip). In v4 the Magikarp was bought at t=16964 s, by the dex-finish stage ("dex finish plan ... MAGIKARP @Route4", `crates/frlg-human/src/play/dexfin.rs:335-339`). That was a dedicated return trip to Route 4 late in the run. The first pass already stands in this Center (item 18). The salesman charges 500 (`decompiled/data/maps/Route4_PokemonCenter_1F/scripts.inc:1,23-53`). Buying then costs a short dialogue and money at a tight point, and it takes a party or box slot. A subagent said dex-finish visits Route 4 anyway for the PC. That looks circular, since the trip was planned around the Magikarp. The model does not include the purchase (`dexfin.rs:5`, `calibration.md:360`), so the money trade-off is untested.

## 20. Persim Berry near the Mt. Moon entrance

- **Saw:** the berry was not picked up.
- **Expected:** depends.
- **Reasoning:** Misty's Pokémon may confuse the starter with Water Pulse. How likely the AI is to
  use that move depends on the starter's Defense and Sp. Def.
- **Certainty:** 70%
- **Claude verdict:** UNSURE, roughly break-even (within about ±3 s). The first pass's reasoning had two errors. v4 never got a Persim: there is no berry event, and the controller has no step for it. `route_wr.rs:25-32` is the WR-runner reference model, and `persim_held` is absent from 1122 and true only in 1507 (`plan:1170`; `sim.rs:4298-4300`, `battle.rs:501-506`). The reviewer is right that Def and Sp. Def matter. Misty has `TRY_TO_FAINT | CHECK_VIABILITY` (`decompiled/src/data/trainers.h:4152`). When no move can KO, the AI demotes all but the highest-damage move (`battle_ai_script_commands.c:966-1020`), and that depends on our defences. The 20% confusion roll itself is flat (`battle_moves.h:4586`). Staryu and Starmie both know Water Pulse (`trainer_parties.h:5619-5632`). Our rough estimate: Water Pulse is favoured by Staryu, and Swift is about equal for Starmie. Over v4's 8-turn fight that is about 4 Water Pulses and a 55% chance of at least one confusion, about 0.6 lost turns (4-6 s). The pickup at Route 4 (5,4) (`Route4/map.json:174-179`), the found-item text and the give menu cost about 8-11 s (guessed).

---

The reviewer stopped watching when the run enters Mt. Moon at 25:51, but more problems likely
occur after that point. This file will be updated later, continuing the numbering.
