#!/usr/bin/env bash
# make-sheets.sh — turn a Main97-layout 1920x1080 stream VOD into timestamped
# contact sheets that a vision model can read cheaply.
#
#   make-sheets.sh VIDEO OUTDIR PASS INTERVAL_SECONDS
#
# PASS is one of:
#   scene    game window + dex bar, 3x4 cells per sheet (what is happening)
#   counter  the "CAUGHT: n  TOTAL: m" bar, full res, 66 cells per sheet
#   timer    bottom of the LiveSplit panel (big timer), 30 cells per sheet
#   chat     right-hand chat column, full res, 3 cells per sheet (expensive —
#            use a long interval or -ss/-t via EXTRA_IN)
#   splits   whole LiveSplit panel, 2x2 cells per sheet (split-table snapshots)
#
# Region geometry is for Main97's overlay as of 2026 (WR video and stream both
# use it): game at 365,30 1190x945 incl. dex bar; counter at 1225,928 330x48;
# timer at 24,950 336x100; chat at 1560,25 356x1040; panel at 24,276 336x780.
# For a different layout, probe single frames first and adjust (see METHOD.md).
#
# Every cell carries its video timestamp (HH:MM:SS), burned in top-left.
# Decoding is keyframe-only (-discard nokey): a frame can show content up to
# ~2 s older than its label, which is fine at the intervals we use (>= 30 s).
#
# EXTRA_IN can hold extra ffmpeg input options, e.g. EXTRA_IN="-ss 3600 -t 1800"
# to window a pass; timestamps stay relative to the window start then — note
# the offset in the sheet directory name to avoid confusion.
set -euo pipefail

VIDEO=$1; OUT=$2; PASS=$3; IVL=$4
FONT=/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf
TS="drawtext=fontfile=$FONT:text='%{pts\\:hms}':x=2:y=2:fontsize=16:fontcolor=yellow:box=1:boxcolor=black@0.6"
mkdir -p "$OUT"

case $PASS in
  scene)
    VF="fps=1/$IVL,crop=1190:945:365:30,scale=420:-2,$TS,tile=3x4"
    EXT=jpg; QUAL=(-q:v 3) ;;
  counter)
    VF="fps=1/$IVL,crop=330:48:1225:928,pad=410:48:80:0,drawtext=fontfile=$FONT:text='%{pts\\:hms}':x=2:y=16:fontsize=15:fontcolor=yellow,tile=3x22"
    EXT=png; QUAL=() ;;
  timer)
    VF="fps=1/$IVL,crop=336:100:24:950,pad=416:100:80:0,drawtext=fontfile=$FONT:text='%{pts\\:hms}':x=2:y=40:fontsize=15:fontcolor=yellow,tile=3x10"
    EXT=png; QUAL=() ;;
  chat)
    VF="fps=1/$IVL,crop=356:1040:1560:25,$TS,tile=3x1"
    EXT=png; QUAL=() ;;
  splits)
    VF="fps=1/$IVL,crop=336:780:24:276,$TS,tile=2x2"
    EXT=png; QUAL=() ;;
  *) echo "unknown pass: $PASS" >&2; exit 2 ;;
esac

# shellcheck disable=SC2086
ffmpeg -v error -y -discard nokey ${EXTRA_IN:-} -i "$VIDEO" \
  -vf "$VF" -vsync vfr "${QUAL[@]}" "$OUT/${PASS}-%03d.$EXT"
echo "$OUT: $(ls "$OUT" | wc -l) sheets ($PASS every ${IVL}s)"
