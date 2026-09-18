## Context

See proposal.md - Why/What Changes for motivation and scope.

**This change originally targeted `libghostty` via C FFI** (a new
`knot-ghostty` crate, `bindgen` bindings, a hand-rolled `NSView`/
`CAMetalLayer` hosting element for GPUI). That approach was abandoned after
extensive attempts to vendor `libghostty` hit a wall of environment/build
issues unrelated to the Rust port itself: a Zig-version/libghostty-tag
compatibility matrix that kept breaking, an upstream `build.zig` regression
where Metal/MetalKit are no longer linked for a plain library artifact
(traced to a stale patch in `scripts/build-libghostty.sh` targeting code
that no longer exists in current Ghostty source), and a `zig build`
requirement for elevated privileges on this machine's Xcode/Metal-toolchain
setup that repeatedly failed non-interactively. After several hours with
zero Rust code to show for it, and recognizing that GPUI has no built-in
way to host a foreign native rendering surface at all (the `NSView`
approach would have been this codebase's first such hack), switching
engines was the better call than continuing to fight vendoring.

Two facts from the current codebase still shape this design, same as
before the pivot:

- **`WorkspaceWindow` (the window `main()` actually opens) spawns no PTY
  session at all today.** `TerminalSession<PtyTransport>::spawn_pty_with_exit`
  is only called from `Shell`, a struct that compiles but is never
  constructed by `main()`. Its raw-byte `OutputBuffer` + polling loop
  (`OUTPUT_POLL_INTERVAL`, `poll_outputs`) was a scaffold predating any
  real terminal engine.
- **`knot-activity`'s `Tracker` already takes abstract signals, not
  bytes**: `on_terminal_activity(&self)`, `on_user_input(&self, key)`,
  `on_process_exit(&self, exit_code)`. Nothing in `knot-activity` assumes a
  particular terminal engine.

Unlike the libghostty design, this one keeps `knot-terminal`'s existing PTY
spawning rather than retiring it - see Decisions.

## Goals / Non-Goals

**Goals:**
- A selected agent's terminal is visible, live, and typeable in
  `WorkspaceWindow`.
- `knot-activity`/`agent-hooks`/`agent-lifecycle` keep working unchanged,
  fed by grid-parser events instead of nothing.
- Pure Rust, no FFI, no vendored native libraries, no platform-specific
  toolchain requirement beyond what already builds this workspace.

**Non-Goals:**
- Split-pane / multiple simultaneous visible surfaces (separate plan).
- GPU-shader terminal effects (cursor trails, background blur, ligature
  rendering beyond what GPUI's text layout already provides) - this is a
  plain cell-grid renderer, not a port of Ghostty's rendering fidelity.
- Linux/Windows-specific polish - the port targets macOS like the rest of
  the app, but nothing here is macOS-only by construction (unlike the
  libghostty approach, which required native `NSView` embedding).
- Migrating `Shell`'s dead code path; it becomes unreachable and gets
  deleted as part of this change's cleanup, not ported forward.

## Decisions

### `alacritty_terminal` for VT/ANSI parsing and grid state

**Decision**: use the `alacritty_terminal` crate to parse PTY output into
a `Term`/grid model, rather than hand-rolling an ANSI parser or
reattempting a libghostty FFI binding.

**Why**: it's a mature, actively maintained pure-Rust VT100/xterm parser
with no build-time dependencies beyond Cargo. Critically, it's the same
approach Zed's own terminal panel takes - Zed is GPUI's origin project
(`gpui-pre`, which `gpui-kit` builds on, is a fork of Zed's `gpui`), so
"parse with `alacritty_terminal`, render the grid with GPUI primitives" is
an established pattern for this exact framework, not a novel integration.

**Alternative considered**: libghostty via FFI (this change's original
design) - abandoned per Context above. **Alternative considered**: `vte`
(the lower-level parser `alacritty_terminal` itself is built on) directly,
building our own grid/cursor/scrollback model on top - rejected as
reinventing what `alacritty_terminal` already provides well; only reach
for `vte` directly if `alacritty_terminal`'s grid model proves too
opinionated for Knot's needs, which nothing in these specs requires.

### `knot-terminal` keeps owning PTY spawning; gains the grid layer on top

**Decision**: unlike the libghostty design (which would have retired
`knot-terminal`'s PTY code in favor of the terminal engine spawning its
own subprocess), `alacritty_terminal` doesn't spawn processes itself - it
only parses bytes fed to it. So `knot-terminal`'s existing
`TerminalSession`/`PtyTransport`/`SessionConfig`/`SessionPlan` stay, and a
new grid-parsing layer sits on top: PTY output bytes feed into an
`alacritty_terminal::Term`, whose resulting grid state the UI reads to
render.

**Why**: no reason to discard working, tested PTY-spawning code when the
new terminal engine has no opinion about process spawning at all - this is
strictly additive to `knot-terminal`, not a replacement.

**Follow-up**: `Shell` (the dead struct in `main.rs` using the old
`OutputBuffer`/polling scaffold) and that scaffold's raw-byte display
still get deleted - not because the PTY layer is being replaced, but
because `WorkspaceWindow` needs its own real session-spawning + grid
wiring, making `Shell`'s parallel implementation redundant dead code.

### `Term` state lives behind a per-agent handle GPUI can read each frame

**Decision**: each running agent's `alacritty_terminal::Term` (plus an
`EventListener` impl that forwards bell/title/clipboard events) lives in
an `Arc<Mutex<_>>` owned alongside its `TerminalSession`, fed by a
background task reading PTY output and calling `Term::input`. GPUI's
render pass for the terminal element locks it briefly to read the visible
grid rows - the same "background task feeds shared state, GPUI reads it
on render" pattern `knot-terminal`'s current PTY-output polling already
uses, just replacing "poll raw bytes into an `OutputBuffer`" with "feed
bytes into a `Term`."

**Why**: keeps the concurrency model consistent with what's already
proven out elsewhere in this codebase (`Arc<Mutex<AgentStore>>`,
`Arc<Mutex<TerminalSession<PtyTransport>>>` in the legacy `Shell`) rather
than introducing a new pattern (channels, actor-style ownership) solely
for the terminal.

### Grid rendering is a plain GPUI element, not a native view

**Decision**: a `TerminalGridElement` (GPUI `Element` impl) iterates the
visible grid rows/cells and paints text runs + a cursor using GPUI's own
text layout and drawing primitives - no native `NSView`, no Metal layer,
no platform-specific code.

**Why**: this is the entire point of the pivot - GPUI already knows how to
draw text and shapes; a terminal grid is fundamentally a styled
character grid, well within what GPUI draws for every other view in this
app. No embedding hack needed.

## Risks / Trade-offs

- [Risk] Visual/behavioral fidelity to the Swift reference (which uses
  Ghostty) won't be pixel-identical - different cursor rendering, no GPU
  shader effects, possibly different wide-character/ligature handling →
  Mitigation: specs describe intended Rust-port behavior, not a literal
  Swift transcription; functional terminal use (running commands, reading
  output, resizing, copy/paste) is the bar, not rendering parity.
- [Risk] Retiring `Shell`/its polling loop removes the only code path
  currently exercising `TerminalSession::spawn_pty_with_exit` in a real
  (non-test) binary → Mitigation: `knot-terminal`'s own unit/integration
  tests keep covering `SessionConfig`/`SessionPlan`/`PtyTransport`
  construction directly; only the binary-crate wiring moves to the new
  grid-backed `WorkspaceWindow` path.
- [Risk] `alacritty_terminal`'s public API surface may shift across minor
  versions (it's primarily maintained for Alacritty's own use, not as a
  stable embedding library) → Mitigation: pin an exact version, isolate
  all direct usage behind `knot-terminal`'s own types so an API change is
  a contained update, not a spread-out one.

## Migration Plan

Additive for the user-visible surface (a placeholder becomes a real
terminal); the `Shell`-for-agents deletion is a cleanup of already-dead
code, not a behavior change for any currently-reachable path. No data
migration - nothing persists PTY/session state today.
