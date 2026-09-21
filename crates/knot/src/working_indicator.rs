use gpui_kit::{Div, ParentElement, Styled, div, hsla, px, rgb};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkingIndicatorState {
    Off,
    Working,
    Idle,
    AwaitingInput,
    Error,
}

pub(crate) fn state(agent_state: knot_agents::AgentState, is_running: bool)
                    -> WorkingIndicatorState {
    if !is_running {
        return WorkingIndicatorState::Off;
    }
    match agent_state {
        knot_agents::AgentState::Running => WorkingIndicatorState::Working,
        knot_agents::AgentState::Idle => WorkingIndicatorState::Idle,
        knot_agents::AgentState::Input => WorkingIndicatorState::AwaitingInput,
        knot_agents::AgentState::Error => WorkingIndicatorState::Error,
    }
}

/// How long one frame of the Working spinner is shown.
const SPINNER_FRAME_MILLIS: u128 = 180;

const SPINNER_MARKS: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Which frame of the Working spinner is current, as a monotonically
/// increasing count rather than an index into `SPINNER_MARKS`.
///
/// `render` reads the clock, so the spinner only advances when something
/// repaints. A surface that is not otherwise redrawing - the sidebar and
/// the dashboard, unlike the panel, redraw only on an event - has to drive
/// its own repaints, and needs to know when a repaint would actually change
/// anything. Comparing this count against the last one repaints at the
/// spinner's own cadence instead of at the poll's.
pub(crate) fn spinner_frame() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_millis()
    / SPINNER_FRAME_MILLIS
}

pub(crate) fn render(agent_state: knot_agents::AgentState, is_running: bool) -> Div {
    let state = state(agent_state, is_running);
    let (mark, color) = match state {
        WorkingIndicatorState::Off => ("", hsla(0., 0., 0., 0.)),
        WorkingIndicatorState::Working => {
            (SPINNER_MARKS[(spinner_frame() as usize) % SPINNER_MARKS.len()], rgb(0xF97316).into())
        }
        WorkingIndicatorState::Idle => ("-", rgb(0x6B7280).into()),
        WorkingIndicatorState::AwaitingInput => ("!", rgb(0x3B82F6).into()),
        WorkingIndicatorState::Error => ("×", rgb(0xEF4444).into()),
    };
    div().w(px(12.))
         .h(px(16.))
         .flex_shrink_0()
         .text_center()
         .text_color(color)
         .child(mark)
}

#[cfg(test)]
mod tests {
    use super::{SPINNER_FRAME_MILLIS, SPINNER_MARKS, WorkingIndicatorState, spinner_frame, state};

    #[test]
    fn spinner_frame_advances_once_per_frame_interval() {
        // The whole point of the count is that a repaint driver can tell
        // "nothing would change" from "the mark moved"; a frame that never
        // advances, or advances every millisecond, breaks that either way.
        let start = spinner_frame();
        std::thread::sleep(std::time::Duration::from_millis(SPINNER_FRAME_MILLIS as u64 * 2));
        let later = spinner_frame();
        assert!(later > start,
                "the spinner frame did not advance across two frame intervals");
        assert!(later - start <= 4,
                "the spinner advanced {} frames across two intervals",
                later - start);
    }

    #[test]
    fn every_spinner_frame_maps_to_a_mark() {
        for frame in 0..(SPINNER_MARKS.len() * 3) {
            assert!(!SPINNER_MARKS[frame % SPINNER_MARKS.len()].is_empty());
        }
    }

    #[test]
    fn maps_running_states_and_off_state() {
        assert_eq!(state(knot_agents::AgentState::Running, true),
                   WorkingIndicatorState::Working);
        assert_eq!(state(knot_agents::AgentState::Idle, true),
                   WorkingIndicatorState::Idle);
        assert_eq!(state(knot_agents::AgentState::Input, true),
                   WorkingIndicatorState::AwaitingInput);
        assert_eq!(state(knot_agents::AgentState::Error, true),
                   WorkingIndicatorState::Error);
        assert_eq!(state(knot_agents::AgentState::Idle, false),
                   WorkingIndicatorState::Off);
    }
}
