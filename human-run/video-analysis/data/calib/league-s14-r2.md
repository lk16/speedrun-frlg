# Round 2 League, Mewtwo "A" (video 2026-11-14, LeafGreen A1), per-turn

Source: main97-youtube-stream-2026-11-14.mp4, video 3:09:03 - 3:21:08 (run = video + 4:08). Data: league-s14-r2.json.
Foe bar from tools/foe-bar.py at 1 s plus 1-2 s contact sheets. A full bar reads 47/48 (not 48). Times are +-1 s.
Mewtwo moves: Psychic / Fire Blast (PP 5) / Safeguard / Shock Wave (PP 20); Earthquake TM at 3:17:12-20 replaces Safeguard (inferred).
Level / max HP: Lv70 248 -> Lv71 251 (3:11:59) -> Lv72 254 (3:15:58) -> Lv73 258 (3:19:04). Stat boxes are skipped by the runner: no Atk/Def/Spe numbers.

## Per fight
- Lorelei (3:09:03-3:10:40), 7 actions, 5 foes, 0 damage taken. Dewgong Lv64 spams Double Team (x2), so X Special x2 (bag 8 -> 6)
  while its evasion rose, then Shock Wave KO. Cloyster SW KO, Piloswine Psychic KO, Jynx Fire Blast KO, Lapras Psychic KO. Every attack 47 -> 0.
- Bruno (3:10:54-3:12:32), 6 actions, 24 damage. Steelix65: Fire Blast MISSED, Rock Tomb 24 (248 -> 224, Speed fell), 2nd FB KO.
  Steelix66 FB KO, Hitmonlee/Hitmonchan/Machamp one Psychic each (all 47 -> 0). Level-up 71 after Hitmonlee.
- Agatha (3:12:48-3:14:24), 6 actions, 87 damage. Gengar66, Crobat, Gengar70: one Psychic each. Misdreavus65: Shock Wave 47 -> 27 only
  (used to save Psychic PP: it was 2/10), Shadow Ball 87 (227 -> 140, SE, SpD fell), then Psychic 27 -> 0. Arbok68 Intimidate, Psychic KO.
  After: Full Restore (5 -> 4) 140 -> 251 and Max Elixir (PP restored) in the menu.
- Lance (3:15:02-3:17:00), 9 actions, 0 damage. Gyarados68: Intimidate; Safeguard then X Special x3 while it Thunder Waved 4x, all blocked
  by Safeguard (X Special 6 -> 3, inferred from counts 6/5/4 and 3 at Champion). Psychic KO. Then Kingdra, Dragonite66, Aerodactyl70, Dragonite72: one Psychic each.
- Champion (3:17:36-3:21:08), 11 actions, 524 damage, ends 171/258.
  Heracross72 Psychic KO. Alakazam73 (5 actions): X Attack, X Attack, Full Restore, X Attack, Earthquake. Damage: Shadow Ball CRIT SE 118
  (254 -> 136), Psychic nve 73 (136 -> 63), [Full Restore +191 -> 254], Psychic 104 (-> 150), Psychic 94 (-> 56). Earthquake CRIT then KO (47 -> 0).
  Tyranitar72 (Sand Stream): EQ KO. Gyarados73: Shock Wave KO at 28 HP (runner's "46-47% to die"). Arcanine73: Full Restore 12 -> 258,
  ExtremeSpeed 55 (258 -> 203), EQ KO (foe 41 -> 0). Venusaur75 Psychic KO.
  Sandstorm: 5 ticks of 16 (1/16 of 258): 60->44, 44->28, 28->12, 203->187, 187->171. Each tick lands AFTER a replacement is sent out
  (and after a surviving foe's action); foes' own ticks cost them 47 -> 44 / 44 -> 41.

## Items
X Special 8 -> 3 (Lorelei 2, Lance 3). X Attack 3 -> 0 (Champion Alakazam, 3 uses). Full Restore 5 -> 2 (after Agatha, Alakazam T3, Arcanine).
Max Elixir 1 -> 0 (after Agatha). Bag counts: message frames show the count BEFORE decrement. Snapshots are in the JSON.

## Commentary (srt, video time)
- 3:13:57 "Max elixir will restore super to special" (transcript garble)
- 3:14:59 "Dragonite +2 Psychic 62%" (he reads a damage chart before Lance)
- 3:17:15 "I meant to do that on a turn frame, but there we go."
- 3:18:03 "oh my god oh fucking hell that's not good ... you're serious" (the crit Shadow Ball, ~3:18:13 on screen)
- 3:18:28 "We're just dead! ... Bro, I'm literally dead"; 3:18:37 "Can I just go at like +2? 100? Okay, that's not..."
- 3:19:14-3:19:21 "what's my only hope ... use earthquake right I mean I can kill with shockwave"
- 3:19:42 "Think I got to go for this. It's 46 47 percent, but I'm just gonna die so I gotta go for it" (Gyarados73 vs Mewtwo at 28)
- 3:19:47 "I'm extreme speed now"; 3:19:56 "Have another healing item, right" (Full Restore #2); 3:20:50 "double spec drop, bro"
- 3:21:48 "It was 47% chance to die at a shockwave, but I think ... I'm glad I did that"

## Could not read / uncertain
- X Special / X Attack "rose" messages mostly hidden by black screen transitions: uses inferred from bag counts; T1 X Attack at Alakazam unseen.
- Exp for Lapras, Hitmonlee, Dragonite66, Aerodactyl, Dragonite72, Alakazam, Arcanine not read; no stat screens; Earthquake slot inferred.
- The 3 Thunder Waves vs. 3 X Specials in Lance: counted 4 TW messages (incl. the Safeguard turn) and 3 item turns; +-1 turn.
- Foe bar +-1 px; before/after taken at plateaus. Shock Wave on Misdreavus (47 -> 27) is the only non-KO attack besides the Fire Blast miss.
