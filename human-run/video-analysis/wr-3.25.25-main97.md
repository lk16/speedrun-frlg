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
