//! The Workspace Manager's roster scrolls once it outgrows the window.
//!
//! The rows were a plain column under a `flex_1` parent with no `min_h_0`,
//! so the column took its content's height, ran past the bottom of the
//! window and was clipped there - the rows below the fold could not be
//! reached. The question is about layout, so it is asked of a drawn window:
//! the list's painted bounds must end inside it.

use gpui_kit::TestAppContext;
use gpui_kit::px;
use gpui_kit::size;

use crate::tests::workspace;
use crate::tests::workspace_dialog::manager;

/// Short enough that `WORKSPACE_COUNT` rows cannot fit in it.
const WINDOW_HEIGHT: f32 = 400.;

const WORKSPACE_COUNT: usize = 40;

#[gpui_kit::test]
fn a_roster_taller_than_the_window_stays_inside_it(cx: &mut TestAppContext) {
    let (store, mut cx, _manager) = manager(cx);
    {
        let mut store = store.lock();
        for index in 0..WORKSPACE_COUNT {
            store.add_workspace(workspace(&format!("Workspace {index}")));
        }
    }
    cx.simulate_resize(size(px(600.), px(WINDOW_HEIGHT)));
    cx.run_until_parked();

    let list = cx.debug_bounds("workspace-manager-list")
                 .expect("the workspace list was never painted");
    assert!(list.bottom() <= px(WINDOW_HEIGHT),
            "the list ends at {:?}, past the window's {WINDOW_HEIGHT}px, so its lower rows are \
             clipped rather than scrolled to",
            list.bottom());
}
