# Synthetic decoder probes

These two fixtures are locally generated solid-blue 32×32, one-frame videos without audio or metadata from users.
They exercise MP4/H.264 and WebM/VP9 in the production decoder readiness probe. No third-party media is included.

Generate with FFmpeg: `-f lavfi -i color=c=blue:size=32x32:rate=1 -frames:v 1 -threads 1`,
using `-c:v libx264 benign.mp4` or `-c:v libvpx-vp9 benign.webm`.
