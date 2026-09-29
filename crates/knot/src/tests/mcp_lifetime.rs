//! The MCP server outlives every window and stops only with the app
//! (issue #514).
//!
//! Its stop sender used to live on the workspace manager's view, so closing
//! that window stopped the server under every agent still running elsewhere.

use gpui_kit::AppContext;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::Render;
use gpui_kit::TestAppContext;
use gpui_kit::Window;
use gpui_kit::WindowOptions;
use gpui_kit::div;
use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;

use crate::mcp_lifetime::hold_mcp_server;
use crate::mcp_lifetime::stop_mcp_server;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
fn closing_every_window_leaves_the_server_running(cx: &mut TestAppContext) {
    let (stop, mut stopped) = oneshot::channel::<()>();
    let window = cx.update(|cx| {
                       hold_mcp_server(stop, cx);
                       cx.open_window(WindowOptions::default(), |_, cx| cx.new(|_| Blank))
                         .expect("failed to open a window")
                   });
    cx.update(|cx| {
          window.update(cx, |_, window, _| window.remove_window())
                .expect("the window should be open");
      });
    cx.run_until_parked();

    assert!(cx.update(|cx| cx.windows().is_empty()));
    assert_eq!(stopped.try_recv(),
               Err(TryRecvError::Empty),
               "closing a window stopped the MCP server");
}

#[gpui_kit::test]
fn stopping_the_app_stops_the_server(cx: &mut TestAppContext) {
    let (stop, mut stopped) = oneshot::channel::<()>();
    cx.update(|cx| {
          hold_mcp_server(stop, cx);
          stop_mcp_server(cx);
          // A second stop, as a quit after an earlier one would make, is
          // harmless.
          stop_mcp_server(cx);
      });
    assert_eq!(stopped.try_recv(), Err(TryRecvError::Closed));
}
