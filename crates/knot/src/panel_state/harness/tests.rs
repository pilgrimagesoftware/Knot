//! Both ways a replayed chunk is recognised as the harness's - the adapter's
//! origin tag, and a chunk made entirely of harness blocks - and the prompts
//! that must survive either.

use serde_json::json;

use super::UserChunk;
use super::classify_user_chunk;

/// A task notification as Claude Code stores it: a plain string, no origin
/// tag on the replayed chunk.
const NOTIFICATION: &str = "<task-notification>
<task-id>bdcwozo6g</task-id>
<status>failed</status>
<summary>Background command \"Watch PR 545 CI\"   failed
  with exit code 1</summary>
</task-notification>";

fn notice_with(summary: &str) -> UserChunk {
    UserChunk::Notice(knot_core::l10n::t_with("panel.harness.task_finished",
                                              &[("summary", summary)]))
}

#[test]
fn a_task_notification_becomes_a_notice_naming_its_summary() {
    assert_eq!(classify_user_chunk(NOTIFICATION, None),
               notice_with("Background command \"Watch PR 545 CI\" failed with exit code 1"));
}

#[test]
fn a_task_notification_without_a_summary_is_still_a_notice() {
    let text = "<task-notification><task-id>x</task-id></task-notification>";

    assert_eq!(classify_user_chunk(text, None),
               UserChunk::Notice(knot_core::l10n::t("panel.harness.task_finished_unnamed")));
}

/// Written for the model, never shown live either.
#[test]
fn a_chunk_of_system_reminders_is_hidden() {
    let text = "  <system-reminder>The date is today.</system-reminder>\n\
                <system-reminder>Be brief.</system-reminder>\n";

    assert_eq!(classify_user_chunk(text, None), UserChunk::Hidden);
}

#[test]
fn a_notification_beside_reminders_is_a_notice() {
    let text = format!("<system-reminder>x</system-reminder>\n{NOTIFICATION}");

    assert!(matches!(classify_user_chunk(&text, None), UserChunk::Notice(_)));
}

/// The acceptance case: naming the tag is not being one.
#[test]
fn a_prompt_that_mentions_a_harness_tag_is_the_users() {
    for text in ["Why did I see a <task-notification> in the panel?",
                 "Here is what it said:\n<task-notification>...</task-notification>",
                 "<task-notification>x</task-notification> - why was this shown?",
                 "<task-notification>never closed",
                 "<task-notifications>not the tag</task-notifications>",
                 "",
                 "fix the build"]
    {
        assert_eq!(classify_user_chunk(text, None),
                   UserChunk::Human,
                   "{text:?} is the user's");
    }
}

/// The adapter's tag wins over the text in both directions: it knows who
/// produced the message, where the text match can only infer it.
#[test]
fn a_non_human_origin_marks_a_chunk_injected_whatever_its_text() {
    let meta = json!({ "_claude/origin": { "kind": "task-notification" } });

    assert_eq!(classify_user_chunk(NOTIFICATION, Some(&meta)),
               notice_with("Background command \"Watch PR 545 CI\" failed with exit code 1"));
    assert_eq!(classify_user_chunk("the harness said this", Some(&meta)),
               UserChunk::Notice(knot_core::l10n::t("panel.harness.automated")),
               "an injected chunk in a shape not recognised is noted, not shown as written");
}

#[test]
fn a_human_origin_keeps_a_chunk_the_users_whatever_its_text() {
    for kind in ["human", "channel"] {
        let meta = json!({ "_claude/origin": { "kind": kind } });

        assert_eq!(classify_user_chunk(NOTIFICATION, Some(&meta)),
                   UserChunk::Human,
                   "a {kind} origin is a person's");
    }
}

/// `_meta` without an origin tag says nothing about who wrote the chunk, so
/// the text decides.
#[test]
fn meta_without_an_origin_falls_back_to_the_text() {
    let meta = json!({ "somethingElse": true });

    assert!(matches!(classify_user_chunk(NOTIFICATION, Some(&meta)),
                     UserChunk::Notice(_)));
    assert_eq!(classify_user_chunk("fix the build", Some(&meta)),
               UserChunk::Human);
}
