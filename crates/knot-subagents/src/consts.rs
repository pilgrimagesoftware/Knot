/// How many finished or failed subagents one agent keeps before the oldest is
/// discarded.
///
/// Running records are never counted against this and never dropped, so the
/// cap can only ever hide work that is already over.
///
/// The section is a status view, not an audit log. A long autonomous turn can
/// dispatch dozens, and an unbounded `Vec` behind a mutex that a frame reads
/// is the wrong shape for one - the whole list is walked to build the header
/// summary on every repaint. Twenty is well past what fits on screen and far
/// short of what would cost anything to walk.
pub const MAX_FINISHED_PER_AGENT: usize = 20;
