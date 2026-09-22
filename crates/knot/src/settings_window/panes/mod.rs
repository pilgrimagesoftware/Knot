//! One module per settings tab.
//!
//! Each owns its `render_*` method plus the label lookups, defaults and
//! setters only that tab uses - so a tab can be read, or changed, without
//! the other seven in view. The window's own state and its tab strip stay in
//! [`super`].

pub(super) mod appearance;
pub(super) mod autopilot;
pub(super) mod coding;
pub(super) mod general;
pub(super) mod mcp;
pub(super) mod personas;
pub(super) mod voice;
