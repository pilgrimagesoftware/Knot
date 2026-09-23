#!/usr/bin/env bash
#
# Regenerates the panel's turn-in-progress animation from the committed app
# icon: one full plane rotation, assembled into a looping animated WebP.
#
# Not wired into `make`. It needs tools CI does not have, and the asset
# changes roughly never - the point of committing the recipe beside the bytes
# is that the bytes are reproducible, not that they are rebuilt.
#
# WebP rather than GIF because the icon has a soft alpha edge and GIF
# transparency is one bit, which fringes against both themes. WebP costs bytes
# for that: measured at these settings, the GIF is 68KB and the WebP 95KB. If
# `img2webp` genuinely cannot be installed, GIF is the documented fallback - it
# plays through the same `img` element and only looks worse at the edge.
#
# ffmpeg's Homebrew build has no libwebp encoder, so the two halves need two
# tools: ffmpeg rotates the artwork, img2webp (`brew install webp`) assembles
# the frames.
#
# Usage: scripts/make-working-animation.sh

set -euo pipefail

# One revolution in FRAMES steps at DELAY ms each, so a revolution takes
# FRAMES*DELAY = 1488ms. Frame FRAMES would be frame 0 again, so generating
# 0..FRAMES-1 is what makes the loop seamless.
FRAMES=24
DELAY=62
# 3x the 24px slot the panel draws it in, which still clears a 2x display's
# 48 device pixels. Not the 4x app-icon-32.png uses for its 16px slot: the
# rotation gives every frame different alpha, so nothing compresses across
# frames and the file scales with area x frames. 4x at 30 frames measured
# 182KB against the design's ~100KB ceiling; this is 95KB. Drop FRAMES or
# SIZE before accepting a larger file.
SIZE=72
QUALITY=80

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

ICON="crates/knot/assets/icon/icon.png"
OUT="crates/knot/assets/working-knot.webp"

for tool in ffmpeg img2webp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        printf 'make-working-animation: %s is not on PATH.\n' "$tool" >&2
        case "$tool" in
            ffmpeg) printf 'Install it with: brew install ffmpeg\n' >&2 ;;
            img2webp) printf 'Install it with: brew install webp\n' >&2 ;;
        esac
        exit 2
    fi
done

if [ ! -f "$ICON" ]; then
    printf 'make-working-animation: %s not found\n' "$ICON" >&2
    exit 2
fi

frames="$(mktemp -d)"
trap 'rm -rf "$frames"' EXIT

# `format=rgba` before the rotation so `c=none` has an alpha channel to fill
# the corners with; without it the rotation lands on an opaque background.
# `n` is the frame number, so the angle sweeps one full turn across the run.
ffmpeg -y -loglevel error \
    -loop 1 -i "$ICON" \
    -vf "format=rgba,rotate=2*PI*n/${FRAMES}:c=none,scale=${SIZE}:${SIZE}:flags=lanczos" \
    -frames:v "$FRAMES" \
    -start_number 0 \
    "$frames/frame-%03d.png"

# A plain glob rather than `find | mapfile`: macOS ships bash 3.2, and the
# frame names sort correctly as written.
img2webp -loop 0 -lossy -q "$QUALITY" -m 6 -d "$DELAY" "$frames"/frame-*.png -o "$OUT"

printf 'make-working-animation: wrote %s (%s bytes, %s frames at %sms)\n' \
    "$OUT" "$(wc -c < "$OUT" | tr -d ' ')" "$FRAMES" "$DELAY"
