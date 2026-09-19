# Tasks

## 1. The mapping

- [ ] 1.1 Add a `status_icon(&str) -> Option<IconName>` mapping
      (`pending`/`in_progress` → Loader, `completed` → CircleCheck,
      `failed` → CircleX, unknown → None) and unit tests asserting every
      status the app emits maps to an icon and unknown values return None.
- [ ] 1.2 Verify the colour decision reuses `card_outline` +
      `PanelStyle::outline_color` without a new mapping; add a test that the
      icon's colour track and the card outline agree for each status.

## 2. The header

- [ ] 2.1 Replace the header's `status_label` text with the status icon where
      the mapping returns one, leaving unknown statuses rendered as inline
      text. Give the icon the tooltip from `status_label` and the outline
      colour from step 1.2. Verify in the app: a running call shows the info
      loader, a failed call the danger icon, a completed call the muted
      check.
- [ ] 2.2 Confirm the header row's layout still truncates a long title
      instead of pushing the status icon off-screen; verify with a long-path
      card that the icon stays visible.

## 3. Verification

- [ ] 3.1 `make rust` passes clean (fmt, clippy, tests, build).
- [ ] 3.2 Exercise a full call lifecycle in the app and confirm the icon
      follows pending → running → done (muted) without the card jumping, and
      that a failed call's icon and outline both use the danger colour.