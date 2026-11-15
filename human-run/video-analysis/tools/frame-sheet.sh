#!/bin/bash
# sheet.sh VIDEO START DUR IVL OUT [W] [COLS] [ROWS] [CROP]
V=$1; S=$2; D=$3; I=$4; O=$5; W=${6:-640}; C=${7:-2}; R=${8:-3}; CROP=${9:-1190:795:365:30}
FONT=/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf
ffmpeg -v error -y -ss $S -t $D -i "$V" -vf "fps=1/$I,crop=$CROP,scale=$W:-2,drawtext=fontfile=$FONT:text='%{pts\:hms}+$S':x=2:y=2:fontsize=16:fontcolor=yellow:box=1:boxcolor=black@0.6,tile=${C}x${R}" -frames:v 1 -q:v 3 $O
