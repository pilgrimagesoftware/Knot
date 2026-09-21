# Spec Delta

## MODIFIED Requirements

### Requirement: Single settings store

The system SHALL expose one settings surface holding scalar values and serialized collections. It SHALL include the compact tool-call display preference, which defaults to disabled, and SHALL persist changes immediately. Settings written before this preference existed SHALL load successfully with compact mode disabled.

#### Scenario: Compact mode persists
- **WHEN** the user enables compact tool-call mode and restarts the app
- **THEN** compact mode remains enabled

#### Scenario: Scalar persists across restart
- **WHEN** the MCP port is set to `9000` and the app restarts
- **THEN** the MCP port reads back as `9000`

#### Scenario: Restore-conversation-on-launch defaults off for legacy settings
- **WHEN** a persisted settings blob written before `restore-conversation-on-launch` existed is decoded
- **THEN** it loads successfully with `restore-conversation-on-launch` set to off

#### Scenario: Autopilot scalars default for legacy settings
- **WHEN** a persisted settings blob written before the autopilot scalars existed is decoded
- **THEN** it loads successfully with `autopilot_enabled` false, `ai_provider` `"openai"`, `ai_api_key` empty, `autopilot_action` `"mark"`, and `autopilot_custom_prompt` empty

#### Scenario: Voice scalars default for legacy settings
- **WHEN** a persisted settings blob written before the voice scalars existed is decoded
- **THEN** it loads successfully with `voice_enabled` false, `voice_engine` `"apple"`, `voice_push_to_talk_key` `54`, and `voice_auto_insert` true

#### Scenario: Legacy settings default compact mode off
- **WHEN** a settings blob predates the compact tool-call preference
- **THEN** it loads successfully with compact mode disabled
