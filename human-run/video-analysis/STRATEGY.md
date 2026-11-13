# How Main97 actually plays Round 2 (consolidated from all committed video evidence)

Consolidation of every committed Main97 finding file, written 2026-11-13. One description of
two routes: the **3:25:25 WR route** (FireRed era, set 2026-08-18) and the **newer LeafGreen
deferred-catch route** (PB 3:27:20, being grinded in Nov 2026). Section 10 lists the gaps
between this and our simulator (`crates/frlg-human`, plan `372-tactic181`).

**Everything here is an external claim**: it says what the runner did or said, never why the game
behaves that way. Routing claims built on it still need a `decompiled/` citation before a plan
relies on them (CLAUDE.md "the one rule"). Speech evidence is whisper small.en output; the
transcript authors repaired names from context.

## Source keys and confidence labels

| Key | File |
| --- | --- |
| WR | `video-analysis/wr-3.25.25-main97.md` (overlay read of the WR run; splits are hard data) |
| S12 | `video-analysis/stream-2026-11-12-main97.md` (overlay/scene read of the Twitch stream; splits of the LG route) |
| T03, T06, T07, T09, T10, T11, T12 | `video-analysis/transcripts/2026-11-DD-{youtube,twitch}.md`; time = video `h:mm:ss` |
| CA, CB | `video-analysis/data/chat-A.md`, `chat-B.md` (Twitch chat text; stream time) |
| WEB | `video-analysis/data/runner-intel-web-2026-11-13.md` (search snippets; every primary fetch 403'd) |
| J-dd | `human-run/journal/2026-11-dd-*.md` (our own analysis, not runner evidence) |
| M | the model: `crates/frlg-human/src/{route,sim,params}.rs`, `plans/372-tactic181.md`, `calibration.md` |

Labels: **[H]** = read off an overlay/splits panel, or an unambiguous quote. **[M]** = clear quote
but meaning inferred. **[L]** = garbled whisper, a single mention, or a guess by an earlier note.
**[CONFLICT]** = sources disagree; both sides are given.

**Duplicate-stream warning [M].** The "2026-11-10 YouTube" transcript (T10) and the "2026-11-12
Twitch" transcript (T12) carry the same events at (almost) the same timestamps: offset
`+114 -> 110` at 1:51:44, "Modest is super good" at 2:48:52 vs 2:48:40, the "3 HP / 10 Def Rash"
choice at 6:36:06 vs 6:36:09, "97.71% runnable" at 1:01:36 vs 1:01, "2 of 12 past Brock" at
8:18:33 vs 8:18, "30 s ahead of my FireRed record after Koga" at 5:24:06 vs 5:24, "7 evos / 5 evos"
at 4:42 / 5:06, the deep run at video 2:21-5:07 vs 2:21-5:00 (S12). Treat T10 + T12 + S12 + CA +
CB as **one stream** (a simulcast, one file mislabelled by date), not as independent
confirmations. T03, T06, T07, T09, T11 are separate sessions.

---

## 1. Starter acquisition

### 1.1 Species, nickname, version

- Squirtle every attempt, on both routes [H: WR 0:01 overlay "A", IVs 15/26/9/30/20/29, Rash,
  seed FRAME 5241; T10 1:01-1:34 "Squirtle"; T12 1.2]. Nickname is a single character ("A",
  "faster text") [H: WR]; the **rival must also be nicknamed** and a mistyped rival nickname "kills
  the run" [M: T10 7:33:52-7:34:11, T12 7:33].
- Version: WR route is FireRed [M: WEB title "FireRed Round 2 Speedrun in 3:25:25"; WR scene log
  has Growlithe, not Vulpix]. The new route is **LeafGreen** [H: S12 Vulpix; T09 0:37:37 "Nido
  level 30 in Leaf Green"]. He states FR/LG is one category [T07 4:14:26-4:14:56]. Reason for the
  LG-specific route: Clefairy costs **250 more coins** in LG, "that's why there's an entire
  different route ... based around one Pokemon" [M: T06 1:55:09]. He is committed to LG for now
  ("not going back", T07 0:44:33; "no fire [red]... enough good things on this route", T10 4:58:40;
  T12 4:59) but earlier "still experimenting ... don't know if it's better or worse" (T03
  1:56:07-1:56:26) and "FireRed is where my muscle memory is" (T03 3:08:18). Chat's idea that a
  Pewter skip makes FR better (no Mr. Mime trade) was left unresolved [CA 5:00:15].

### 1.2 The frame-manipulation procedure

- The starter is **chosen by frame, not by rerolling blindly**: "you're aiming for a specific
  Squirtle on an exact frame ... every frame is a different Squirtle, and it's a one-frame press
  to generate this Squirtle. The one you see the stats on screen is guaranteed to be this one"
  [H: T03 7:19:29-7:20:29]. Verification is the stat screen (IV box + nature + the seed FRAME
  shown in his overlay [H: WR; METHOD.md overlay map]); a miss is a soft reset [T06 3:13:21-3:13:57].
- He describes it as **two frame-perfect manips to start the run**: a starter ("Squirtle") manip
  and an **encounter manip** (section 2) [H: T10 1:34:42, T12 1:34, T06 0:14:50-0:15:22].
- Tooling [M]: an audio-beep timer program (FlowTimer, plus a program he calls "Context"; version
  2.4.3 is "Icy's" good one) with a per-computer **ms offset**; +114 ms one day, 110 the next
  ("4 ms is a quarter of a frame") [T07 2:36:05 "positive 114, was 126 yesterday"; T10 1:51:44-
  1:52:09; T12 1:51]. It reports every press as early/late in ms with a hit probability ("-7 ms
  56% hit chance") [T03 6:05:30-6:05:48]. Working settings: beeps on the device clock, atomic
  sync, high priority; "keep the device awake between beeps" OFF (T06 0:15:22-0:15:49). Restart or
  reopen the program between attempts (T06 3:12:08-3:12:21, 4:03:10). Input device: GBA SP
  buttons (sticky) -> wants a GameCube controller on the GBI [T03 0:41:00, 6:55:13-6:55:38,
  7:56:33-7:57:03]; he says the encounter-manip press on a GameCube pad "was frame perfect first
  try" [T03 8:07:22-8:07:40].
- **Lab NPC parity rule [M]**: "if the girl moves in the first, on frame 30 and it's odd parity,
  you can't hit the Squirtle"; Context has to predict lab NPC movement and can be wrong
  [T06 1:27:02-1:27:40, 3:13:21-3:13:57, 3:34:10-3:35:06]; "I just lost 22 seconds because I get
  fucked by parity" [T06 0:25:27]. Three named failure sources: one-frame press skill, NPC
  prediction, and "the program was designed for emulator" [T06 3:13:21].
- A different, later frame costs real timer time: a candidate that was "**15 seconds later**" and
  Modest was weighed against an earlier 3-HP-IV Rash [M: T10 6:36:06-6:37:26, T12 6:36-6:37]. So
  the frame table is time-ordered and he trades frame wait against stat quality.
- He will not start from a pre-made save of a good Squirtle ("starting at New Game", "feels
  cheated") [T06 3:19:35-3:20:22] - consistent with the PROMPT timing rule.

### 1.3 Accuracy / hit rate

| Observation | Source | Conf. |
| --- | --- | --- |
| "the way it started out it was 90% accurate ... I can live with 10% Context fails" | T06 0:24:23-0:24:29 | M |
| 6 Squirtles in a row the day before; then "limbs of hope ... despair" | T06 0:15:09, 1:23:06 | M |
| After close/reopen fix: four in a row, "formula for Squirtle manip success ... only took two months", "I don't think I've missed one that was in Context since switching" | T06 3:23:36, 6:00:22, 6:10:52 | M |
| "majority of the runs I've lost have just been from Context failing, not not hitting the frame" | T06 6:09:52-6:10:10 | M |
| 11-07 session: "I've just missed 30 times", "12 misses in a row", ~1.5 h of the stream lost to drift (offset moves 1/4-1 frame overnight; computer restart fixes) | T07 3:00:10, 3:10:58, 2:48:23-3:01:24, 0:02:19 | M |
| Failure signature: "12 times in a row one frame early -> adjust offset"; "two to three frames early every time", manual +4 | T07 3:06:46-3:07:11; T03 6:59:26, 7:13:01 | M |
| 11-12 session: "hit the first two Squirtles, then late six times", "lost five or six attempts", "missed the encounter 12 times in a row" | T10 1:51:44, 1:37, 7:50 | M |
| "every time I start the stream it takes two attempts to get a Squirtle" | T11 1:35:18 | L |
| "97.71% for a runnable Squirtle and I get two dead IDs in a row" - meaning/direction unclear | T10 1:01:36, T12 1:01 | L |
| Fatigue: execution degrades at ~6-7 h ("seven hours is probably too much"); late-day errors: A pressed before Start, Right+A same frame | T10 7:28:21, 7:26:18; T03 | M |

Reading: **P(manip hits) is a runner/tool state, not a constant** - ~90% in a good regime
(hours-long), 0-30% in a drifted regime. Regime changes day to day. "Context fail" is the main
non-game reset cause [T03 0:33:44-0:33:46 "every run is a context death"; T06 6:09:52].

### 1.4 Miss cost and attempt economy

- A miss is a soft reset before/at the stat screen: cost is **off-timer wall clock**. Observed
  cycle: reset -> New Game -> starter in hand at timer ~**1:10-1:30** [H: S12 attempt economy].
- 45 attempts visible in 8.4 h (~11 min average wall time per attempt; one reached the E4 door)
  [H: S12].
- A bad day: ~1.5 h lost to drift (T07), "two full weeks of inconsistent manip" (T10 5:33:49-5:34:02).
- His own accounting: "Early game deaths are completely fine, as long as the manip doesn't take a
  [expletive] hour" [T06 1:32:36].

### 1.5 Accepted natures / IVs - and the contradictions

User statement (task): "only Rash and Mild are run". **Evidence does not support exclusivity.**

| Source | What it says | Conf. |
| --- | --- | --- |
| WR 0:01 overlay | the 3:25:25 starter is **Rash**, IVs HP15 Atk26 Def9 SpA30 SpD20 Spe29 | H |
| S12 chat bot | `1/12/8/26/5/2 with Calm nature does not have a runnable nature` - a nature whitelist exists; Calm rejected | H |
| T03 7:17:44-7:18:35 (answering chat) | "needs to be [more] Special Attack nature, **not minus Speed nature, and 30 Special at a minimum**" (first clause garbled "less special attack nature") | M |
| T07 3:31:59-3:34:47 | "**three** runnable"; "No, **Mild is terrible** ... Mild is the worst of the three"; reason: Rock Tomb does 10-12 on Mild vs 8-10 on Rash so no HP both keeps Torrent for the first three fights and avoids Rock Tomb range; Mild also bad in E4 R1 (Lance Hyper Beam range) | M |
| T07 2:39:03, 2:43:07, 2:45:23 | **Modest** played: "I don't get it but it's modest", "you go into that fight as Modest without Bubble" - three separate hits, attached to the same Squirtle that is called "a very good round two Squirtle" at 2:36:17 | M |
| T09 0:42:50-1:12:58, 4:06:14 | **Mild constantly handed to him** ("how do I end up with Milds every run", "God, you get so punished for running Milds", "31 defense mild man, it's just not good"); says "I'm just not gonna run Mild anymore" at 0:44:35 but keeps playing Mild (4:06:14) | M |
| T09 6:05:57-6:06:06 | one **Rash** damage-calc remark | L |
| T10 1:59:35-2:00:04 | "zero special defense **Rash** is probably a waste of time" (-> plays glitchless instead) | M |
| T10 2:48:52-2:48:55 | "low-key **Modest** is really, like, super good" (tankiness) | M |
| T10 6:36:06-6:37:26 (= T12 6:36) | rejects "3 HP and Defense Rash" ("you generally avoid low HP"), weighs it against a "15 seconds later and it's **Modest**" candidate | M |
| T11 7:22:08 | "I'm [expletive] mild bro" at a Brock fight of his best early game of that day | L |

**[CONFLICT] with the user's "only Rash and Mild":**
- Modest is named as played/"super good" in two independent sessions (T07, T10/T12) and as a
  candidate in a third mention. Mild is spoken of as the *worst* acceptable. Rash appears least in
  speech but is the WR's nature.
- The set that fits every statement is **{Rash, Modest, Mild}**: the +SpA natures are Modest
  (-Atk), Mild (-Def), Rash (-SpD) and Quiet (-Spe); "not minus Speed" removes Quiet, leaving
  exactly "the three" of T07. Calm/neutral/others are rejected. Ranking by his words: Rash ~
  Modest > Mild (-Def hurts Rock Tomb/Brock survival). **Treat the user's claim as the
  common-case subset, not as exclusive, until the bot's nature list is read from a chat log or
  frame.** [inference, M]

**IV thresholds (what is and is not sourced):**
- SpA IV >= 30: one spoken source only (T03 7:18:35, whisper-garbled); consistent with WR (30) and
  "Nice. Big 31 special. That's fine." (T09 1:53:21, 6:02:53). No other stream mentions it [M].
- HP: the stat he screens hardest early: "20 HP, un-real", "don't be 19 HP", "I really don't like
  that HP", HP IV 3 rejected unless the alternative is worse [T12 1:36:54, 7:56:33, 6:37:04]. HP
  must permit Torrent at Brock and a Rare Candy to Blastoise later (bad HP IV "is fucked", T10
  3:47:53-4:02:06). Too-*high* HP is also bad: "the only issue here is our HP, it's too high"
  (T11 5:00:29) [M].
- Def: Mild's -Def; "31 Def Mild is not good defense"; Def IV 10 "not great" (T09 1:12:58; T10
  6:37:04) [M]. SpD: matters for Rash (0 SpD IV rejected; T10 1:59:35). Atk: matters for Bite/Mega
  Kick ranges despite the SpA nature (T11 0:36:04, 4:38:50); a 15-Atk Squirtle is acceptable (T07
  0:18:31) [M].
- Speed: **no starter Speed threshold** in any stream except the glitchless-branch minimum of
  **26 Speed** (T06 6:12:42-6:13:18). In R2, Speed is made up with EVs/Carbos/candy level ("I
  technically need one speed EV this run", "24+ speed outspeeds Rapidash", "on 29+ speed skip
  candying Blastoise") [T09 4:09:48, 8:16:16-8:17:07; T07 1:47:03-2:09:08] [M]. WR starter Spe IV 29.
- "Runnable" is a bot/program verdict; he can still reject a runnable one ("that was runnable,
  it's really bad though" T03 6:34:58-6:35:25) or take a poor one and switch to glitchless/Round 1
  from the same start ("not a good squirtle for round two, so this is probably going to be a good
  one [for the other category]", T09 5:22:13; T10 4:47, 5:28:36).
- Persim Berry is bought per starter by Defense/confusion risk (T03 1:59:48-2:00:21) in earlier
  streams; later "always Persim, never Revive" (T07 1:23:20-1:23:37).

### 1.6 Same start serves Round 1 / glitchless

He decides Round 2 vs Round 1 vs glitchless after ~10 min from the same New Game [T03 0:22:22-
0:22:51; T06 6:10:22-6:10:40 "90% of runs are going to be Round 2"]. Glitchless/R1 times he cites
(2:36-2:39) are another category; Brock-split times of 11:41-12:17 (T06 6:14-6:17) are very likely
glitchless/R1 numbers and **[CONFLICT]** with the R2 Brock split of 14:05-14:40 (S12, WR).

---

## 2. Encounter-RNG manipulation

- The second frame-perfect press (the "encounter manip") fixes the **early-game wild encounter
  sequence** [M: T06 0:14:50-0:15:22; T10 1:34:42]. User-supplied (S12 post-analysis): the aim is
  very few encounters in the Brock split; our own math (J-13 morning, `docs/tricks/mkdasher-rng-notes.md`)
  says P(<= 2 hits in the first 60 forest rolls) = 0.7% of seeds, so resetting selects quiet seeds.
- What it buys, in his words: "encounter manip saves a minute", "this route saves 11 seconds" [T07
  4:08:34-4:08:38]; "you're always gonna be one minute ahead with encounter manip" [T09 3:44:34];
  "with encounter manip and YOLO Brock ... pretty much every run past Brock that survives is usually
  over a minute ahead of record; it's the surviving part that's hard" [T06 4:06:35-4:07:04]; "if
  encounter manip didn't exist my record would be hard as hell to beat" [T07 4:42:59-4:43:19];
  "the route doesn't save [more than] 11 s vs the WR route if you take away the encounter manip save"
  [T06 2:12:55-2:13:40, garbled]. Benchmarks: both-bugs + Pikachu split **14:14** now vs **14:20
  pre-manip** [T09 3:46:30] - a 6 s difference, which sits oddly beside "a minute"; the minute is
  presumably the encounters *avoided* over the whole Brock split plus fewer deaths **[CONFLICT/L]**.
- Mechanism he states: the forest encounter "shows up four or five tiles after getting to the
  grass"; there is no secret path, only tile count [T09 4:28:00]. "Good seed" = a Viridian Forest
  with the encounters he wants, e.g. "a perfect two encounter forest" [T09 4:31:05]; Route 1 two
  kills with Bubble, forest bugs, Pikachu [T06 3:48:26-3:48:51]. No seed number or threshold is
  ever spoken.
- Hit rate: "75% chance ..." (garbled, T07 1:11:15) [L]; "I just missed the encounter 9 to 12 times
  in a row" [T10 7:51:17-7:52:24]; `keep the device awake` broke it for 7 attempts [T06 0:15:22].
- It cannot be held through the run: later encounters are real RNG ("catching Pokemon is random
  because you don't know what you're going to encounter", T06 2:00:10); J-13 morning refuted
  extending it to Mt. Moon.
- Stalling for the 2nd bug "always takes forever with encounter manip" (T10 2:30:36) - manipped
  runs have *fewer* encounters, so camping a bug costs steps/time.
- Brock-split survival with the manip: 75% on a good-luck day [T07 2:33:33]; 2 of 12 and 3 of 14 on
  bad days [T11 0:31:57, 1:46:37; T10 8:18:33]; "80% Brock deaths" one bad week [T09 5:38:43]; "it's
  always early-game deaths" [T10 6:56:49]. Both-manips-good runs that pass Brock "usually die on the
  next splits" (Misty split deaths exceed Brock deaths that day) [T07 2:32:08-2:34:01].

---

## 3. Full segment order with split times

### 3.1 The two routes' split tables [H: WR final LiveSplit panel; S12 "LG PB" column]

Split names are his; `seg` is time since the previous split. Catch pair is (WR route, LG route)
from the dex counter in his comparison columns.

| Split | WR time | WR seg | LG PB time | LG seg | dex (WR, LG) |
| --- | --- | --- | --- | --- | --- |
| Brock | 14:40 | 14:40 | 14:23 | 14:23 | 3, 4 |
| Misty | 28:01 | 13:21 | 27:43 | 13:20 | 8, 8 |
| Bill | 38:10 | 10:09 | 37:49 | 10:06 | |
| Bike | 46:14 | 8:04 | 45:44 | 7:55 | 9, 9 |
| Rock Tunnel | 55:39 | 9:25 | 55:02 | 9:18 | |
| Giovanni 1 | 1:04:56 | 9:17 | 1:04:09 | 9:07 | 11, 11 |
| Snorlax | 1:19:47 | 14:51 | 1:19:24 | 15:15 | 14, 14 (15 WR) |
| Koga | 1:31:42 | 11:55 | 1:28:15 | 8:51 | 20, 19 |
| Blaine | 1:42:54 | 11:12 | 1:36:19 | 8:04 | 29, 22 |
| Sevii | 1:58:07 | 15:13 | 1:50:42 | 14:23 | 36, 29 |
| Sabrina | 2:11:54 | 13:47 | 2:01:29 | 10:47 | 46, 34 |
| Erika | 2:14:50 | 2:56 | 2:06:02 | 4:33 | 46, 36 |
| Giovanni 3 | 2:23:53 | 9:03 | 2:22:15 | 16:13 | 51, 51 |
| Start E4R1 | 2:36:05 | 12:12 | 2:37:40 | 15:25 | 60, 60 |
| Lorelei 1 | 2:38:58 | 2:53 | | | |
| Bruno 1 | 2:41:03 | 2:05 | | | |
| Agatha 1 | 2:42:53 | 1:50 | | | |
| Lance 1 | 2:45:39 | 2:46 | | | |
| Champ 1 | 2:48:50 | 3:11 | | | |
| Icefall Cave | 2:58:24 | 9:34 | | | |
| Rocket Warehouse | 3:07:48 | 9:24 | | | |
| Lorelei 2 | 3:13:59 | 6:11 | | | |
| Bruno 2 | 3:16:03 | 2:04 | | | |
| Agatha 2 | 3:17:56 | 1:53 | | | |
| Lance 2 | 3:20:30 | 2:34 | | | |
| Champ 2 | 3:24:12 | 3:42 | | | |
| End | **3:25:25.3** | 1:13 | **3:27:20** (LG PB end) | | |

Other facts [H unless marked]: WR-run attempt counter **15811**, comparisons "Bounty" (aggressive
gold pace, final 3:25:30) and "PB/WR" (old 3:26:31) [WR]; the LG stream is at attempt 18415-18427,
so ~2600 attempts since [S12]. E4 R1 as a block 2:36:05 -> 2:48:50 = **12:45**; R2 block from
Lorelei 2 fight to End ~**11:26** [WR]. Post-HoF1 chain Champ 1 -> Lorelei 2 = **25:09** of Sevii
questline [WR]. In the LG route the E4 segments themselves are near-identical to the WR's [S12].

### 3.2 Route order, WR route (FireRed-era) [M: WR scene log at 60 s sampling; run time ~ video - 35 s]

1. Lab (rival 1) -> Route 1 -> Viridian -> Route 2 -> Viridian Forest -> Pewter; **Weedle caught
   with a plain Poke Ball** ~0:09 (chat: weaken first, "usually 78%") ; Pewter mart (Parlyz Heal;
   money 4508) ~0:15; Water Gun learned Lv13 pre-Brock; **Brock 14:40** (3 caught).
2. Route 3 Bug Catchers -> **Wartortle ~0:21** -> PC box deposit ~0:22 -> Mt. Moon (Rocket,
   fossils) -> Cerulean: **Misty 28:01** (Staryu Lv18; Squirtle moveset Tackle/Water Gun/Bite/
   Mega Kick/Water Pulse) (8 caught).
3. Nugget Bridge + Route 25 trainers (Water Pulse one-shots) -> **Bill 38:10** -> S.S. Anne + rival
   ~0:41 -> HM01 -> **Bicycle by Vermilion** (Bike 46:14, 9 caught). No Surge split: **Lt. Surge is
   folded into the flute leg** (Lv21 Voltorb right at the Snorlax wake, ~1:18).
4. Rock Tunnel (Flash, 55:39) -> Celadon: Game Corner coins ~0:58, Rocket Hideout, **Giovanni 1
   1:04:56** (11 caught), **Clefairy bought** ~1:05, dept store (Leaf Stones, X items) ~1:06.
5. Cut tree ~1:07 -> Pokemon Tower: **traded Farfetch'd "CH'DING" used as Gastly fodder** ~1:10,
   Marowak Lv30, Poke Flute ~1:16; save screen at 1:17: Vermilion, **2 badges**, dex 14 (so Surge not
   yet fought) -> Surge -> **Snorlax 1:19:47**.
6. **Safari Zone before Koga** (~5 min, 1:21-1:27, 29 -> 20 Safari Balls, 14 -> 21 caught; Nidoran/
   Nidorina/Exeggcute/Rhyhorn) -> Fly -> **Koga 1:31:42** (Weezing Lv43 vs the Squirtle at 1 HP).
7. **Power Plant** 1:32-1:34 (Pikachu caught by an Eevee Lv25 mid-paralysis) -> Surf Route 21 ->
   Mansion (Raticate, Grimer, Ultra Balls) -> **Blaine 1:42:54** (Arcanine Lv47, Water Pulse) (29).
8. **Sevii trip 1** 1:43-1:58: Tri-Pass, One Island, Berry Forest/Lostelle ~1:55, Meowth 1:56 ->
   **Sevii 1:58:07** (36). Then **Omanyte revived** 1:59; Pikachu -> Raichu (Thunder Stone) 2:00;
   Venonat -> Tangela trade ("NORMA") 2:01; Silph rival (Venusaur Lv40) 2:04; **Weedle -> Kakuna ->
   Beedrill farmed by evolution** 2:05-2:07; wild Nidorino 2:08; **Master Ball** 2:09; X-item bag at
   2:11: Leaf Stone x2, X Attack x3, X Speed x5, **X Special x20**, X Accuracy x3, Guard Spec x2.
9. **Sabrina 2:11:54** (45/46) -> **Erika 2:14:50** (46) [Erika is *after* Sabrina] -> wild Tangela
   2:14 -> Gold Teeth -> Warden (Strength) 2:15 -> **Nidorino -> Nidoking (Moon Stone), Clefairy ->
   Clefable** 2:16-2:17 (46 -> 50) -> Viridian gym **Giovanni 3 2:23:53** (Nidoking Lv45; 52) -> Route 22
   rival 7 (Venusaur Lv53) 2:25 -> Victory Road 2:28-2:31 (**Psyduck** catch 53 -> 54, boulders) -> Indigo
   2:32-2:35: party shuffle (Persian Lv32, Growlithe Lv32, Pidgeotto, Gloom, Exeggcute) and a PC
   deposit; **60/60 at 2:33; Start E4R1 2:36:05**.
10. **E4 R1** 2:36-2:48 with Blastoise "A" Lv50 -> 56 (Lorelei Dewgong 52/Lapras 54; Bruno Onix 51/
    Hitmonlee 53; Agatha Gengar; Lance Gyarados 56/Aerodactyl 58; Champion Pidgeot 59/Venusaur 63;
    Blizzard/Bite/Surf seen).
11. **Post-HoF** 2:49-3:10: One Island 2:51, Cubone/Mt Ember ~2:52, **Ruby to Celio** 2:55, **Icefall
    Cave** 2:57-2:58 (Lorelei scene; split 2:58:24), Dotted Hole 3:00, **Rocket Warehouse** 3:01-3:06
    (Vileplume Lv54 admin; split 3:07:48), Magneton 3:07, Surf/bike 3:09-3:10; a TM/HM teach to
    Clefable ("ABLE") 3:11; **Mewtwo caught unsampled between ~3:08 and 3:12**; Lorelei 2 already
    fought at ~3:12-3:13 (Dewgong Lv64).
12. **R2 E4 with Mewtwo Lv70-73**: Fire Blast seen vs Jynx Lv66 (231 -> 241 HP); Agatha 2 Misdreavus
    Lv65; Champion rematch Gyarados/Arcanine/Alakazam Lv73; HoF card shows in-game time 3:15.

Dex pace on the WR route: ~half by Sabrina, +14 between Erika and E4R1, **60/60 before the first
Hall of Fame** - all dex requirements are done pre-E4; nothing dex-related happens after HoF1 [H].
What "TOTAL: m" in his tracker counts is unresolved [WR]; at the deep-run reset the chat says "total
61" [CA 4:55:00].

### 3.3 Route order, LeafGreen deferred-catch route [H splits, M order]

Differences from the WR route (his own list, T10 2:11:11-2:12:21, T12 2:12): **shop once, in
Cinnabar** (not a Fuchsia shop plus a One Island shop); **Mom-heal skip** (keeps Torrent longer);
**Rare Candy on Blastoise** ("candy blast"); **different PC box visits**; **evolve both bugs on the
same split**. Order as spoken (T06 8:43:05-8:43:24, 8:53:25; T09; S12): Brock -> Mt. Moon -> Misty -> Bill
-> Bike -> Rock Tunnel (**Route 8 Lum Berry grab**, T10 1:45) -> Celadon / Rocket Hideout (Giovanni 1) ->
Pokemon Tower / Lavender rival / flute -> **Surge folded in with the Snorlax leg** -> **Safari ->
Koga -> (Power Plant / fishing) -> Mansion -> Blaine -> Sevii -> Sabrina -> Erika -> Giovanni 3 ->
Victory Road -> E4 R1** -> post-game (Cerulean Cave Mewtwo) -> E4 R2.

Structure of the change [H: S12 split table]: **catches are deferred**. By Sabrina the new route has
34 dex vs the WR route's 46 and is **10:25 ahead**; it then pays most back in the Erika (4:33 vs
2:56), Erika->Giovanni 3 (16:13 vs 9:03) and ->E4 (15:25 vs 12:12) blocks, arriving at E4R1 **1:35
behind** WR pace (2:37:40 vs 2:36:05). Koga->Blaine->Sevii is genuinely faster (Safari trimmed 19 vs
21 caught by Koga; Mansion trimmed 22 vs 29 by Blaine), with those species recovered in the Berry
Forest leg (Weepinbell, Hypno seen) and the late block. Its stated value is **consistency rather than
luck-spikes**: "with the old route you'd get on good paces by taking a risky camp and spiking a
unique encounter" [T10 5:26:56-5:27:36 = T12 5:26-5:27] and "the route doesn't save [more than] 11
seconds" at equal luck [T06 2:12:55] - gain is mainly manip + doomed attempts dying cheaper (S12
reading, M). "Execution matters more in glitchless than Round 2" / "little catch variance, it's a
very tight route: hit your kicks, don't die" [T06 4:51:28, 5:50:12].

The late block (Erika -> 60/60), from the deep run [M: S12 + CA]: Power Plant (Magnemite caught by a
**Ponyta** as ball-thrower), Fighting Dojo (gift **Hitmonlee**), Moon/Leaf-stone evolutions (Moon
Stone Nidoqueen + Clefable; Leaf Stone Victreebel; Metapod -> Butterfree), Route 22 rival, Victory
Road (Onix caught by a **Victreebel**, Machop by a **Nidoqueen** with a Great Ball), ending 60/60
immediately before the pace reset. A planned **Sevii catch ("otto") was missed**, forcing improvised
Victory Road catches (Marowak, Onix, Geodude that self-destructed) [CA 4:52-5:03]. A fill plan
quoted for another run: "Pidgey on Route 1 after Giovanni (Max Repel), 3 in Victory Road including 2
candy mons" [T10 4:39:32, 4:43:17; T12 4:38-4:48].

---

## 4. Per-segment tactics

(All [M] unless marked; quotes are whisper-level.)

**Lab / Rival 1.** Rival Bulbasaur. If he Growls on turn 1 you need 24 Attack to keep attacking, else
Tail Whip twice; a turn-2 Growl is "really bad" [T06 5:56:39-5:57:45]. A late manip frame plus Growl
is a possible pace reset ("left the lab at 3:30") [T06 6:04:30-6:05:22]. A "slow Rival 1 fight, slow
lab exit" is a quality hit on a low-HP Squirtle [T10 6:37:37].

**Route 1 / Forest (first split).** Kill both Route 1 encounters with Bubble; "never catch anything on
Route 1" [T06 3:47:35]. Never enter the forest without Bubble ("don't ever risk a Pikachu without
Bubble") [T06 3:47:50]. **Sammy rule, "completely scripted now"** [T10 2:18:21-2:21:35, T12 2:18]:
if **Sammy poisons you** (needed for low HP/Torrent) -> catch no bugs and leave; if not poisoned ->
**camp for both bugs**; rarer: camp, a Weedle poisons you -> leave with one bug ("one bug is enough:
evolve it on Koga/revive"); a bug poisons only ~15% per hit. Full Torrent -> no reason to camp (it
costs HP). Catch odds quoted: weakened pre-Sammy bug 50-62%, after Bubble 62-78%; "78% is not
guaranteed", 31% HP = 78% (HP-IV dependent) [T10 2:15:20-2:41:38]; breakouts at 78% kill runs [T06
1:30:43, 4:39:54, 5:32:24]. **Forest Pikachu**: catch rate "garbage (~40%)", "I arguably should have
thrown the ball after one Bubble turn" [T07 3:18:48-3:20:30]; "having both bugs and Pikachu is
pretty uncommon" [T03 1:25:01]; Pikachu crit/Thundershock kills runs [T06 6:02:44]. Antidote picked
up at the forest tree; Mark-guy potion every run ("I bought six potions, want three for the run")
[T06 2:55:15, 3:33:25].

**Pewter / Brock (the 75%-to-17% gate).** Torrent logic [T10 2:02:02-2:05:37, T12 2:02-2:08, T07
3:32:14-3:34:04]: start the gym fights at an HP that keeps **Torrent (HP <= 1/3) for the L8, L9 and
L10 mons** but is **not in Rock Tomb range at L11** (Onix); Rock Tomb 8-10 (Rash) / 10-12 (Mild),
**Withdraw on turn 1 -> 5-7**; if Bind misses, stall a turn (Tackle) to keep Torrent. For one
Squirtle the thresholds were 5 -> 7 -> 9 HP. Fastest finishing is Tackle ("Bind adds text") [T10
0:31:28]. Withdraw on **Geodude** is bad ("Defense Curl") [T07 3:37:06-3:38:47]; Onix Rock Tomb
unreliable to use [T07 3:27:21]. Need "two of three of {poison on Squirtle, scratch damage, the damage
range}" [T06 3:47:24, 4:06:35]. **Never heal in/before the gym; skip the PC potion** ("fastest gym
entry I've had, I skipped the potion") [T10 9:49, 2:34:37; T12 0:09]. Gym entry level **14** preferred
(12 with both bugs and no Torrent "not good enough") [T07 3:19:27-3:34:04]. "YOLO Brock" [T06
4:06:35]. Liam (Pewter camper) fought with Tackle to keep range [T12 2:34]. Brock splits 14:05 (best
LG, S12) - 14:40 (WR); R2 Brock deaths dominate ("brock, brock, brock, ..." CB 6:52:00).

**After Brock: Route 3 / Mt. Moon.** He wants Torrent for Route 3 "through level 17" [T09 1:07:12;
T10 2:07:55; T07 4:26:57 "stall six times for Torrent"]. Branch (T09 0:45:51-1:25:26): **Route 3
bug catchers (Colton)** "out of Torrent" vs **Mt. Moon Rocket** ("a terrible fight", try to avoid) vs
**Zubat + Josh** (Josh = Mt. Moon 1F Youngster). Route 3 Jigglypuff L3 cannot be weakened (only
Sing) -> throw at full HP "and pray, it saves five minutes" / avoid Route 3 with Torrent [T03
1:02:42-1:04:08; T11 0:24:05-0:27:22; ~50% L3 and half carry an Oran Berry]. Gold Route 3: Spearow
(Peck) + Zubat weakened with a Torrent move, Mankey bad catch rate, **Route 4 "guaranteed catch rate
on most things"** [T11 0:28:43-0:30:00]. **Center skip needs ~6 potions**; "double potion into full HP
to keep the Center" [T09 3:54:12; T11 0:30:06]. Mt. Moon Grimer: be in Torrent, "a range in Torrent,
not a range out of Torrent" [T10 2:08:23]; Arbok rocket: hit through paralysis rather than Para Heal
into poison [T11 1:44]. "Camp for Route 3" = deliberately take Spearow/Pidgey damage to be in Torrent
(Torrent "16" / "14") [T06 6:24:52-6:25:41].

**Rival 2 / Misty.** **Mega Kick** (TM, 75% acc) is the workhorse ("how are you going to get past
Rival 2 without Mega Kick" T10 3:00:38) and its misses are his biggest variance ("one for six", "five
for ten", 6% for two in a row at 25% miss) [T06 2:01:38, 2:59:16-3:04:16; T09 6:17:17 "five Mega
Kicks every run"]; kick-select input errors nearly killed runs (Right+A same frame) [T10 2:49:21].
Rival Ivysaur/Raticate/Kadabra: use Bite only with Overgrow considerations, compute Hyper Fang +
Confusion high rolls to decide Water Gun vs kick, "29 max" rule [T09 6:04:31-6:07:26]; swap Spearow
to tank Vine Whip [T06 2:07:19-2:08:49]. **Misty**: double Water Pulse; "Water Pulse above half is
scary" (confusion -> **Persim Berry every run**), Starmie crit deaths; "always 24 Misty"; "spec strats"
(special-move fallback) lost ~1.5-2 min when kicks missed [T11 3:54:57]. **Camper Jeff (Route 6)**: fight
**Nob** (Route 25 Hiker) first purely to cross the L25 -> L26 breakpoint so Jeff's Quick Attack cannot
kill ("Dan"/"Nob" = Route 25 Youngster/Hiker XP trainers) [T09 4:05:46-4:06:14, 4:00:39]; optional
trainers are XP purchases skipped when already above range.

**S.S. Anne rival, Bill, bike.** "Died on boat rival: kick miss, Poison Powder, crit Vine Whip" [T06
2:32:25]. Bike: dismount two tiles early (frame-1 dismount impossible, "Select ... gets off 5% of the
time") [T03 1:42:23-1:42:55]. Cut-tree "deload" skip on the city entrance [T09 2:05:30-2:12:40]; Rock
Tunnel Flash line-of-sight dodging ("bonk the ledge, see if the guy looked up") [T03 2:06:08].

**Lavender / Surge / Snorlax.** Lavender rival Ivysaur: kick first, then Leech Seed/Sand Attack; "I'll
lose 30 seconds" [T06 6:50:52-6:54:02]. **Surge** is folded in the flute leg (WR) because it keeps
Torrent from the Lavender rival for Surge's Voltorb: kick first then special, heal right after "to be
right outside Torrent for the next fight" [T06 1:56:44-1:57:42]. Early-Surge variants floated, "does
work weirdly enough", "I hate this fight on late Surge" [T11 4:46:21-4:50:40; T07 0:38:21, 2:00:37].
Pokemon Tower: Rattata Quick Attack double-crit death (1/64) [T09 4:43:37-5:06:34]; he declines a
Super Potion because of the extra menu time.

**Koga (11:55 WR / 8:51 LG).** "Koga is the record's best split by a lot" [T10 5:24:24]; 30 s ahead of
FR record after Koga [T10 5:24:06]. **Lum Berry** (Route 8 grab after Rock Tunnel, held until Koga)
so Will-O-Wisp/sludge status does not kill; "Koga deaths are a huge problem on this route" [T09
6:22:48-6:26:10; T10 1:45:17-1:45:51]. Weezing/Arbok: know the Torrent state; non-Torrent Water Pulse
bites, Torrent Pulse kills [T09 6:29:54-6:30:19]. Always go for the range [T09 2:19:18]; **double
candy** (to Lv41-42 after the Juggler) was introduced to stop Koga/Erika deaths [T07 2:11:56; T10
4:28:16]. WR: Weezing Lv43 vs the Squirtle at **1 HP** [WR].

**Blaine / Sabrina / Erika / Giovanni.** Blaine: outspeed Rapidash/Growlithe - Speed 24+, Carbos, or
candy; "on 29+ speed skip candying Blastoise" [T09 8:16:16; T07 1:47:03; T11 6:15:56]. Sabrina: lost to
10% Psybeam confusion, Future Sight; heal vs YOLO unresolved [T09 3:20:11-3:35:29]; WR carries X
Special x20 by then. Erika: "you just die to Erika every time"; **Blizzard x2**, antidote on the swap
turn for Poison Powder, **Antidote carried over** [T10 5:17:04-5:17:33; T11 4:51:31-4:56:54]. Giovanni 3:
Super Potion on Gyarados etc. [T11 3:29]; WR beats it with Nidoking Lv45 in party.

**E4 R1 (12:45 WR; 2:35-2:37 entry target).** Blastoise Lv50-56, **X Special** on Agatha and Lance
("one extra on Agatha, one on Lance") and Guard Spec on Bruno; Bruno: HP/Def lets Blastoise tank Hyper
Beam/Thrash; Lance Dragonair Blizzard vs Dragon Rage; Gyarados 7% death per Blizzard turn; "heal on
Gyarados rather than risk Blizzard" [T03 3:11-3:34, 6:07:51-6:11:01, 4:12:46-4:12:55, 4:51:41]. **Black
Glasses "required for Lorelei, that's the main reason they exist"** [T06 4:56:44-4:57:44]. Evolve a
**Moon-Stone Nido during the Lance fight** is an untried idea (Bace) - enter at 58, evolve to 60;
needs a decomp check on when the dex count is read [T07 1:16:02-2:19:51].

**Cerulean Cave / Mewtwo / E4 R2.** Mewtwo (Master Ball from the Silph president, WR 2:09) is the R2
attacker. **Five cave encounters cost ~1:30** ("losing a minute and a half") even with Repel [T03
3:54:51-4:07; 4:30:38 "about a minute from the cave"]. Mewtwo stats checked "180 attack, 230 special";
**Shock Wave is slot one**, Fire Blast seen in the WR vs Jynx; Champion 2 Gyarados does not die to
230 Special without a boost: **X Special (+1) then Guard Spec then Shock Wave**; "I didn't have an X
Special for Champ 2" is why a strat failed; wrong move/missed Fire Blasts on Champ 2 cost the run
(a possible record: "another 237" is ambiguous [L]) [T03 4:02:35, 4:13:08-4:14:12, 4:36:38]. R2 fights
are mostly one Mewtwo hit per mon: 92-196 s each [J-13 mismatch hunt, from the WR video].

---

## 5. Catches

### 5.1 Quick list of what is caught where (both routes, [M] from scene logs and speech)

| Phase | Catches / entries | Tactic |
| --- | --- | --- |
| Route 1 | none ("never catch anything"); **Pidgey deferred to after Giovanni 3** with Max Repel (LG) | T06 3:47:35; T10 4:26, 4:39 |
| Viridian Forest | Weedle + Caterpie ("both bugs", evolve later), Pikachu (WR gets Pikachu at Power Plant instead) | weaken with Bubble; plain Poke Ball; 50-78% |
| Route 3 | Spearow (also Farfetch'd trade input), Mankey ("early Mankey", Net Ball at 62%), Zubat (Mt. Moon), Rattata ("early rat") | Peck/Pound/Bubble weaken; Poke Ball; Route 4 near-guaranteed |
| Nugget Bridge/Route 25 | hunt, WR: Oddish etc. skipped | -- |
| Game Corner | **Clefairy bought** (coins); Abra | FR cheaper (see 1.1); T06 "5 sets of coins -> more Full Restores" |
| Pokemon Tower | Gastly (Farfetch'd as fodder), Marowak | WR 1:10-1:13 |
| **Safari Zone** (before Koga) | target 7-10: Nidoran M (L30 Area 3) + F (L30 Area 2), Nidorina/Nidorino, Exeggcute, Venonat, Doduo, Rhyhorn; Kangaskhan etc. | **Area 3 "zerg"** preferred to Area 1 "Entrance camp" (fewer rock mons, "rock mons break out"); stop when you hit ~7 planned catches; Safari Balls, Rock twice for Venonat/Parasect; Repel-Safari with an extra Repel |
| Route 15 / Route 7 / Route 8 | forced **Venonat 30%** (for the Tangela trade), Meowth/Vulpix (Route 7), Sandshrew/Pidgey (Route 8, 20%) | "force or kill the run"; Meowth 5th try cost 45 s |
| Power Plant (or fishing) | Pikachu, Magnemite/Magneton, Tentacool, Voltorb | WR: Pikachu with Eevee as thrower; LG: Ponyta Fire-Spins everything; 25% slots, 8-11 duplicates = ~2 min |
| Mansion (Cinnabar) | Grimer, Raticate, **Vulpix (LG, 40% camp)**, Koffing | Max Repel for floors 1-2 Rat, Ponyta weaken |
| Sevii (Berry Forest) | Weepinbell, Hypno (forced Lostelle), Drowzee, Venonat, Slowpoke, Pidgeotto; Meowth | camp 40-45% for unique mon; Three Island forced fights |
| Late block | Power Plant/fishing remainder, Dojo Hitmonlee gift, Route 1 Pidgey, **Victory Road 3 (Machop/Onix/Geodude/Marowak)**, Psyduck | "desperation catches" Magneton, Onix [T07 4:43:44]; VR "never feels good" |
| Gifts/trades | Eevee, Lapras, Snorlax (fight), Hitmonlee, Aerodactyl (Pewter museum/Strength), Omanyte/Kabuto fossils, Farfetch'd (Spearow), Tangela (Venonat), Mr. Mime (Abra) | trade evolutions optional on LG ("No trades first. This is a different route", T12 4:15) |
| Post-game | Mewtwo (Master Ball) | Cerulean Cave, Repel, 5 encounters ~1:30 |

### 5.2 Evolutions as dex currency (his words: "evos", "candy mons")

- Count: **WR route 12-14 of 60 entries are not catches** (J-13 fill audit from the video). LG route:
  "7 evos" with a skipped South/Sevii PC, "would have been 5 on that plan" [T10 4:42:13-5:06:35]; "four candy
  mons in this route" [T06 7:58:19]; "five candies, one on Blastoise" [T09 7:51:20]; "I only have 2 candies
  doing this", "can't do five" [T03 0:52:38, 2:47:09].
- **Stones**: Moon (Nidorino/Nidorina/Clefairy; "late moon", a free stone on the forced Sevii path -
  J-13), Leaf (Exeggcute/Weepinbell/Gloom), Fire (Vulpix/Growlithe/Eevee), Thunder (Pikachu).
  "Nido, Moon Stone, Nido, Nido, Moon Stone" [T06 8:24:03]. "I don't want to do late moon" when the
  Safari gave enough [T12 4:39].
- **Rare Candy mons**: Nidoran M -> Nidorino saves a candy ("worth two", "Big Nido ... Exeggcute: two
  of the most valuable catches in the run" T03 1:35:21-1:35:43), Kakuna -> Beedrill, Ponyta -> Rapidash
  ("I always want a candy for Ponyta"), Paras -> Parasect, Pidgey -> Pidgeotto, Venonat -> Venomoth,
  Meowth -> Persian. **Warden candy** (a Rare Candy tied to the Safari Warden / Strength; exact source
  **[L] unresolved**) is the extra candy; skipping it costs one candy [T03 1:16:10; T07 4:37:02-4:38:43].
  Never double candy [T07 2:11:43] **[CONFLICT]** with "double candy every run" [T10 4:28:16, 5:20:18;
  T11 5:21:34] - the later sessions adopt double candy to avoid Koga/Erika deaths.
- **Trades**: Spearow -> Farfetch'd "CH'DING" (WR; also the Cut carrier and fodder), Venonat -> Tangela
  "NORMA" (WR 2:01 and LG T03 0:56:36), Abra -> **Mr. Mime** (needed because Clefairy has to be afforded:
  "the reason it exists as a trade is because we have to go back and buy Clefairy so you just can afford
  to buy two Abras" T06 7:30:34). Mr. Mime is irrelevant with a Pewter skip. Trade-evolution "swap-
  evolve on Lance/Champ" ideas [T10 4:17; T11 6:31:52].
- **Strength carrier choices**: Aerodactyl (learns Strength immediately: "that's why Pewter is good"),
  Nido/Nidoking, Clefairy (needs a forgotten move), Cubone via Warden-candy skip [T03 2:21:49-2:22:16;
  T07 0:51:04, 4:32:40].

### 5.3 Optional levers (all "dex for time")

| Lever | What is said | Value / cost |
| --- | --- | --- |
| **Pewter detour** (Aerodactyl, Diglett/Dugtrio, Mr. Mime) | "3 Pokemon, all pretty much guaranteed ... **54 seconds per catch**" [T03 1:16:59-1:17:32; T06 1:49:49 "base: 55 s/Pokemon"] | consistent; Aerodactyl doubles as Strength carrier |
| **Power Plant** | "you could easily get cooked", "25% encounter + huge detour"; 8 duplicates + 3 on the E4 split = **~2 min** ("some 15 s each"), **11th try Pikachu** | but integral for Pikachu/Tentacool/Magnemite when Pewter skipped [T06 7:53:37-8:07:57, 8:50:09-8:52:33; T07 0:39:03] |
| **Safari stop count** | leave at ~7 planned catches; "eight to ten from Safari"; bad Safari = 3:20 for 2 mons, -2 to -3 min; a Safari with no breakouts was a 25-sub goal | T03 0:49:22, 5:54:58; T10 5:44:09-6:51:36; T11 6:50:33 |
| **Route 15 Venonat force** | 30%; "just kill your run if you don't get it in time" | T03 1:35:15 |
| **Route 7 Vulpix/Meowth** (10-20%) | "gotta figure out a good way to do Route 7" | T09 3:28:08; 45 s for a 5th try (T10 1:43:42) |
| **Fishing** (new, LG): Good/Super Rod from the Cinnabar backyard, fly back and fish again | "replacement for the Power Plant", "much better than the FR Good Rod", no timings, collaborator's proposal (Tentacool/Krabby-type, ~80% outcomes) | T07 0:30:39-0:36:27; T10 4:22:50-4:27:09, 5:18:46 |
| **Cinnabar Mansion 40% camp** | for a unique mon (Meowth/Vulpix-like); cost a Revive and ~2 min in one run | T10 3:54:13-4:20:32 |
| **Pewter skip** | see below | |
| **Warden candy skip** | "the Safari analog of the old-route Pewter skip" | T07 0:28:19 |

**Pewter skip [CONFLICT, trending against]:** T06 (1:42-1:58, 7:34, 7:38): "the only thing we change is
we skip Pewter", executed on the best run (conditions: both bugs in hand; Mime trade irrelevant; extra
Repel for Repel-Safari; 3 catches shifted to Victory Road). T09 (0:25:53 "no-brainer", 4:07-4:08 "lean
into skipping Pewter *and* Plants"). T10/T12 experiment: Mansion 40% camp + Victory Road breakouts
(Marowak, Machoke, Onix, Geodude choke) -> reset at entry pace; verdict "maybe it's bad, I tried" then
"plan wasn't bad" [T10 4:44-5:02; T12 5:01]. T11 7:36:01: **"I'm not doing Pewter skip. I don't even
think it's good in this route."** Default is "Pewter and Plants both"; best runs skip one of the two,
"both done hovers 2:37-2:39 entry" [T09 0:29:44-0:30:03]; skipping both is his emergency salvage lever
when behind [T09 4:53:31-4:58:30].

### 5.4 Balls and weakening

- Early mart (Pewter): **1-2 Poke Balls** ("what am I buying? one Poke Ball", "two Poke Balls") plus
  Parlyz Heal/Potions; extra Poke Balls only to cover Jigglypuff (1 ball, full HP) [T09 3:43:19, T10
  2:34:56-2:36:34; WR 0:15 money 4508]. Safari: Safari Balls. Mid/late: Great Balls ("Great Ball you can
  guarantee on so many more things in this route"), a Net Ball (Water/Bug), **Ultra Balls bought at
  Fuchsia onwards** ("ten Ultras left" T06 8:38:10; Marowak has horrendous catch rate until Ultras
  T11 5:21:56) [T10 3:30:55-3:32:29]. Budget trade: **1 fewer Ultra Ball buys 2 Full Heals** [T09
  6:38:11]; "drop one Ultra Ball, add one Great Ball" (Icy's shop) [T11 7:14:46].
- Weakening: Bubble/Tackle/Peck/Pound/Water Gun/Bite (Black-Glasses boosted vs Rattata), Ponyta
  **Ember/Fire Spin** as the late ball-thrower ("Fire Spin on Pikachu, Ember is too strong"), Eevee in
  the WR [T06 8:13:39-8:16:47; WR 1:34]. A Poke Ball on low-HP Pidgey (7/22) failed [T03 4:43:18].
  Ponyta, Nidoking, Victreebel, Nidoqueen all used as ball-throwers late [S12].
- Catch rates: bugs "weaken twice unless it would die, not 78%" [T06 3:33:20]; "an average based on HP
  IV, can be higher than 78" [T06 4:16:54]; Rock twice for Venonat/Parasect; 25 s per improvised
  catch incl. heal [T12 5:03].
- Pace tool: a "Safari Zone tracker" and a hope to "reset pace based on Safari" - not yet a rule
  [T10 5:35:13-5:35:43; T12 5:35].

---

## 6. Money, items, shopping

- **Early**: Mart at Pewter/Route 2 side: Poke Balls (1-3), Parlyz Heal, Potions; he wants ~3 Potions
  for the run + Antidote; "stop buying" when Mega Kick PP is enough [T06 2:55:18; T12 3:19]; **Center skip
  needs ~6 Potions** [T09 3:54:12]. X Special bought at Pewter ("buy an extra special") and "buying six
  items is hard" (menu) [T07 0:14:17, 1:21:52]. **Escape Rope** kept for bag-cursor consistency and
  for rope-skip options [T03 0:13:12, 1:04:46; T06 3:50:55]. **Revive: never taken early again**
  ("it curses the run", "can't break my Revive rule") [T03 1:09:18, T09 3:51:12, T11 0:48:09]; a later
  Revive/Full Restore remains as the late-block buffer [T10 4:09:42, T12 3:39].
- **Persim Berry every run** (Misty's Water Pulse confusion / Supersonic) [T07 1:23:20; T06
  3:05:09-3:07:09 "take off the Persim and equip the black glasses in the teleport menu before Lavender
  rival"]. **Black Glasses** (Bite boost) normally; required for Lorelei and Mansion [T06 3:07:39, 4:56:44].
- **Lum Berry (LG route)**: Route 8 grab after Rock Tunnel, equipped until Koga (so Will-O-Wisp/sludge
  does not kill / burn an X Special), re-equip Black Glasses from the menu after; per-run buy/skip ("can't
  justify Lum this run, Safari and good mid-game" T11 0:50:01) [T09 2:03:21-2:04:21, 6:22:48; T10
  1:45:17; T12 1:45].
- **LG shop plan**: "one Cinnabar shop for everything instead of a Fuchsia shop and a One Island shop"
  [T06 1:53:56-1:54:05; T10 2:12:00]; Max Repel from Cinnabar mart; "buy four [Max Repels]", extra Repel for
  Repel-Safari and Route-1 Max Repel (Pidgey after Giovanni), Cinnabar water encounters risk a useless
  Slowpoke [T03 3:06:19-3:13:08, 2:02:06; T10 4:19:39]. Money is binding there: "X Acts are so expensive",
  "Specials tighter than they should be", "can't use Carbos and also do the shop" [T10 3:34:25-3:36:04;
  T12 3:35-3:36]. Celadon dept store (WR): Leaf Stones, X Special x20, X Speed x5, X Attack x3, X
  Accuracy x3, Guard Spec x2 (bag at 2:11); also Elixir/Max Elixir/Protein, "Max Elixir, Repel, Potion"
  [T03 2:14:46; T09 2:35:10]; **Carbos** only via the glitch-free Speed tradeoff vs candy [T09 8:16].
  "I wonder if it's worth buying 5 sets of coins just to buy more Full Restores" [T06 8:13:13]. "Don't get
  the Protein here" [T06 4:57:44].
- **X items**: X Special on every big fight (Agatha, Lance, Champ 1/2, Sabrina), X Accuracy for Mega
  Kick (reassigned between fights: "take away the X-Act from Champ 1 to allow me to double candy"
  T11 5:35:02), Guard Spec on Bruno, X Speed on Growlithe (glitchless) [T03 4:12:46; T11 5:10:40].
- **Rare Candy** (4-5 per run) on Blastoise ("candy blast", Lv39/41/42 targets), mons, and range
  tweaks ("Oran Berry + Candy to Lv41") [T03 2:04:26, 5:40:04; T09 4:49:23-5:02:51].
- **Money figures** in the evidence: WR 4508 at Pewter mart [WR 0:15]; "Clefairy bought for 180 coins"
  (low-confidence scene read, FR) and +250 coins in LG [WR; T06 1:55:09]; "buy five sets of coins".
  No other totals; no prices spoken.
- **Healing**: "no heals mid-fight outside the League; enters gyms at low HP; two heals + a Max Elixir
  between E4 fights; Full Restores sparingly" (J-13, read off the WR video).
- **Menu/bag layout is muscle memory**: item order and cursor positions are scripted; buying/keeping
  an item "for the consistency of that menu" (Escape Rope) [T03 1:04:46, T09 2:13:10-2:18:12, T06
  5:08:30]. A changed notes entry caused a wrong menu [T11 5:19:30].

---

## 7. Reset and pace rules

| Rule | Evidence |
| --- | --- |
| **Starter gate** at the stat screen (nature + HP/Def/SpA IVs; section 1.5); manip miss = reset | T03 7:17:44; T10 1:07 |
| **Early deaths are accepted, not salvaged**: no early Revive, no Ether/Elixir salvage ("it's so much better to just get another run"), "I'm not resetting if it's two seconds slow, I'm just dying" | T03 1:09:18-1:58:18; T09 1:55:34; T07 4:10:05 |
| **"I don't think it's worth playing it safe whatsoever in the first hour"**; playing safe costs 15-20 s/run | T03 1:57:26; T06 0:22:51, 0:23:21 |
| Early-game death share: "four hours of early game deaths into a good run"; deaths list of the stream day: brock, brock, brock, rival 1, rival 2, round 1 pace, sammy, liam, brock, sammy, brock | T07 4:11:56; CB 6:52-6:56; S12 |
| **Round 1 pace gate at the E4 door**: "if I enter at 2:38 it is **impossible** at a certain point the record on any run"; a 60/60 run was **reset at 2:39:37** | T07 4:11:20-4:11:52; S12 |
| Entry targets: **low 2:37 / 2:36** needed, 2:35 sometimes; "I don't have to get a 2:35 entry to get the record, there is some room"; 2:34 + 47 min post-game = 3:21; "most realistic good time 3:23, next PB high 3:24" | T10 5:23:20-5:24:30; T07 3:55:34-4:08:09 |
| WR yardstick: Start-E4R1 2:36:05 -> End 3:25:25 = 49:20 post entry | WR |
| **Koga is a hard-ish gate on the FR route** ("you pretty much know at Koga whether it's a reset"); on LG you can't tell by Koga, so runs are played past Safari; wants a **Safari-based** reset rule | T03 1:56:50-1:57:05; T10 5:35:13 |
| Mid-run: pace-reset examples after a late manip frame ("might be a pace reset"), a run that's "cooked on pace", a run dead to "four minutes behind"; plays slow runs out for data/entry time | T06 6:05:22, 5:46:09; T09 3:08:15 |
| Emergency lever when behind: **skip Plants and Pewter** | T09 4:53:31 |
| Not resetting: "I'm just going to play it out and see what we get on entry" | T09 3:08:15 |
| Gate on LG early splits: "I'm 21:31 with 15 [caught], down 4 Pokemon, a minute 20 behind the target early game, pretty bad" | T11 6:47:30 |
| Hybrid: R2 vs glitchless vs R1 decided from the same start; ~10 min in | T03 0:22:22; T06 6:10:22 |
| Per-day session limits: hands/neck, "always after six hours" errors; stream ends by choice | T09 6:58:00; T10 7:28 |

Death rates (his): "2 of 12 past Brock", "3/14", "four for nine, 40% today", "I've already died like six times
in round two", Brock-split win 75% on a good day, deaths on the next 30-50 min splits [T11, T07, T06, T10].

---

## 8. Quoted time costs (mistakes, catches, misc.)

| Cost | Source |
| --- | --- |
| Catch overall "around **40 s** per catch"; Pewter-type detour mon **54-55 s** | T06 2:00:03, 1:49:49; T03 1:17:32 |
| **Clemi skip ~2 s (2.68 s external) per catch** | T09 2:09:09; WEB (Kotaku 2.68 s) |
| 11 duplicate encounters (Power Plant -> R1 entry) ~**2 min**; Power Plant duplicates ~15 s each; Victory Road dups ~10 s each; 3 dups on E4-door split ~30 s | T06 8:50:09-8:52:33; T09 0:19:18 |
| Safari bad: **3:20 for 2 Pokemon**, "3 min to leave", "lost 2-3 min"; Safari+Mansion "lost 3 minutes" | T10 5:44:09, 6:50:27; T11 6:50:33 |
| Cerulean Cave 5 encounters ~**1:30**; "about a minute from the cave" | T03 4:00:21-4:30:38 |
| Meowth 5th try **45 s** | T10 1:43:42; T12 1:43 |
| Optional-trainer trigger via a missed bike line ~**40 s**; "she instantly spun up" | T09 7:52:26; T10 1:43:02 |
| Trainer not moving / bag-manip reaction ~5 s | T09 1:42:14 |
| Manip parity miss "lost 22 seconds"; playing safe 15-20 s/run; next-frame wait for Modest **15 s** | T06 0:25:27; T03 6:36 |
| Menu mid-fight "lose like 10 seconds"; Oran Berry/menu 10 s; Mark-guy potion; "improvised ball catch + heal ~25 s" | T03 3:26:34; T12 5:03 |
| Spec-strats fallback (2 fights) ~**1.5-2 min** | T11 3:54:57 |
| Early Revive detour **~1 min** each | T03 1:51:55 |
| Mistake bundle in one run: "a minute of mistakes, a minute early-game not fast, a minute from the cave" ~**3 min** | T03 4:30:15-4:30:45 |
| Leech Seed turn ~30 s | T06 6:54:02 |
| Starter reset cycle (reset -> starter in hand) ~**1:10-1:30** timer | S12 |
| Manip drift: ~**1.5 h** of a stream; "30 misses" | T07 3:00:10 |
| Poison text "really slow"; Tackle faster than Bind-text moves; poison tick can break bike-run audio timing | T10 2:17:51, 0:31:28, 3:08:40 |
| Whole-run budget: E4R1 entry 2:35-2:37 + ~47-49 min post-game | T07, WR |

---

## 9. Legality statements

- **Category facts** (PROMPT.md, retrieved 2026-11-09): Round 2, English, console; timer from New Game
  to the first black frame after HoF2; National Dex + Rocket Warehouse + upgraded E4/Champion; **glitches
  not allowed**; FR vs LG free (WR is on a GBI, GBA timing).
- **Frame manips (starter + encounter)**: treated as ordinary play, never described as banned [T03 A10,
  T06, T10 "two frame-perfect manips to even start"]. Save-and-quit manips ("Zapdos SnQ ... that breaks
  the game") are *not* part of his plan [T10 0:24:20, 3:04:47].
- **Clemi skip** (Help menu on L during the caught jingle): he treats it as legal and fundamental
  ("the whole reason that we're all here ... about two seconds by pressing L", "accepted by the category")
  [T09 2:08:15-2:09:16]. WEB: found by iamClemi on the Switch port, 2.68 s/catch (Kotaku), ~1.5 min/Round 2
  per Gunnermaniac (GamesRadar), Switch-specific per games.gg - but **no board ruling found**, and the
  journal measured it working on the GBA game itself (J-13 morning). Whether the 3:25:25 WR used it is
  **unknown** [L]. The Round-2 console-board legality remains a human check (HOST-REQUESTS.md item 5).
- **Tree deload skip** (Cut tree at the city entrance never planted by entering/leaving at a specific
  tile): his argument: "not a glitch ... the player is doing something intended by the game that could be
  done accidentally"; distinguishes Glitched category (arbitrary code execution) [T09 2:05:30-2:12:40]. This
  is **his** view, not a rules ruling [M].
- "Skipping Clefairy/Power Plant is not possible" statements are practical, not rules [T03 1:25:30; 0:57:21].
- 60 caught as the dex requirement is assumed throughout; needs a decomp citation (PROMPT.md).
- "Glitchless" is a *different category* he uses as the fallback; do not treat its splits as R2 (11:4x Brock
  times, 31 s behind record at Giovanni 1, 42/43 Surge) [T06 5:34:42-5:37:00].

---

## 10. Gaps between this and our model (prioritised)

Model = plan `372-tactic181`, EHT(WR+160m) 11.73 h, no plan reaches WR+80m, no-reset mean ~5:40:00 vs
a 3:25:25 human reality (J-13 mismatch hunt, J-10 conclusions). Effect sizes are rough estimates
from the evidence above, not simulator runs; labelled where guessed.

**A. Level of the whole model (largest)**

1. **Fight and segment durations are ~1.5-2x real.** J-13 reality table (from the WR video): Brock
   31-51 s vs model 1:36 (pre-fix); Misty 54 s vs 2:09; SS Anne rival 76 s vs 6:41; Koga 72 s vs 3:19;
   Sabrina 80 s vs 3:04; Giovanni 78 s vs 5:33; Route 22 rival 125 s vs 4:14; E4R1 12:45 vs 19:28; fights
   "20-35 turns at the E4, ~2x real runs'". Even after fixes the model mean is ~5:40 vs 3:25. **Effect:
   ~2 h of the 2:15 gap**; fixing it moves EHT by orders of magnitude (no WR-rung finishes at all now).
   Cause: the model plays "safe" turn sequences, real play has 1-2 turn kills with X items, Water Pulse
   one-shots, Mega Kick and Torrent.
2. **No manipulation model at all** (the "RNG model stays random seed" line in J-10 is obsolete):
   - **Starter manip** is frame selection with ~90% hit rate in a good regime, a stat screen verdict
     and a time-ordered frame table (next acceptable frame 0-15+ s later). Model: a *random* gift mon
     drawn per attempt, gate `total >= 74, HP >= 19, SpA >= 11`, a rejected starter = lost attempt (~114-
     140 s of pre-starter text frames). Real cost per reset ~70-90 s + 10-25% manip misses, but the
     expected accepted starter is **much better** than a random draw (it can pick the best IVs/nature in
     reach) at a time cost, not a reset cost. J-10 said the starter gate was the biggest plan lever
     (EHT 54 h -> ~30 h). **Effect: large on EHT (probably 2x or more) - the sign is that the model
     *over*-penalises starters and under-selects them.**
   - **Encounter manip** (quiet pre-Brock seed): "saves a minute", more survivable Brock split; model:
     pre-Brock mean 20:48 (post-cooldown fix) vs 14:05-14:40 real (~6 min gap, of which ~1 min is the manip,
     the rest is fights). P(<= 2 hits in 60 forest rolls) = 0.7% of seeds (J-13 morning).
   - **Nickname**/rival nickname mistyping and drift regimes: ~11 min/attempt average wall time and
     ~1.5 h bad-day losses are not in the EHT; model has `mistake_rate` 1.0 per 100 tiles only.
3. **Brock split survival rate and the Torrent window.** Real: 75% pass on a good day, 2/12-3/14 on a bad
   day; Torrent (HP <= 1/3) is a *first-20-minute* resource (Brock L8-L10, Route 3, Mt. Moon, Grimer), poison
   from Sammy is a wanted resource, PC potion/Center/Mom-heal skipped, Withdraw vs Onix, HP thresholds 5/7/9.
   Model: Torrent kept globally was refuted (+8.7 h worse in the exact engine) but only as a *global* policy;
   an early-only Torrent plan is not expressible. **Effect: Brock death rate and 30-60 s first-split time,
   plus the dominant early reset sink (most attempts die before Misty).**

**B. Route order**

4. **Gym/badge order.** Real (both routes): Brock, Misty, Bill/Bike, Rock Tunnel, Celadon/Giovanni 1,
   Tower/**Surge+Snorlax**, Safari, **Koga**, Power Plant/Mansion, **Blaine, Sevii trip 1, Sabrina, Erika
   (after Sabrina), Giovanni 3**, Route 22/VR, E4. Model: Surge **early** (right after S.S. Anne), Dig
   TM early, **Erika after the tower** (before Cycling Road/Koga: `erika_after_tower`), **Sabrina before
   Blaine**, then Blaine, Sevii, Giovanni. Differences: Surge timing (Torrent from Lavender rival; "early
   Surge" is an option he rejects on LG but toys with); Erika/Sabrina/Blaine/Sevii order (his Erika is a 2:56-4:33
   post-Sabrina stop while ours is a separate Celadon leg); Safari is before Koga in both.
   **Effect: minutes of walking/fly and level/Torrent state at each leader; not individually estimable.**
5. **Dex completion before the E4.** Both his routes finish 60/60 before E4R1 (WR at 2:33, LG block Erika
   -> E4R1 15:25 incl. VR); model: the National Dex check (`oak-dex-check`) is **after HoF1**, fill sites are
   walked after the League (post-HoF fills + Oak + Celio chain + Cerulean Cave all in the 47-49 min budget).
   Real post-HoF1 is **25:09** (Icefall/Warehouse chain) + Mewtwo + R2 E4 (~11:26) = 49:20 with nothing
   dex-related. **Effect: the model's fill (44.5 min at ~110 s/species per J-13) lives where the real run
   has 0; the E4R1 pace gate (2:36-2:39) cannot be mirrored.** Pre-E4 fills also run with Repel/Max Repel
   (Pidgey Route 1, Victory Road 3, fishing).
6. **Post-game chain is under-specified.** WR splits show Oak/Celio/Ruby -> **Icefall Cave (9:34)** ->
   Rocket Warehouse (9:24) -> Mewtwo/Lorelei 2 (6:11); model has Mt. Ember/Rainbow Pass/Five Island/
   Sapphire/network machine with guessed text frames (~3000 each) and **no Icefall Cave segment** (verify
   in decomp whether it gates the warehouse - one of the audited gaps). **Effect: a few minutes, mostly
   scene-text guesses.**
7. **Sevii trip 1 position/timing.** Real: Blaine -> Sevii 14:23-15:13 -> Sabrina; model rebuilt it
   segment by segment (10 min forced content, J-13) - consistent in magnitude. Remaining unknown: the
   Moon-Stone/Hypno/Berry Forest catch value (model plan 369 catches the Hypno). Low gap.

**C. Catches and dex**

8. **Catch cost per entry.** Real blended ~40-55 s per catch (and 12-14 of 60 entries are evolutions/
   trades/gifts, i.e. ~0 s); model ~110 s per fill species, 47% of it intros + fleeing duplicates.
   **Effect: ~20-25 min if the species count is equal** (44.5 min fill; at ~50 s that is ~20 min).
9. **Safari as a planned 7-10 catch block** (Area 3 zerg, stop at ~7, Safari Balls, Rock-twice, 3:20-for-2
   downside) vs the model's `safari-walk` + fill sites `SAFARI_ZONE_CENTER/WEST` and `safari_model`.
   Missing: a stop-at-N rule, Area choice, the specific species value (Nido M+F, Exeggcute, Venonat, Doduo,
   Rhyhorn) and the variance (-2-3 min bad Safari). **Effect: swing +-2-3 min per run and the main LG
   pace-gate candidate.**
10. **Evolutions as currency.** Real: 5-7 evos (Moon x2-3, Leaf, Fire, Thunder stones + 3-5 candy mons +
    Metapod/Kakuna chain), planned from the bodies already caught; "Warden candy"; Moon Stone late. Model:
    stones x5 and candy chains accepted (371), but candies are a count of items bought (`elixirs` kind
    `candy`), not tied to a Warden pickup, no "5 vs 7 evos" PC choreography (Sevii PC skip), and the
    in-E4 (Lance) evolve idea is absent. **Effect: -1 to -2 h in EHT already realised; the party-slot and PC
    logistics (6 slots, deposit/withdraw sequences ~20-40 s each) are un-modelled cost.**
11. **Trades set.** Real WR: Spearow->Farfetch'd (CH'DING), Venonat->Tangela (NORMA), Abra->Mr. Mime
    (needed for Clefairy money; skipped on a Pewter skip). Model also trades **Electrode for Raichu**
    (no evidence the runner does this) and buys Abra at 2400. Lower confidence; **small**.
12. **Optional levers priced.** Pewter detour 54 s/catch (3 guaranteed), Power Plant ~2 min of duplicates,
    Route 15 Venonat force (30%), Route 7 (45 s per extra try), Mansion 40% camp; model has `to-pewter`
    Aerodactyl gift (2800 f) but no Pewter-skip/Plants-skip branch rule and no emergency "skip both when
    behind" rule. **Effect: -1 to +3 min per run, plus tail control; the reset rule needs the lever.**
13. **LG-route structure**: deferred catches + Cinnabar-single shop + Mom-heal skip + candy Blastoise
    (before Koga) + Lum Berry (Route 8) + Route 1 Pidgey after Giovanni (Max Repel). None are plan knobs;
    the descent explored fills/addsites but not this shape. **Effect: unknown; the real LG PB is 1:55 slower
    than the FR WR, so structural gains need the pace-gate story (item 14), not the finish time.**

**D. Resets, money, items**

14. **Reset policy.** Real: manip-miss and starter-gate resets (off-timer), Brock/early-death acceptance
    with *no salvage*, E4-door gate (~2:37-2:39 on a 3:25:25 target; reset a 60/60 at 2:39:37), a Koga
    pace gate (FR), Safari-based gate (LG, aspirational), "skip both Pewter+Plants when behind". Model:
    aggr-parameterised splits (`reset_aggr_at`: brock 0.8, koga 0.9, hall-of-fame-1 0.6, indigo-heal 0.5)
    and no E4R1-door-specific rule (since the dex is after HoF1). **Effect: at the current model level
    (no WR-rung finishes) the reset rules cannot be validated; once item 1 is fixed, re-tune with the real
    gates.**
15. **Money/items.** Real: single Cinnabar shop (LG), ~1-3 Poke Balls early then Great/Ultra + Net Ball,
    1 Ultra ~ 2 Full Heals trade, Persim every run, Black Glasses (Lorelei), Lum Berry, X Special x20 +
    X Accuracy (Mega Kick) + Guard Spec, Max Repel/Repel (Safari, Mansion, Cave, Route 1), Max Elixir
    between E4 fights, Clefairy via Game Corner coins (+250 coins LG). Model: `xbuys` (special 4+34),
    `ball great`, money model with `3000` start money and `BALL_PRICE` tiers, Clefairy bought after HoF1 for
    15000 (guess); no Lum/Persim/Black Glasses/Guard Spec/Escape-Rope-bag logic, no coin economy.
    **Effect: a few minutes; mostly the X Special/Accuracy stock (a failed Champ 2 without an X Special is
    explicitly noted).**
16. **Mega Kick as the mid-game damage engine** (75% acc, PP-limited, X Accuracy reassigned between
    fights, "spec strats" fallback costing 1.5-2 min, bought PP restoratives). The model's plan 372 has
    **no Mega Kick anywhere** (grep of `crates/frlg-human/src` and the plan: no `MOVE_MEGA_KICK`; its TMs
    are Mega Punch, Dig, Blizzard, Earthquake, Water Pulse, Fire Blast). Where the TM comes from is
    not stated in the evidence (WR Misty moveset shows it) - find the source in decomp. **Effect: fight
    length and death rate in the Brock -> Surge stretch; high.**
17. **Mewtwo final fights.** Model uses Mewtwo with Water Pulse + Fire Blast (after journal correction;
    Surf illegal). Real: Mewtwo Lv70-73, Shock Wave slot one, Fire Blast, X Special + Guard Spec on Champ 2,
    5 cave encounters ~90 s. Model's rematch chain "exempt from PP" and Cerulean Cave rope variant
    (`rope_cerulean_cave`). **Effect: small if durations are fixed (92-196 s per fight real).**

**E. Smaller / other**

18. **Clemi skip**: credited 140 frames per successful catch (consistent with the 2.0-2.68 s per catch =
    ~2 min over 60 catches; only catches count, not evolutions/gifts). Legality unconfirmed on the console
    board; WR use unknown. Level-up jingle variant unmeasured. **Effect: ~1-2 min, sign confirmed.**
19. **Execution losses**: bonks (8-9 per run, T03 3:40:15), bike dismount, optional-trainer line-of-sight
    (+40 s), menu/bag scripted cursor errors; model: `mistake_rate` 1.0 per 100 tiles with 60-300
    frame penalties. Roughly right in order of magnitude; **~1-3 min per run unmodelled variance.**
20. **Cut-tree deload skip, Rock Tunnel line-of-sight dodging, Select-register, Escape Rope for menu
    consistency**: small saves (tens of seconds each) not in the model. Legality of the tree skip is his
    argument only.
21. **Version**: real routes diverge on Clefairy price (+250 coins LG), Vulpix (LG) vs Growlithe (FR); the
    model's FR-LG difference is a wash (+0.10 h [-0.12, 0.34]) and the LG-tuned fills slightly favour LG.
    Consistent with the runner (LG PB 3:27:20 vs FR WR 3:25:25, "route saves 11 s"). **No gap, but
    FR-specific WR route tests should be re-descended (CHAIN-FR) before deciding.**
22. **Attempt economy realism**: model's EHT sums wall time across attempts with resets; real session has
    regimes (manip drift, fatigue after 6-7 h, 45 attempts / 8.4 h, ~1 past the E4 gate); EHT should be
    reported per good-regime hour and per bad-regime hour, not a single constant.

### Things the model already matches (so not gaps)

Squirtle as carry; lone Blastoise carry through E4R1 with X Specials (WR: Blastoise Lv50-56, 11-20 X
Specials); Abra teleport; Mewtwo (Master Ball) for the rematch E4 (essential per both model and WR);
Fire Blast on Mewtwo (WR video); Fighting Dojo Hitmonlee (LG route); Clemi skip; Lostelle Hypno;
Super Rod fills (LG fishing); candy chains; forced Sevii trip magnitude (~10 min); version a wash.

## Open items for a human / next pass

- Read the nature whitelist and IV thresholds from the `Main97Bot` chat or the IV box frames (the
  only authoritative source); confirm whether Modest is accepted.
- Confirm Clemi-skip legality on the console Round 2 board and whether the 3:25:25 used it.
- The unresolved terms: "Warden candy" source, "turn frame", "bag manip", "Context", Dan/Nob XP targets,
  "13/41" readout, "97.71%", "TOTAL: m", "1:25:22 Safari split".
- A full-run LG VOD and R2-E4 footage are still the highest-value downloads (HOST-REQUESTS.md).
