# Tasks

## 1. Author the animation

- [x] 1.1 Install the WebP tools (`brew install webp`) and confirm `img2webp`
      runs; if it cannot be installed, take the GIF fallback in design.md and
      say so in the script's header comment
- [x] 1.2 Write `scripts/make-working-animation.sh`: rotate
      `crates/knot/assets/icon/icon.png` through one full turn with `ffmpeg`
      (`rotate=...:c=none` to keep alpha, scaled to 72×72), assemble the frames
      into a looping animated WebP, and write it to
      `crates/knot/assets/working-knot.webp`. The script checks for its tools
      first and exits with a message naming the missing one; verify by running
      it from a clean checkout of the frames directory
- [x] 1.3 Run the script and check the result: full alpha, one full revolution
      with no visible seam at the loop point, ~1.5s per revolution, and under
      100KB; verify with `ffprobe` for frame count and dimensions and by opening
      the file
- [x] 1.4 Commit the asset and the script together, so the bytes and the recipe
      that produced them land in the same commit

## 2. Load and render it

- [x] 2.1 Add the asset's loading site to `crates/knot/src/app_support.rs` beside
      `app_titlebar_icon` — `include_bytes!` plus a `KnotAssets` asset source
      serving it at `working-knot.webp`, drawn by `img(WORKING_KNOT_PATH).id(..)`
      in a 24×24 box fixed in both dimensions — with a comment saying GPUI's
      `img` supplies the frame timing, the reduced-motion behavior and the
      repaint scheduling; verify with `make build`.
      **Not** `Image::from_bytes(ImageFormat::Webp, ..)` as first planned: that
      source decodes through `Image::to_image_data`, which sends every format
      but GIF to `decode_static_image` and yields one frame. Only the resource
      loader behind `img("<path>")` asks `WebPDecoder::has_animation`. The
      `.id(..)` is load-bearing for the same reason — `img` keeps the frame
      index in element state and gates both the advance and
      `request_animation_frame` on the element having a global id
- [x] 2.2 Register `KnotAssets` in `crates/knot/src/app_bootstrap.rs` in place of
      the bare `AllAssets`, delegating every other path to it, so the asset
      reaches `img` by the animating path; verify with `make build`
- [x] 2.3 Point the `PanelRow::Working` arm in
      `crates/knot/src/panel_view/render.rs` at the new element and drop its
      `working_indicator` call and import; verify with `make lint`
- [x] 2.4 Update the module doc on `crates/knot/src/working_indicator.rs` to
      record that the panel no longer calls it and why, pointing at the
      `working-indicator` spec; verify with `make lint`

## 3. Check nothing else moved

- [x] 3.1 Confirm the dashboard card still renders the braille spinner across
      all five states and that `spinner_repaint_due`,
      `WORKING_INDICATOR_MIN_REPAINT` and `last_spinner_frame` are untouched;
      verify with `cargo test -p knot working_indicator` and by watching a
      dashboard with a working agent.
      3/3 tests pass, the diff touches none of `consts.rs`,
      `workspace_window/window.rs`, `open.rs` or `repaint.rs`, and the live
      dashboard was watched in a debug build
- [x] 3.2 Confirm the panel's row bookkeeping is unchanged — `row_count` still
      counts `turn_active` and `row_at` still yields `Working` last; verify with
      `cargo test -p knot panel_view` (26/26 pass)

## 4. Walk the spec

- [x] 4.1 Run a turn and confirm the animated icon appears as the last row,
      animates continuously through a quiet stretch with no output, and is gone
      once the turn ends
- [x] 4.2 Confirm the indicator sits below a pending permission prompt while one
      is awaiting a decision
- [x] 4.3 Turn on Reduce Motion in System Settings > Accessibility > Display and
      confirm the row is still present, shows the icon held still, and occupies
      the same space as when animating
- [x] 4.4 Run `make` and confirm the whole gate passes — `fmt-check`,
      `size-check`, `clippy -D warnings`, tests, build
