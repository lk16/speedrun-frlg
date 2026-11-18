# WR video, Round 2 League, per-turn (league-wr-r2.json)

Source: frlgr2-wr-3.25.25-main97.mp4, video 3:12:00-3:24:30 (run = video - 0:35). 3 s sheets over the whole
window, 1-2 s zooms on ambiguous turns, foe bars via foe-bar.py at 1 s. Times +-3 s; bar +-1 px (full reads 47/48).
Player: one Mewtwo "A": Lv70 231 -> 71 235 (Bruno) -> 72 238 (end Lance) -> 73 241 (Champion, after Alakazam).

## Fights (video start..end, player actions, mons)
| Fight | video | actions | foe mons | damage taken |
|---|---|---|---|---|
| Lorelei | 3:12:18-3:14:01 | 8 | Dewgong64 (3), Cloyster63 (1), Piloswine63 (1), Jynx66 (1), Lapras66 (2) | 218 |
| Bruno | 3:14:34-3:16:08 | 5 | Steelix65, Steelix66, Hitmonlee65, Hitmonchan65, Machamp68 (1 each) | 0 |
| Agatha | 3:16:19-3:17:56 | 6 | Gengar66, Crobat66, Gengar70 (1 each), Misdreavus65 (2), Arbok68 (1) | 172 |
| Lance | 3:18:37-3:20:29 | 7 | Gyarados68 (3), Kingdra66, Dragonite66, Aerodactyl70, Dragonite72 (1 each) | 0 |
| Champion | 3:20:55-3:24:11 | 11 | Heracross72 (1), Alakazam73 (4), Tyranitar72 (1), Gyarados73 (2), Arcanine73 (2), Venusaur75 (1) | 204 + 6 sandstorm ticks |

Every foe mon without a listed attack died before moving, except where noted. Almost every KO is ONE Mewtwo hit
taking the whole bar (47 -> 0, so only a lower bound on damage); Psychic and Fire Blast / Earthquake are all one-shots
at these levels. The only multi-hit cases: Dewgong (Psychic 47->15, then a Psychic MISS from Double Team evasion, then
15->0) and Lapras Lv66 (Psychic 47->20, then 20->0). Foe bar after every other hit is 0.

## Damage taken per enemy move (HP numbers read from the player box)
- Dewgong Surf 63 (231 -> 168). Lapras Ice Beam CRIT 155 (168 -> 13). Misdreavus Shadow Ball CRIT 172 (217 -> 45),
  "A critical hit!" + super effective text seen. Alakazam Shadow Ball 62 (238 -> 176, SE, no crit text seen).
- Champion Gyarados Hydro Pump 79 (149 -> 70), NOT 94/109: v4's 164 -> 109 -> 55 included two sandstorm ticks.
- Arcanine Extremespeed 63 (241 -> 178), NOT ~48/24: v4 mixed it with the heal.
- Sandstorm tick = 15 (1/16 of 241), applied to Mewtwo AND to non-Rock foes (foe bar -3 px) at every end of turn
  and also right after a replacement is sent out (Tyranitar entry 179->164, Gyarados entry 164->149, then 70->55,
  55->40 on Arcanine entry, 178->163, 163->148 on Venusaur entry).
- Lance Gyarados: Dragon Dance (Atk+Spd) then Thunder Wave twice, both blocked by Safeguard. It acted BEFORE Mewtwo
  on turns 2 and 3 (Speed +1). No HP lost in Lance or Bruno.

## Heals and items (name, HP before/after)
- Hyper Potion 13 -> 213 (+200) between Lorelei and Bruno (identity inferred: +200, list row vanishes). Then Max Elixir.
- Full Restore 45 -> 235 between Agatha and Lance (party ticks 58/88/207/235; count 3 -> 2). Then Elixir (count 1 -> 0).
- Champion Arcanine, turn 1 = Full Restore 40 -> 241 in battle (count 2 -> 1, inferred); then Extremespeed -63.
- X Special used 3x: Agatha Misdreavus (T1), Lance Gyarados (T2), Champion Alakazam (T1). 6 -> 3 left.
- X Attack used 2x, both vs Champion Alakazam (T2,T3): 3 -> 1 left. Alakazam turns: X Special, X Attack, X Attack,
  Earthquake crit KO (foe did Shadow Ball, Calm Mind, Calm Mind in between). Tyranitar then fell to one EQ (Atk +2).
- Safeguard used once (Lance Gyarados T1; PP 25/25 shown). Guard Spec x1 never used.
- PP seen: Psychic 9/10 and 8/10 (Dewgong), Fire Blast 3/5 (after 2 uses), Safeguard 25/25, Earthquake 8/10.
- Bag snapshots (counts per time) are in the JSON bag_snapshots.

## Champion turn by turn (T = video seconds -> h:mm:ss in JSON)
1 Heracross Psychic KO. 2-5 Alakazam: X Special / SB 62 / X Attack / Calm Mind / X Attack / Calm Mind / EQ crit KO.
6 Tyranitar EQ KO (sandstorm starts; Mewtwo 179->164). 7 Gyarados Psychic 44->19, Hydro Pump 79, 8 Psychic KO.
9 Arcanine: Full Restore, Extremespeed 63. 10 Arcanine EQ KO. 11 Venusaur Psychic KO. Ends 148/241.

## Not readable / uncertain
- Level-up stat boxes never appeared in any frame (checked 1 s zooms at Lv71/72/73): no Atk/Def/SpA/SpD/Spe totals.
- Mewtwo crit/miss text on non-listed turns was inferred from bar+HP changes where the text frame was between samples.
- The Hyper Potion name and the post-Arcanine Full Restore count are inferences. Exact turn order inside a turn
  (+-1 s) comes from sheet frames, not from the log. Elixir/Max Elixir target-move PP not shown.
- Heal items used: no Potion/Super Potion (x2/x1) were touched.
