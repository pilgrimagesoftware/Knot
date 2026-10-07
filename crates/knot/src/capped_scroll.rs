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

use gpui_kit::AnyElement;
use gpui_kit::App;
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
    CappedScroll { id: id.into(),
                   max_h,
                   content: content.into_any_element() }
}

#[derive(IntoElement)]
pub(crate) struct CappedScroll {
    id:      ElementId,
    max_h:   Pixels,
    content: AnyElement,
}

impl RenderOnce for CappedScroll {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Kept in window state rather than created per frame, so the offset
        // survives redraws; the scrolled element and the bar share it.
        let handle = window.use_keyed_state((self.id.clone(), "scroll-handle"), cx, |_, _| {
                               ScrollHandle::new()
                           })
                           .read(cx)
                           .clone();
        div().relative()
             .child(div().id(self.id)
                         .max_h(self.max_h)
                         .overflow_y_scroll()
                         .track_scroll(&handle)
                         .child(self.content))
             .vertical_scrollbar(&handle)
    }
}
