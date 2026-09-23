"""One-shot: port develop's ! shell-passthrough changes into the split input files."""

from pathlib import Path


def sub(path, old, new):
    p = Path(path)
    s = p.read_text()
    assert s.count(old) == 1, f"{path}: expected exactly one of:\n{old}"
    p.write_text(s.replace(old, new))


ENTRY = "crates/knot/src/workspace_window/panel/input/entry.rs"
CONTROLS = "crates/knot/src/workspace_window/panel/input/controls.rs"
AREA = "crates/knot/src/workspace_window/panel/input/area.rs"

# entry.rs: a `!` command is sendable when a prompt would not be.
sub(ENTRY,
    """    /// `can_send` and the send tooltip are derived here rather than passed
    /// in: both are functions of the buffer and the send-chord setting,
    /// which this row already has in hand.
    pub(super) fn render_panel_entry_row(&self, id: Uuid, input: &Entity<PanelInputState>,
                                         blocked: bool, turn_active: bool,
                                         cx: &mut Context<Self>)
                                         -> impl IntoElement + use<> {
        let can_send = !blocked && !turn_active && !input.read(cx).value().trim().is_empty();""",
    """    /// `can_send` and the send tooltip are derived here rather than passed
    /// in: both are functions of the buffer and the send-chord setting,
    /// which this row already has in hand.
    pub(super) fn render_panel_entry_row(&self, id: Uuid, input: &Entity<PanelInputState>,
                                         blocked: bool, turn_active: bool, is_shell: bool,
                                         cx: &mut Context<Self>)
                                         -> impl IntoElement + use<> {
        // A `!` command runs locally and never touches the session, so none
        // of the reasons a prompt may not be sent apply to it: not a pending
        // permission, not a turn in flight.
        let can_send = {
            let value = input.read(cx).value();
            crate::panel_commands::can_send(&value, blocked, turn_active)
        };""")

sub(ENTRY,
    """                    .child(if turn_active {""",
    """                    .children(turn_active.then(|| {""")

sub(ENTRY,
    """                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.stop_panel_prompt(id, cx);
                            }))
                            .into_any_element()
                    } else {""",
    """                            .on_click(cx.listener(move |view, _: &ClickEvent, _, cx| {
                                view.stop_panel_prompt(id, cx);
                            }))
                    }))
                    // Shown beside stop rather than instead of it while a `!`
                    // command is typed during a turn: the command has to be
                    // runnable and the turn has to stay interruptible, and
                    // swapping one control for the other would cost whichever
                    // it replaced.
                    .children((!turn_active || is_shell).then(|| {""")

sub(ENTRY,
    """                            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                view.send_panel_prompt(id, window, cx);
                            }))
                            .into_any_element()
                    }),""",
    """                            .on_click(cx.listener(move |view, _: &ClickEvent, window, cx| {
                                view.send_panel_prompt(id, window, cx);
                            }))
                    })),""")

# controls.rs: while the buffer is a command, the hint says so.
sub(CONTROLS,
    """    pub(super) fn render_panel_control_bar(&self, id: Uuid, expanded: bool,
                                           config_options: &[knot_acp::ConfigOption],
                                           cx: &mut Context<Self>)
                                           -> impl IntoElement + use<> {""",
    """    pub(super) fn render_panel_control_bar(&self, id: Uuid, expanded: bool, is_shell: bool,
                                           config_options: &[knot_acp::ConfigOption],
                                           cx: &mut Context<Self>)
                                           -> impl IntoElement + use<> {""")

sub(CONTROLS,
    """                                    .child(Self::panel_prompt_send_hint(shift_to_send)),""",
    """                                    // While the buffer is a command, the
                                    // hint says so instead of saying how to
                                    // send: what happens on Enter is the
                                    // thing the user needs to know before
                                    // pressing it.
                                    .child(if is_shell {
                                        knot_core::l10n::t("panel.shell.marker")
                                    }
                                    else {
                                        Self::panel_prompt_send_hint(shift_to_send).to_string()
                                    }),""")

# area.rs decides it once, so the two rows cannot disagree about what is typed.
sub(AREA,
    """        // An appearance switch re-renders without editing, so this frame""",
    """        // Recomputed per frame from the buffer rather than kept as state,
        // which is what stops the mark and the control from drifting apart
        // from what is actually typed.
        let is_shell = crate::panel_commands::is_shell_command(&input.read(cx).value());
        // An appearance switch re-renders without editing, so this frame""")

sub(AREA,
    "                .child(self.render_panel_entry_row(id, input, blocked, turn_active, cx))",
    "                .child(self.render_panel_entry_row(id, input, blocked, turn_active, is_shell,\n"
    "                                                   cx))")

sub(AREA,
    "                .child(self.render_panel_control_bar(id, expanded, config_options, cx))",
    "                .child(self.render_panel_control_bar(id, expanded, is_shell, config_options,\n"
    "                                                    cx))")

print("shell passthrough ported into the split")
