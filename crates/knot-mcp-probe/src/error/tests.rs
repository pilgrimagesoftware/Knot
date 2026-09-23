use super::ProbeError;

#[test]
fn every_variant_names_its_program() {
    let cases = [ProbeError::Missing { program: "claude".to_owned(), },
                 ProbeError::TimedOut { program: "claude".to_owned(),
                                        seconds: 30, },
                 ProbeError::Command { program: "claude".to_owned(),
                                       output:  "boom".to_owned(),
                                       code:    1, },
                 ProbeError::Unrecognized { program:    "claude".to_owned(),
                                            first_line: "???".to_owned(), },
                 ProbeError::Io { program: "claude".to_owned(),
                                  message: "broken pipe".to_owned(), }];

    for case in &cases {
        assert_eq!(case.program(), "claude", "{case:?} must name its program");
    }
}

#[test]
fn command_display_carries_program_code_and_output() {
    let err = ProbeError::Command { program: "claude mcp list".to_owned(),
                                    output:  "not logged in".to_owned(),
                                    code:    1, };
    let text = err.to_string();

    assert!(text.contains("claude mcp list"));
    assert!(text.contains("exit 1"));
    assert!(text.contains("not logged in"));
}

#[test]
fn unrecognized_output_shows_the_line_it_could_not_read() {
    let err = ProbeError::Unrecognized { program:    "claude".to_owned(),
                                         first_line: "Usage: claude mcp".to_owned(), };

    assert!(err.to_string().contains("Usage: claude mcp"),
            "the section shows what it failed to read, not just that it failed");
}

/// Only a missing binary is worth suppressing on the next refresh. A timeout
/// or a bad exit can succeed on the retry, and treating those as standing
/// conditions would leave a recoverable failure stuck on screen.
#[test]
fn only_a_missing_binary_is_persistent() {
    assert!(ProbeError::Missing { program: "codex".to_owned(), }.is_persistent());
    assert!(!ProbeError::TimedOut { program: "claude".to_owned(),
                                    seconds: 30, }.is_persistent());
    assert!(!ProbeError::Command { program: "claude".to_owned(),
                                   output:  String::new(),
                                   code:    1, }.is_persistent());
    assert!(!ProbeError::Unrecognized { program:    "claude".to_owned(),
                                        first_line: String::new(), }.is_persistent());
}
