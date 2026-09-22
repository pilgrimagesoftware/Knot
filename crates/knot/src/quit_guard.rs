//! The one boundary every application quit request passes through.
//! Contract: `openspec/specs/quit-warning/spec.md`.
//!
//! Quitting while an agent works kills its command mid-flight, so a quit
//! request is checked against the agent store before it reaches
//! `App::quit`. It is checked *here* rather than at each menu item
//! because gpui's `on_app_quit` runs too late to cancel anything: by the
//! time it fires the decision is made. The `Quit` action is the last
//! point that can still say no, and every user-facing quit path - the
//! application menu's Quit Knot item and the `cmd-q` binding - dispatches
//! it.
//!
//! One path deliberately stays outside: `quit_on_terminal_signals` exits
//! the process from a signal handler off the main thread, which cannot
//! reach the app to show anything. Ctrl-C at the launching terminal is a
//! kill, not a quit request, and is expected to be abrupt.

use std::sync::Arc;

use gpui_kit::App;
use gpui_kit::component::WindowExt;
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use parking_lot::Mutex;

use crate::working_indicator;

/// The agent store the guard consults, plus the one-shot bypass the
/// confirmation sets.
///
/// A global because an action handler is a plain `fn(&Quit, &mut App)`
/// with nowhere to carry the store, the same reason `AgentsMenuState` is
/// one. Defaults to no store: a quit arriving before `run` installs one
/// has nothing to protect and must not be blocked.
#[derive(Default)]
pub(crate) struct QuitGuard {
    pub(crate) agents: Option<Arc<Mutex<knot_agents::AgentStore>>>,
    /// Set by the warning's confirm action and consumed by the next quit
    /// request, so the confirmed quit passes straight through instead of
    /// meeting the same warning it just answered.
    confirmed:         bool,
}

impl gpui_kit::Global for QuitGuard {}

impl QuitGuard {
    pub(crate) fn new(agents: Arc<Mutex<knot_agents::AgentStore>>) -> Self {
        Self { agents:    Some(agents),
               confirmed: false, }
    }
}

/// Whether one agent counts as working.
///
/// Delegates to [`working_indicator::state`] rather than matching
/// `AgentState` here, so "working" means exactly what the spinner in the
/// sidebar and on the dashboard means. An agent that is not activated has
/// no session to lose no matter what state it last reported, which is why
/// the flag is part of the question.
pub(crate) fn is_working(state: knot_agents::AgentState, activated: bool) -> bool {
    working_indicator::state(state, activated) == working_indicator::WorkingIndicatorState::Working
}

/// How many of `agents` are working.
pub(crate) fn working_agent_count(agents: &[knot_agents::Agent]) -> usize {
    agents.iter()
          .filter(|agent| is_working(agent.state, agent.activated))
          .count()
}

/// The working count the guard will decide on.
///
/// A store that is missing or whose lock is poisoned reports zero: the
/// guard's job is to protect work, and it cannot claim work exists when
/// it cannot see any. Refusing to quit on a poisoned lock would leave the
/// user unable to close the app at all.
fn working_agents(cx: &App) -> usize {
    let Some(guard) = cx.try_global::<QuitGuard>()
    else {
        return 0;
    };
    let Some(agents) = guard.agents.as_ref()
    else {
        return 0;
    };
    working_agent_count(agents.lock().agents())
}

/// Takes the one-shot bypass, clearing it.
fn take_confirmed(cx: &mut App) -> bool {
    if cx.try_global::<QuitGuard>().is_none() {
        return false;
    }
    let confirmed = cx.global::<QuitGuard>().confirmed;
    if confirmed {
        cx.global_mut::<QuitGuard>().confirmed = false;
    }
    confirmed
}

/// Arms the one-shot bypass for the quit request the confirmation makes.
fn confirm(cx: &mut App) {
    if cx.try_global::<QuitGuard>().is_none() {
        cx.set_global(QuitGuard::default());
    }
    cx.global_mut::<QuitGuard>().confirmed = true;
}

/// The warning's title.
pub(crate) fn warning_title() -> String {
    knot_core::l10n::t("quit.working_title")
}

/// The warning's body for `count` working agents.
///
/// One agent is named in words rather than as "1 agent", which reads as
/// though a number were being reported when there is nothing to count.
pub(crate) fn warning_message(count: usize) -> String {
    if count == 1 {
        knot_core::l10n::t("quit.working_message_one")
    }
    else {
        knot_core::l10n::t_with("quit.working_message_many",
                                &[("count", &count.to_string())])
    }
}

/// The label that goes through with the quit.
pub(crate) fn confirm_label() -> String {
    knot_core::l10n::t("quit.working_confirm")
}

/// The label that abandons the quit.
pub(crate) fn cancel_label() -> String {
    knot_core::l10n::t("quit.working_cancel")
}

/// What a quit request resolves to once the guard has looked at it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitDecision {
    /// Quit now: nothing is working, or the user already said so.
    Quit,
    /// Hold the quit and warn about this many working agents.
    Warn(usize),
}

/// The decision alone, with the bypass consumed if it was set.
///
/// Split from [`request_quit`] so the rule - bypass first, then the
/// working count - can be asserted without a window to open a dialog on.
pub(crate) fn decide(cx: &mut App) -> QuitDecision {
    if take_confirmed(cx) {
        return QuitDecision::Quit;
    }
    match working_agents(cx) {
        0 => QuitDecision::Quit,
        count => QuitDecision::Warn(count),
    }
}

/// The guarded quit entry point: warns if agents are working, otherwise
/// shuts down.
pub(crate) fn request_quit(cx: &mut App) {
    let count = match decide(cx) {
        QuitDecision::Quit => return shutdown(cx),
        QuitDecision::Warn(count) => count,
    };
    // Falls back to any open window, as About Knot does: `active_window`
    // can be empty at the moment the menu fires.
    let window = cx.active_window().or_else(|| cx.windows().first().copied());
    let Some(window) = window
    else {
        // No window means no way to ask. Quitting anyway is the lesser
        // harm: the alternative is an app that refuses to close and
        // cannot say why.
        eprintln!("quit guard: {count} agents are working, but there is no window to warn on");
        return shutdown(cx);
    };
    // Deferred for the same reason About Knot's dialog is: the menu runs
    // this inside the active window's update, and a re-entrant
    // `window.update` is reported as a missing window.
    cx.defer(move |cx| {
          let result = window.update(cx, |_, window, cx| {
                                 window.open_alert_dialog(cx, move |alert, _, _| {
                                           alert.title(warning_title())
                    .description(warning_message(count))
                    .button_props(DialogButtonProps::default().ok_text(confirm_label())
                                                              .ok_variant(ButtonVariant::Danger)
                                                              .cancel_text(cancel_label())
                                                              .show_cancel(true))
                    .on_ok(|_, _, cx| {
                        // Arm the bypass, then re-enter the same
                        // guarded path: the request the user confirmed
                        // is the one that goes through, and it takes
                        // the bypass on the way, so the warning cannot
                        // open twice.
                        confirm(cx);
                        request_quit(cx);
                        true
                    })
                                       });
                             });
          if let Err(error) = result {
              eprintln!("quit guard: the window went away before the warning opened: {error}");
          }
      });
}

/// The normal shutdown path, unguarded. Every caller reaches it through
/// [`request_quit`].
fn shutdown(cx: &mut App) {
    cx.quit();
}

#[cfg(test)]
pub(crate) fn set_confirmed_for_test(cx: &mut App) {
    confirm(cx);
}
