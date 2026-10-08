//! The sidebar draws each agent's state dot as its own hoverable element,
//! in the full and the compact row alike, so the tooltip from #582 has a
//! target in either layout. What the tooltip says is covered by
//! `tests::layout_model`; a headless window cannot read a tooltip's text.

use gpui_kit::TestAppContext;

use super::shortcuts_tests::Fixture;
use super::shortcuts_tests::window_with;
use super::shortcuts_tests::window_with_agents;

fn dot_drawn(fixture: &mut Fixture) -> bool {
    fixture.window.run_until_parked();
    fixture.window.debug_bounds("agent-status-dot").is_some()
}

#[gpui_kit::test]
fn the_full_row_draws_a_hoverable_dot(cx: &mut TestAppContext) {
    let mut fixture = window_with_agents(1, cx);
    assert!(dot_drawn(&mut fixture));
}

#[gpui_kit::test]
fn the_compact_row_draws_a_hoverable_dot(cx: &mut TestAppContext) {
    let compact_width = knot_core::consts::SIDEBAR_WIDTH_MIN;
    let mut fixture = window_with(1, |settings| settings.sidebar_width = compact_width, cx);
    assert!(fixture.view.read_with(&fixture.window, |view, cx| {
                            super::sidebar_is_compact(view.sidebar_width(cx))
                        }),
            "the sidebar is not compact");
    assert!(dot_drawn(&mut fixture));
}
