# Tasks

## 1. The system-color bridge

- [ ] 1.1 Add `crates/knot/src/macos/mod.rs` and
      `crates/knot/src/macos/system_color.rs` (cfg-gated to `target_os =
      "macos"`) querying `controlAccentColor`, `windowBackgroundColor` /
      `controlBackgroundColor`, and `separatorColor`, resolving each to sRGB
      channel values via the current effective appearance.
- [ ] 1.2 Convert those to GPUI `Hsla`; add the non-macOS stub returning the
      existing fixed palette. Verify the crate compiles on macOS and that a
      no-op build passes on a non-Apple target if one is used in CI.

## 2. Palette ingestion

- [ ] 2.1 In `app_support.rs`, derive the theme accents (`primary*`,
      `button_primary*`, `ring`, `selection`) from the resolved system
      accent, computing hover/active by lightness adjustment, and the prompt
      foreground by relative luminance (white for dark accents, near-black
      for light ones). Re-derive `ThemeTokens` and call `Theme::sync_base`
      after mutation so components do not paint stale token colors.
- [ ] 2.2 Map neutral chrome to the resolved neutrals: root/window background,
      panel input and card surfaces from the background colors, and card/
      divider borders from `separatorColor`. Keep `ERROR_COLOR`,
      `SAFE_COLOR` and the agent-state status colors exactly as they are.

## 3. Panel surfaces

- [ ] 3.1 In `panel_view`, drive the user prompt bubble from the system
      accent with the contrast-computed foreground instead of
      `rgb(0x2563EB)`/white, and swap `CARD_BG`/`CARD_BORDER` usages for the
      themed surface colors.
- [ ] 3.2 Confirm focused controls in the panel paint their border/ring with
      the accent (audit inputs, buttons and any element drawing a focus
      border for hardcoded focus colors; swap any found).
- [ ] 3.3 Hook palette re-ingestion into the existing appearance observer so
      an appearance flip re-resolves colors and repaints without a settings
      change or restart.

## 4. Verification

- [ ] 4.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 4.2 Manual macOS verification: (a) change the system accent - the
      prompt bubble, primary buttons and focus tint follow it and the prompt
      text stays readable; (b) flip light/dark - neutral surfaces and window
      background repaint live; (c) a focused input shows the accent border;
      (d) agent status dots and risk tinting are unchanged; (e) a very light
      accent renders a dark prompt foreground rather than unreadable white.