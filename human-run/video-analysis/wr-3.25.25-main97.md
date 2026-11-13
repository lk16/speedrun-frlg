# WR video: frlgr2-wr-3.25.25-main97.mp4

Elite 4 Round 2 world record, **3:25:25.3** by Main97, set 2026-08-18 (category
facts in `PROMPT.md`). 3:26:00 of 1080p60 video; the run starts ~0:00:35 into
the file. Everything below was read off the video overlay on 2026-11-13 with
the pipeline in `METHOD.md`. **External evidence**: it records what the runner
did; mechanisms still need `decompiled/` citations before a plan relies on them.

Overlay facts: LiveSplit attempt counter **15811**; comparisons shown were
"Bounty" (aggressive gold-pace targets, final 3:25:30) and "PB/WR" (the old WR
pace **3:26:31**, which this run beat). Footer: "World Record is 3:26:31 by
me" — so WR-holder racing his own record; he has thousands of attempts of
practice on this exact route.

## Starter policy (observed at 0:01)

Squirtle, nicknamed "A" (1-char name: faster text). Overlay IV box: IVs
**15/26/9/30/20/29** (HP/Atk/Def/SpA/SpD/Spe), nature **Rash**, seed FRAME
**5241**. His chat bot judges starters by IVs+nature ("X/X/X/X/X/X with Calm
nature does not have a runnable nature"), i.e. the reset gate is on
IVs/nature, evaluated at stat display — the displayed "FRAME" suggests he also
tracks which boot frame the seed came from. SpA IV 30 and Spe IV 29 on the
accepted starter; our plan's gate (L5 stats: total >= 71, HP >= 19, SpA >= 12)
filters a similar thing with less information.

## The run's actual split times (read off the final LiveSplit frame, t=12345s)

Split names are his; `(a,b)` after a name is the dex-caught pair shown for the
two comparison routes at that split (Bounty-route, PB/WR-route). `seg` is the
delta to the previous split.

| Split | time | seg |
| --- | --- | --- |
| Brock (3,3) | 14:40 | 14:40 |
| Misty (8,8) | 28:01 | 13:21 |
| Bill | 38:10 | 10:09 |
| Bike (9) | 46:14 | 8:04 |
| Rock Tunnel | 55:39 | 9:25 |
| Giovanni 1 (11) | 1:04:56 | 9:17 |
| Snorlax (14,15) | 1:19:47 | 14:51 |
| Koga (19,20) | 1:31:42 | 11:55 |
| Blaine (27,29) | 1:42:54 | 11:12 |
| Sevii (35,36) | 1:58:07 | 15:13 |
| Sabrina (45,46) | 2:11:54 | 13:47 |
| Erika (46,46) | 2:14:50 | 2:56 |
| Giovanni 3 (52,51) | 2:23:53 | 9:03 |
| Start E4R1 (60,60) | 2:36:05 | 12:12 |
| Lorelei 1 | 2:38:58 | 2:53 |
| Bruno 1 | 2:41:03 | 2:05 |
| Agatha 1 | 2:42:53 | 1:50 |
| Lance 1 | 2:45:39 | 2:46 |
| Champ 1 | 2:48:50 | 3:11 |
| Icefall Cave | 2:58:24 | 9:34 |
| Rocket Warehouse | 3:07:48 | 9:24 |
| Lorelei 2 | 3:13:59 | 6:11 |
| Bruno 2 | 3:16:03 | 2:04 |
| Agatha 2 | 3:17:56 | 1:53 |
| Lance 2 | 3:20:30 | 2:34 |
| Champ 2 | 3:24:12 | 3:42 |
| End | 3:25:25.3 | 1:13 |

Reading of the structure:

- **All 60 catches are done before Round 1 of the Elite 4** ("Start E4R1
  (60,60)"), i.e. National Dex requirements are complete going into the first
  Hall of Fame; nothing dex-related happens after it. Our current plans
  backload fills to after HoF 1 — this is a direct structural difference to
  test.
- The split names imply the badge order ... Bill -> Bike -> Rock Tunnel ->
  Giovanni 1 (Game Corner?) -> Snorlax -> Koga -> Blaine -> Sevii -> Sabrina
  -> Erika -> Giovanni 3; so Blaine and the first Sevii trip happen *before*
  Sabrina and Erika, unlike our plan's gym order. (Which trainers each split
  boundary contains still needs the scene log below to pin down.)
- Dex pacing by split (from the `(a,b)` pairs): ~half the dex by Sabrina,
  then +14 between Erika and E4 R1 across Giovanni 3 — a late catch block
  exists in the WR route too, just pre-E4 rather than post-HoF.
- E4 R1 full round: 2:36:05 -> 2:48:50 (**12:45** incl. walking in);
  R2 round: 3:13:59 start-of-Lorelei-2 ... End 3:25:25 (**~11:26** from
  entering Lorelei 2 fight; Lorelei-2 split itself contains the walk).
  Post-HoF1 segment (Champ 1 -> Icefall Cave -> Rocket Warehouse -> Lorelei 2)
  is 25:09 of Sevii questline.

## Scene log (60 s sampling, Haiku transcription)

See the appendix table at the bottom; summarized findings are folded into the
sections above. Sampling is 1 frame/min: anything shorter than ~1 min can hide
between samples. Not extracted: audio (no STT in sandbox), chat column for
this video (the stream file's chat was mined instead), per-fight turn detail.

## Appendix: condensed route reconstruction (from `data/wr-scene-*.jsonl`)

Catch counter `[caught/total]` is his tracker. Highlights, video-minute based
(run starts ~0:00:35, so run time ~= video minus 35 s):

- 0:02 rival (Bulbasaur) at lab; 0:04-0:11 Route 1/22/2, Weedle caught with a
  plain Poké Ball at 0:09 (chat: weaken first, "usually 78%" not guaranteed);
  0:12 Viridian Forest double-battle statues? (building) -> 0:14 A Lv13
  learns Water Gun pre-Brock; 0:15 Pewter mart (Parlyz Heal; money 4508).
- 0:18-0:20 Route 3 Bug Catchers; 0:21 evolution (Wartortle); 0:22 PC box
  deposit; 0:23-0:25 Mt Moon (Rocket, fossils); 0:27-0:29 Misty (Staryu
  Lv18): moveset then Tackle/Water Gun/Bite/**Mega Kick**/Water Pulse.
- 0:30-0:40 Nugget Bridge + Route 25 trainers (Water Pulse one-shots), Bill;
  0:41-0:42 S.S. Anne + rival; 0:44 HM01 cutscene; 0:46 **Bicycle** in bag by
  Vermilion; 0:47-0:50 Routes 6/11?; Surge not sampled here (no Surge split —
  he is fought at the flute leg, see 1:18).
- 0:51 Rock Tunnel (flash-lit circle: **Flash in use**); 0:52-0:56 tunnel
  trainers; 0:57-1:06 Celadon: Game Corner coins (0:58), Rocket Hideout
  (1:00-1:04, Giovanni 1 ~1:05), **Clefairy bought 180 coins** (1:05), dept
  store (1:06, Leaf Stones/X items bought around here).
- 1:07 Cut tree; 1:10-1:16 Pokémon Tower (CH'DING the traded Farfetch'd used
  as Gastly fodder at 1:10), Marowak Lv30 (1:13), Poke Flute (1:16); 1:17
  save screen: VERMILION CITY, 2 badges(!), dex 14, time 1:13 — **only 2
  badges before the flute leg** (Boulder, Cascade): Surge comes next (Lv21
  Voltorb at 1:18), Snorlax 1:19:47.
- 1:21-1:27 **Safari Zone before Koga** (~5 min: Nidoran/Nidorina/Exeggcute/
  Rhyhorn, 29->20 Safari Balls, 14->21 caught); 1:28 Fly; 1:29-1:31 Koga
  (Weezing Lv43 vs A at 1 HP!); 1:32-1:34 **Power Plant** (Pikachu caught by
  Eevee Lv25 mid-paralysis); 1:35 Surf Route 21; 1:36-1:38 Mansion (Raticate,
  Grimer, Ultra Balls); 1:40-1:42 Blaine (Arcanine Lv47, Water Pulse).
- 1:43-1:58 Sevii trip 1: Tri-Pass 1:44, One Island, Berry Forest/Lostelle
  1:55 (+catches), Meowth 1:56; 1:59 **Omanyte revived** (fossil);
  2:00 Pikachu -> Raichu (Thunder Stone); 2:01 Venonat->Tangela trade
  ("NORMA"); 2:04 Silph rival (Venusaur Lv40), 2:05-2:07 **Weedle->Kakuna->
  Beedrill farmed by evolution** (38->46 caught), 2:08 wild Nidorino, 2:09
  **Master Ball** from president; 2:11 X-item bag check: Leaf Stone x2,
  X Attack x3, X Speed x5, **X Special x20**, X Accuracy x3, Guard Spec x2.
- 2:11:54 Sabrina; 2:14 wild Tangela caught; 2:15 Gold Teeth -> Warden
  (Strength); 2:16-2:17 **Nidorino->Nidoking (Moon Stone), Clefairy->
  Clefable** (46->50); 2:19-2:20 Viridian gym (Machoke, Marowak...?);
  2:21 Victory Road trainer; 2:23 Giovanni 3 (Nidoking Lv45); 2:25 Route 22
  rival 7 (Venusaur Lv53, Growlithe...); 2:27 a dex-list check; 2:28-2:31
  Victory Road (Psyduck catch 53->54, boulders); 2:32-2:35 Indigo: heal, a
  **Persian Lv32 + Growlithe Lv32 + Pidgeotto + Gloom + Exeggcute** party
  shuffle and PC **deposit** before E4 (60/60 reached at 2:33).
- 2:36-2:48 E4 R1 with Blastoise "A" Lv50->56 (Lorelei Dewgong 52 / Lapras
  54; Bruno Onix 51, Hitmonlee 53; Agatha Gengar; Lance Gyarados 56,
  Aerodactyl 58; Champion Pidgeot 59, Venusaur 63 — Blizzard/Bite/Surf seen).
- 2:49-3:10 post-HoF: One Island 2:51, Mt Ember?/Cubone 2:52, **Ruby to
  Celio 2:55**, Icefall Cave 2:57-2:58 (Lorelei scene), Dotted Hole 3:00,
  Rocket Warehouse 3:01-3:06 (Vileplume Lv54 admin), Magneton 3:07, Surf/bike
  3:09-3:10; 3:11 HM/TM teach to Clefable ("ABLE") — Cerulean Cave prep;
  Mewtwo caught unsampled (between ~3:08 and 3:12); 3:12 Lorelei 2 already
  (Dewgong Lv64).
- R2 E4 fought by **Mewtwo Lv70-73** (nicknamed "A"? panel shows player name
  — moves seen: **Fire Blast** vs Jynx Lv66 at 3:13, 231->241 HP): Agatha 2
  Misdreavus Lv65 3:17, rematch Champion Gyarados/Arcanine/Alakazam Lv73
  3:21-3:23; HoF card at 3:25 shows in-game TIME 3:15.

Unresolved overlay details: what "TOTAL: m" counts (grows 22->60 over the
run; not simply caught), the "Bounty" comparison's exact meaning (aggressive
target pace summing to 3:25:30), the deposit/party logistics at Indigo.
