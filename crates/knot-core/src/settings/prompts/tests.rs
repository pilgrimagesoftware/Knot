use serde_json::json;

use super::*;

#[test]
fn prompt_refuses_blank_name_or_text() {
    assert!(Prompt::new("  ", "text").is_none());
    assert!(Prompt::new("name", "\n\t ").is_none());
    assert!(Prompt::new("name", "text").is_some());
}

#[test]
fn prompt_keeps_multi_line_text_verbatim() {
    let prompt = Prompt::new("gate", "one\ntwo\n  three").unwrap();
    let reloaded: Prompt = serde_json::from_str(&serde_json::to_string(&prompt).unwrap()).unwrap();
    assert_eq!(reloaded, prompt);
    assert_eq!(reloaded.text, "one\ntwo\n  three");
}

#[test]
fn startup_prompt_variants_round_trip() {
    let id = Uuid::new_v4();
    for value in [StartupPrompt::Library(id),
                  StartupPrompt::Custom("do it".into())]
    {
        let text = serde_json::to_string(&value).unwrap();
        assert_eq!(serde_json::from_str::<StartupPrompt>(&text).unwrap(), value);
    }
    assert_eq!(serde_json::to_value(StartupPrompt::Library(id)).unwrap(),
               json!({ "kind": "library", "value": id }));
}

#[test]
fn blank_custom_text_is_none() {
    assert_eq!(StartupPrompt::custom("   "), None);
    assert_eq!(StartupPrompt::custom("go"),
               Some(StartupPrompt::Custom("go".into())));
}
