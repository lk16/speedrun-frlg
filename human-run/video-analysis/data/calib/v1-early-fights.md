# WR video (frlgr2-wr-3.25.25-main97.mp4) early fight durations, 4 s sampling
Sheets: /tmp/claude-1000/-home-luuk-projects-speedrun-frlg/788d68b1-7160-44ef-9445-8101618d7646/scratchpad/calib/v1/{brock,brock-pre,misty,rival-ss-anne}/
Cell time = window offset + cell label. Keyframe-only decode: +-2-4 s label error (brock-pre vs brock overlap showed ~3 s drift). Player mon is nicknamed "A" (Squirtle line).
Timer offset: video = run time + ~0:35.

## Brock (Pewter Gym leader)  -- Squirtle Lv11, 10/32 HP at start
- Intro (Brock sprite slides in): brock-pre 1:12 -> video ~822 s (13:42). Geodude Lv12 sent out ~826 s.
- Enemy mons: Geodude Lv12, Onix Lv14 (only two; FRLG Brock).
- Geodude: 1 turn (Bubble, super effective, HP bar empty by brock +8 s). Squirtle HP stays 10/32.
- Onix: sent out ~brock +16..20 s; "A used BUBBLE" +20..24 s; Squirtle drops 10 -> 5/32 (Onix hit first, faster); Onix gone by +28 s. Best estimate 1 turn (2 turns unlikely: 2 Onix hits would have KO'd the 10 HP squirtle; cannot exclude a miss).
- Total turns: 2 (range 2-3). Moves: Bubble only (Squirtle). No heals, no X items, no crit seen.
- Victory ("gained 324 EXP") at brock +28 s = video ~853 s (14:13). Lv12 (+28 s) -> Lv13 (+32 s), HP 9/36; Water Gun learn prompt +44..48 s (~873 s); BOULDERBADGE text 877 s (14:37).
- Duration: intro->victory/EXP ~31 s; intro -> end of level-up/learn screens ~51 s; intro -> badge ~55 s.
- Pre-fight (not part of Brock): gym trainer Camper (Geodude Lv10, Sandshrew Lv11) ending ~750+48 s; Squirtle was Lv8 4/26 then, leveled to 11 via trainer.

## Misty (Cerulean Gym) -- Wartortle Lv20, 56/56
- Trainer intro / transition: ~1:25-27 into window (map view at +24, battle shown at +28 "LEADER MISTY sent out STARYU!") -> start video ~1631 s (27:11).
- Staryu Lv18: foe Harden (+32), foe Water Pulse (+40, A 56->49), A Bite (+44). Staryu faints ~+50. ~3 turns (2-4).
- Starmie Lv21: sent out +52; Starmie hit A 49->37 (super effective noted at +56, likely Water Pulse); A used TACKLE at +64 (A 25/56), 12/56 at +68; Starmie faints ~+72-76. ~3-4 turns. Starmie got 2 hits in (49->37->25->12 => 3 foe attacks) so Starmie fight >=3 turns.
- Total turns: ~6-7 (range 6-8). Player moves seen: Bite (Staryu), Tackle (Starmie; no Water Gun used visibly), A HP floor 12/56 (21%) -> after levelup Lv21 15/59.
- Victory: "grew to LV 21" +76 s, "got P2100 for winning" +80 s = video ~1685 s (28:05). Duration intro -> prize message ~54 s (1631->1685); to ~ +84 s dialogue end ~58 s.
- Aftermath: Pokecenter at ~+100 s, TM learn Water Pulse screen at +152 s on party (A heals to 35/59 via heal?).  Not needed.

## Rival 2 (SS Anne) -- Wartortle Lv26, 71/71
- Dialogue "Imagine seeing you here!" at +12 s; battle transition +16..20 s; Pidgeotto out and "Go! A!" at +24 s. Battle start ~ +18 s => video ~2508 s (41:48).
- Observed enemy order: Pidgeotto Lv19, Ivysaur Lv20, Raticate Lv16, Kadabra Lv18 (this is the order seen; 4 mons).
- Pidgeotto: 1 turn, "A critical hit!" at +28/+32 (HP bar already empty), EXP 459 at +36 s. A HP unharmed 71/71.
- Ivysaur: out ~+38 s with low-ish HP bar already (yellow) at +40; foe Sleep Powder (+40/+44; A 71/71 still); A Bite (+52/+56, A now 44/71, took 27 from Ivysaur attack, probably Vine Whip/Razor Leaf); Ivysaur faints ~+58. ~2-3 turns (Sleep Powder turn may have failed/hit; note player HP drop 27 between).
- Raticate Lv16: sent out +60 s, "Foe RATICATE's attack missed" +64/+68, A 44/71 unchanged; "A..." +72 faints. ~1-2 turns, probably 1 (A kills first or after miss).
- Kadabra Lv18: out +76 s, move menu open at +76/+80 (Bite selected, cursor; moves: Water Pulse, Water Gun, Bite, Mega Kick), super effective at +84; HP empty. ~1 turn (A 44/71 unchanged, so Kadabra never acted/dealt damage; A went faster). Bite used.
- Victory: "grew to LV. 27" at +88/+92 s (A 46/73). Black at +96 s. End ~ video 2582 s (43:02).
- Total turns ~5-7 (best 6). Moves: Bite (every mon). Notable: one crit on Pidgeotto, Sleep Powder from Ivysaur, Raticate miss, no heals/X items; HP 71 -> 44 only.
- Duration: intro -> last-victory/level-up text ~76 s (2508 -> 2584); transition-to-battle (start of fade) 2506 -> ~78 s.

## Summary (video duration s; turns)
Brock ~31 s to EXP (~51 s to end of level-up screens); ~2 turns.
Misty ~54 s; ~6-7 turns.
Rival SS Anne ~76 s; ~6 turns (4 mons).
