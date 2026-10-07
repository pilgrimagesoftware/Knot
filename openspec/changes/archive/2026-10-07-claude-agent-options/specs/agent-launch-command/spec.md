# Spec Delta

## ADDED Requirements

### Requirement: User options for an ACP-launched agent

For a Claude agent launched through its ACP adapter, the system SHALL pass the
options set for the `claude` type in Settings - Coding - Agent Options to the
adapter's session, as `_meta.claudeCode.options.extraArgs` on `session/new` and
`session/load`. Options SHALL be split into words as a shell would, honoring
quotes. A word `--name=value`, or `--name` followed by a word that does not
start with `-`, SHALL be forwarded as `name` with that value; a `--name` with
no value SHALL be forwarded as a bare flag.

`--dangerously-skip-permissions` SHALL instead select the adapter's
`bypassPermissions` mode, and `--permission-mode <mode>` the mode it names in
any spelling Claude Code accepts, once the session opens. That mode SHALL win
over a persisted session mode and over a mode set in Claude's settings files,
and SHALL NOT be requested when the adapter does not offer it.

Flags the adapter passes itself to drive the session, short flags, stray words,
and all the options when their quoting is unbalanced SHALL NOT be forwarded,
and the system SHALL log what it did not forward. Options for agent types whose
adapter has no confirmed way to take them SHALL NOT be forwarded and SHALL be
logged the same way.

When the adapter refuses the session with an error carrying details, the
failure the panel shows SHALL include those details.

#### Scenario: Extra flags reach the agent

- **WHEN** the Claude options are `--remote-control --add-dir /tmp/x`
- **THEN** the session opens with `extraArgs` `{"remote-control": null, "add-dir": "/tmp/x"}`

#### Scenario: Skipping permissions selects bypass

- **WHEN** the Claude options include `--dangerously-skip-permissions` and the
  adapter offers `bypassPermissions`
- **THEN** the session is moved to `bypassPermissions` after it opens, even if
  the agent's persisted mode is another one

#### Scenario: Adapter-owned flags are dropped

- **WHEN** the Claude options include `--output-format text`
- **THEN** it is not forwarded, and is logged as not passed to the adapter

#### Scenario: No options

- **WHEN** no options are set for the Claude type
- **THEN** the session opens with no `_meta`

#### Scenario: A mistyped option names itself

- **WHEN** an option is one `claude` does not know and the adapter fails the
  session
- **THEN** the panel's error includes the CLI's message naming the option
