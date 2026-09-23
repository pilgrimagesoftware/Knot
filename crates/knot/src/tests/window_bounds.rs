//! A remembered window frame reconciled with the displays actually attached
//! (`openspec/specs/window-lifecycle`, "A restored window opens where the user
//! can reach it").
//!
//! `reconcile_bounds` is a pure function over rectangles precisely so these
//! assertions need no window and no display. The case that produced the
//! defect, a frame remembered on a monitor that is no longer plugged in,
//! cannot be staged in a test any other way.

use gpui_kit::Bounds;
use gpui_kit::Pixels;
use gpui_kit::point;
use gpui_kit::px;
use gpui_kit::size;

use crate::consts;
use crate::window_options::reconcile_bounds;

/// A rectangle, in the order the assertions below read best.
fn rect(x: f32, y: f32, width: f32, height: f32) -> Bounds<Pixels> {
    Bounds { origin: point(px(x), px(y)),
             size:   size(px(width), px(height)), }
}

/// A laptop display at the origin, and an external one to its right - the
/// arrangement a window gets stranded by when the external one goes away.
fn laptop() -> Bounds<Pixels> {
    rect(0., 0., 1440., 900.)
}

fn external() -> Bounds<Pixels> {
    rect(1440., 0., 1920., 1080.)
}

#[test]
fn bounds_already_on_a_display_are_left_alone() {
    let frame = rect(100., 80., 960., 640.);
    assert_eq!(reconcile_bounds(frame, &[laptop()]), Some(frame));
}

#[test]
fn bounds_on_a_second_display_are_left_alone_while_it_is_attached() {
    let frame = rect(1600., 100., 960., 640.);
    assert_eq!(reconcile_bounds(frame, &[laptop(), external()]),
               Some(frame));
}

#[test]
fn a_frame_on_a_disconnected_display_falls_back_to_the_default() {
    // Remembered on the external display, reopened with only the laptop, so
    // the frame overlaps nothing. `None` is the caller's signal to leave the
    // centred default placement in place, which is what puts the window fully
    // on an attached display with its title bar grabbable - the spec's "the
    // display is gone" scenario. Nudging a frame 760px off the nearest edge
    // back on would be a translation large enough that keeping the remembered
    // position has stopped meaning anything.
    let frame = rect(2200., 100., 960., 640.);
    assert_eq!(reconcile_bounds(frame, &[laptop()]), None);
}

#[test]
fn a_frame_hanging_off_the_right_edge_is_moved_not_resized() {
    let frame = rect(1380., 100., 960., 640.);
    let reconciled = reconcile_bounds(frame, &[laptop()]).expect("should be placed");
    assert_eq!(reconciled.size, frame.size, "moved, never resized");
    assert_eq!(reconciled.origin.y, frame.origin.y,
               "the axis that was fine is untouched");
    assert!(reconciled.origin.x <= px(1440. - consts::WINDOW_MIN_VISIBLE_WIDTH),
            "enough of the window has to remain grabbable");
}

#[test]
fn a_frame_above_the_top_edge_is_brought_down() {
    // A negative y is what a display arranged above the primary one leaves
    // behind; macOS puts the menu bar at the top, so nothing may sit above it.
    let frame = rect(100., -400., 960., 640.);
    let reconciled = reconcile_bounds(frame, &[laptop()]).expect("should be placed");
    assert_eq!(reconciled.origin.y, px(0.));
    assert_eq!(reconciled.origin.x, frame.origin.x,
               "the axis that was fine is untouched");
}

#[test]
fn the_whole_title_bar_lands_on_screen_not_a_sliver() {
    let frame = rect(100., 880., 960., 640.);
    let reconciled = reconcile_bounds(frame, &[laptop()]).expect("should be placed");
    let bottom_of_strip = f32::from(reconciled.origin.y) + consts::WINDOW_GRAB_STRIP_HEIGHT;
    assert!(bottom_of_strip <= 900.,
            "the grab strip must be fully on the display");
}

#[test]
fn a_frame_larger_than_every_display_falls_back() {
    let frame = rect(0., 0., 4000., 3000.);
    assert_eq!(reconcile_bounds(frame, &[laptop(), external()]), None);
}

#[test]
fn a_frame_overlapping_nothing_falls_back() {
    let frame = rect(6000., 4000., 960., 640.);
    assert_eq!(reconcile_bounds(frame, &[laptop()]), None);
}

#[test]
fn no_displays_falls_back() {
    assert_eq!(reconcile_bounds(rect(0., 0., 960., 640.), &[]), None);
}

#[test]
fn a_straddling_frame_is_pulled_onto_the_display_it_overlaps_most() {
    // Mostly on the external display, with a sliver on the laptop. Dropping
    // the laptop and keeping the external one should leave it where it is;
    // the reverse should pull it left.
    let frame = rect(1340., 100., 960., 640.);
    assert!(f32::from(reconcile_bounds(frame, &[laptop(), external()]).expect("placed")
                                                                      .origin
                                                                      .x)
            >= 1340.,
            "with both attached it stays put or moves onto the display it mostly occupies");

    let onto_laptop = reconcile_bounds(frame, &[laptop()]).expect("placed");
    assert!(onto_laptop.origin.x < frame.origin.x,
            "with only the laptop it is pulled left");
}
