//! The agent editor dialog - creating an agent, and editing one.
//!
//! `fields` holds the form's values and the rules that turn them into an
//! agent's data, `window` owns the dialog and its state, `submit` is what
//! pressing the button does, `pickers` are the two native choosers, and
//! `render` draws it.

mod fields;
mod pickers;
mod render;
mod submit;
mod window;

pub(crate) use fields::AgentPrefill;
pub(crate) use fields::created_agent_type;
pub(crate) use fields::parse_capability_tags;
pub(crate) use fields::persona_choices;
pub(crate) use window::AgentEditor;
pub(crate) use window::AgentEditorRequest;
pub(crate) use window::open_agent_editor;
