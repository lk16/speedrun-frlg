# 2026-11-22: S0 + S1, the overlay renderer, and the first review video

Branch `human-run-play` (worktree agent). This replaces the renderer the 11-21 sandbox lost.

## S0 (3c199775)

ffmpeg 8.0.1 in the image has `libx264`, `aac`, and the `ass`, `subtitles` and `drawtext` filters;
fontconfig sees DejaVu Sans Mono. `bin/frlg-artifacts-gc` gets a `videos` cap of 8 GiB. The hard
ceiling went from 20 to 28 GiB, because the caps now sum to 22. The 10 s test clip from the 11-21
sandbox (`videos/s0-plumbing-test.mp4`) was not re-rendered.

## S1 (34cd72c4, 2d726074)

`frlg video --overlay events.jsonl --input run.ilog [--plan plan.md] [-o out.mp4]`. The new
module `crates/frlg-route/src/overlay.rs` holds the panels; `video.rs` gains `Format::Review`
(x264 CRF 20 `faster`, yuv420p, AAC 160k) and the overlay filter chain. There is still one
renderer: the same two replays, audio capture and ffmpeg pipe. Overlay mode skips the tier-2
gate and writes no description. How to run it: `human-run/VIDEO.md`.

- Events are parsed with serde_json in frlg-route (`{frame,t,kind,text,nums}`) and stable-sorted
  by frame (the log has a checkpoint written before a split of a lower frame number).
  Duplicate splits (same name and time; Giovanni 1 is logged twice) become one row.
- The dex grid comes from the first `nums.dex_planned`; the plan's `SPECIES_*` tokens are the
  fallback. A species is lit by `catch` events that carry `species` plus `caught` or `gift`, and
  by the starter event. `CAUGHT n` is the larger of the in-game `caught` count and the number of
  distinct species lit.
- The chat is wrapped in Rust at 38 columns, continuation lines indented under the text with
  `\h`, and keeps 65 lines. A message scrolls off as a whole. Tests check that no line is over
  38 characters, that the indent is right and that no text is lost.
- The timer is `drawtext` `%{pts:hms:-t0}`, right-aligned at x=336 and enabled from the New Game
  frame. It shows dashes before that and turns green, frozen at the final time, over the tail.
- **Speed.** Panel backgrounds drawn as ASS `\p1` boxes (or `drawbox`) halved the encode rate
  (70 against 133 fps, measured on a synthetic source). The canvas is now built from nested
  `pad`s plus four thin `drawbox` gutter strips, which looks the same at twice the speed.

**Check.** I first rendered the first 20000 frames and then the first 66000 (through Brock), on
the real run rather than a hand-written log over `defeat-brock`, and looked at stills. Before New
Game the timer shows dashes and the chat stamps read `[--:--]`. At 5:19 the timer matches the
newest chat stamps. The 66000-frame preview at 18:04 shows Brock 0:17:53 / 0:14:36 / +3:17,
matching the log's +197 s, and CAUGHT 4 / 60 with SQUIRTLE, PIDGEY and WEEDLE lit at their
times. No line overflows the chat column. The first still showed the timer overflowing the left
column (pts `hms` pads a sign column), which right-aligning fixed. The second showed an orphaned
continuation line at the top of the chat, fixed in 2d726074.

## The review video

`$FRLG_ARTIFACTS/videos/1122-lgdescent-s11-giovanni1.mp4`, with its panels in
`1122-lgdescent-s11-giovanni1.ass` next to it. It is 854 MiB (895883649 bytes) at 1920x1080,
h264 yuv420p + AAC, 331173 frames (331053 run + 120 tail), 5544.7 s. The render took 3384 s
(56 min) wall on 16 cores. Sound was captured at 65536 Hz with 3 rate changes. On the final still
the timer is frozen at 1:32:12.025, matching the Giovanni 1 split (5532.02 s). The splits table
reads Brock +3:17, Misty +12:02, Bill +15:33, Bike +20:17, Rock Tunnel +26:51, Giovanni 1
+31:18. CAUGHT 5 / 60 (Squirtle, Pidgey, Weedle, Paras, Eevee) and rollbacks 4. After the
render, `bin/frlg-artifacts-gc` reports videos at 2.4 of 8 GiB.

## Next

- The events give no dex credit for an evolution (Wartortle, for example). The controller should
  log those as `catch` events with `species` and `caught`.
- For a 4-hour run, expect about 2.5 h of render time and about 2.2 GB. If that is too slow, try
  preset `veryfast` (the chat blend in libass is now the main cost, so the preset hardly
  matters) or fewer chat lines.
- The left column is empty between the splits table and the plan line until about 45 splits
  exist. A forecast line (running total against 4:23) would fit there once the controller logs
  `model-vs-actual`.
