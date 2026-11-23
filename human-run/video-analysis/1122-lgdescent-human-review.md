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

## 2. Text speed left at the default (entire run)

- **Saw:** text speed is set to Mid (visible on screen at 00:50).
- **Expected:** text speed is set to Fast. Changing it must not break the RNG seed setup.
- **Reasoning:** a lot of the run is spent waiting for text to print, and faster text cuts that
  time. If possible, set it before Oak's intro speech.
- **Certainty:** 100%

## 3. Rival name (around 00:30)

- **Saw:** the preset name RED was selected.
- **Expected:** a one-character name.
- **Reasoning:** less text to print.
- **Certainty:** 100%

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

## 5. Poor pathfinding: extra steps without bumping

- **Saw:** after leaving the house, the player walks right, down, right, up, as if going around
  something.
- **Expected:** no extra steps. There are probably more cases like this; they are not listed here,
  but they need to be fixed too.
- **Reasoning:** extra steps lose time.
- **Certainty:** 100%

## 6. Special Attack selection

- **Saw:** a Sp. Atk of 29.
- **Expected:** a Sp. Atk of 30 or more.
- **Reasoning:** the water starter uses special attacks for most of the run.
- **Certainty:** 70%

## 7. Weaken wild encounters when possible (07:28)

- **Saw:** wild Pokémon are not weakened before throwing balls.
- **Expected:** weaken them first.
- **Reasoning:** this raises the catch rate and avoids losing time and balls to breakouts.
- **Certainty:** 70%

## 8. Antidote not picked up (10:08)

- **Reasoning:** the antidote is needed later.
- **Certainty:** 100%

## 9. Potion next to the Sammy fight not picked up (~10:00)

- **Reasoning:** the potion is needed later.
- **Certainty:** 100%

## 10. Starter allowed to faint (11:50)

- **Reasoning:** the starter needs to take the EXP, and a fainted starter forces a Pokémon Center
  visit later.
- **Certainty:** 100%

## 11. Water starter not brought into Brock's gym at low HP for Torrent

- **Reasoning:** Torrent makes the fights against Brock's Pokémon faster. The usual setup is to
  leave the Sammy fight with the starter poisoned and let the poison whittle it down to about 5 HP.
- **Certainty:** 100%

## 12. Avoid the Pokémon Center visit in Pewter

- **Reasoning:** the visit costs time. PP should be sufficient, and low HP is fine because of
  Torrent.
- **Certainty:** 100%

## 13. Pewter Mart visit

- **Saw:** 6 Potions and 3 Repels bought.
- **Expected:** buy some Potions and some Repels, but also Poké Balls. The TM from Brock was also
  not sold for extra money.
- **Certainty:** 100%

## 14. Repel on the route east of Pewter (18:53)

- **Saw:** a Repel is used long before any encounter tiles.
- **Expected:** use it only when entering Mt. Moon. There may be a few grass tiles before that, but
  a Repel there is probably not worth it.
- **Certainty:** 100%

## 15. Trainer fights on the route east of Pewter

- **Saw:** no Torrent.
- **Expected:** Torrent active and Water Gun available here.
- **Reasoning:** Torrent-boosted Water Gun one-hit KOs most Pokémon, which shortens the fights.
- **Certainty:** 100%

## 16. Youngster Ben fight

- **Saw:** the fight was done.
- **Expected:** skip it.
- **Reasoning:** he can be walked around on his north side. The EXP from this trainer is not
  needed and the fight is very slow.
- **Certainty:** 100%

## 17. Lass Robin fight

- **Saw:** the fight was done.
- **Expected:** skip it.
- **Reasoning:** she can be passed without a fight by timing the walk past her.
- **Certainty:** 100%

## 18. Pokémon Center heal at the Mt. Moon entrance

- **Saw:** the party was healed.
- **Expected:** no heal.
- **Reasoning:** not necessary.
- **Certainty:** 100%

## 19. Magikarp purchase at the Pokémon Center near the Mt. Moon entrance

- **Saw:** Magikarp was not bought.
- **Expected:** buy it.
- **Reasoning:** this is probably the fastest species to pick up in the game, and well worth the
  ₽500.
- **Certainty:** 100%

## 20. Persim Berry near the Mt. Moon entrance

- **Saw:** the berry was not picked up.
- **Expected:** depends.
- **Reasoning:** Misty's Pokémon may confuse the starter with Water Pulse. How likely the AI is to
  use that move depends on the starter's Defense and Sp. Def.
- **Certainty:** 70%

---

The reviewer stopped watching when the run enters Mt. Moon at 25:51, but more problems likely
occur after that point. This file will be updated later, continuing the numbering.
