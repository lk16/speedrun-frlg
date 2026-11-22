#!/bin/bash
# nfw-sheet.sh START IVL OUT [COLS] [ROWS] [W]: COLS x ROWS cells from video second START every IVL s
S=$1; I=$2; O=$3; C=${4:-6}; R=${5:-5}; W=${6:-300}
D=$(python3 -c "print($I*$C*$R)")
FONT=/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf
ffmpeg -v error -y -ss $S -t $D -i /run/sandbox/source/ignored/downloaded/frlgr2-wr-3.25.25-main97.mp4 -vf "fps=1/$I,crop=1190:795:365:30,scale=$W:-2,drawtext=fontfile=$FONT:text='%{eif\:t+$S\:d}':x=2:y=2:fontsize=18:fontcolor=yellow:box=1:boxcolor=black@0.7,tile=${C}x${R}" -frames:v 1 -q:v 4 $O
