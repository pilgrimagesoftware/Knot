"""One-shot: reconcile on edit, and remove a chip when its row is dismissed."""

from pathlib import Path


def sub(path, old, new, count=1):
    p = Path(path)
    s = p.read_text()
    assert s.count(old) == count, f"{path}: expected {count} of:\n{old}"
    p.write_text(s.replace(old, new))


PANEL = "crates/knot/src/workspace_window/panel"

# Every edit reconciles: a deleted chip detaches its row. Before the
# restyle, so the styling is computed from the table the edit left behind
# rather than the one it invalidated.
sub(f"{PANEL}/prompt.rs",
    """                                                   InputEvent::Change => {
                                                       let palette = Palette::of(cx);
                                                       view.restyle_panel_composer(id,
                                                                                   palette,
                                                                                   cx);
                                                       cx.notify();
                                                   }""",
    """                                                   InputEvent::Change => {
                                                       let palette = Palette::of(cx);
                                                       // The buffer decides:
                                                       // a chip the edit
                                                       // removed detaches
                                                       // its row, and the
                                                       // restyle below then
                                                       // draws the table
                                                       // that is left.
                                                       let detached =
                                                           view.reconcile_panel_attachments(id,
                                                                                            cx);
                                                       if detached {
                                                           view.restyle_panel_attachments(id,
                                                                                          palette,
                                                                                          cx);
                                                       }
                                                       view.restyle_panel_composer(id,
                                                                                   palette,
                                                                                   cx);
                                                       cx.notify();
                                                   }""")

# Dismissing a strip entry deletes its chip, which reports as an ordinary
# edit and detaches the row through the reconciliation above.
sub(f"{PANEL}/prompt.rs",
    """    /// Removes one attached path from `id`'s pending context by index.
    pub(in crate::workspace_window) fn remove_panel_context(&mut self, id: Uuid, index: usize,
                                                            cx: &mut App) {
        let removed = match self.panel_pending_context.get_mut(&id) {
            Some(paths) if index < paths.len() => {
                paths.remove(index);
                true
            }
            _ => false,
        };
        if removed {
            self.restyle_panel_attachments(id, Palette::of(cx), cx);
        }
    }""",
    """    /// Removes one attached path from `id`'s pending context by index.
    ///
    /// Deletes the chip rather than the row: the deletion reports as an
    /// ordinary edit, and the reconciliation that follows every edit is
    /// what drops the row. One direction, so the strip and the buffer
    /// cannot disagree about which of them was right.
    ///
    /// The row is removed directly only when there is no chip to delete -
    /// a buffer that never received one, which is what an attachment
    /// dismissed before its deferred insertion ran looks like.
    pub(in crate::workspace_window) fn remove_panel_context(&mut self, id: Uuid, index: usize,
                                                            window: &mut Window, cx: &mut App) {
        let Some(path) = self.panel_pending_context
                             .get(&id)
                             .and_then(|paths| paths.get(index))
                             .cloned()
        else {
            return;
        };
        if self.remove_attachment_reference(id, &path, window, cx) {
            return;
        }
        if let Some(paths) = self.panel_pending_context.get_mut(&id)
           && index < paths.len()
        {
            paths.remove(index);
        }
        if let Some(queued) = self.panel_pending_attachments.get_mut(&id) {
            queued.retain(|queued| queued != &path);
        }
        self.restyle_panel_attachments(id, Palette::of(cx), cx);
    }""")

sub(f"{PANEL}/input/chips.rs",
    """                                    .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                        view.remove_panel_context(id, index, cx);
                                        cx.notify();
                                    })),""",
    """                                    .on_click(cx.listener(move |view,
                                                          _: &ClickEvent,
                                                          window,
                                                          cx| {
                                        view.remove_panel_context(id, index, window, cx);
                                        cx.notify();
                                    })),""")

print("reconciliation wired")
