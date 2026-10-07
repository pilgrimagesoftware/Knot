//! A scroll region capped at a maximum height, with a scrollbar (#583).
//!
//! A region that fills the space it is given takes gpui-component's
//! `overflow_y_scrollbar` directly. A capped one cannot: that wrapper sizes
//! itself `size_full()` and keeps only the cap, so in a definite-height
//! column a two-row list stretches to its full cap, and one that overflows
//! does not scroll. `tests::scrollbars` pins both.
//!
//! This keeps the region exactly as it was - `max_h` and `overflow_y_scroll`
//! on the element that holds the content, so it is as tall as its content up
//! to the cap - and lays the scrollbar over it from a wrapper sized by that
//! element.
//!
//! A list whose selection moves by keyboard (the panel's slash lookup, #557)
//! also needs to scroll to the selected row. [`capped_scroll_list`] makes
//! the caller's list the scrolled element itself, so its rows are the
//! direct children `ScrollHandle::scroll_to_item` indexes, and
//! [`CappedScroll::track`] hands the region the caller's handle to drive.

use gpui_kit::App;
use gpui_kit::Div;
use gpui_kit::ElementId;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Pixels;
use gpui_kit::RenderOnce;
use gpui_kit::ScrollHandle;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::div;

/// `content`, scrolling once it is taller than `max_h`, with a scrollbar
/// shown while it scrolls.
///
/// `id` keys the scroll position, so a region drawn once per agent needs the
/// agent in its id, as any scrolled element's id did.
pub(crate) fn capped_scroll(id: impl Into<ElementId>, max_h: Pixels, content: impl IntoElement)
                            -> CappedScroll {
    capped_scroll_list(id, max_h, div().child(content))
}

/// [`capped_scroll`], with `list` as the scrolled element rather than
/// wrapped in one: its children are the items a handle's `scroll_to_item`
/// counts, which a list wrapped one level down would hide.
pub(crate) fn capped_scroll_list(id: impl Into<ElementId>, max_h: Pixels, list: Div)
                                 -> CappedScroll {
    CappedScroll { id: id.into(),
                   max_h,
                   scrolled: list,
                   handle: None }
}

#[derive(IntoElement)]
pub(crate) struct CappedScroll {
    id:       ElementId,
    max_h:    Pixels,
    /// The element that scrolls: a plain wrapper around the content, or the
    /// caller's own list.
    scrolled: Div,
    /// The caller's handle, when it moves the region itself; otherwise the
    /// region keeps one of its own in window state.
    handle:   Option<ScrollHandle>,
}

impl CappedScroll {
    /// Scroll through `handle`, so the caller can scroll the region - to
    /// keep a keyboard selection in view - and not only the wheel.
    pub(crate) fn track(mut self, handle: &ScrollHandle) -> Self {
        self.handle = Some(handle.clone());
        self
    }
}

impl RenderOnce for CappedScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Kept in window state rather than created per frame, so the offset
        // survives redraws; the scrolled element and the bar share it.
        let handle = self.handle.unwrap_or_else(|| {
                                    window.use_keyed_state((self.id.clone(), "scroll-handle"),
                                                           cx,
                                                           |_, _| ScrollHandle::new())
                                          .read(cx)
                                          .clone()
                                });
        div().relative()
             .child(self.scrolled
                        .id(self.id)
                        .max_h(self.max_h)
                        .overflow_y_scroll()
                        .track_scroll(&handle))
             .vertical_scrollbar(&handle)
    }
}
