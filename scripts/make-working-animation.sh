#!/usr/bin/env bash
#
# Rebuilds the panel's turn-in-progress animation from the authored source
# animation committed under `images/`.
#
# Not wired into `make`. It needs tools CI does not have, and the asset
# changes roughly never - the point of committing the recipe beside the bytes
# is that the bytes are reproducible, not that they are rebuilt.
#
# The artwork is authored, not generated. An earlier version of this script
# built the animation itself by rotating `assets/icon/icon.png` one full turn;
# the source of truth is now `images/knot-progress-pulse.webp`, so all this
# does is scale that down and re-encode it at a size the panel can embed.
#
# The source ships in two formats. WebP is the one to use: the icon has a soft
# alpha edge, GIF transparency is one bit, and the difference fringes against
# both themes. `images/knot-progress-pulse.gif` is the documented fallback if
# the WebP is ever unavailable - it plays through the same `img` element and
# only looks worse at the edge.
#
# ffmpeg's Homebrew build has no libwebp encoder, so the two halves need two
# tools: ffmpeg decodes and scales the frames, img2webp (`brew install webp`)
# reassembles them.
#
# Usage: scripts/make-working-animation.sh

set -euo pipefail

# The source is 36 frames at 42ms, so a cycle takes 1512ms. FRAME_DELAY has to
# match what the source declares or the rebuilt animation plays at a different
# speed than the artwork was timed for.
FRAME_DELAY=42
# 3x the 24px slot the panel draws it in, which still clears a 2x display's
# 48 device pixels. Not the 4x app-icon-32.png uses for its 16px slot: every
# frame differs, so nothing compresses across frames and the file scales with
# area x frames. At 72px and q80 this measures 111KB for 36 frames - above the
# ~95KB the 24-frame rotation cost, but cheaper per frame (3.1KB against
# 4.0KB). Drop SIZE or QUALITY before accepting a materially larger file.
SIZE=72
QUALITY=80

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

SOURCE="images/knot-progress-pulse.webp"
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

if [ ! -f "$SOURCE" ]; then
    printf 'make-working-animation: %s not found\n' "$SOURCE" >&2
    exit 2
fi

frames="$(mktemp -d)"
trap 'rm -rf "$frames"' EXIT

# ffmpeg decodes the animated WebP to RGBA frames, so the soft alpha edge
# survives the scale; `lanczos` keeps it from muddying at a 7x reduction.
ffmpeg -y -loglevel error \
    -i "$SOURCE" \
    -vf "scale=${SIZE}:${SIZE}:flags=lanczos" \
    -start_number 0 \
    "$frames/frame-%03d.png"

# A plain glob rather than `find | mapfile`: macOS ships bash 3.2, and the
# frame names sort correctly as written.
img2webp -loop 0 -lossy -q "$QUALITY" -m 6 -d "$FRAME_DELAY" "$frames"/frame-*.png -o "$OUT"

printf 'make-working-animation: wrote %s (%s bytes, %s frames at %sms)\n' \
    "$OUT" "$(wc -c < "$OUT" | tr -d ' ')" \
    "$(ls "$frames"/frame-*.png | wc -l | tr -d ' ')" "$FRAME_DELAY"
