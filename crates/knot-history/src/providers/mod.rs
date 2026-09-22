//! The four history providers, and the reading they share.
//!
//! Each provider knows one agent's on-disk layout and nothing else. What
//! they had in common was the *scan*: walk a log's entries, find the first
//! one of the right kind, pull the message out of it, and turn that into a
//! title. That was written four times, differing only in field names, so it
//! lives here and each provider contributes the schema closure that says
//! where its own text is.

pub mod claude;
pub mod codex;
pub mod copilot;
pub mod gemini;

use serde_json::Value;

use crate::title::{extract_title, is_valid_title, truncate};

/// Every well-formed JSON object in a line-delimited log, in file order.
///
/// Blank lines and lines that do not parse are skipped rather than ending
/// the scan: a transcript is appended to while the agent runs, so the last
/// line of a live file is routinely half-written, and one bad line in the
/// middle is not a reason to lose the rest.
pub(crate) fn jsonl_entries(content: &str) -> impl Iterator<Item = Value> + '_ {
    content.lines()
           .map(str::trim)
           .filter(|line| !line.is_empty())
           .filter_map(|line| serde_json::from_str::<Value>(line).ok())
}

/// The first entry that yields a usable title.
///
/// `text_of` is the provider's schema: it returns the message text for an
/// entry of the kind that can title a session, and `None` for every other
/// entry - which is what makes "the first user message" mean the same thing
/// across four different file formats. An entry of the right kind whose text
/// is not a usable title (a registration prompt, an empty string) is skipped
/// like any other, so the scan continues to the next one.
pub(crate) fn first_title(entries: impl IntoIterator<Item = Value>,
                          text_of: impl Fn(&Value) -> Option<&str>)
                          -> Option<String> {
    entries.into_iter()
           .filter_map(|entry| {
               let text = text_of(&entry)?;
               if !is_valid_title(text) {
                   return None;
               }
               extract_title(text)
           })
           .next()
}

/// The title a provider recorded for a session, or the one its transcript
/// yields.
///
/// Three providers keep a summary of their own - Codex in its database,
/// Copilot in `workspace.yaml`, Gemini in `logs.json` - which is preferred
/// when it is usable because reading it costs nothing. `fallback` opens the
/// transcript, so it runs only when the recorded summary is missing or is
/// something like a registration prompt.
pub(crate) fn resolve_title(recorded: &str, fallback: impl FnOnce() -> Option<String>) -> String {
    if is_valid_title(recorded) {
        return truncate(recorded);
    }
    fallback().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// The text of a `user` entry, the shape three of the four providers
    /// reduce to once their field names are accounted for.
    fn user_text(entry: &Value) -> Option<&str> {
        if entry.get("type").and_then(Value::as_str) != Some("user") {
            return None;
        }
        entry.get("message").and_then(Value::as_str)
    }

    #[test]
    fn a_half_written_last_line_does_not_lose_the_rest() {
        let content = "{\"type\":\"user\"}\n\n{\"type\":\"assist";
        assert_eq!(jsonl_entries(content).count(), 1);
    }

    #[test]
    fn a_bad_line_in_the_middle_is_skipped_not_fatal() {
        let content = "{\"a\":1}\nnot json\n{\"b\":2}";
        let entries: Vec<_> = jsonl_entries(content).collect();
        assert_eq!(entries, vec![json!({"a": 1}), json!({"b": 2})]);
    }

    #[test]
    fn the_first_usable_entry_titles_the_session() {
        let entries = vec![json!({"type": "assistant", "message": "ignored"}),
                           json!({"type": "user", "message": "Fix the flaky test"}),
                           json!({"type": "user", "message": "later"})];
        assert_eq!(first_title(entries, user_text).as_deref(),
                   Some("Fix the flaky test"));
    }

    /// An entry of the right kind whose text cannot title anything is
    /// skipped like any other, rather than ending the scan - which is what
    /// lets a session opened by Knot's own registration prompt still be
    /// titled by the user's first real message.
    #[test]
    fn an_unusable_title_does_not_end_the_scan() {
        let registration = crate::title::is_registration_prompt;
        let prompt = "You are part of a team of agents working on this repository.";
        assert!(registration(prompt),
                "fixture must be a registration prompt");

        let entries = vec![json!({"type": "user", "message": prompt}),
                           json!({"type": "user", "message": ""}),
                           json!({"type": "user", "message": "Rename the crate"})];
        assert_eq!(first_title(entries, user_text).as_deref(),
                   Some("Rename the crate"));
    }

    #[test]
    fn a_recorded_summary_is_preferred_to_reading_the_transcript() {
        let title = resolve_title("Ship the release",
                                  || panic!("must not read the transcript"));
        assert_eq!(title, "Ship the release");
    }

    #[test]
    fn an_unusable_summary_falls_back_to_the_transcript() {
        assert_eq!(resolve_title("", || Some("From the log".to_string())),
                   "From the log");
        assert_eq!(resolve_title("", || None), "");
    }
}
