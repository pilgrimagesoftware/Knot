# Proposal

## Why

Knot is a macOS-first desktop app that does not look like a macOS app. The
UI hardcodes a web-born palette - `rgb(0x2563EB)` prompt bubbles,
`rgb(0x3B82F6)` accent buttons, `rgb(0x1E1E1E)`/`rgb(0x333333)` card chrome -
instead of following the platform. The button tint ignores the user's
default accent color; the prompt bubble ignores light mode entirely; the
focused-control border is an invented blue, not the system's. A macOS user
expects the app to pick up their tint, their appearance, and their contrast
settings. Today every accent-colored pixel is a lie about the platform.

## What Changes

On **macOS**, the app adopts the system palette for its chrome through one
small Objective-C bridge over the `objc2`/`objc2-app-kit` dependencies the
crate already has - no new dependencies, no GPUI fork:

- **Button tint**: `primary`/`primary_hover`/`primary_active`,
  `button_primary*` and `ring` are derived from the system accent color
  (`NSColor.controlAccentColor`) instead of fixed `0x3B82F6` family. Hover
  and active are computed from the accent by lightness, so the components'
  interaction states keep working for any user-chosen tint.
- **Prompt background color**: the user prompt bubble's hardcoded
  `0x2563EB` becomes the system accent, with a contrast-aware foreground
  (white or near-black chosen from the accent's luminance) so text stays
  readable in both light and dark modes and across accent choices.
- **Focused control border**: the focus ring/tinted border on inputs,
  buttons and other focused controls uses the system accent via the `ring`
  theme color instead of an invented blue.
- **"etc."**: neutral chrome - panel/card backgrounds and borders, root
  window background - derives from system dynamic colors
  (`windowBackgroundColor`, `controlBackgroundColor`, `separatorColor`) so
  cards, dividers and window surfaces track the appearance automatically.
- Semantic colors are **not** replaced: agent-by-idle/pending/running/error
  status dots, risk-tint red/green, and danger accents stay fixed - they
  carry meaning, not theme, and must read identically on any tint.
- Non-macOS targets keep the existing fixed palette unchanged.

The palette resolves at the *current* effective appearance and re-resolves
when the OS appearance changes (light/dark flip), so switching the system
theme updates the app live - no settings round-trip.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `acp-panel-ui`: the panel's chrome (prompt bubble, input area, cards,
  controls, focus) is described as adopting the macOS system palette rather
  than hardcoded colors.

## Impact

- `crates/knot`: new `macos` module (cfg-gated) wrapping `NSColor` queries,
  resolving dynamic colors to GPUI colors; `app_support.rs` ingestion
  switches from fixed constants to the resolved palette for the surfaces
  above; `panel_view` swaps its hardcoded `CARD_BG`/`CARD_BORDER`/prompt
  and focus colors for theme/accent values.
- A one-time appearance-tracking hook so palette changes happen live.

No crates below the app crate change.