//! Unit tests for [`super`]'s embedded assets.
//!
//! These read the WebP container directly rather than decoding it. The
//! question is not what the animation looks like - that is the artwork's
//! business - but whether the bytes still declare themselves *animated*,
//! which is the one property the panel silently depends on and no other
//! check covers. `img` renders a still WebP perfectly happily: it draws
//! frame 0 and stops, which looks exactly like an icon that loaded fine.
//! The gate is green either way, so the failure reaches a human only if
//! someone happens to watch a turn run.
//!
//! `scripts/make-working-animation.sh` is the path that would break it -
//! an ffmpeg or img2webp invocation that drops to a single frame, or a
//! source file replaced with a still image.

use super::WORKING_KNOT_WEBP;

/// Offset of the 4-byte chunk FourCC that follows the 12-byte RIFF header.
const FIRST_CHUNK: usize = 12;

/// Walks the RIFF chunk list, returning each chunk's FourCC and payload.
///
/// WebP chunks are `FourCC` + little-endian `u32` size + payload, padded to
/// an even length. Returning borrowed slices keeps this allocation-free and
/// lets a caller read a payload's own fields.
fn chunks(bytes: &[u8]) -> Vec<(&[u8], &[u8])> {
    let mut found = Vec::new();
    let mut at = FIRST_CHUNK;
    while at + 8 <= bytes.len() {
        let fourcc = &bytes[at..at + 4];
        let size = u32::from_le_bytes([bytes[at + 4], bytes[at + 5], bytes[at + 6], bytes[at + 7]])
                   as usize;
        let start = at + 8;
        let end = match start.checked_add(size) {
            Some(end) if end <= bytes.len() => end,
            _ => break,
        };
        found.push((fourcc, &bytes[start..end]));
        at = end + (size & 1);
    }
    found
}

#[test]
fn working_knot_asset_is_a_riff_webp() {
    assert!(WORKING_KNOT_WEBP.len() > 12,
            "the embedded asset is too short to be a WebP");
    assert_eq!(&WORKING_KNOT_WEBP[0..4],
               b"RIFF",
               "the embedded asset is not a RIFF container");
    assert_eq!(&WORKING_KNOT_WEBP[8..12],
               b"WEBP",
               "the embedded asset is not a WebP");
}

#[test]
fn working_knot_asset_declares_itself_animated() {
    // The animation bit lives in VP8X's first byte. Without VP8X the file
    // is a plain still WebP, which is the regression this guards: GPUI
    // asks `WebPDecoder::has_animation` and holds frame 0 when it says no.
    let chunks = chunks(WORKING_KNOT_WEBP);
    let vp8x = chunks.iter()
                     .find(|(fourcc, _)| *fourcc == b"VP8X")
                     .map(|(_, payload)| *payload)
                     .expect("the embedded asset has no VP8X chunk, so it is a still image");
    assert!(!vp8x.is_empty(), "the VP8X chunk carries no flags");
    const ANIMATION_FLAG: u8 = 0b0000_0010;
    assert!(vp8x[0] & ANIMATION_FLAG != 0,
            "the embedded asset's VP8X chunk does not set the animation flag");
}

#[test]
fn working_knot_asset_has_more_than_one_frame() {
    // One ANMF per frame. A single-frame animation would satisfy the flag
    // above and still render as a still image.
    let frames = chunks(WORKING_KNOT_WEBP).iter()
                                          .filter(|(fourcc, _)| *fourcc == b"ANMF")
                                          .count();
    assert!(frames > 1,
            "the embedded asset has {frames} frame(s); it would render as a still image");
}

#[test]
fn working_knot_asset_loops_forever() {
    // Loop count 0 means infinite. Any other value leaves the indicator
    // frozen partway through a turn that outlasts the animation.
    let anim = chunks(WORKING_KNOT_WEBP).iter()
                                        .find(|(fourcc, _)| *fourcc == b"ANIM")
                                        .map(|(_, payload)| *payload)
                                        .expect("the embedded asset has no ANIM chunk");
    assert!(anim.len() >= 6, "the ANIM chunk is truncated");
    let loop_count = u16::from_le_bytes([anim[4], anim[5]]);
    assert_eq!(loop_count, 0,
               "the embedded asset loops {loop_count} time(s) rather than forever");
}
