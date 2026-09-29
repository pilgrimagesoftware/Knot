//! What keeps the MCP server running: the app, not any one window.
//!
//! Contract: `openspec/specs/mcp-server/spec.md`.
//!
//! The supervisor returns when its stop oneshot fires or its sender is
//! dropped. The sender used to live on the workspace manager's view, so
//! closing that window stopped the server under every agent still running in
//! a workspace window, and nothing could start it again short of relaunching
//! (issue #514). It is held in a global now and dropped only when the app
//! quits.

use gpui_kit::App;
use tokio::sync::oneshot;

/// The MCP server's stop sender, for as long as the server should run.
struct McpServerLifetime(Option<oneshot::Sender<()>>);

impl gpui_kit::Global for McpServerLifetime {}

/// Keeps the server running until the app quits.
pub(crate) fn hold_mcp_server(stop: oneshot::Sender<()>, cx: &mut App) {
    cx.set_global(McpServerLifetime(Some(stop)));
    // Stopped here rather than left to process exit so the server releases
    // its port and writes its "stopped" entry, as it did when the manager
    // window's close stopped it.
    cx.on_app_quit(|cx| {
          stop_mcp_server(cx);
          async {}
      })
      .detach();
}

/// Stops the server. Idempotent: a second call finds nothing to drop.
pub(crate) fn stop_mcp_server(cx: &mut App) {
    if cx.has_global::<McpServerLifetime>() {
        // Dropping the sender is the stop signal; the supervisor treats a
        // closed channel the same as a sent `()`.
        cx.global_mut::<McpServerLifetime>().0.take();
    }
}
