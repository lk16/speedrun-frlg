# WR video mid-game fight durations (5 s sampling, +/-5 s per edge; video t = window offset + cell stamp)
Sheets: /tmp/claude-1000/-home-luuk-projects-speedrun-frlg/788d68b1-7160-44ef-9445-8101618d7646/scratchpad/calib/v2/<fight>/scene-00N.jpg
Duration = first battle frame (trainer intro) to "Player defeated" box. Turns are estimates (action msgs + HP changes).

## Koga (window 5370 s). Player Blastoise "A"
| t(win) | event |
|---|---|
| 0:00-0:35 | tail of Juggler Kirk fight (Drowzee 31, Kadabra 31, Surf, Lv39->40) - NOT Koga |
| 0:58-1:00 | Koga intro; Blastoise 33/120 Lv40 (low HP, no heal used) |
| 1:05 | Koffing 37 |
| 1:20-1:30 | Muk 39 (Muk evasiveness up, Surf x2 turns), 1311 exp |
| 1:35 | Koffing 37 (Surf) |
| 1:50-2:05 | Weezing 43; Blastoise drops 33 -> 1/120 (hit), Surf/Bite; Lv41 4/123 after |
| ~2:10 | Koga defeated (win box not captured; Lv41 4/123 at 2:10) |
Enemy: Koffing37, Muk39, Koffing37, Weezing43 (4 mons). No X items/heals seen. Sacrifices none.
Duration ~72 s (0:58 -> ~2:10). Turns ~8 (Muk 2-3 incl. its evasion move, Weezing 2-3, Koffings 1 each).

## Blaine (window 6045 s). Blastoise Lv41 109/123
| 0:47 | Blaine intro (fire gym, after quiz) |
| 0:55-1:10 | Growlithe 42, Surf x1 (maybe 2) |
| 1:15 | Ponyta 40, Surf |
| 1:25-1:35 | Rapidash 42, Surf (super effective, KO) |
| 1:40-2:00 | Arcanine 47: Water Pulse, took Fire Blast 109->89->71/123 |
| 2:05-2:10 | win, Lv42 74/126, TM (Rain Dance not learned) |
Enemy: Growlithe42, Ponyta40, Rapidash42, Arcanine47 (4 mons wait: Blaine's Magmar absent in this ROM version team). No X items/heals.
Duration ~80 s (0:47 -> ~2:08). Turns ~6 (4 KOs + Arcanine 2-3 turns).

## Sabrina (window 7785 s). Blastoise Lv45 43/135 at start (damaged going in)
| 0:50-0:55 | Sabrina intro |
| 1:05-1:10 | Kadabra 38; Water Pulse; Lv46 on KO |
| 1:20-1:30 | Mr. Mime 37; Water Pulse, Sabrina Hyper Potion (x1 visible), HP 46->25/138 |
| 1:35-1:40 | Surf KOs Mr. Mime |
| 1:45 | player bag open, X SPEED selected (1 X Speed used; 5 -> 4 left) |
| 1:50 | Venomoth 38, Surf; HP 11/138; 1123 exp |
| 2:05-2:10 | Alakazam 43, Surf, Player defeated Sabrina at ~2:10 |
Enemy: Kadabra38, Mr.Mime37, Venomoth38, Alakazam43 (4 mons). Items: 1 X Speed. No heal; HP ended 11/138.
Duration ~80 s (0:50 -> ~2:10). Turns ~9 (Kadabra 1-2, Mime 2-3 incl. Hyper Potion stall, X Speed 1, Venomoth 1-2, Alakazam 1).

## Giovanni (window 8505 s). Blastoise Lv48 35/143
| 0:20 | (pre-fight: Metapod->Butterfree evolution, 0:00-0:20) |
| 0:50 | Giovanni intro |
| 0:55-1:10 | Rhyhorn 45 (Water Pulse x2 seen, super effective) |
| 1:10-1:15 | Dugtrio 42 |
| 1:20-1:30 | Nidoqueen 44 (Lv49 143->147 after) |
| 1:35 | blue screen (level-up/transition) |
| 1:40-1:45 | Nidoking 45 |
| 1:50-2:00 | Rhyhorn 50 (Water Pulse) |
| 2:05-2:10 | Player defeated Giovanni; Lv49 39/147; Earth Badge |
Enemy: Rhyhorn45, Dugtrio42, Nidoqueen44, Nidoking45, Rhyhorn50 (5 mons; order as shown). No items/heals.
Duration ~78 s (0:50 -> ~2:08). Turns ~5-7 (about 1 per mon; Rhyhorn 45 may be 2).

## Rival Route 22 (window 8685 s). Blastoise Lv49 76/147
| ~0:05-0:10 | battle intro (transition at 0:05), Pidgeot 47 |
| 0:10-0:30 | bag: X ACCURACY ("accuracy rose") and X SPECIAL ("Sp. Atk rose") = 2 X items (stock X Special 20) |
| 0:35 | Surf KOs Pidgeot (1731 exp) |
| 0:45 | Venusaur 53, Blizzard; Lv50 79/150 (levelled) |
| 1:05 | Rhyhorn 45 (Bite), 1300 exp |
| 1:15-1:30 | Gyarados 45 (Intimidate; rain started; Blizzard x2 -> 2 turns/miss) |
| 1:40-1:45 | Growlithe 45, Bite |
| 1:50-2:00 | Alakazam 47 (Calm Mind x2), Surf |
| ~2:10 | Player defeated RIVAL A |
Enemy: Pidgeot47, Venusaur53, Rhyhorn45, Gyarados45, Growlithe45, Alakazam47 (6 mons).
Items: X Accuracy + X Special (2 turns in battle; HP untouched).
Duration ~125 s (~0:07 -> ~2:10; start may be slightly before the 0:05 sample). Turns ~12 (2 X + 6 KO turns + extra Blizzard/Calm Mind turns).

## Summary
koga ~72 s / ~8t; blaine ~80 s / ~6t; sabrina ~80 s / ~9t; giovanni ~78 s / ~6t; rival ~125 s / ~12t (2 X items).
Note: Blastoise enters most fights at low HP (Koga 33/120, Giovanni 35/143, Sabrina 43/135), no Revives/potions seen except Sabrina's own Hyper Potion.
