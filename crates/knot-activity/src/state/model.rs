//! The automatic status state machine: pure, clock-injected rules over
//! `tokio::time::Instant`. The tokio actor in [`crate::tracker`] owns the
//! timers and calls these methods with `Instant::now()`.

use std::time::Duration;

use knot_agents::AgentState;
use tokio::time::Instant;

use crate::consts::{
    DEFAULT_IDLE_TIMEOUT, HOOK_FALLBACK_IDLE_TIMEOUT, REGISTRATION_FIRST_IDLE_DELAY_LONG,
    REGISTRATION_FIRST_IDLE_DELAY_SHORT, REGISTRATION_SUBSEQUENT_IDLE_DELAY,
    USER_INPUT_IDLE_TIMEOUT,
};
use crate::events::{ActivitySource, Effect, KeyEvent, StatusEvent};
use crate::tracking::ActivityTracking;

/// Configuration for one tracker, fixed for the agent's lifetime.
#[derive(Debug, Clone)]
pub struct TrackerConfig {
    /// Agent type this tracker is attached to.
    pub agent_type:              String,
    /// Idle timeout after terminal output for non-hook agents.
    pub idle_timeout:            Duration,
    /// Idle timeout after a user keystroke, and the guard window.
    pub user_input_idle_timeout: Duration,
    /// Terminal-output idle timeout for hook-managed agents.
    pub hook_fallback_timeout:   Duration,
    /// Hook-managed agent: keystrokes do not drive the state machine.
    pub is_hook_based:           bool,
    /// MCP server enabled. When false, no registration gating.
    pub mcp_enabled:             bool,
    /// Agent registers through CLI arguments; skip deferred registration.
    pub inline_registration:     bool,
    /// Slow-starting agent: use the long first-idle registration delay.
    pub slow_startup:            bool,
    /// Registration delay after the first idle for fast-starting agents.
    pub first_idle_delay_short:  Duration,
    /// Registration delay after the first idle for slow-starting agents.
    pub first_idle_delay_long:   Duration,
    /// Registration delay after subsequent idles.
    pub subsequent_idle_delay:   Duration,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self { agent_type:              String::new(),
               idle_timeout:            DEFAULT_IDLE_TIMEOUT,
               user_input_idle_timeout: USER_INPUT_IDLE_TIMEOUT,
               hook_fallback_timeout:   HOOK_FALLBACK_IDLE_TIMEOUT,
               is_hook_based:           false,
               mcp_enabled:             true,
               inline_registration:     false,
               slow_startup:            false,
               first_idle_delay_short:  REGISTRATION_FIRST_IDLE_DELAY_SHORT,
               first_idle_delay_long:   REGISTRATION_FIRST_IDLE_DELAY_LONG,
               subsequent_idle_delay:   REGISTRATION_SUBSEQUENT_IDLE_DELAY, }
    }
}

impl TrackerConfig {
    /// Defaults for `agent_type`. Hook flags and MCP wiring are set by the
    /// caller once they are known.
    pub fn for_agent_type(agent_type: impl Into<String>) -> Self {
        Self { agent_type: agent_type.into(),
               ..Self::default() }
    }
}

/// An armed idle timer: when to fire and the window it was armed with.
#[derive(Debug, Clone)]
struct IdleTimer {
    at:     Instant,
    window: Duration,
}

/// The automatic status model for one agent.
#[derive(Debug, Clone)]
pub struct ActivityState {
    cfg:                     TrackerConfig,
    tracking:                ActivityTracking,
    status:                  AgentState,
    changed_at:              Instant,
    last_activity_at:        Option<Instant>,
    last_activity_source:    ActivitySource,
    idle_due:                Option<IdleTimer>,
    guard_until:             Option<Instant>,
    has_become_idle:         bool,
    idle_count:              u64,
    registration_ready_at:   Option<Instant>,
    registration_attempted:  bool,
    registration_prompt:     Option<String>,
    did_inject_registration: bool,
}

impl ActivityState {
    /// Starts in Idle with nothing tracked.
    pub fn new(cfg: TrackerConfig, tracking: ActivityTracking) -> Self {
        Self { cfg,
               tracking,
               status: AgentState::Idle,
               changed_at: Instant::now(),
               last_activity_at: None,
               last_activity_source: ActivitySource::Terminal,
               idle_due: None,
               guard_until: None,
               has_become_idle: false,
               idle_count: 0,
               registration_ready_at: None,
               registration_attempted: false,
               registration_prompt: None,
               did_inject_registration: false }
    }

    /// The current status.
    pub fn status(&self) -> AgentState {
        self.status
    }

    /// Time of the last status change, for dashboard sorting.
    pub fn changed_at(&self) -> Instant {
        self.changed_at
    }

    /// When the armed idle timer fires, if any.
    pub fn idle_deadline(&self) -> Option<Instant> {
        self.idle_due.as_ref().map(|t| t.at)
    }

    /// When the input-protection guard expires, if armed.
    pub fn guard_deadline(&self) -> Option<Instant> {
        self.guard_until
    }

    /// When the registration prompt may next be evaluated, if scheduled and
    /// not yet consumed.
    pub fn registration_deadline(&self) -> Option<Instant> {
        (!self.registration_attempted).then_some(self.registration_ready_at)
                                      .flatten()
    }

    /// Schedule deferred registration, gated on MCP being enabled and the
    /// agent not registering inline. No-op otherwise.
    pub fn start(&mut self, now: Instant) {
        self.schedule_registration(self.first_idle_delay(), now);
    }

    /// Downgrade (or otherwise change) the tracked activity sources at runtime.
    pub fn set_tracking(&mut self, tracking: ActivityTracking) {
        self.tracking = tracking;
    }

    /// Set the registration prompt text to inject once conditions are met.
    pub fn set_registration_prompt(&mut self, prompt: String) {
        self.registration_prompt = Some(prompt);
    }

    /// Terminal output observed.
    pub fn on_terminal_activity(&mut self, now: Instant) -> Vec<Effect> {
        // Always cancel a pending idle timer, even when terminal output is not
        // being tracked or the agent is awaiting input.
        self.idle_due = None;
        if self.status == AgentState::Input
           || !self.tracking.contains(ActivityTracking::TERMINAL_OUTPUT)
        {
            return Vec::new();
        }
        self.last_activity_at = Some(now);
        self.last_activity_source = ActivitySource::Terminal;
        let window = self.terminal_idle_timeout();
        self.idle_due = Some(IdleTimer { at: now + window,
                                         window });
        let mut effects = Vec::new();
        self.set_status(AgentState::Running,
                        ActivitySource::Terminal,
                        now,
                        &mut effects);
        effects
    }

    /// A significant user keystroke observed.
    pub fn on_user_input(&mut self, now: Instant, key: KeyEvent) -> Vec<Effect> {
        if !self.tracking.contains(ActivityTracking::USER_INPUT) {
            return Vec::new();
        }
        self.last_activity_at = Some(now);
        self.last_activity_source = ActivitySource::UserInput;
        self.guard_until = Some(now + self.cfg.user_input_idle_timeout);

        let mut effects = Vec::new();
        if self.status == AgentState::Input {
            // Awaiting-input is exited only by a keystroke.
            match key {
                KeyEvent::Return => {
                    self.idle_due = None;
                    self.set_status(AgentState::Running,
                                    ActivitySource::UserInput,
                                    now,
                                    &mut effects);
                }
                KeyEvent::Escape => {
                    self.idle_due = None;
                    self.mark_idle(now, ActivitySource::UserInput, &mut effects);
                }
                KeyEvent::Other => {}
            }
        }
        else if !self.cfg.is_hook_based {
            let window = self.cfg.user_input_idle_timeout;
            self.idle_due = Some(IdleTimer { at: now + window,
                                             window });
            self.set_status(AgentState::Running,
                            ActivitySource::UserInput,
                            now,
                            &mut effects);
        }
        effects
    }

    /// A hook reported a status authoritatively, carrying an optional message
    /// for the awaiting-input notification.
    pub fn apply_hook_status(&mut self, now: Instant, status: AgentState,
                             message: Option<String>)
                             -> Vec<Effect> {
        // Any hook status cancels the input-protection guard.
        self.guard_until = None;
        let mut effects = Vec::new();
        if status == AgentState::Idle {
            self.mark_idle(now, ActivitySource::Hook, &mut effects);
        }
        else {
            self.set_status(status, ActivitySource::Hook, now, &mut effects);
            if status == AgentState::Input {
                effects.push(Effect::AwaitingInput(message));
            }
        }
        effects
    }

    /// An ACP session event drove the status for a Panel-mode agent: turn
    /// start (Running), turn end with no pending permission (Idle), a
    /// permission request (Input, carrying its message), or a session
    /// error (Error). No idle timer or input-protection guard is involved,
    /// since these transitions are driven directly by ACP events, never by
    /// inferred silence.
    pub fn apply_acp_status(&mut self, now: Instant, status: AgentState, message: Option<String>)
                            -> Vec<Effect> {
        self.guard_until = None;
        self.idle_due = None;
        let mut effects = Vec::new();
        if status == AgentState::Idle {
            self.mark_idle(now, ActivitySource::Acp, &mut effects);
        }
        else {
            self.set_status(status, ActivitySource::Acp, now, &mut effects);
            if status == AgentState::Input {
                effects.push(Effect::AwaitingInput(message));
            }
        }
        effects
    }

    /// The terminal process exited.
    pub fn on_process_exit(&mut self, now: Instant, exit_code: Option<i32>) -> Vec<Effect> {
        self.idle_due = None;
        self.guard_until = None;
        let mut effects = Vec::new();
        let status = match exit_code {
            Some(code) if code != 0 => AgentState::Error,
            _ => AgentState::Idle,
        };
        self.set_status(status, ActivitySource::Terminal, now, &mut effects);
        if status == AgentState::Idle {
            self.mark_idle(now, ActivitySource::Terminal, &mut effects);
        }
        effects
    }

    /// The armed idle timer fired.
    pub fn idle_timer_fired(&mut self, now: Instant) -> Vec<Effect> {
        if self.status == AgentState::Input {
            // Idle timers never move an awaiting-input agent.
            self.idle_due = None;
            return Vec::new();
        }
        let Some(IdleTimer { at, window }) = self.idle_due.take()
        else {
            return Vec::new();
        };
        let since = self.last_activity_at.unwrap_or(at);
        if now.saturating_duration_since(since) >= window {
            self.idle_due = None;
            let mut effects = Vec::new();
            self.mark_idle(now, self.last_activity_source, &mut effects);
            effects
        }
        else {
            // Newer activity exists; reschedule for the remaining interval.
            self.idle_due = Some(IdleTimer { at: since + window,
                                             window });
            Vec::new()
        }
    }

    /// The input-protection guard expired.
    pub fn guard_expired(&mut self, now: Instant) -> Vec<Effect> {
        let Some(until) = self.guard_until
        else {
            return Vec::new();
        };
        if now < until {
            return Vec::new();
        }
        self.guard_until = None;
        let mut effects = Vec::new();
        if let Some(inject) = self.maybe_inject_registration(now) {
            effects.push(inject);
        }
        effects.push(Effect::CheckMessages);
        effects
    }

    /// The scheduled registration time arrived.
    pub fn registration_ready_fired(&mut self, now: Instant) -> Vec<Effect> {
        let Some(ready) = self.registration_ready_at
        else {
            return Vec::new();
        };
        if now < ready || self.registration_attempted {
            return Vec::new();
        }
        self.registration_attempted = true;
        self.maybe_inject_registration(now).into_iter().collect()
    }

    fn terminal_idle_timeout(&self) -> Duration {
        if self.cfg.is_hook_based {
            self.cfg.hook_fallback_timeout
        }
        else {
            self.cfg.idle_timeout
        }
    }

    fn first_idle_delay(&self) -> Duration {
        if self.cfg.slow_startup {
            self.cfg.first_idle_delay_long
        }
        else {
            self.cfg.first_idle_delay_short
        }
    }

    fn registration_enabled(&self) -> bool {
        self.cfg.mcp_enabled && !self.cfg.inline_registration
    }

    fn schedule_registration(&mut self, delay: Duration, now: Instant) {
        if self.registration_enabled() && !self.did_inject_registration {
            self.registration_ready_at = Some(now + delay);
            self.registration_attempted = false;
        }
    }

    fn maybe_inject_registration(&mut self, now: Instant) -> Option<Effect> {
        if self.did_inject_registration || !self.registration_enabled() {
            return None;
        }
        if !self.has_become_idle {
            return None;
        }
        let ready = self.registration_ready_at?;
        if now < ready {
            return None;
        }
        if self.guard_until.is_some() {
            return None;
        }
        let prompt = self.registration_prompt.take()?;
        self.did_inject_registration = true;
        self.registration_ready_at = None;
        Some(Effect::InjectRegistration(prompt))
    }

    /// Transition to Idle: checks unread messages and schedules the next
    /// registration evaluation only on a real transition out of another state.
    fn mark_idle(&mut self, now: Instant, source: ActivitySource, effects: &mut Vec<Effect>) {
        let was_idle = self.status == AgentState::Idle;
        self.set_status(AgentState::Idle, source, now, effects);
        if was_idle {
            return;
        }
        self.has_become_idle = true;
        if !self.did_inject_registration && self.registration_enabled() {
            self.idle_count += 1;
            let delay = if self.idle_count == 1 {
                self.first_idle_delay()
            }
            else {
                self.cfg.subsequent_idle_delay
            };
            self.schedule_registration(delay, now);
        }
        effects.push(Effect::CheckMessages);
    }

    fn set_status(&mut self, status: AgentState, source: ActivitySource, now: Instant,
                  effects: &mut Vec<Effect>) {
        if self.status == status {
            return;
        }
        self.status = status;
        self.changed_at = now;
        effects.push(Effect::Status(StatusEvent { status,
                                                  source,
                                                  changed_at: now }));
    }
}

#[cfg(test)]
mod tests;
