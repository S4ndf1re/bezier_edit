#!/bin/bash
for f in *.mkv; do
  echo "$f"
  ffmpeg -i "$f" -c:v libx264 -c:a aac "${f%.mkv}.mp4"
done
