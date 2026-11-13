# What to download on the host for the next session

Drop everything into `ignored/downloaded/` (gitignored; the sandbox reads it through the
read-only host mount). One directory per VOD is fine too — the pipeline globs. Nothing
here is blocking: the next session ingests whatever exists.

## 1. One-time: enable in-sandbox speech-to-text

```
tools/host-prep.sh          # only the new `whisper` step runs (stamped); then
                            # start the next sandbox fresh so it sees the deps
```
After that, `bin/frlg-transcribe VIDEO` works in-sandbox (txt + srt, video-time
stamps). Untested on a real host yet; it fails loudly if the build or model
download goes wrong. `WHISPER_MODEL=medium.en tools/host-prep.sh --force whisper`
upgrades the model if small.en transcripts look rough.

## 2. VODs, in value order

1. **A finished run on the new LeafGreen route** — ideally the 3:27:20 LG PB, else any
   completed run from the last few weeks. This is the single most valuable file: it
   gives the complete new-route ground truth end to end.
2. **Any session that reaches Elite 4 Round 2** — R2 fight footage is our scarcest data
   (one sample so far).
3. **Streams where the route is explained** (route talk > gameplay). Rough timestamps in
   the filename or a sidecar note help: `...-routetalk-at-1h20m.mp4`.
4. **Runner-up runs** (the 3:25:47 and 3:26:41 runners) — separates "Main97's execution"
   from "the route's cost".
5. Skip: more early-game reset grinding; one stream of it was enough.

## 3. Per Twitch VOD: the chat log (cheap, high value)

Twitch VODs have no downloadable captions, but the chat replay is fetchable and beats
our frame-OCR of the chat column — it contains every Main97Bot message verbatim
(deaths lists, starter IV verdicts, route notes):

```
TwitchDownloaderCLI chatdownload --id <vod-id> -o <name>.chat.json
# or: pip install chat-downloader && chat_downloader <vod-url> --output <name>.chat.json
```

Name it after the video file (`foo.mp4` -> `foo.chat.json`) and the sandbox parses it
with stdlib.

## 4. If any run is mirrored on YouTube

YouTube auto-captions are a free transcript — worth grabbing for the WR run especially:

```
yt-dlp --write-auto-subs --sub-langs en --skip-download -o "<name>" <url>
```

## 5. Optional extras

- A whisper transcript made on the host (`whisper <file> --model small.en`) does the
  same job as item 1's in-sandbox path, if you want transcripts before rebuilding deps.
- speedrun.com category-rules text for FRLG Elite 4 Round 2 (copy-paste into a .txt):
  settles whether the Clemi skip is legal on the console boards — plan 370 leans on it.
