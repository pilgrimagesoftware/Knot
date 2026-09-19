## MODIFIED Requirements

### Requirement: Open In from the context menu
The Open In… submenu SHALL list VS Code, Zed, Xcode, a divider, Finder and
Terminal, and selecting one SHALL open the agent's folder in that
application. A missing application SHALL fail quietly rather than
reporting an error the user cannot act on.

#### Scenario: Open the agent's folder in an editor
- **WHEN** the user selects VS Code from Open In…
- **THEN** the agent's folder is opened in VS Code

#### Scenario: Open the agent's folder in Zed
- **WHEN** the user selects Zed from Open In…
- **THEN** the agent's folder is opened in Zed

#### Scenario: The chosen application is not installed
- **WHEN** the user selects an application that is not installed
- **THEN** nothing opens and the app does not present an error dialog
