#!/usr/bin/env bash
# Extracts the parser's text regions from a gameplay recording at 5 fps.
# Only small crops are written (never full frames):
#   fixtures/frames/<name>/{exp,level,notif}/NNNN.png   frame N is at (N-1)/5 s
#
# Usage: scripts/extract-crops.sh <name> <start_s> <end_s> <exp> <level> <notif>
# where each region is ffmpeg crop syntax W:H:X:Y in video pixels.
# grinding-01 (0–122 s, before the window moves):
#   scripts/extract-crops.sh grinding-01 0 122 200:27:2141:1882 60:45:1305:1890 525:255:2850:1400
set -euo pipefail

name=$1 start=$2 end=$3 exp=$4 level=$5 notif=$6
video="fixtures/video/$name.mov"
out="fixtures/frames/$name"
mkdir -p "$out/exp" "$out/level" "$out/notif"

ffmpeg -v error -ss "$start" -to "$end" -i "$video" -filter_complex \
  "[0:v]fps=5,split=3[a][b][c];[a]crop=$exp[e];[b]crop=$level[l];[c]crop=$notif[n]" \
  -map '[e]' "$out/exp/%04d.png" -map '[l]' "$out/level/%04d.png" -map '[n]' "$out/notif/%04d.png"

echo "wrote $(find "$out/exp" -name '*.png' | wc -l | tr -d ' ') frames to $out"
