use super::*;

fn extra_args(options: &AdapterOptions) -> Value {
    options.session_meta
           .as_ref()
           .and_then(|meta| meta.pointer("/claudeCode/options/extraArgs"))
           .cloned()
           .unwrap_or(Value::Null)
}

#[test]
fn the_reported_options_reach_claude() {
    // #501's own settings.
    let options = adapter_options("claude", "--dangerously-skip-permissions --remote-control");
    assert_eq!(extra_args(&options), json!({ "remote-control": null }));
    assert_eq!(options.mode, Some(claude_mode("bypassPermissions")));
    assert!(options.ignored.is_empty());
}

#[test]
fn a_flag_takes_the_word_after_it_or_after_its_equals_sign() {
    let options = adapter_options("claude", "--model opus --add-dir=/tmp/a --verbose");
    assert_eq!(extra_args(&options),
               json!({ "model": "opus", "add-dir": "/tmp/a", "verbose": null }));
}

#[test]
fn quoted_values_stay_whole() {
    let options = adapter_options("claude", r#"--append-system-prompt "be brief""#);
    assert_eq!(extra_args(&options),
               json!({ "append-system-prompt": "be brief" }));
}

#[test]
fn a_permission_mode_in_any_accepted_spelling_is_the_starting_mode() {
    for (text, id) in [("--permission-mode plan", "plan"),
                       ("--permission-mode=acceptEdits", "acceptEdits"),
                       ("--permission-mode Manual", "default"),
                       ("--permission-mode bypass", "bypassPermissions")]
    {
        let options = adapter_options("claude", text);
        assert_eq!(options.mode, Some(claude_mode(id)), "{text}");
        assert_eq!(options.session_meta, None, "{text}");
    }
}

#[test]
fn an_unknown_permission_mode_is_ignored() {
    let options = adapter_options("claude", "--permission-mode yolo");
    assert_eq!(options.mode, None);
    assert_eq!(options.ignored, vec!["--permission-mode".to_owned()]);
}

#[test]
fn flags_the_adapter_owns_are_not_forwarded() {
    let options = adapter_options("claude", "--output-format=text --resume --model sonnet");
    assert_eq!(extra_args(&options), json!({ "model": "sonnet" }));
    assert_eq!(options.ignored,
               vec!["--output-format=text".to_owned(), "--resume".to_owned()]);
}

#[test]
fn short_flags_and_stray_words_are_ignored() {
    let options = adapter_options("claude", "-c stray");
    assert_eq!(options.session_meta, None);
    assert_eq!(options.ignored, vec!["-c".to_owned(), "stray".to_owned()]);
}

#[test]
fn unbalanced_quotes_forward_nothing() {
    let options = adapter_options("claude", r#"--model "opus"#);
    assert_eq!(options.session_meta, None);
    assert_eq!(options.ignored, vec![r#"--model "opus"#.to_owned()]);
}

#[test]
fn empty_options_ask_nothing() {
    assert_eq!(adapter_options("claude", "  "), AdapterOptions::default());
    assert_eq!(adapter_options("codex", ""), AdapterOptions::default());
}

#[test]
fn another_type_reports_its_options_unforwarded() {
    let options = adapter_options("codex", "--full-auto");
    assert_eq!(options.session_meta, None);
    assert_eq!(options.mode, None);
    assert_eq!(options.ignored, vec!["--full-auto".to_owned()]);
}
