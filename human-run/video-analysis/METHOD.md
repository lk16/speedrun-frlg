# Analyzing run videos without re-watching them

External video evidence (WR VODs, streams) lands on the host in
`ignored/downloaded/` (gitignored; inside the sandbox it is only reachable via
the read-only host mount `/run/sandbox/source/ignored/downloaded/`). Videos are
huge and the sandbox dies, so the rule is: **extract once, commit the findings
here, never re-open the video for a question a committed file already answers.**
One findings file per video, named after the video file, listing what was
extracted, at what sampling, and what was *not* extracted (so the next session
knows what a re-pass could still add).

Findings from videos are **external claims**, like everything in `docs/tricks/`:
they say what the runner did, never why the game behaves that way. Routing
claims derived from them still need a `decompiled/` citation before a plan may
rely on the mechanism.

## Pipeline (what worked, 2026-11-13)

1. **Probe single frames first** (`ffmpeg -ss T -i V -frames:v 1`), read them
   directly, and map the overlay layout: where the splits panel, timer, dex
   counter, chat column sit. Do not skip this — all later crops depend on it,
   and a new overlay version moves regions.
2. **One frame can be worth the whole video.** A LiveSplit panel after the last
   split carries every actual split time of the run; the pre-run frame carries
   the comparison columns (old PB/WR routes). Grab those before any mass pass.
3. **Mass-extract contact sheets** with `tools/make-sheets.sh` (regions are
   documented in the script header). Keyframe-only decode (`-discard nokey`)
   scans an 8 h 22 GB VOD in ~3 min per pass, so passes are cheap; run them
   concurrently in the background. Every cell has its video timestamp burned in.
   Keep each sheet near ~1.2 MP — the vision input downscales to roughly that,
   and tiling tighter makes text unreadable.
4. **Fan the sheets out to cheap subagents** (Haiku, one agent per handful of
   sheets) with a fixed per-cell transcription schema, writing JSONL to
   scratchpad; verify a sample of cells yourself, then synthesize. Rough cost:
   a sheet is ~1.5k image tokens, so a whole stream is a few hundred k tokens
   of Haiku, not of the main model.
5. **Targeted second passes** (chat column, splits panel around one event) only
   where the coarse timeline says something happened; `EXTRA_IN="-ss .. -t .."`
   windows a pass.
6. **Commit**: findings file(s), anything added to the tools, and a journal
   entry. The sheets and JSONL stay in scratchpad and die with the session.

## What there is no tool for

- **Audio**: no speech-to-text exists in the sandbox (stdlib Python, no models,
  no network). Runner commentary is lost unless it shows up in chat text. If a
  video's main value is spoken explanation, say so in the findings file and ask
  the host for a transcript instead of burning sheets on it.
- **OCR**: none scriptable; reading text is what the vision subagents are for.

## Overlay facts for Main97's layout (both 2026 videos)

1920x1080; game window 365,30 1190x795 with his dex tracker bar directly under
it (planned species grid, "CAUGHT: n TOTAL: m" at its right edge, readable in
scene cells); LiveSplit panel 24,276 336x780 with three columns (two comparisons
+ `Time`), attempt counter top-right, big timer at the bottom; starter IV box
1340,985 220x72 showing HP/ATK/DEF/SPA/SPD/SPE IVs, nature, and the seed FRAME;
Twitch chat 1560,25 356x1040 — `Main97Bot` posts structured messages worth
grepping visually: per-starter IV/nature verdicts ("not runnable"), `!deaths`
lists (where today's attempts died), `!sqroll` etc.
