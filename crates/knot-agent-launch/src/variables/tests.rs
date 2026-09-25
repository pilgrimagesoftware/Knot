use super::*;

fn context() -> PromptContext {
    PromptContext { agent_name: "Knot 3".into(),
                    agent_id:   "b7f0".into(),
                    agent_type: "claude".into(),
                    folder:     "/src/knot".into(),
                    workspace:  "Main".into(),
                    branch:     "471-restart".into(),
                    date:       "2026-09-25".into(), }
}

#[test]
fn every_name_round_trips_and_names_are_case_sensitive() {
    for variable in PromptVariable::ALL {
        assert_eq!(variable.name().parse::<PromptVariable>(), Ok(variable));
        assert_eq!(variable.to_string(), variable.name());
    }
    assert_eq!("Folder".parse::<PromptVariable>(),
               Err(UnknownVariable("Folder".into())));
}

#[test]
fn each_variable_expands_to_its_value() {
    let ctx = context();
    let cases = [("{{agent.name}}", "Knot 3"),
                 ("{{agent.id}}", "b7f0"),
                 ("{{agent.type}}", "claude"),
                 ("{{folder}}", "/src/knot"),
                 ("{{folder.name}}", "knot"),
                 ("{{workspace}}", "Main"),
                 ("{{branch}}", "471-restart"),
                 ("{{date}}", "2026-09-25")];
    for (text, expected) in cases {
        assert_eq!(expand(text, &ctx), expected, "{text}");
    }
    assert_eq!(expand("Work in {{folder.name}} on {{branch}}", &ctx),
               "Work in knot on 471-restart");
}

#[test]
fn whitespace_inside_the_braces_is_allowed() {
    assert_eq!(expand("{{ agent.name }}", &context()), "Knot 3");
}

#[test]
fn an_unknown_name_is_left_as_typed() {
    assert_eq!(expand("see {{issue}} now", &context()), "see {{issue}} now");
}

#[test]
fn an_escaped_variable_is_literal_without_the_backslash() {
    assert_eq!(expand(r"\{{folder}}", &context()), "{{folder}}");
    assert_eq!(expand(r"a \{{b}} {{branch}}", &context()),
               "a {{b}} 471-restart");
}

#[test]
fn an_unclosed_brace_is_left_alone() {
    assert_eq!(expand("use {{ carefully", &context()), "use {{ carefully");
}

#[test]
fn a_value_is_not_expanded_twice() {
    let ctx = PromptContext { agent_name: "{{date}}".into(),
                              ..context() };
    assert_eq!(expand("{{agent.name}}", &ctx), "{{date}}");
}

#[test]
fn empty_values_expand_to_nothing() {
    let ctx = PromptContext { branch: String::new(),
                              ..context() };
    assert_eq!(expand("[{{branch}}]", &ctx), "[]");
}

#[test]
fn unknown_names_are_reported_once_in_order() {
    assert_eq!(unknown_variables("{{foldr}} {{ issue }} {{foldr}} {{branch}}"),
               ["foldr", "issue"]);
    assert!(unknown_variables(r"\{{foldr}} {{folder}} {{ unclosed").is_empty());
}
