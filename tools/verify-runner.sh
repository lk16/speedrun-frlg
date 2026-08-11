#!/usr/bin/env bash
#
# Drains the tier-2 verification queue. Host-only: it runs BizHawk, which the sandbox cannot.
#
#   tools/verify-runner.sh            one pass over the queue, then exit
#   tools/verify-runner.sh --watch    keep going, polling every 10s
#   tools/verify-runner.sh --check    preflight only: say whether tier 2 could run at all
#
# Modifiers, which may be combined with the above:
#
#   --headless    run EmuHawk on a throwaway X server (xvfb-run) instead of the desktop
#   --realtime    replay at 100% with sound and video, the way a person would watch it
#
# The contract, which both sides depend on, is in docs/route.md ("Tier 2"). In short:
#
#   in    $FRLG_ARTIFACTS/verify/queue/<id>.bk2    the movie to replay
#         $FRLG_ARTIFACTS/verify/queue/<id>.json   what the sandbox expects of it (optional)
#   out   $FRLG_ARTIFACTS/verify/results/<id>.json the verdict
#
# A queue nobody drains is worse than no queue -- it makes "queue it and keep working" look like
# progress -- so this always writes a result, including for its own failures, and always removes
# the queue entry. Silence is never a valid answer.

set -uo pipefail

DEPS="${FRLG_DEPS:-${FRLG_DEPS_DIR:-$HOME/.cache/speedrun-frlg/deps}}"
ARTIFACTS="${FRLG_ARTIFACTS:-${FRLG_ARTIFACTS_DIR:-$HOME/.cache/speedrun-frlg/artifacts}}"
BIZHAWK="${BIZHAWK_HOME:-$DEPS/bizhawk}"
ROM="${FRLG_ROM:-$ARTIFACTS/rom/pokefirered.gba}"
QUEUE="$ARTIFACTS/verify/queue"
RESULTS="$ARTIFACTS/verify/results"
BIOS="${FRLG_GBA_BIOS:-$BIZHAWK/Firmware/GBA_bios.rom}"
BIOS_SHA1=300c20df6731a33952ded8c436f7f186d25d3492
# EmuHawk writes config.ini, savestates and a SaveRAM tree next to itself. The deps tree is
# mounted read-only into the sandbox and is rebuilt by host-prep, so keep that churn out of it.
USERDATA="${FRLG_BIZHAWK_USERDATA:-$ARTIFACTS/verify/bizhawk-userdata}"
TIMEOUT="${FRLG_VERIFY_TIMEOUT:-900}"
HEADLESS="${FRLG_VERIFY_HEADLESS:-0}"
REALTIME="${FRLG_VERIFY_REALTIME:-0}"

RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; DIM=$'\033[2m'; OFF=$'\033[0m'
say()  { printf '%s==>%s %s\n' "$GREEN" "$OFF" "$*"; }
warn() { printf '%s !! %s %s\n' "$YELLOW" "$OFF" "$*" >&2; }
die()  { printf '%s !! %s %s\n' "$RED" "$OFF" "$*" >&2; exit 1; }

sha1() { sha1sum "$1" 2>/dev/null | cut -d' ' -f1; }

# ------------------------------------------------------------------ preflight --
# Every one of these is a way for a run to fail as a GUI dialog rather than an exit code, which
# in an unattended runner means "hangs until the timeout kills it". Check them up front and say
# which one is missing.
preflight() {
  local problems=()
  command -v mono >/dev/null || problems+=("mono is not installed (sudo apt install mono-complete)")
  [ -f "$BIZHAWK/EmuHawkMono.sh" ] || problems+=("no BizHawk at $BIZHAWK -- run tools/host-prep.sh")
  [ -f "$ROM" ] || problems+=("no ROM at $ROM")

  # The one that is easy to get wrong. Loading a movie sets DeterministicEmulationRequested, and
  # MGBAHawk's constructor throws MissingFirmwareException("A BIOS is required for deterministic
  # recordings!") when no GBA BIOS is configured -- read out of
  # dll/BizHawk.Emulation.Cores.dll, not guessed. EmuHawk turns that into the Firmware Manager
  # dialog, so without this check the runner would sit at a modal window forever.
  if [ ! -f "$BIOS" ]; then
    problems+=("no GBA BIOS at $BIOS -- tier 2 cannot run deterministically without one
      it must be the World BIOS: sha1 $BIOS_SHA1, 16384 bytes")
  elif [ "$(sha1 "$BIOS")" != "$BIOS_SHA1" ]; then
    problems+=("$BIOS is not the World GBA BIOS (sha1 $(sha1 "$BIOS"), wanted $BIOS_SHA1)")
  fi

  if [ "$HEADLESS" = 1 ] && ! command -v xvfb-run >/dev/null; then
    problems+=("--headless needs xvfb-run, which is not installed (sudo apt install xvfb)")
  fi

  [ ${#problems[@]} -eq 0 ] && return 0
  for p in "${problems[@]}"; do warn "$p"; done
  return 1
}

# --------------------------------------------------------------------- config --
# EmuHawk's own defaults are the ones a person wants at a desk: 100% speed, sound on, every
# frame rendered. A verification replay wants the opposite of all three -- nobody is watching,
# and at 100% the replay costs exactly what the TAS costs (12713 frames is ~3.5 minutes). Its
# config.ini is plain JSON, so the settings are seeded from here rather than clicked in a GUI
# that an unattended runner never opens.
#
# None of these touch emulation. The core is deterministic and the movie drives it frame by
# frame; throttling, audio output and rendering are host-side presentation, which is the
# premise every BizHawk movie already rests on. It is checked rather than assumed: the same
# movie replays to the same EWRAM+IWRAM fingerprint and the same per-frame probe trace fast and
# silent as it did at 100% with sound (docs/journal.md, 2026-08-11).
#
#   Unthrottled          run frames as fast as the host can; ClockThrottle/VSyncThrottle/
#                        SoundThrottle are the three things that would otherwise pace it
#   DispSpeedupFeatures  0 makes MainForm's render path return immediately without touching the
#                        video provider at all (EmuHawk.exe IL, MainForm::UpdateVideo)
#   SoundOutputMethod    3 = ESoundOutputMethod.Dummy (monop on BizHawk.Client.Common.dll), so
#                        no audio device is opened -- which also removes OpenAL from what a
#                        headless run needs
#
# The rest are dialog suppression: every modal window is a hang in an unattended runner.
seed_config() {
  local cfg="$USERDATA/config.ini"
  python3 - "$cfg" "$BIZHAWK_VERSION" "$REALTIME" <<'PY'
import json, os, sys

path, version, realtime = sys.argv[1], sys.argv[2], sys.argv[3] == "1"

# Presentation. Everything here is host-side output, not emulation.
if realtime:
    speed = {
        "ClockThrottle": True, "Unthrottled": False, "VSyncThrottle": False,
        "SoundThrottle": False, "SpeedPercent": 100, "DispSpeedupFeatures": 2,
        "SoundEnabled": True, "SoundOutputMethod": 2, "SoundVolume": 100,
        "DisplayMessages": True,
    }
else:
    speed = {
        "ClockThrottle": False, "Unthrottled": True, "VSyncThrottle": False,
        "SoundThrottle": False, "SpeedPercent": 100, "DispSpeedupFeatures": 0,
        "SoundEnabled": False, "SoundOutputMethod": 3, "SoundVolume": 0,
        "DisplayMessages": False,
    }

# Nothing may wait for a human.
quiet = {
    "PauseWhenMenuActivated": False, "RunInBackground": True, "SingleInstanceMode": False,
    "StartPaused": False, "StartFullscreen": False, "SuppressAskSave": True,
    "UpdateAutoCheckEnabled": False,
}

# An existing config is patched, never replaced: EmuHawk rewrites the whole file on exit, and
# anything it learned (paths, firmware, window state) is worth keeping. A fresh one gets only
# these keys -- the C# Config object supplies its own defaults for every key left out, so a
# partial file is a valid one. LastWrittenFrom is stamped so BizHawk does not read it as
# a config from an older release.
try:
    with open(path) as f:
        cfg = json.load(f)
except (OSError, ValueError):
    cfg = {"LastWrittenFrom": version, "LastWrittenFromDetailed": "Version " + version}

cfg.update(speed)
cfg.update(quiet)

os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "w") as f:
    json.dump(cfg, f, indent=2)
    f.write("\n")
PY
}

# ------------------------------------------------------------------- one item --
# $1 = id. Always writes $RESULTS/<id>.json and always consumes the queue entry.
run_one() {
  local id="$1"
  local bk2="$QUEUE/$id.bk2" req="$QUEUE/$id.json"
  local expect_frames="" expect_ram="" expect_ilog=""
  local trace_file="" trace_domain="" trace_offset=""

  if [ -f "$req" ]; then
    expect_frames=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("frames") or "")' "$req" 2>/dev/null)
    expect_ram=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("ram_hash") or "")' "$req" 2>/dev/null)
    expect_ilog=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("ilog_sha1") or "")' "$req" 2>/dev/null)
    # The per-frame probe trace `frlg route export` queues beside the movie
    # (docs/route.md): lets the Lua turn "desync" into "desync at frame N".
    trace_file=$(python3 -c 'import json,sys; print((json.load(open(sys.argv[1])).get("trace") or {}).get("file") or "")' "$req" 2>/dev/null)
    trace_domain=$(python3 -c 'import json,sys; print((json.load(open(sys.argv[1])).get("trace") or {}).get("domain") or "")' "$req" 2>/dev/null)
    trace_offset=$(python3 -c 'import json,sys; print((json.load(open(sys.argv[1])).get("trace") or {}).get("offset") if (json.load(open(sys.argv[1])).get("trace") or {}).get("offset") is not None else "")' "$req" 2>/dev/null)
  fi
  local trace_path=""
  if [ -n "$trace_file" ] && [ -f "$QUEUE/$trace_file" ]; then
    trace_path="$QUEUE/$trace_file"
  fi

  local work; work=$(mktemp -d)
  local dump="$work/ram.bin" status="$work/status.txt"
  mkdir -p "$USERDATA"

  # The Lua the sandbox can iterate on: an override dropped in artifacts wins over the
  # checked-in copy. This runner executes from the *host* checkout while Lua fixes are
  # authored in the sandbox's clone, so without the override every tweak would need a
  # repo round trip before it could even be tried.
  local lua="$ARTIFACTS/verify/verify-runner.lua"
  if [ -f "$lua" ]; then
    say "using the artifacts-side Lua override: $lua"
  else
    lua="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/verify-runner.lua"
  fi

  seed_config

  # Headless is a throwaway X server, not a hidden window: EmuHawk is WinForms under mono and
  # will not start without a display, whatever it does or does not draw on it. -a takes the
  # first free display number, so two runners do not collide; the screen exists only to be
  # ignored, since DispSpeedupFeatures 0 means nothing is rendered onto it.
  local -a launcher=()
  [ "$HEADLESS" = 1 ] && launcher=(xvfb-run -a -s "-screen 0 640x480x24")

  # EmuHawk exits when the script calls client.exit(); the timeout is for the case where it
  # does not get that far. --config keeps its config out of the deps tree; --userdata is NOT
  # a data directory (it is movie key:value metadata, and a bare path makes the flag parser
  # throw "malformed userdata" and exit 1 -- found the hard way on the first real replay).
  local t0=$SECONDS
  FRLG_VERIFY_DUMP="$dump" FRLG_VERIFY_STATUS="$status" FRLG_VERIFY_FRAMES="${expect_frames:-0}" \
  FRLG_VERIFY_TRACE="$trace_path" FRLG_VERIFY_TRACE_DOMAIN="$trace_domain" \
  FRLG_VERIFY_TRACE_OFFSET="$trace_offset" \
  timeout -k 10 "$TIMEOUT" "${launcher[@]}" "$BIZHAWK/EmuHawkMono.sh" \
    --config="$USERDATA/config.ini" \
    --movie="$bk2" \
    --lua="$lua" \
    "$ROM" >"$work/emuhawk.log" 2>&1
  local rc=$?
  local duration=$((SECONDS - t0))

  # The Lua rewrites the status file as it goes (first probe mismatch, heartbeat, onexit), so
  # even a timed-out or killed run usually leaves the frame it reached and any desync frame it
  # had already found. Parse whatever is there before deciding the verdict.
  local verdict notes ram="" desync_frame=""
  local played="" frames="" desync_got="" desync_want="" probe=""
  if [ -s "$status" ]; then
    played=$(grep -m1 '^played=' "$status" | cut -d= -f2)
    frames=$(grep -m1 '^frames=' "$status" | cut -d= -f2)
    desync_frame=$(grep -m1 '^desync_frame=' "$status" | cut -d= -f2)
    desync_got=$(grep -m1 '^desync_got=' "$status" | cut -d= -f2)
    desync_want=$(grep -m1 '^desync_want=' "$status" | cut -d= -f2)
    if [ -n "$desync_frame" ] && [ "$desync_frame" != "-1" ]; then
      probe="; probe first differs at frame $desync_frame (got $desync_got, tier 1 had $desync_want)"
    elif [ "$desync_frame" = "-1" ]; then
      probe="; the trace compare itself failed in Lua (read API?), no frame number"
      desync_frame=""
    fi
  fi
  if [ $rc -eq 124 ]; then
    verdict=error
    notes="EmuHawk did not finish within ${TIMEOUT}s -- most likely a modal dialog. Last Lua status: frame ${frames:-none}$probe. Log: $(tail -3 "$work/emuhawk.log" | tr '\n' ' ')"
  elif [ ! -s "$status" ]; then
    verdict=error
    notes="EmuHawk exited ($rc) without the Lua script reporting. Log: $(tail -3 "$work/emuhawk.log" | tr '\n' ' ')"
  else
    # The Lua side is the only thing that knows the movie actually played rather than merely
    # loaded.
    ram=$( [ -s "$dump" ] && sha1 "$dump" )
    if [ "$played" != "yes" ]; then
      verdict=error
      notes="the movie loaded but did not play to its end (stopped at frame ${frames:-?})$probe"
    elif [ -n "$expect_ram" ] && [ -n "$ram" ] && [ "$expect_ram" != "$ram" ]; then
      verdict=desync
      notes="replayed $frames frames; EWRAM+IWRAM fingerprint $ram, the sandbox expected $expect_ram$probe"
    elif [ -n "$expect_ram" ] && [ -z "$ram" ]; then
      verdict=error
      notes="replayed $frames frames but produced no memory dump, so nothing was compared$probe"
    elif [ -n "$desync_frame" ]; then
      # The final fingerprint matched (or was never expected) but the probe
      # diverged mid-run: report it, do not call it a pass.
      verdict=desync
      notes="replayed $frames frames$probe"
    else
      verdict=pass
      notes="replayed $frames frames${expect_ram:+; fingerprint matches}${trace_path:+; probe trace matched every frame}"
    fi
  fi

  EXPECT_JSON=$(python3 -c 'import json,sys; print(json.dumps({"ram_hash": sys.argv[1] or None, "ilog_sha1": sys.argv[2] or None}))' "$expect_ram" "$expect_ilog")
  export EXPECT_JSON
  # How the replay was run belongs in the result: a pass at 100% with sound and a pass
  # unthrottled and silent are the same verdict, and the sandbox should be able to see which
  # one it got without asking.
  local mode="fast"; [ "$REALTIME" = 1 ] && mode="realtime"
  [ "$HEADLESS" = 1 ] && mode="$mode+headless"

  python3 - "$RESULTS/$id.json" "$id" "$verdict" "$notes" "$ram" \
           "$(sha1 "$bk2")" "$(sha1 "$ROM")" "$BIZHAWK_VERSION" "$desync_frame" \
           "$duration" "$mode" <<'PY'
import json, os, sys, datetime
out, id, verdict, notes, ram, bk2, rom, biz, desync, duration, mode = sys.argv[1:12]
expect = json.loads(os.environ["EXPECT_JSON"])
with open(out, "w") as f:
    json.dump({
        "id": id,
        "bk2_sha1": bk2 or None,
        "ilog_sha1": expect["ilog_sha1"],
        "rom_sha1": rom or None,
        "bizhawk_version": biz or None,
        "verdict": verdict,
        "desync_frame": int(desync) if desync else None,
        "ram_hash": ram or None,
        "expected_ram_hash": expect["ram_hash"],
        "replay_mode": mode,
        "duration_s": int(duration),
        "finished_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "notes": notes,
    }, f, indent=2, sort_keys=True)
    f.write("\n")
PY

  rm -rf "$work"
  rm -f "$bk2" "$req"
  [ -n "$trace_path" ] && rm -f "$trace_path"
  case "$verdict" in
    pass)   printf '  %s ok  %s %-24s %s%s (%s, %ss)%s\n' "$GREEN" "$OFF" "$id" "$notes" "$DIM" "$mode" "$duration" "$OFF" ;;
    desync) printf '  %s !! %s %-24s %s%s (%s, %ss)%s\n' "$YELLOW" "$OFF" "$id" "$notes" "$DIM" "$mode" "$duration" "$OFF" ;;
    *)      printf '  %s FAIL%s %-24s %s%s (%s, %ss)%s\n' "$RED" "$OFF" "$id" "$notes" "$DIM" "$mode" "$duration" "$OFF" ;;
  esac
}

# ------------------------------------------------------------------------ run --
mkdir -p "$QUEUE" "$RESULTS"
BIZHAWK_VERSION=$(grep -m1 -oE '[0-9]+\.[0-9]+\.[0-9]+' "$DEPS/.resolved/bizhawk" 2>/dev/null || echo unknown)

WATCH=0
CHECK=0
for arg in "$@"; do
  case "$arg" in
    --check)    CHECK=1 ;;
    --watch)    WATCH=1 ;;
    --headless) HEADLESS=1 ;;
    --realtime) REALTIME=1 ;;
    *) die "unknown argument: $arg (try --watch, --check, --headless, --realtime)" ;;
  esac
done

if [ "$CHECK" = 1 ]; then
  if preflight; then
    say "tier 2 can run: BizHawk $BIZHAWK_VERSION, BIOS ok, ROM ok"
    say "replay would be: $( [ "$REALTIME" = 1 ] && echo '100% with sound and video' || echo 'unthrottled, silent, nothing rendered')$( [ "$HEADLESS" = 1 ] && echo ', on an xvfb display' || echo ', on this desktop')"
    exit 0
  else
    die "tier 2 cannot run yet (see above)"
  fi
fi

preflight || die "refusing to run with the problems above -- a request that dies in a dialog
  looks identical to one nobody picked up, which is exactly what this runner exists to prevent."

while :; do
  shopt -s nullglob
  items=("$QUEUE"/*.bk2)
  shopt -u nullglob
  if [ ${#items[@]} -gt 0 ]; then
    say "${#items[@]} queued"
    for f in "${items[@]}"; do run_one "$(basename "$f" .bk2)"; done
  fi
  [ "$WATCH" = 1 ] || break
  sleep 10
done
