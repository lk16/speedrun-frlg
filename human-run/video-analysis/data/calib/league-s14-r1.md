# League round 1, main97 2026-11-14 stream (attempt A1, LG, Blastoise "A", Rash)

Per-turn data: `league-s14-r1.json`. Video times; run time = video + 4:08. Windows: Lorelei 2:33:56-2:36:12, Bruno 2:36:24-2:38:20,
Agatha 2:38:38-2:40:15, Lance 2:40:54-2:42:37, Champion 2:43:21-2:45:55. HP max by level 52..57 = 160/163/166/169/172/175 (+3 per level).
Method: foe-bar.py at 1 s plus 1 s/2 s/3 s sheets. Turn times +-2 s; foe bar +-1 px, full bar reads 47 (written 48), KO bars are lower bounds.

## Actions and damage taken (per enemy move)
- **Lorelei (11 actions):** Dewgong T1-T4 = four X Special (19 -> 15); foe Safeguard, Ice Beam 24, Hail (sets weather), Surf 24, Surf 22 (all not very
  effective); hail chip 10 x4 (T3-T6). Then Bite (bar 48 -> 22), Bite KO. Cloyster/Slowbro Bite OHKO, Jynx Surf OHKO, Lapras Bite (bar 48 -> 8, runner says
  crit, flinch) then Sitrus +6 px (8 -> 14), Bite KO. 160 -> 50 (+3 at the Lv53 level-up) = 53/163 end. Total 110, 40 of it hail.
- **Bruno (10 actions):** T1 Max Potion (not Full Restore) 53 -> 163, Rock Tomb 17 (+Speed -1). Then X Special, X Special, X Speed, Guard Spec while Onix
  Earthquakes 29/31/29/28 (163 -> 29 total 134), Surf KO. Hitmonchan, Hitmonlee, Machamp (Lv56), Onix (Lv54) one Surf each, no foe moves. End 32/166 Lv54.
- **Agatha (7 actions):** Gengar T1 X Special (Double Team by foe), T2 X Speed (Shadow Punch 31: 32 -> 1), T3 Surf KO. Golbat, Arbok, Gengar Lv58, Haunter:
  one Surf each. Level 55 (1 -> 4/169, "did not learn Skull Bash"). Only damage: 31.
- **Lance (7 actions):** Gyarados (Intimidate): X Special, X Accuracy, Blizzard crit KO; Bite 18 + Bite 18 (169 -> 133). Dragonair x2 and Dragonite: one Blizzard
  each (OHKO). Aerodactyl: Wing Attack 38 first (136 -> 98), Surf crit KO. Level 56 after Dragonite. Total 74. Runner's "double Blizzard" strat = Dragonair x2.
- **Champion (13 actions):** Pidgeot Lv59: Guard Spec (stat drop blocked), X Speed, X Special, X Special, Surf crit KO; Aerial Ace 67 (crit) / 33 / 37 (172 -> 35).
  Venusaur Lv63: X Special (Growth), Blizzard KO (Solar Beam only charging). Alakazam Lv57 and Rhydon: Surf OHKO, Alakazam never moved. Gyarados Lv59: Full Restore
  (38 -> 175), Thrash 45; Bite (bar 48 -> 26, flinch), Blizzard KO. Arcanine (Intimidate): Surf OHKO. Ends 130/175 Lv57.

## Items (counts are pre-use while the item message is on screen)
- X Special 19 -> 8: Lorelei 4, Bruno 2, Agatha 1, Lance 1, Champion 3. X Speed 3 -> 0 (Bruno T4, Agatha T2, Champion T2).
  X Accuracy 2 -> 1 (Lance Gyarados). Guard Spec 2 -> 0 (Bruno Onix T5, Champion Pidgeot T1). X Attack x3 never used.
- Heals: Max Potion in Bruno T1, Full Restore in Champion Gyarados T1 (x5 -> x4); between fights Max Potion (Agatha -> Lance, 4 -> 169) and
  Hyper Potion (Lance -> Champion, 98 -> 172). Bag snapshots in the JSON.

## Surprises / for the simulator
- Items always act before the foe's move; weather chip lands after a replacement send-out (end of turn). Hail lasted exactly 5 turns incl. the Hail turn.
- Onix EQ rolls 28-31 on 163 HP at Lv53/51 with Blastoise Lv53: tight spread. Shadow Punch 31 on a 32 HP mon: the runner called it a max roll.
- Ice Beam 24 and Surf 24/22 from Dewgong at 160 HP (not very effective), Bite -18 from Gyarados after Intimidate.
- Three crits on the player side (Gyarados Blizzard, Pidgeot Surf, Aerodactyl Surf); one on the foe side (Pidgeot Aerial Ace 67 = 172 -> 105).

## Runner commentary (srt, video times)
- 2:33:50 "I have no idea how this is gonna work, but I'm guessing with 52 there's probably a chance" / 2:33:58 "you just hit heal range on ... lap" (Lapras).
- 2:35:49 "Ohh, simply just critical. Nice. Yeah, nice. Good one. I'm already 53" (Lapras Bite crit; Lv53 was the Jynx kill).
- 2:37:07 "Oh, I don't even have a super potion." 2:37:24 "Normally when you get EQ there, you have the super potion because you're in the Mach Punch range."
- 2:37:42 "we were taking way less damage so 53 was not gonna cut it" / 2:38:21 "I was guaranteed to get 53, but 54 for Agatha ... That wasn't supposed to happen. Just the extra experience."
- 2:39:34 "I had 32. And that did 31" / 2:39:39 "54 surely I was safe there right, that just max rolled" / 2:40:06 "I'm level 54 so maybe that increases your [defense]".
- 2:41:01 "potential gold here ... unbeatable gold" / 2:41:21 "well now you double bite for it" / 2:41:47 "an upside of 55 strat ... nasty gold" / 2:42:14 "ancient powers or some shit I hope not".
- 2:43:08 "one of the upsides of double candy ... you just Blizzard times two, so it's healess, and then if you crit it ... insane."
- 2:43:52 "38 ... almost enough to act here but no, she'll just kill me here, it's actually really close." 2:44:18 "Oh, I guess I should special here, huh? ... was that bad?"
  2:44:44 "I had two more Blizzards ... it's a free turn. No, I didn't lose a turn, maybe that was bad."
- 2:45:10 "It's a bummer I can't risk this here. It's not quite enough HP" / 2:45:15 "I hate how there's like hyper potions max potions fucking full restores"
  / 2:46:00 "40 41 to 49. I was at 38. He would not have survived ... I was at 38 on Gyarados." (the 2:43:52 "38" is the Alakazam kill HP; Gyarados Thrash 45 -> the Full Restore).

## Not readable
- Level-up stat boxes (only a one-frame blue flash; stats_seen null). Foe names of Pidgeot's blocked move. Exact message of Lorelei T1-T2 / Bruno T2-T4 / Agatha T1-T2 /
  Lance T1-T2 item turns (identified from counts). Foe max HP not measured. Arbok Intimidate not seen.
