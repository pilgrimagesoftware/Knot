## Why

`WorkspaceWindow`'s content pane is a static placeholder ("Terminal display
will appear here") - no agent's terminal is actually visible or usable yet.
Every other ported subsystem (agent lifecycle, MCP, activity detection,
hooks) drives or reacts to a running terminal session, but nothing renders
one. This is the last blocking gap between the Rust port and an actually
usable app: without it, `knot-terminal`'s PTY sessions have no on-screen
surface, and dogfooding the Rust build is impossible.

## What Changes

- Parse each agent's PTY output into a terminal grid (cells, styling,
  cursor, scrollback) using `alacritty_terminal`, a pure-Rust VT/ANSI
  engine with no C toolchain or FFI - GPUI's own upstream project, Zed,
  renders its terminal panel the same way, so this follows an established
  pattern for this exact framework rather than inventing a new one.
  (An initial attempt to bind `libghostty` via C FFI was abandoned - see
  design.md's Context for why.)
- Render that grid as a GPUI element inside `WorkspaceWindow`'s content
  pane: text runs per line with per-cell foreground/background/attributes,
  a cursor, and scroll position - ordinary GPUI drawing, no native-view
  embedding hack required.
- Keyboard input: GPUI key events translated to the bytes/escape sequences
  the running program expects (arrow keys, control combinations, etc.) and
  written to the PTY.
- Mouse input: clicks/drags/scroll translated and written to the PTY when
  the running program has enabled mouse reporting; otherwise drives the
  terminal's own scrollback and text selection.
- Resize handling: the grid's row/column count updates to match the
  content pane's size, and the PTY is resized to match (`SIGWINCH`).
- Selection and clipboard: click-drag text selection over the grid, with
  copy going to the OS pasteboard.
- Replace `WorkspaceWindow`'s terminal placeholder with the real grid view
  for the selected agent, switching on selection change and tearing one
  down when its agent is removed or restarted.
- **BREAKING**: none - this is purely additive against currently-inert UI.

## Capabilities

### New Capabilities
- `terminal-rendering`: PTY-output-to-grid parsing (`alacritty_terminal`)
  and the GPUI-hosted grid view (resize, DPI-independent since it's plain
  GPUI drawing, not a native surface).
- `terminal-input`: keyboard and mouse event translation and dispatch to
  a running PTY session, plus text selection.
- `terminal-actions`: handling terminal-originated events (title changes,
  bell, clipboard OSC 52 requests) and routing them into the rest of the
  app (window/sidebar title, clipboard).

### Modified Capabilities
- none. `agent-lifecycle`, `activity-detection`, and `agent-hooks` already
  specify the state machine and hook triggers this connects to; this change
  gives them an actual terminal to observe rather than changing their specs.

## Impact

- `crates/knot-terminal` gains the grid/VT-parsing layer (via
  `alacritty_terminal`) on top of its existing PTY spawning - the earlier
  libghostty-based design would have retired `knot-terminal`'s PTY code in
  favor of libghostty spawning its own subprocess; this approach keeps and
  extends what's already there instead.
- `crates/knot/src/main.rs`'s `WorkspaceWindow` gains a real terminal grid
  view and wires up session spawning (see design.md's Context - no code
  path in `main()` spawns a PTY today).
- New dependency: `alacritty_terminal` (pure Rust, cross-platform, no
  build-script/toolchain requirements beyond what Cargo already needs).
- No native FFI, no vendored static libraries, no Zig/Metal toolchain
  dependency - this runs anywhere the rest of the Rust workspace builds.
