# Rendering a review video

The review video is one played attempt (an `.ilog` from power-on) with its decision log
(`events.jsonl`) burned in as panels. Layout and purpose: `PROMPT.md`, "Next phase: a playing
controller and a review video". Code: `crates/frlg-route/src/overlay.rs` (panels, as an ASS track)
and `crates/frlg-route/src/video.rs` (replay + ffmpeg), driven by `frlg video --overlay`.

```sh
cargo build --release
R=$FRLG_ARTIFACTS/runs/human-play/1122-lgdescent-s11
target/release/frlg video \
  --rom $FRLG_ARTIFACTS/rom/pokeleafgreen.gba \
  --input $R/run.ilog --overlay $R/events.jsonl \
  --plan human-run/plans/1122-lgdescent.md \
  -o $FRLG_ARTIFACTS/videos/1122-lgdescent-s11-giovanni1.mp4
bin/frlg-artifacts-gc
```

- `--overlay` needs `--input`. It bypasses the tier-2 gate of the publishable `frlg video` and
  writes no description: a review video is not a publication.
- `--rom` defaults to the ROM whose sha1 the log names (both versions live in
  `$FRLG_ARTIFACTS/rom`); the log's sha1 is checked either way.
- `-o` defaults to `$FRLG_ARTIFACTS/videos/<run dir name>.mp4`. **Never write into the repo or
  `ignored/`**: `$FRLG_ARTIFACTS/videos/` is on the host disk and has its own 8 GiB cap in
  `bin/frlg-artifacts-gc`. The panels land next to the video as `<name>.ass` (a few MB); they are
  the overlay's exact content and can be inspected without the video.
- `--preview-frames N` renders only the first N frames (the timer freezes at frame N). Use it to
  check a layout change, then pull stills:
  `ffmpeg -ss 330 -i out.mp4 -frames:v 1 still.png`.
- `--tail-frames` (default 120) holds the last frame; the timer shows the frozen final time in
  green over it.

## What is on screen

| Region | Content | Source in `events.jsonl` |
| --- | --- | --- |
| Game 360,16 1200x800 | 5x nearest-neighbour | the replay |
| Left 16,16 328x1048 | run timer (`drawtext`, pts offset so New Game reads 0); splits table (actual, model p50, delta, newest last); plan / attempt / seed / rollback count; starter box | the event with `t == 0`; `split` events' `nums.name/actual/p50`; `nums.attempt/seed`; `rollback` events; the event with `nums.starter` (`nature`, `ivs`, `resets`, `off_timer`) |
| Dex 360,832 1200x232 | the planned species grid, lit with the run time when obtained; `CAUGHT n / N` | the first `nums.dex_planned` (fallback: every `SPECIES_*` in `--plan`); `catch` events with `nums.species` plus `nums.caught` or `nums.gift`; the starter |
| Chat 1576,16 328x1048 | every event, `[h:mm:ss]` run time (`[--:--]` before New Game), wrapped at 38 columns with continuation lines under the text, 65 lines, newest at the bottom, coloured by kind | all events |

Chat colours: split green, decision grey, battle light blue, catch gold, encounter teal, warn
orange, reset-verdict magenta, rollback red, model-vs-actual violet.

## Encoding

x264 CRF 20, preset `faster`, yuv420p, AAC 160k, `+faststart`. The canvas is built with `pad`
filters (panel boxes included) rather than drawn: an ASS or `drawbox` fill of the panels is a
blend over ~1 M pixels per frame and halved the rate. Measured on 16 cores with the real 1122 run
(331053 frames, 1:32:25 of video): 3384 s wall for both replays and the encode (~98 fps overall),
854 MiB, panels 1.9 MB (`journal/2026-11-22-12-45-overlay-renderer.md`). A four-hour run should
come to ~2.5 h and ~2.2 GB.

Needs ffmpeg with libx264, libass and drawtext (libfreetype), and DejaVu Sans Mono (the timer
uses `/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf` directly). Check with
`ffmpeg -filters | grep -E " (ass|drawtext) "` and `fc-list | grep "DejaVu Sans Mono"`.
