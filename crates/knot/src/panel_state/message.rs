//! What the panel's message list is made of: one entry per prompt, streamed
//! response, tool-call card or reported failure.

use knot_acp::ToolCallContent;
use knot_processes::{ShellRunState, ShellStatus};
use serde_json::Value;
use uuid::Uuid;

/// Pretty-prints a legacy `tool_call_result` payload for display, falling
/// back to its compact form if it somehow can't be re-serialized.
pub(super) fn render_json(output: &Value) -> String {
    serde_json::to_string_pretty(output).unwrap_or_else(|_| output.to_string())
}

/// One entry in the panel's message list: a user-sent prompt, streamed
/// assistant text, a tool call's card, or a failure the session reported
/// out of band - kept as distinct variants per `acp-panel-ui`'s "visually
/// distinguish user messages, assistant messages, and system/tool
/// content" requirement.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelMessage {
    User(String),
    Assistant(String),
    /// Boxed because this variant is far larger than the other three, and a
    /// long conversation holds thousands of them: an unboxed card made every
    /// plain text entry in the list cost the card's width. Carrying the two
    /// identification fields is what pushed it past the point where that
    /// mattered, but the imbalance predates them.
    ToolCall(Box<ToolCallCard>),
    /// A shell command the *user* ran from the prompt with a leading `!`,
    /// per `panel-shell-passthrough`. Deliberately not a `ToolCall`: that
    /// variant means the agent asked for something and the panel is
    /// reporting it, and the whole point of this entry is that the reader
    /// can tell at a glance which of them ran it.
    Shell(ShellCard),
    /// Something the session could not do: a refused prompt, a send that
    /// never reached the agent. These arrive as a JSON-RPC error response
    /// rather than a session update, so nothing in the event stream
    /// records them - without this entry the only trace of, say, an
    /// exhausted model quota was a line on stderr the user never sees.
    Error(String),
}

/// Where a finished command's output stands on its way to the agent.
///
/// A closed vocabulary because each state draws differently and only one of
/// them may be attached to a prompt. The card carries this rather than the
/// pending list owning it alone: the entry has to say which it is, and a
/// second place to look would be a second place to get it wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellDelivery {
    /// Nothing to hand over: still running, or it did not run to its own
    /// end, or the user discarded it.
    None,
    /// Ran to its own end and is waiting for the next prompt to carry it.
    Pending,
    /// Went to the agent with a prompt.
    Shared,
}

/// One `!` command's entry: what ran, where, what it produced, and whether
/// the agent has been told.
///
/// Plain data, copied from the live run rather than borrowing it. A
/// `PanelMessage` is cloned and compared once per frame, so a lock in here
/// would put the render path behind the command's drain threads - which is
/// the one thing `no I/O on the render path` exists to prevent.
#[derive(Debug, Clone, PartialEq)]
pub struct ShellCard {
    /// Identifies the card so an off-thread update can find it again.
    pub id:       Uuid,
    /// The command as the user typed it, without the `!`.
    pub command:  String,
    /// The folder it ran in - the agent's own, shown because a command's
    /// meaning depends entirely on where it ran.
    pub cwd:      String,
    /// Captured output and status, as of the last poll.
    pub run:      ShellRunState,
    pub delivery: ShellDelivery,
}

impl ShellCard {
    /// A card for a command that has just been submitted.
    pub fn starting(id: Uuid, command: String, cwd: String, run: ShellRunState) -> Self {
        Self { id,
               command,
               cwd,
               run,
               delivery: ShellDelivery::None }
    }

    pub fn status(&self) -> &ShellStatus {
        &self.run.status
    }

    /// Whether the command is still going.
    pub fn is_running(&self) -> bool {
        !self.run.status.is_terminal()
    }

    /// Whether its result is waiting to ride the next prompt.
    pub fn is_pending(&self) -> bool {
        self.delivery == ShellDelivery::Pending
    }

    /// Takes the new state of the run, moving to `Pending` if this is the
    /// moment it finished on its own.
    ///
    /// The promotion happens here, where the status change is observed, so
    /// there is one rule for what may be shared rather than one per caller.
    pub fn absorb(&mut self, run: ShellRunState) {
        let became_shareable = self.delivery == ShellDelivery::None
                               && !self.run.status.is_terminal()
                               && run.status.ran_to_completion();

        self.run = run;

        if became_shareable {
            self.delivery = ShellDelivery::Pending;
        }
    }

    /// Drops a pending result without touching what is on screen.
    pub fn discard(&mut self) {
        if self.delivery == ShellDelivery::Pending {
            self.delivery = ShellDelivery::None;
        }
    }

    /// Records that the result went to the agent.
    pub fn mark_shared(&mut self) {
        self.delivery = ShellDelivery::Shared;
    }
}

/// A tool call's rendered state: `kind` (execute, read, edit, ...), the
/// agent's human-readable `title`, its lifecycle `status`, and whatever
/// content has arrived so far - per the "Turn ends mid tool call"
/// scenario, the last known state is kept, never dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolCallCard {
    pub id:        String,
    pub kind:      String,
    pub title:     String,
    /// `pending`, `in_progress`, `completed` or `failed` - defaulted to
    /// `pending` at start rather than left unknown, per the ACP spec.
    pub status:    String,
    /// Output blocks as they arrive. A later update's `content` replaces
    /// this wholesale (the spec's updates carry the full current content,
    /// not a delta); an update with no `content` at all leaves it alone.
    pub content:   Vec<ToolCallContent>,
    /// The call's `rawInput`, verbatim, when the agent sent one.
    ///
    /// Not rendered. It is here because `kind` and `title` cannot say what
    /// tool ran - `kind` is an icon hint, `title` is prose - so this and
    /// [`Self::meta`] are the only fields a recognizer can read to tell a
    /// delegation from an ordinary call. See `knot_subagents::recognize`.
    pub raw_input: Option<Value>,
    /// The call's `_meta` envelope, verbatim, vendor keys included. Also not
    /// rendered, and for the same reason.
    pub meta:      Option<Value>,
}

impl ToolCallCard {
    /// Whether the call has finished, either way - the renderer shows an
    /// in-progress placeholder only while this is false.
    pub fn is_finished(&self) -> bool {
        matches!(self.status.as_str(), "completed" | "failed")
    }

    // Completes the status trio with `is_finished`/`succeeded`; the renderer
    // happens to need only the other two.
    #[allow(dead_code)]
    pub fn failed(&self) -> bool {
        self.status == "failed"
    }

    /// Whether the call finished without failing - the one status that
    /// collapses by default, per `acp-panel-ui`'s "A finished tool call
    /// collapses to its header" requirement.
    pub fn succeeded(&self) -> bool {
        self.status == "completed"
    }
}
