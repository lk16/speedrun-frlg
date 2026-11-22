#!/bin/bash
# feat.sh VIDEO CROP OUT [SS DUR]: per-frame 13 RGB cells (whole game window, 8 bottom-band cells, 4 top-band cells), rgb24 raw
V=$1; CROP=$2; O=$3
EXTRA=""
[ -n "$4" ] && EXTRA="-ss $4 -t $5"
ffmpeg -v error -threads 4 $EXTRA -i "$V" -an -filter_complex "[0:v]crop=$CROP,split=3[a][b][c];[a]scale=1:1:flags=area[a1];[b]crop=iw:ih*0.20:0:ih*0.76,scale=8:1:flags=area[b1];[c]crop=iw:ih*0.20:0:ih*0.05,scale=4:1:flags=area[c1];[a1][b1][c1]hstack=inputs=3,format=rgb24[o]" -map "[o]" -f rawvideo "$O"
