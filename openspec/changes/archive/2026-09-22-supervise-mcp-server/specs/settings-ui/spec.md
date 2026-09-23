# Spec Delta

## MODIFIED Requirements

### Requirement: MCP tab

The MCP tab SHALL show an "Enable MCP server" toggle bound to
`mcp_server_enabled`, a port field bound to `mcp_server_port`, a read-only
server URL derived from the current port with a copy action of its own, and
an installation-command generator (agent-type picker + the exact command for
that type + copy action), each persisting on change. The installation
command's agent-type picker SHALL offer Claude, Codex, OpenCode, Gemini and
Copilot — not Shell, which runs no MCP client to register.

The URL SHALL be the MCP endpoint an agent connects to, not the server's
root: it carries the `/mcp` path, and a URL without it is one an agent
cannot use.

The tab SHALL additionally show a read-only server-state row reflecting the MCP server's
current lifecycle state, updating as that state changes while the window is open, without the
user reopening or switching tabs. The row SHALL show:

- when running, that it is running, and the address it is bound to;
- when retrying, that it is retrying, the attempt number, and the error from the last attempt;
- when starting, stopped, or disabled, which of those it is;
- nothing about a state the server is not in.

The state row SHALL be read-only. It offers no control of its own: the toggle above it is how
the server is turned off, and recovery from a failure is automatic.

#### Scenario: Toggling the MCP server persists

- **WHEN** the user turns off "Enable MCP server"
- **THEN** `mcp_server_enabled` is saved as `false` immediately

#### Scenario: Changing the port persists and updates the URL

- **WHEN** the user sets the port field to `9000`
- **THEN** `mcp_server_port` is saved as `9000` and the displayed URL
  updates to reflect port `9000`, keeping its `/mcp` path

#### Scenario: Copying the server URL

- **WHEN** the user clicks the copy action beside the URL
- **THEN** the system clipboard receives exactly the displayed URL,
  including its `/mcp` path

#### Scenario: Installation command matches the selected agent type

- **WHEN** the user selects "Codex" in the agent-type picker
- **THEN** the displayed command is the Codex-specific registration
  command containing the current server URL

#### Scenario: Copy action copies the exact displayed command

- **WHEN** the user clicks the copy action
- **THEN** the system clipboard receives exactly the currently-displayed
  command text

#### Scenario: State row shows a running server's address

- **WHEN** the MCP server is running and the user opens the MCP tab
- **THEN** the state row reports it as running and shows the bound address

#### Scenario: State row follows a failure without reopening the window

- **WHEN** the server fails while the MCP tab is open
- **THEN** the state row changes to report retrying, with the attempt number and the last
  error, while the window stays open

#### Scenario: State row reports a disabled server

- **WHEN** "Enable MCP server" is off
- **THEN** the state row reports the server as disabled rather than showing an address or an
  error
