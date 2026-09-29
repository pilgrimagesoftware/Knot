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

fn config(options: &AdapterOptions) -> Value {
    options.env_config
           .as_ref()
           .map_or(Value::Null, |(_, config)| Value::Object(config.clone()))
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| (*word).to_owned()).collect()
}

#[test]
fn codex_flags_become_config_overrides() {
    let options = adapter_options("codex",
                                  "-m o3 --profile work -c model_reasoning_effort=high \
                                   -c features.web_search=true --enable plan_tool");
    assert_eq!(options.env_config.as_ref().map(|(name, _)| *name),
               Some("CODEX_CONFIG"));
    assert_eq!(config(&options),
               json!({ "model": "o3",
                       "profile": "work",
                       "model_reasoning_effort": "high",
                       "features": { "web_search": true, "plan_tool": true } }));
    assert!(options.args.is_empty(),
            "codex-acp never passes argv to Codex");
    assert!(options.ignored.is_empty());
}

#[test]
fn a_codex_override_value_is_read_as_toml_where_it_can_be() {
    let options = adapter_options("codex",
                                  r#"-c retries=3 -c 'sandbox_workspace_write.writable_roots=["/tmp"]'"#);
    assert_eq!(config(&options),
               json!({ "retries": 3,
                       "sandbox_workspace_write": { "writable_roots": ["/tmp"] } }));
}

/// The adapter sets approval and sandbox policy itself on every turn, so
/// these would be overwritten before they took effect.
#[test]
fn codex_flags_the_adapter_owns_are_ignored() {
    let options = adapter_options("codex", "-s read-only --full-auto -c noequals -m o3");
    assert_eq!(config(&options), json!({ "model": "o3" }));
    assert_eq!(options.ignored,
               words(&["-s", "read-only", "--full-auto", "-c noequals"]));
}

#[test]
fn opencode_model_and_agent_go_to_its_config_and_switches_to_its_argv() {
    let options = adapter_options("opencode",
                                  "--model anthropic/claude-sonnet-5 --agent plan --print-logs \
                                   --log-level DEBUG --continue");
    assert_eq!(options.env_config.as_ref().map(|(name, _)| *name),
               Some("OPENCODE_CONFIG_CONTENT"));
    assert_eq!(config(&options),
               json!({ "model": "anthropic/claude-sonnet-5", "default_agent": "plan" }));
    assert_eq!(options.args,
               words(&["--print-logs", "--log-level", "DEBUG"]));
    assert_eq!(options.ignored, words(&["--continue"]));
}

#[test]
fn gemini_takes_its_accepted_flags_on_its_command_line() {
    let options = adapter_options("gemini",
                                  "-m gemini-2.5-pro --approval-mode=auto_edit -y \
                                   --include-directories /tmp/a");
    assert_eq!(options.args,
               words(&["--model",
                       "gemini-2.5-pro",
                       "--approval-mode",
                       "auto_edit",
                       "--yolo",
                       "--include-directories",
                       "/tmp/a"]));
    assert_eq!(options.env_config, None);
    assert!(options.ignored.is_empty());
}

/// `-p` would turn the ACP session into a one-shot headless run, and a word
/// after a switch would reach Gemini as its first prompt.
#[test]
fn gemini_flags_that_change_the_session_and_stray_words_are_ignored() {
    let options = adapter_options("gemini", "-p hello --resume latest --sandbox stray --debug");
    assert_eq!(options.args, words(&["--sandbox"]));
    assert_eq!(options.ignored,
               words(&["-p", "hello", "--resume", "latest", "stray", "--debug"]));
}

#[test]
fn copilot_options_are_reported_not_passed() {
    let options = adapter_options("copilot", "--model gpt-5");
    assert_eq!(options.ignored, words(&["--model gpt-5"]));
    assert!(options.args.is_empty());
    assert_eq!(options.env_config, None);
}

#[test]
fn unbalanced_quotes_forward_nothing_for_any_adapter() {
    for agent_type in ["codex", "opencode", "gemini"] {
        let options = adapter_options(agent_type, r#"--model "o3"#);
        assert_eq!(options.ignored, words(&[r#"--model "o3"#]), "{agent_type}");
        assert_eq!(options.env_config, None, "{agent_type}");
        assert!(options.args.is_empty(), "{agent_type}");
    }
}

/// A flag overrides a config file; the instructions are merged in after,
/// so no option can displace them.
#[test]
fn options_layer_over_the_inherited_environment() {
    let options = adapter_options("codex", "-m o3 -c features.b=true");
    let inherited = r#"{"model":"gpt-5","features":{"a":true},"approval_policy":"never"}"#;
    let layered: Value = serde_json::from_str(&options.layered_env("CODEX_CONFIG",
                                                                   Some(inherited.to_owned()))
                                                      .expect("set")).expect("json");
    assert_eq!(layered,
               json!({ "model": "o3",
                       "features": { "a": true, "b": true },
                       "approval_policy": "never" }));
    assert_eq!(options.layered_env("OTHER", Some("x".to_owned()))
                      .as_deref(),
               Some("x"),
               "another variable passes through untouched");
    assert_eq!(AdapterOptions::default().layered_env("CODEX_CONFIG", None),
               None);
}
