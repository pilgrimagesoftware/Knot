//! The editor dialog's element tree, and the row/hint/section chrome it is
//! built from.
//!
//! Separate from [`super`], which owns the window, the form state and what
//! submitting it does - this only decides how that state is drawn.

use gpui_kit::Context;
use gpui_kit::InteractiveElement;
use gpui_kit::IntoElement;
use gpui_kit::ParentElement;
use gpui_kit::Render;
use gpui_kit::StatefulInteractiveElement;
use gpui_kit::Styled;
use gpui_kit::Window;
use gpui_kit::base::Disableable;
use gpui_kit::base::h_flex;
use gpui_kit::base::v_flex;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::Button;
use gpui_kit::component::button::ButtonVariants;
use gpui_kit::component::group_box::GroupBox;
use gpui_kit::component::group_box::GroupBoxVariants;
use gpui_kit::component::input::Input;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::menu::PopupMenuItem;
use gpui_kit::component::switch::Switch;
use gpui_kit::div;
use gpui_kit::px;

use super::AgentEditor;
use crate::agent_editor::persona_choices;
use crate::settings_window::SettingsWindow;

impl AgentEditor {
    /// A `LabeledContent`-style row: label at the leading edge, control(s)
    /// trailing - matching the Swift reference's `Form` rows, as opposed to
    /// the Settings window's fixed right-aligned label column.
    fn dialog_row(label: impl Into<gpui_kit::SharedString>, control: impl IntoElement)
                  -> impl IntoElement {
        h_flex().justify_between()
                .items_center()
                .gap_3()
                .child(div().child(label.into()))
                .child(control)
    }

    /// Muted description text under a row, mirroring the settings window's
    /// `hint()` but laid out for this dialog: its rows are
    /// leading-label/trailing-control rather than the settings window's
    /// fixed label column, so the hint spans the card instead of being
    /// indented past a column that isn't there.
    fn dialog_hint(cx: &Context<Self>, text: impl Into<gpui_kit::SharedString>)
                   -> impl IntoElement {
        div().text_sm()
             .whitespace_normal()
             .text_color(cx.theme().muted_foreground)
             .child(text.into())
    }

    /// A card grouping related rows - the Swift reference's `Form` sections
    /// use a filled, borderless card rather than the Settings window's
    /// titled, outlined `GroupBox`.
    fn dialog_section(_cx: &Context<Self>, rows: Vec<gpui_kit::AnyElement>) -> impl IntoElement {
        GroupBox::new().fill()
                       .child(v_flex().gap_3().children(rows).into_any_element())
    }
}

impl AgentEditor {
    /// The name and avatar rows, which every agent has whatever its type.
    fn identity_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            Self::dialog_row(knot_core::l10n::t("agent_editor.name"), Input::new(&self.name_input).w(px(200.))).into_any_element(),
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.avatar"),
                h_flex()
                    .gap_2()
                    .child(Input::new(&self.avatar_input).w(px(48.)))
                    .child(
                        crate::controls::icon_button(
                            "agent-avatar-picker",
                            "icons/face-grinning.svg",
                            knot_core::l10n::t("agent_editor.choose_character"),
                            false,
                        )
                        .on_click(
                            cx.listener(|editor, _, window, cx| editor.choose_avatar(window, cx)),
                        ),
                    ),
            )
            .into_any_element(),
        ]
    }

    /// The rows that depend on what kind of agent this is: its type, a
    /// shell command when it is one, a persona when any exist, and how it
    /// activates.
    /// The rows that depend on what kind of agent this is.
    fn agent_rows(&self, personas: Vec<knot_core::Persona>, cx: &mut Context<Self>)
                  -> Vec<gpui_kit::AnyElement> {
        let mut rows = vec![self.agent_type_row(cx)];
        if knot_core::agent_type::is_shell(&self.agent_type) && self.edit_target.is_none() {
            rows.push(self.shell_command_row(cx));
        }
        if !personas.is_empty() {
            rows.push(self.persona_row(personas, cx));
        }
        rows.extend(self.activation_rows(cx));
        rows
    }

    /// Which coding agent this is - stated rather than asked when the
    /// editor is creating a companion.
    fn agent_type_row(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        let editor = cx.entity();
        // A companion is a shell agent by definition: `create_shell_companion`
        // hardcodes the type, and the MCP `create-agent` tool refuses
        // `companion` for anything else. Offering the picker here would let
        // this one path create a companion the rest of the stack rejects, so
        // it states the type instead of asking for it.
        if self.creating_a_companion() {
            Self::dialog_row(
                    knot_core::l10n::t("agent_editor.coding_agent"),
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child(SettingsWindow::agent_type_label(knot_core::agent_type::SHELL)),
                )
                .into_any_element()
        }
        else {
            Self::dialog_row(
                    knot_core::l10n::t("agent_editor.coding_agent"),
                    Button::new("agent-type-picker")
                        .label(SettingsWindow::agent_type_label(&self.agent_type))
                        .dropdown_caret(true)
                        .dropdown_menu({
                            let editor = editor.clone();
                            move |mut menu, _, _| {
                                // Matches the Swift reference's `availableAgents`
                                // list (`CodingSettingsView.swift`).
                                // Every known type, including the user's own
                                // custom commands - this is where an agent's
                                // type is chosen, so the roster is offered
                                // whole.
                                for (agent_type, label) in
                                    knot_core::agent_type::ALL.iter()
                                                              .map(|kind| (kind.id, kind.label))
                                {
                                    let editor = editor.clone();
                                    menu = menu.item(PopupMenuItem::new(label).on_click(
                                        move |_, _, app| {
                                            editor.update(app, |e, _| {
                                                e.agent_type = agent_type.to_string()
                                            })
                                        },
                                    ));
                                }
                                menu
                            }
                        }),
                )
                .into_any_element()
        }
    }

    /// The command a shell agent runs, offered only while creating one -
    /// `EditRequest` has no field for it.
    fn shell_command_row(&self, cx: &mut Context<Self>) -> gpui_kit::AnyElement {
        Self::dialog_row(knot_core::l10n::t("agent_editor.command"),
                         Input::new(&self.shell_command_input).w(px(200.))
                                                              .font_family(cx.theme()
                                                                             .mono_font_family
                                                                             .clone())).into_any_element()
    }

    /// The persona picker, shown only when the user has personas.
    fn persona_row(&self, personas: Vec<knot_core::Persona>, cx: &mut Context<Self>)
                   -> gpui_kit::AnyElement {
        let editor = cx.entity();
        Self::dialog_row(
                    knot_core::l10n::t("agent_editor.persona"),
                    Button::new("agent-persona-picker")
                        .label(
                            self.persona_id
                                .and_then(|id| {
                                    personas.iter().find(|p| p.id == id).map(|p| p.name.clone())
                                })
                                .unwrap_or_else(|| knot_core::l10n::t("agent_editor.persona_none")),
                        )
                        .dropdown_caret(true)
                        .dropdown_menu({
                            let editor = editor.clone();
                            move |menu, _, _| {
                                // The persona list is user-grown and has no
                                // ceiling, while this dialog's window is 500px
                                // tall. Without this the menu lays out at its
                                // full content height, and everything past the
                                // window edge is clipped and unreachable -
                                // `PopupMenu` only caps its height and scrolls
                                // when told to.
                                let mut menu = menu.scrollable(true);
                                let none = knot_core::l10n::t("agent_editor.persona_none");
                                menu = menu.item(PopupMenuItem::new(none).on_click({
                                    let editor = editor.clone();
                                    move |_, _, app| editor.update(app, |e, _| e.persona_id = None)
                                }));
                                for persona in &personas {
                                    let id = persona.id;
                                    menu = menu.item(
                                        PopupMenuItem::new(persona.name.clone()).on_click({
                                            let editor = editor.clone();
                                            move |_, _, app| {
                                                editor.update(app, |e, _| e.persona_id = Some(id))
                                            }
                                        }),
                                    );
                                }
                                menu
                            }
                        }),
                )
                .into_any_element()
    }

    /// The activation switch and the sentence explaining what it means.
    fn activation_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let editor = cx.entity();
        // A switch with the mode named beside it. The segmented control
        // this replaced made the two options equally prominent and left
        // which one was chosen to a fill colour, which did not read at a
        // glance; a switch has one unambiguous position, and the label
        // spells out what that position currently means so the reader
        // never has to work it out from the switch alone.
        let activation_mode = self.activation_mode;
        let is_active = activation_mode == knot_core::ActivationMode::Active;
        vec![
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.activation"),
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new("agent-activation-mode")
                            .checked(is_active)
                            .on_click({
                                let editor = editor.clone();
                                move |checked, _, app| {
                                    let mode = if *checked {
                                        knot_core::ActivationMode::Active
                                    } else {
                                        knot_core::ActivationMode::Passive
                                    };
                                    editor.update(app, |e, cx| {
                                        e.activation_mode = mode;
                                        cx.notify();
                                    });
                                }
                            }),
                    )
                    .child(div().child(knot_core::l10n::t(if is_active {
                        "agent_editor.active"
                    } else {
                        "agent_editor.passive"
                    }))),
            )
            .into_any_element(),
            Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.activation_hint"))
                .into_any_element(),
        ]
    }

    /// Registry metadata: what this agent is for, what it can be asked to
    /// do, and what asking costs. None of it affects how the agent
    /// launches, so editing it never restarts a running agent.
    fn registry_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        let editor = cx.entity();
        let cost_tier = self.cost_tier;
        vec![
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.description"),
                Input::new(&self.description_input).w(px(260.)),
            )
            .into_any_element(),
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.capabilities"),
                Input::new(&self.capabilities_input).w(px(260.)),
            )
            .into_any_element(),
            Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.capabilities_hint"))
                .into_any_element(),
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.cost_tier"),
                Button::new("agent-cost-tier-picker")
                    .label(cost_tier_label(cost_tier))
                    .dropdown_caret(true)
                    .dropdown_menu({
                        let editor = editor.clone();
                        move |mut menu, _, _| {
                            for tier in knot_core::CostTier::ALL {
                                let tier = *tier;
                                let editor = editor.clone();
                                menu = menu.item(
                                    PopupMenuItem::new(cost_tier_label(tier)).on_click(
                                        move |_, _, app| {
                                            editor.update(app, |e, cx| {
                                                e.cost_tier = tier;
                                                cx.notify();
                                            })
                                        },
                                    ),
                                );
                            }
                            menu
                        }
                    }),
            )
            .into_any_element(),
            Self::dialog_hint(cx, knot_core::l10n::t("agent_editor.cost_tier_hint"))
                .into_any_element(),
        ]
    }

    /// The folder row: the chosen path, or a prompt to choose one.
    fn folder_rows(&self, cx: &mut Context<Self>) -> Vec<gpui_kit::AnyElement> {
        vec![
            Self::dialog_row(
                knot_core::l10n::t("agent_editor.folder"),
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .max_w(px(220.))
                            .text_sm()
                            .whitespace_normal()
                            .text_color(cx.theme().muted_foreground)
                            .child(if self.folder_path.is_empty() {
                                knot_core::l10n::t("agent_editor.no_folder")
                            } else {
                                self.folder_path.clone()
                            }),
                    )
                    .child(
                        crate::controls::icon_button(
                            "choose-agent-folder",
                            "icons/folder-open.svg",
                            knot_core::l10n::t("agent_editor.choose_folder"),
                            false,
                        )
                        .on_click(cx.listener(|editor, _, _, cx| editor.choose_folder(cx))),
                    ),
            )
            .into_any_element(),
        ]
    }
}

/// The localized name of a cost tier.
fn cost_tier_label(tier: knot_core::CostTier) -> gpui_kit::SharedString {
    match tier {
        knot_core::CostTier::Low => knot_core::l10n::t("agent_editor.cost_tier_low"),
        knot_core::CostTier::Medium => knot_core::l10n::t("agent_editor.cost_tier_medium"),
        knot_core::CostTier::High => knot_core::l10n::t("agent_editor.cost_tier_high"),
    }.into()
}

impl Render for AgentEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let personas = persona_choices(&self.settings);
        let identity_rows = self.identity_rows(cx);
        let agent_rows = self.agent_rows(personas, cx);
        let registry_rows = self.registry_rows(cx);
        let folder_rows = self.folder_rows(cx);

        v_flex()
            .size_full()
            .gap_3()
            .px_5()
            .pt_5()
            .pb_6()
            .bg(cx.theme().background)
            .child(
                // Scrolls in place instead of pushing the action row (which
                // must stay visible) off the bottom of the window - the
                // folder path row can wrap to more than one line.
                div()
                    .id("new-agent-content")
                    .flex_1()
                    .overflow_y_scroll()
                    .child(
                        v_flex()
                            .gap_3()
                            .child(Self::dialog_section(cx, identity_rows))
                            .child(Self::dialog_section(cx, agent_rows))
                            .child(Self::dialog_section(cx, registry_rows))
                            .child(Self::dialog_section(cx, folder_rows))
                            .children(self.error.as_ref().map(|error| {
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().danger)
                                    .child(error.clone())
                            })),
                    ),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .justify_end()
                    .gap_2()
                    .child(
                        Button::new("cancel-agent-editor")
                            .label(knot_core::l10n::t("agent_editor.cancel"))
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("create-agent-editor")
                            .label(knot_core::l10n::t(if self.edit_target.is_some() {
                                "agent_editor.save"
                            } else {
                                "agent_editor.add"
                            }))
                            .primary()
                            .disabled(!self.can_submit(cx))
                            .on_click(cx.listener(|editor, _, window, cx| {
                                if editor.edit_target.is_some() {
                                    editor.save_edit(window, cx);
                                } else {
                                    editor.create(window, cx);
                                }
                            })),
                    ),
            )
            .children(crate::app_support::root_overlays(window, cx))
    }
}
