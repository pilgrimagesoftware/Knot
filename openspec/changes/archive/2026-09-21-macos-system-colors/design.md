# Design

## Context

See proposal.md - Why. The mechanics this rests on:

- The theme is a GPUI global (`cx.global_mut::<Theme>()`), mutated in
  `app_support.rs` with **fixed constants** today: accents
  `0x3B82F6`/`0x2563EB`/`0x1D4ED8` become `colors.primary*`,
  `button_primary*` and `ring`; `tokens` snapshot those same values for the
  components' paint; `Theme::sync_base` mirrors radius/typography.
- `panel_view` hardcodes the surface chrome: prompt bubble `0x2563EB` with
  white text, `CARD_BG`/`CARD_BORDER` constants, mute grays.
- GPUI exposes each window's effective appearance
  (`window.appearance()`, `WindowAppearance`) and an
  `observe_window_appearance` callback; the panel already tracks appearance
  for its overflow/terminal surfaces.
- `objc2-app-kit` is already a dependency of `crates/knot`, so `NSColor`
  dynamic colors are reachable in-crate with no new dependency and no GPUI
  fork/patch.

## Goals / Non-Goals

**Goals:**

- The accent-driven chrome (buttons, focus ring, prompt bubble) follows the
  user's macOS accent color and stays readable for any choice of it.
- Neutral chrome follows the system's dynamic neutrals and repaints live on
  appearance flips.
- Semantic colors are untouched, so state keeps its meaning on any tint.
- Non-macOS behavior is byte-identical to today.

**Non-Goals:**

- No redesign; the shapes, spacing, typography and radii do not change.
- No per-surface hardcoded color survives where a system value exists - but
  the *fixed state palette* is out of scope by design.
- No GPUI fork: the bridge lives in the app crate.
- No new settings surface; the system palette is the default and only
  behavior.

## Decisions

### One `macos` module resolves NSColor; a stub serves the rest
A `crates/knot/src/macos/system_color.rs` (`#[cfg(target_os = "macos")]`)
queries the accent and the dynamic neutrals through `objc2-app-kit`. The
queries read `NSColor`, resolve at the window's current effective appearance
via `usingColorSpace(_: sRGB)` + `getRed:green:blue:alpha:`, and convert to
GPUI `Hsla`. A non-macOS stub returns the fixed palette, so the call sites
stay unconditional and every non-Apple surface is provably untouched.

Queried colors and their surfaces:
- `controlAccentColor` → `colors.primary*`, `button_primary*`, `ring`,
  `selection` (accent @ 25%), prompt bubble
- `windowBackgroundColor` / `controlBackgroundColor` → root window and
  bubble/card/input surfaces
- `separatorColor` → `CARD_BORDER`/divider borders
- Muted grays (`MUTED`, status grays) stay the existing muted shades, which
  are neutral enough for either appearance.

### Hover/active/prompt-foreground come from the accent's luminance
`controlAccentColor` is a single color; components need two extra
interaction states and the prompt needs foreground. Rather than invent a
second NSColor, the design adjusts lightness in GPUI's `Hsla` space around
the resolved accent (hover lighter, active darker), and picks the prompt
foreground by relative luminance of the accent: darkness measured via the
standard sRGB channel coefficients; white when the accent is dark, a
near-black when it is light. This keeps a single source of truth (the
system) and keys every derived color off the one query result.

### Focus and the ring share the accent source
The focused-control border is the theme `ring` color plus the `focus_ring`
drawing path (both already wired). Setting `ring` from the system accent
fixes focus on every control that asks, rather than per-widget color swaps.
Tasks verify an actual focused input/button to confirm the panel's widgets
paint through `ring`/tokens and none hardcode a focus color.

### Appearance changes re-resolve through the existing observer
Knot already observes window appearance (it repaints overflow/terminal
surfaces). That same callback re-runs palette ingestion - re-querying the
system colors, re-deriving hover/active/contrast, re-syncing `tokens` - and
requests a repaint of the panel windows. No settings change is involved, and
the system mode (`appearance_mode`) continues to mean what it does today.

### Non-macOS gets a stub, not a fork
The bridge is cfg-gated to macOS; Windows/Linux call sites resolve to the
existing fixed palette. Removing a runtime `cfg` branch inside the panel code
keeps the diff reviewable: `panel_view` and `app_support.rs` stop naming
`0x2563EB`-style constants for the *tint/neutral surfaces* and read from the
palette, while `ERROR_COLOR`/`SAFE_COLOR`/agent-state colors remain as they
are.

## Risks / Trade-offs

- [Luminance derivation is approximate vs AppKit's own contrast rules]
  → Acceptable: it only picks white vs near-black for the prompt foreground
  and hover/active shades, and the pair is validated by a contrast check in
  verification. Mismatch risk is limited to exotic accent colors selected in
  System Settings.
- [A very light/low-contrast user accent could still be borderline] → The
  foreground rule is conservative (near-black instead of pure black) and the
  semantic palette is intact, so the worst case is a slightly low-contrast
  bubble, not an unreadable one.
- [Accent value differences between GPUI's color space and AppKit's]
  → Resolution goes through sRGB on both sides of the boundary; any gamma
  nuance is invisible at interaction-state granularity.
- [Themed tokens are a snapshot] → Ingestion must re-derive `ThemeTokens`
  and call `Theme::sync_base`, exactly as `app_support.rs` does today, or
  buttons would keep painting stale colors. Called out in tasks.