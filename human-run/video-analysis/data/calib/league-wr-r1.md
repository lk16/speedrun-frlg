# WR Round 1 League, per-turn (frlgr2-wr-3.25.25-main97.mp4)

Window video 2:36:20-2:49:00 (run = video - 0:35). Blastoise "A" solo, Surf/Bite/Water Pulse/Blizzard.
Data: `league-wr-r1.json`. Foe bar = tool reading of 48 px (full reads 47); 0 after a KO is a lower bound.
Bite is Dark (special in gen 3), hence the X Special stacking. External claims, no decomp citation.

## Lorelei (2:36:20-2:39:04, 13 actions, 261 dmg taken, Lv50 -> 52)
- Dewgong 52: X Special x4 (SpA +4) while it chips: Hail (0), Surf 24, Surf 22, Ice Beam 22, Surf 21, hail 9/turn x4
  (150 -> 25 HP). Then Bite 47 -> 19 px (nve), Bite KO. Hail ended after 5 turns.
- Cloyster 51 / Slowbro 52 (se) Bite one-shot, Jynx 54 Surf one-shot, no damage. Lv51 box: 153/108/119/123/111/107.
- Lapras 54: Full Restore (28 -> 153) T1; Body Slam 34, then CRIT 69, then 33; Bite 47 -> 27 -> 9, Sitrus Berry +6 px, Bite KO.
## Bruno (2:39:12-2:41:08, 9 actions, 108 dmg, Lv52 -> 53)
- Onix 51: Full Restore (20 -> 156) then EQ 34; X Special (Rock Tomb 16, Spe -1); X Special (EQ 30); X Speed (EQ 28);
  Surf KO. HP 48 for the rest. Hitmonchan 53, Hitmonlee 53, Machamp 56, Onix 54: Surf one-shots, no foe move.
## Agatha (2:41:20-2:42:56, 7 actions, 0 dmg, Lv53 -> 54)
- Gengar 54 Double Team x2 (evasion +2): X Speed, X Special, then Surf KO. Golbat, Arbok (Intimidate likely), Gengar 58,
  Haunter: Surf one-shots. Lv54 gains +3/+2/+2/+5/+1/+3 (HP/Atk/Def/SpA/SpD/Spe), max HP 162.
## Lance (2:43:20-2:45:40, 10 actions, 197 dmg, Lv54 -> 55)
- Overworld Full Restore before (54 -> 162). Gyarados 56: Intimidate; X Accuracy (DR 40), X Special (DR 40), Full Restore
  (82 -> 162, Hyper Beam MISSED), Bite 47 -> 34 (DR 40), Bite 34 -> 21 (DR 40), Blizzard KO. Four Dragon Rages = 160.
- Dragonair 54 x2 and Dragonite 60: one Blizzard each (KO from full). Lv55 gains +3/+3/+2/+2/+2/+2 (165).
- Aerodactyl 58: moved first, Ancient Power 37 (85 -> 48), Surf KO.
## Champion (2:46:20-2:49:00, 13 actions, 36 dmg, Lv55 -> 56)
Pre-fight: overworld Full Restore (55 -> 165) and one Elixir (PP). Start 165/165.
| foe | turn | action | foe move / dmg | bar before -> after |
|---|---|---|---|---|
| Pidgeot 59 | 1 | Guard Spec (Mist) | Sand-Attack blocked | 47 -> 47 |
| | 2 | X Speed | Sand-Attack blocked | 47 |
| | 3 | X Special (SpA+1) | Aerial Ace 36 (165 -> 129) | 47 |
| | 4 | X Special (+2) | Sand-Attack blocked (move name unread) | 47 |
| | 5 | X Special (+3) | Sand-Attack blocked, Mist wore off | 47 |
| | 6 | Surf | none | 47 -> 0 |
| Venusaur 63 | 1 | X Accuracy | Growth (SpA +1 for foe) | 47 |
| | 2 | Blizzard, CRIT | none | 47 -> 0 |
| Alakazam 57 | 1 | Surf | none | 47 -> 0; Lv56, 131/167 |
| Rhydon 59 | 1 | Surf, CRIT | none | 47 -> 0 |
| Gyarados 59 | 1 | Bite | flinched | 47 -> 26 |
| | 2 | Blizzard | none | 26 -> 0 |
| Arcanine 61 | 1 | Surf | none (Intimidate at entry) | 47 -> 0 |
Items: Guard Spec 1, X Speed 1, X Special 3, X Accuracy 1. Stages at Pidgeot KO: SpA +3, Spe +1.

## HP max by level (EV inference)
150 / 153 / 156 / 159 / 162 / 165 / 167 for Lv50..56. With base HP 79: IV + floor(EV/4) is 22-23 at Lv50 but
exactly 24 from Lv53 on (Lv53 needs >=24, Lv56 needs <=24), so about 4 HP EV points arrived mid-League. IV/EV split not identifiable.

## Items overall
Full Restore x5 (Lapras, Bruno Onix, overworld pre-Lance, Lance Gyarados, overworld pre-Champion; start count 9 inferred, 4 left).
X Special 18 -> 7 (11 used: 4 + 2 + 1 + 1 + 3), X Speed 4 -> 1, X Accuracy 2 -> 0, Guard Spec 2 -> 1, Elixir 2 -> 1.

## Could not read / uncertain
- Level-up stat boxes for Lv52, Lv53, Lv56 were skipped (only max HP known). No Atk/Def/SpD/Spe totals beyond Lv51.
- Bruno T4 X Speed and Agatha T1 X Speed inferred from bag cursor + counts (message not framed).
- Champion T4/T5 foe move assumed Sand-Attack (only "protected by MIST" read). Arbok entry text only "Foe AR..".
- Plateau bar values +-1 px; turn times +-2 s. Dragonair/Dragonite hit sizes are KO lower bounds only.
