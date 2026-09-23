# Proposal

## Why

While an agent is answering, the conversation shows a 12×16 braille spinner on a
row of its own — a terminal idiom borrowed from the dashboard card's per-agent
state mark, which is a different job. The card's mark has to distinguish four
states at a glance in a dense grid; the conversation's row has exactly one thing
to say, that this turn is still running, and it says it with a character that
reads as a rendering artifact at the size it is drawn. The panel gives that away
in its own code: it calls the shared renderer with the state pinned to
`(Running, true)`, so four of the five states it can draw are unreachable there.

An animated Knot icon says the same thing in the app's own voice, at a size a
person actually notices, and gives the conversation a mark of its own instead of
a borrowed one.

## What Changes

- The conversation's turn-in-progress row shows an animated version of the Knot
  application icon in place of the braille spinner.
- A new animated asset is authored as part of this change — the repository has
  only still icons today — and committed alongside the existing
  `assets/app-icon-32.png`.
- The animation plays through the toolkit's own animated-image support, so it
  respects the system's reduced-motion setting and stops while the window is
  inactive without the app scheduling anything itself.
- The dashboard's agent card keeps the braille spinner unchanged. It is a
  different indicator doing a different job, and this change says so in the
  contract rather than leaving the divergence to look like drift.

Non-goals:

- The dashboard card's indicator, its four states, and the repaint poll that
  drives it (`spinner_repaint_due`). None of them change.
- Replacing the application icon, or adding an animated icon anywhere else — the
  title bars and the About window keep the still one.
- Adding text beside the indicator. The row is a like-for-like replacement, not
  a new status line.
- A general animation facility for the app. This change uses what the toolkit
  already provides and introduces no animation framework of its own.

## Capabilities

### New Capabilities

None. Both surfaces involved already have capabilities.

### Modified Capabilities

- `acp-panel-ui`: gains a requirement for the conversation's turn-in-progress
  indicator — that the row exists while a turn is active, what it shows, and how
  it behaves under reduced motion. The conversation's working row is in the code
  today (`PanelRow::Working`) but appears in no spec, which is why it ended up
  borrowing the dashboard's mark by default rather than by decision.
- `working-indicator`: its "one shared implementation" requirement is amended to
  say the conversation's turn indicator is deliberately not this indicator, so a
  later reader does not reunify them as though the split were an accident.

## Impact

- New binary asset under `crates/knot/assets/` — an animated WebP, chosen over
  GIF because the icon has a soft alpha edge and GIF transparency is one bit.
  Authored from the existing `assets/icon/icon.png` (512×512); `ffmpeg` is the
  only tool needed and is already on the machine.
- `crates/knot/src/panel_view/render.rs` — the `PanelRow::Working` arm stops
  calling `working_indicator::render`.
- `crates/knot/src/app_support.rs` — a sibling to `app_titlebar_icon` that
  builds the animated image, so the asset has one loading site, plus the
  asset source that serves its bytes.
- `crates/knot/src/app_bootstrap.rs` — registers that asset source in place of
  the bare `AllAssets`, delegating every other path to it. Not foreseen when
  this was written: `img` only decodes an animated WebP as more than one frame
  when the bytes arrive as an embedded resource path, not as an `Arc<Image>`.
- `crates/knot/src/working_indicator.rs` — unchanged in behavior; its module doc
  gains the note that the panel no longer calls it.
- No new crate dependency: `gpui`'s `img` element already decodes animated WebP,
  honors per-frame delays, skips animation under `reduce_motion`, and requests
  its own animation frames.
