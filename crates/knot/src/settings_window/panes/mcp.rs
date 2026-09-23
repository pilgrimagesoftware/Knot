use gpui_kit::ClipboardItem;
use gpui_kit::Context;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Styled;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::div;
use gpui_kit::px;
use knot_mcp::ServerState;

use crate::settings_window::SettingsWindow;

impl SettingsWindow {
    /// The status row's text for `state`.
    ///
    /// One catalog entry per state, substituted through `t_with`, so each
    /// reads as a sentence a translator wrote rather than as fragments
    /// assembled here - and so a state's text can only mention what that
    /// state carries. A running server has no attempt count and a retrying
    /// one has no address, which is the point of the payloads living in the
    /// variants.
    pub(crate) fn mcp_state_text(state: &ServerState) -> String {
        match state {
            ServerState::Disabled => knot_core::l10n::t("settings.mcp.status_disabled"),
            ServerState::Starting => knot_core::l10n::t("settings.mcp.status_starting"),
            ServerState::Running { addr } => knot_core::l10n::t_with("settings.mcp.status_running",
                                                                     &[("address",
                                                                        &addr.to_string())]),
            ServerState::Retrying { attempt, error, .. } => {
                knot_core::l10n::t_with("settings.mcp.status_retrying",
                                        &[("attempt", &attempt.to_string()), ("error", error)])
            }
            ServerState::Stopped => knot_core::l10n::t("settings.mcp.status_stopped"),
        }
    }

    /// Whether the status row needs redrawing, remembering `current` when
    /// it does.
    ///
    /// The poll that calls this runs whether or not anything changed, and a
    /// `cx.notify()` per tick would repaint the settings window twice a
    /// second for the whole time it is open.
    pub(crate) fn mcp_state_changed(last: &mut ServerState, current: ServerState) -> bool {
        if *last == current {
            return false;
        }
        *last = current;
        true
    }

    /// Delegated so the URL shown here - and the `mcp add` command built
    /// from it below, which users copy verbatim - is the same one Knot
    /// hands its own agents over ACP. It was built separately and without
    /// the `/mcp` path, so anyone following Knot's own instructions
    /// registered a server that answers 405 and never connects.
    pub(crate) fn mcp_server_url(port: u16) -> String {
        knot_agent_launch::mcp_url_for_port(port)
    }

    /// The command to copy for registering `agent_type` against Knot's MCP
    /// server, ported from the Swift reference's
    /// `MCPCommandView.mcpCommandCopy` (Skwad -> Knot renamed).
    pub(crate) fn mcp_install_command(agent_type: &str, url: &str) -> String {
        match agent_type {
            "claude" => format!("claude mcp add --transport http --scope user knot {url}"),
            "codex" => format!("codex mcp add knot --url {url}"),
            "opencode" => "opencode mcp add".to_string(),
            "gemini" => format!("gemini mcp add --transport http knot {url} --scope user"),
            _ => String::new(),
        }
    }

    pub(crate) fn save_mcp_port(&mut self, cx: &mut Context<Self>) {
        let value = self.mcp_port_input.read(cx).value().to_string();
        if let Ok(port) = value.parse::<u16>() {
            self.settings.mcp_server_port = port;
            self.persist();
        }
    }

    fn select_mcp_agent_type(&mut self, agent_type: &str, cx: &mut Context<Self>) {
        self.mcp_selected_agent_type = agent_type.to_string();
        cx.notify();
    }

    pub(crate) fn render_mcp(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let settings_window = cx.entity();
        let mcp_server_enabled = self.settings.mcp_server_enabled;
        let server_url = Self::mcp_server_url(self.settings.mcp_server_port);
        let agent_type_label = Self::agent_type_label(&self.mcp_selected_agent_type);
        let install_command = Self::mcp_install_command(&self.mcp_selected_agent_type, &server_url);

        v_flex()
            .gap_3()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(
                        knot_core::l10n::t("settings.mcp.blurb"),
                    ),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.mcp.server_settings"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.mcp.enable"),
                        Switch::new("mcp-server-enabled")
                            .checked(mcp_server_enabled)
                            .on_click({
                                let settings_window = settings_window.clone();
                                move |checked, _, app| {
                                    let checked = *checked;
                                    settings_window.update(app, |view, _| {
                                        view.settings.mcp_server_enabled = checked;
                                        view.persist();
                                    })
                                }
                            }),
                    ))
                    .child(Self::row(
                        knot_core::l10n::t("settings.mcp.port"),
                        Input::new(&self.mcp_port_input).w(px(100.)),
                    ))
                    .child(Self::text_row(
                        knot_core::l10n::t("settings.mcp.url"),
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Self::mono_text(cx, server_url.clone())
                                    .text_color(cx.theme().muted_foreground),
                            )
                            .child(
                                crate::controls::icon_button(
                                    "mcp-copy-url",
                                    "icons/copy.svg",
                                    "Copy URL",
                                    false,
                                )
                                .on_click({
                                    let server_url = server_url.clone();
                                    move |_, _, app| {
                                        app.write_to_clipboard(ClipboardItem::new_string(
                                            server_url.clone(),
                                        ));
                                    }
                                }),
                            ),
                    ))
                    // Read-only, and offering no control of its own: the
                    // toggle above turns the server off, and recovery from a
                    // failure is automatic.
                    .child(Self::text_row(
                        knot_core::l10n::t("settings.mcp.status"),
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(Self::mcp_state_text(&self.last_mcp_state)),
                    )),
            )
            .child(
                crate::controls::group(knot_core::l10n::t("settings.mcp.install_command"))
                    .child(Self::row(
                        knot_core::l10n::t("settings.mcp.agent"),
// The agents that have an MCP server to register with:
                        // not a shell, and not a custom command whose
                        // registration Knot knows nothing about.
                        Self::dropdown("mcp-agent-type-picker",
                                       agent_type_label,
                                       knot_core::agent_type::ALL.iter()
                                                                 .filter(|kind| {
                                                                     !kind.is_custom
                                                                     && !kind.is_shell
                                                                 })
                                                                 .map(|kind| {
                                                                     (kind.label.into(), kind.id)
                                                                 })
                                                                 .collect(),
                                       settings_window.clone(),
                                       |view, value, _, cx| view.select_mcp_agent_type(value, cx)),
                    ))
                    .child(Self::text_row(
                        knot_core::l10n::t("settings.mcp.command"),
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_2()
                            .items_start()
                            .child(if install_command.is_empty() {
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .text_sm()
                                    .child(knot_core::l10n::t("settings.mcp.no_setup"))
                                    .into_any_element()
                            } else {
                                Self::mono_text(cx, install_command.clone())
                                    .flex_1()
                                    .min_w_0()
                                    .whitespace_normal()
                                    .into_any_element()
                            })
                            .child(
                                crate::controls::icon_button(
                                    "mcp-copy-install-command",
                                    "icons/copy.svg",
                                    "Copy command",
                                    false,
                                )
                                .on_click(move |_, _, app| {
                                    if !install_command.is_empty() {
                                        app.write_to_clipboard(ClipboardItem::new_string(
                                            install_command.clone(),
                                        ));
                                    }
                                }),
                            ),
                    )),
            )
    }
}
