//! The artifact panel: the side panel holding an agent's markdown file and
//! its diagram at once (`openspec/specs/artifact-panel`).
//!
//! Ports `Skwad/Views/Artifacts/ArtifactPanelView.swift`. The two sections it
//! holds were ported first and live in
//! [`crate::workspace_window::panel::pane`]; what was missing was the container
//! around them - the panel's width, its expand toggle, the draggable divider
//! between the sections, and the collapse chevron on each.
//!
//! Before this existed the two panes were alternatives, tried markdown-first
//! in `render/content.rs`, so a diagram shown while a file was open was stored
//! and never drawn.
//!
//! [`layout`] is the arithmetic, pure and tested on its own, the way the Swift
//! reference's `sectionHeight` is a static function with its own suite.
//! [`state`] is the per-agent arrangement, which lives on the window and is
//! discarded with it.
//!
//! What the sections act on came later (issue #535): [`closed`] remembers
//! what the user closed so the panel can be reopened, [`document`] reads the
//! markdown file off the render path, [`review`] is the Approve / Review flow,
//! and [`controls`] draws the buttons for all three.

pub(super) mod closed;
pub(super) mod controls;
pub(super) mod document;
pub(super) mod layout;
pub(super) mod render;
pub(super) mod review;
pub(super) mod section;
pub(super) mod state;
