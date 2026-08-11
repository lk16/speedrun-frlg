#!/usr/bin/env bash
#
# Drains the tier-2 verification queue. Host-only: it runs BizHawk, which the sandbox cannot.
#
#   tools/verify-runner.sh            one pass over the queue, then exit
#   tools/verify-runner.sh --watch    keep going, polling every 10s
#   tools/verify-runner.sh --check    preflight only: say whether tier 2 could run at all
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

  [ ${#problems[@]} -eq 0 ] && return 0
  for p in "${problems[@]}"; do warn "$p"; done
  return 1
}

# ------------------------------------------------------------------- one item --
# $1 = id. Always writes $RESULTS/<id>.json and always consumes the queue entry.
run_one() {
  local id="$1"
  local bk2="$QUEUE/$id.bk2" req="$QUEUE/$id.json"
  local expect_frames="" expect_ram="" expect_ilog=""

  if [ -f "$req" ]; then
    expect_frames=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("frames") or "")' "$req" 2>/dev/null)
    expect_ram=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("ram_hash") or "")' "$req" 2>/dev/null)
    expect_ilog=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1])).get("ilog_sha1") or "")' "$req" 2>/dev/null)
  fi

  local work; work=$(mktemp -d)
  local dump="$work/ram.bin" status="$work/status.txt"
  mkdir -p "$USERDATA"

  # EmuHawk exits when the script calls client.exit(); the timeout is for the case where it
  # does not get that far. --userdata keeps its config and savestates out of the deps tree.
  FRLG_VERIFY_DUMP="$dump" FRLG_VERIFY_STATUS="$status" FRLG_VERIFY_FRAMES="${expect_frames:-0}" \
  timeout "$TIMEOUT" "$BIZHAWK/EmuHawkMono.sh" \
    --userdata="$USERDATA" \
    --movie="$bk2" \
    --lua="$(dirname "${BASH_SOURCE[0]}")/verify-runner.lua" \
    "$ROM" >"$work/emuhawk.log" 2>&1
  local rc=$?

  local verdict notes ram=""
  if [ $rc -eq 124 ]; then
    verdict=error
    notes="EmuHawk did not finish within ${TIMEOUT}s -- most likely a modal dialog. Log: $(tail -3 "$work/emuhawk.log" | tr '\n' ' ')"
  elif [ ! -s "$status" ]; then
    verdict=error
    notes="EmuHawk exited ($rc) without the Lua script reporting. Log: $(tail -3 "$work/emuhawk.log" | tr '\n' ' ')"
  else
    # The Lua side writes key=value lines; it is the only thing that knows the movie actually
    # played rather than merely loaded.
    local played; played=$(grep -m1 '^played=' "$status" | cut -d= -f2)
    local frames; frames=$(grep -m1 '^frames=' "$status" | cut -d= -f2)
    ram=$( [ -s "$dump" ] && sha1 "$dump" )
    if [ "$played" != "yes" ]; then
      verdict=error
      notes="the movie loaded but did not play to its end (stopped at frame ${frames:-?})"
    elif [ -n "$expect_ram" ] && [ -n "$ram" ] && [ "$expect_ram" != "$ram" ]; then
      verdict=desync
      notes="replayed $frames frames; EWRAM+IWRAM fingerprint $ram, the sandbox expected $expect_ram"
    elif [ -n "$expect_ram" ] && [ -z "$ram" ]; then
      verdict=error
      notes="replayed $frames frames but produced no memory dump, so nothing was compared"
    else
      verdict=pass
      notes="replayed $frames frames${expect_ram:+; fingerprint matches}"
    fi
  fi

  EXPECT_JSON=$(python3 -c 'import json,sys; print(json.dumps({"ram_hash": sys.argv[1] or None, "ilog_sha1": sys.argv[2] or None}))' "$expect_ram" "$expect_ilog")
  export EXPECT_JSON
  python3 - "$RESULTS/$id.json" "$id" "$verdict" "$notes" "$ram" \
           "$(sha1 "$bk2")" "$(sha1 "$ROM")" "$BIZHAWK_VERSION" <<'PY'
import json, os, sys, datetime
out, id, verdict, notes, ram, bk2, rom, biz = sys.argv[1:9]
expect = json.loads(os.environ["EXPECT_JSON"])
with open(out, "w") as f:
    json.dump({
        "id": id,
        "bk2_sha1": bk2 or None,
        "ilog_sha1": expect["ilog_sha1"],
        "rom_sha1": rom or None,
        "bizhawk_version": biz or None,
        "verdict": verdict,
        "desync_frame": None,
        "ram_hash": ram or None,
        "expected_ram_hash": expect["ram_hash"],
        "finished_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "notes": notes,
    }, f, indent=2, sort_keys=True)
    f.write("\n")
PY

  rm -rf "$work"
  rm -f "$bk2" "$req"
  case "$verdict" in
    pass)   printf '  %s ok  %s %-24s %s\n' "$GREEN" "$OFF" "$id" "$notes" ;;
    desync) printf '  %s !! %s %-24s %s\n' "$YELLOW" "$OFF" "$id" "$notes" ;;
    *)      printf '  %s FAIL%s %-24s %s\n' "$RED" "$OFF" "$id" "$notes" ;;
  esac
}

# ------------------------------------------------------------------------ run --
mkdir -p "$QUEUE" "$RESULTS"
BIZHAWK_VERSION=$(grep -m1 -oE '[0-9]+\.[0-9]+\.[0-9]+' "$DEPS/.resolved/bizhawk" 2>/dev/null || echo unknown)

case "${1:-}" in
  --check)
    if preflight; then say "tier 2 can run: BizHawk $BIZHAWK_VERSION, BIOS ok, ROM ok"; exit 0
    else die "tier 2 cannot run yet (see above)"; fi ;;
  --watch) WATCH=1 ;;
  "") WATCH=0 ;;
  *) die "unknown argument: $1 (try --watch or --check)" ;;
esac

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
