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

pub(crate) fn render(agent_state: knot_agents::AgentState, is_running: bool) -> Div {
    let state = state(agent_state, is_running);
    let (mark, color) = match state {
        WorkingIndicatorState::Off => ("", hsla(0., 0., 0., 0.)),
        WorkingIndicatorState::Working => {
            let marks = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                                     .unwrap_or_default()
                                                     .as_millis();
            (marks[(millis / 180) as usize % marks.len()], rgb(0xF97316).into())
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
    use super::{WorkingIndicatorState, state};

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
