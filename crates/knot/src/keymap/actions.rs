//! The configurable shortcuts' actions.
//!
//! The numbered families are nine unit actions each rather than one action
//! carrying an index. Moving a binding unbinds it by action *name*
//! (`gpui::Unbind`), and a payload action would give all nine one name, so
//! unbinding one digit would unbind every digit.

use gpui_kit::KeyBinding;
use gpui_kit::actions;

actions!(knot_app,
         [SelectWorkspace1,
          SelectWorkspace2,
          SelectWorkspace3,
          SelectWorkspace4,
          SelectWorkspace5,
          SelectWorkspace6,
          SelectWorkspace7,
          SelectWorkspace8,
          SelectWorkspace9,
          SelectAgent1,
          SelectAgent2,
          SelectAgent3,
          SelectAgent4,
          SelectAgent5,
          SelectAgent6,
          SelectAgent7,
          SelectAgent8,
          SelectAgent9,
          FocusAgentInput,
          ToggleDashboard,
          TogglePullRequests]);

/// Builds a context-less binding of one action to a chord string.
pub(crate) type BindFn = fn(&str) -> KeyBinding;

/// The workspace family's binders, index 0 for digit 1.
pub(crate) const SELECT_WORKSPACE: [BindFn; 9] = [|c| KeyBinding::new(c, SelectWorkspace1, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace2, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace3, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace4, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace5, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace6, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace7, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace8, None),
                                                  |c| KeyBinding::new(c, SelectWorkspace9, None)];

/// The agent family's binders, index 0 for digit 1.
pub(crate) const SELECT_AGENT: [BindFn; 9] = [|c| KeyBinding::new(c, SelectAgent1, None),
                                              |c| KeyBinding::new(c, SelectAgent2, None),
                                              |c| KeyBinding::new(c, SelectAgent3, None),
                                              |c| KeyBinding::new(c, SelectAgent4, None),
                                              |c| KeyBinding::new(c, SelectAgent5, None),
                                              |c| KeyBinding::new(c, SelectAgent6, None),
                                              |c| KeyBinding::new(c, SelectAgent7, None),
                                              |c| KeyBinding::new(c, SelectAgent8, None),
                                              |c| KeyBinding::new(c, SelectAgent9, None)];
