# Spec Delta

## ADDED Requirements

### Requirement: User options for the other ACP-launched agents

For a Codex, OpenCode or Gemini agent, the system SHALL pass the options set
for its type in Settings - Coding - Agent Options to its ACP adapter, split
into words as a shell would, as follows. Only the flags listed for a type
SHALL be forwarded, each in its long or one-letter form, with a value given
either after `=` or as the next word that does not start with `-`.

- Codex: SHALL set each `-c`/`--config` `key=value` as that key, a dotted key
  naming nested tables, with the value read as JSON where it parses and as a
  string otherwise; `-m`/`--model` as `model`; `-p`/`--profile` as `profile`;
  and `--enable <name>` / `--disable <name>` as `features.<name>` set to true
  or false. These SHALL go into the JSON object of the adapter subprocess's
  `CODEX_CONFIG`, with nested tables merged into what that variable already
  holds.
- OpenCode: SHALL set `-m`/`--model` as `model` and `--agent` as
  `default_agent` in the adapter subprocess's `OPENCODE_CONFIG_CONTENT`, and
  SHALL pass `--print-logs`, `--log-level <level>` and `--pure` on the `opencode
  acp` command line.
- Gemini: SHALL append `--model`, `--sandbox`, `--yolo`, `--approval-mode`,
  `--policy`, `--admin-policy`, `--allowed-mcp-server-names`,
  `--allowed-tools`, `--extensions`, `--include-directories`, `--raw-output`
  and `--accept-raw-output-risk` to the adapter's command line under their long
  names.

The options SHALL override the value the environment variable has in Knot's
own environment, and the standing instructions SHALL be merged in after them
(see "Standing instructions through the agent's system channel"), so that no
option removes or replaces them. The variable SHALL still be set, with the
options, when the instructions fall back to the first turn.

A flag not listed for the type, together with the word after it when that word
is not a flag; a listed flag that needs a value and has none; a stray word; all
the options when their quoting is unbalanced; and every option for Copilot,
whose adapter has no confirmed way to take them, SHALL NOT be forwarded, and
the system SHALL log what it did not forward.

#### Scenario: Codex options become config overrides

- **WHEN** the Codex options are `-m o3 -c features.web_search=true`
- **THEN** the adapter starts with `CODEX_CONFIG` holding `model` `o3`,
  `features.web_search` `true` and the knot's `developer_instructions`

#### Scenario: An option cannot displace the instructions

- **WHEN** the Codex options include `-c developer_instructions=Mine`
- **THEN** `developer_instructions` holds `Mine` followed by the knot
  instructions

#### Scenario: OpenCode starts in the chosen agent

- **WHEN** the OpenCode options are `--agent plan --print-logs`
- **THEN** `OPENCODE_CONFIG_CONTENT` sets `default_agent` to `plan` and the
  adapter runs as `opencode acp --print-logs`

#### Scenario: A Gemini flag that would end the ACP session is dropped

- **WHEN** the Gemini options are `-p hello --model gemini-2.5-pro`
- **THEN** the adapter runs as `gemini --acp --skip-trust --model
  gemini-2.5-pro`, and `-p hello` is logged as not passed

#### Scenario: Copilot options are not forwarded

- **WHEN** options are set for the Copilot type
- **THEN** none are passed to its adapter, and all are logged as not passed
