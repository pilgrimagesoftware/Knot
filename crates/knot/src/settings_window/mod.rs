//! The settings window - one instance, tabbed, saving as you go
//! (`openspec/specs/settings-ui`).
//!
//! `window` owns the entity and opens it, `tab` names the panes, `render`
//! draws the frame around whichever pane is selected, and each pane's own
//! content lives under `panes`.

mod bench_editor;
mod controls;
/// Public within the crate because its rule is asserted directly by
/// `tests::settings_font_preview`; the panes themselves are not.
pub(crate) mod font;
mod keyboard;
mod panes;
mod persona_editor;
mod prompt_editor;
mod render;
mod tab;
mod window;

pub(crate) use tab::SettingsTab;
pub(crate) use window::SettingsWindow;
pub(crate) use window::open_settings_window;
