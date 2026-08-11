#!/usr/bin/env bash
#
# Regenerates route/template.bk2 and reads it back. Host-only: it needs mono and the BizHawk
# tree that tools/host-prep.sh downloads, neither of which exists in the sandbox.
#
#   tools/bk2-template.sh          regenerate route/template.bk2, then read it back
#   tools/bk2-template.sh --check  read the committed template back, change nothing
#
# Why this exists instead of "record a one-frame movie in BizHawk and commit it": the two facts
# the template carries -- the Input Log column order and the mGBA SyncSettings blob -- come out
# of compiled CIL, and a hand-recorded movie carries whatever settings that machine's config.ini
# happened to hold. This asks the shipped assemblies directly and writes the file with BizHawk's
# own Bk2Movie.Write, so the answer is reproducible and re-derivable when BizHawk moves.
#
# It never starts EmuHawk. Loading a ROM into the mGBA core for a movie sets
# DeterministicEmulationRequested, and MGBAHawk's constructor throws MissingFirmwareException
# ("A BIOS is required for deterministic recordings!") when no GBA BIOS is configured -- which
# surfaces as the Firmware Manager dialog, not as an exit code. Everything here works off static
# members and the movie serialiser, so no core is ever instantiated.

set -euo pipefail

REPO="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
DEPS="${FRLG_DEPS:-${FRLG_DEPS_DIR:-$HOME/.cache/speedrun-frlg/deps}}"
ARTIFACTS="${FRLG_ARTIFACTS:-${FRLG_ARTIFACTS_DIR:-$HOME/.cache/speedrun-frlg/artifacts}}"
DLL="${BIZHAWK_HOME:-$DEPS/bizhawk}/dll"
ROM="${FRLG_ROM:-$ARTIFACTS/rom/pokefirered.gba}"
OUT="$REPO/route/template.bk2"

RED=$'\033[31m'; GREEN=$'\033[32m'; DIM=$'\033[2m'; OFF=$'\033[0m'
die() { printf '%s !! %s %s\n' "$RED" "$OFF" "$*" >&2; exit 1; }

command -v mono >/dev/null || die "mono is not installed; this script only runs on the host.
  sudo apt install mono-complete"
command -v mcs >/dev/null || die "mcs (the mono C# compiler) is missing from mono-complete."
[ -d "$DLL" ] || die "no BizHawk at $DLL -- run tools/host-prep.sh first."

BUILD="$(mktemp -d)"; trap 'rm -rf "$BUILD"' EXIT
mcs -langversion:latest -out:"$BUILD/template.exe" "$REPO/tools/bk2-template/Template.cs" \
  || die "could not compile tools/bk2-template/Template.cs"

# The mGBA core's static constructor dlopens libmgba.dll.so out of the same directory.
export LD_LIBRARY_PATH="$DLL${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"

if [ "${1:-}" != "--check" ]; then
  [ -f "$ROM" ] || die "no ROM at $ROM -- build it, or set FRLG_ROM."
  printf '%s==>%s writing %s\n' "$GREEN" "$OFF" "$OUT"
  mono "$BUILD/template.exe" write "$DLL" "$OUT" "$ROM"
fi

[ -f "$OUT" ] || die "$OUT does not exist; run without --check to write it."
printf '\n%s==>%s reading it back through BizHawk'"'"'s own loader\n' "$GREEN" "$OFF"
mono "$BUILD/template.exe" read "$DLL" "$OUT"
printf '\n%ssha1 %s%s\n' "$DIM" "$(sha1sum "$OUT" | cut -d' ' -f1)" "$OFF"
