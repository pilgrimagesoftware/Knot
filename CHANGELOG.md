
## 1.11.0 - 2026-09-21

### Added
- Virtualize the agent content panel
- Add tool call status icons
- Add working indicator
- Queue panel prompts while agents are busy
- Queue panel prompts during responses
- Use icons for queued prompt controls


### Documentation
- Document feature PR preflight


### Fixed
- Pad virtualized panel rows, not the list
- Resolve adapters on a GUI-safe PATH
- Spawn ACP adapters and installs on the merged PATH
- Collapse long tool call titles
- Auto-install Codex ACP adapter
- Keep bottom selector menus clickable
- Sync panel working status
- Make panel selector options clickable
- Left-align popup menu labels
- Format all the things
- Clear delivered panel prompts from queue
- Name queued prompt delivery result
- Add panel stop button
- Use icons for panel controls
- Style panel stop control
- Use solid stop glyph
- Preserve panel stop implementation
- Compact queued panel messages
- Push queued controls to row edge
- Track queued prompt delivery state
- Release panel state before queued delivery
- Render context usage indicator



## 1.10.0 - 2026-09-19

### Added
- Scaffold Cargo workspace and skwad-git runner
- Porcelain v2 status and unified-diff parsing
- Combined diff stats via numstat + untracked lines
- Staging, commit, and discard operations
- Branch name and ahead/behind queries
- Wire GPUI Kit for the window shell (closes task 1.5)
- Worktree detection, creation, and path suggestion
- Filesystem repo scan with debounced watch
- Durable settings store
- Conversation history providers and cache
- Persona CRUD, lookup, and restore-defaults
- Agent lifecycle port
- Local MCP HTTP server port
- Agent-to-agent message queue port
- MCP tool catalog implementing the mcp-tools spec
- Start the local MCP server with the real tool catalog
- Debounced directory watch with pause/resume
- Shell command builder for agent terminal launch
- Port activity detection state machine and tracker
- Add agent hook parsing
- Wire agent hook routes
- Restore persisted agents
- Persist agent mutations
- Route hooks through trackers
- Create agent worktrees
- Add session bridge
- Add PTY transport
- Wire PTY session events
- Inject registration prompts
- Track HTTP sessions
- Render restored agent layout
- Select and filter workspaces
- Attach a terminal session to the selected agent
- Share one AgentStore between the shell and the MCP catalog
- Render the agent state badge in agent rows
- Send commands to attached terminals
- Surface delivered MCP messages
- Show unread MCP message counts
- Wire terminal status events to agents
- Inject idle inbox checks
- Surface awaiting-input notices
- Add usable workspace and agent management UI
- Add workspace manager window
- Restore last conversation on layout restore
- Raise desktop notifications on awaiting-input
- Add a General settings window
- Add a tab strip to the settings window
- Fill in the Coding settings tab
- Fill in the Personas settings tab
- Add Autopilot settings scalars and tab
- Add Voice settings scalars and tab
- Fill in the MCP settings tab
- Fill in the Terminal settings tab
- Embed Inter and a blue accent color
- Switch UI font to Manrope; tighten spacing; scroll settings panes
- Redesign the settings window per feedback
- Settings window polish pass
- Switch UI font to Adamina; wrap MCP command instead of scrolling
- Propose the Dashboard port, add its launcher and a titlebar icon
- Add the workspace and Command Center agent dashboards
- Parse PTY output into an alacritty_terminal grid
- Render a live terminal grid in WorkspaceWindow
- Resize the terminal grid/PTY to match the content pane
- Dispatch keyboard input to the focused terminal
- Dispatch mouse clicks/scroll to mouse-aware terminal programs
- Route terminal title changes into the agent store
- Write OSC 52 clipboard requests to the OS pasteboard
- Text selection and copy in the terminal grid
- List every Swift-supported coding agent in the New Agent dialog
- Select the new agent on creation; match the Swift terminal header
- Restore Manrope, add JetBrains Mono, unify the workspace header
- Scaffold ACP client crate
- Add ACP adapter registry and launch-path branching
- Add view_mode and acp_session_id to Agent
- Add AcpSession for Panel-mode ACP connections
- Add acp-updates tracking source
- Add panel_state module folding ACP updates into panel state
- Populate adapter registry, fix 3 wire-format bugs found live
- Wire Panel-mode GPUI UI into the workspace window
- Add prompt input to the Panel view
- Auto-install a missing ACP adapter on first Panel use
- Add ACP agent panel UI
- Add tool-call kind icons to panel message list
- Add response action bar and track toggle to agent panel
- Add agent panel input-area control bar
- Implement ACP's stabilized Session Config Options
- Add right-click context menu for agent-list rows
- Fix agent panel feedback (effort, markdown, paste, send key)
- Color permission modes and add keybindings
- Panel-only launch for non-shell agents, drop the toggle
- Show each agent's type in the sidebar row
- Give each agent type an icon and its own line in the sidebar
- Report what a connecting agent is waiting on
- Color the header diff stat and localize its noun
- Bind the standard macOS menu shortcuts
- Open a workspace by double-clicking its row
- Remember each workspace window's position and size
- Color the dashboard cards' diff stats, and offer a connect retry
- Uniform dashboard cards, and close a companion when its shell exits
- Quit on Ctrl-C from the launching terminal
- Model the agent context menu as ordered entries with dividers
- Let the agent editor open pre-filled from an existing agent
- Render an agent's markdown file, and add the Open In targets
- Complete the agent context menu against the Swift reference
- Tell agents how to work with their knot
- Deliver the idle-time inbox nudge agents were never getting
- Collapse finished tool calls to their header
- Label every agent row detail line with an icon
- Add Zed to the Open In submenu
- Colour and font a tool call card by its state
- Add agent activation modes


### Changed
- Split oversized modules into submodules for the file-size limit
- Scope open_in's Command import to macOS


### Documentation
- Settings spec
- Note the discovery watcher-teardown fix
- Note the Access-event feedback-loop fix
- Describe Rust GPUI application
- Note character-palette pattern in UI conventions
- Update worktree convention path
- Require worktrees for all code changes, no exceptions
- Hand-off notes for acp-only-agent-launch open items
- Rewrite the hand-off for the current state of the branch


### Fixed
- License
- Tear down the old watcher synchronously
- Ignore Access events to stop a Linux-only feedback loop
- Target UI in OpenSpec config
- Validate companion agents
- Reclaim stale sessions
- Give the shell a tokio runtime for terminal sessions
- Refine workspace manager windows
- Transparency on the app icon
- Copyright symbol
- Sync the Base theme layer after overriding font/accent
- Recompute legacy ThemeTokens after overriding colors
- Icon visibility, MCP command scroll, URL copy button
- Polish settings window layout and use native font panel
- Stop personas title/border drift and fix native font panel
- Polish the new agent dialog
- Remove redundant window titles and title bar seam
- Fix invisible choose-character icon
- Validate the new agent dialog before submit
- Match the Swift reference for the agent dialog and list row
- Use blue for the awaiting-input state dot
- Workspace window title, color seam, and agent cell polish
- Gate Duration import to macOS, matching its only usage
- Fix terminal font, stale rendering, sidebar clipping, and shell-start truncation
- Measure real terminal cell size instead of guessing
- Wait for the shell to go quiet, not a fixed delay, before sending the init command
- Move the new-agent/dashboard toolbar to the sidebar header
- Polish the New Agent dialog
- Scroll the New Agent dialog's content instead of pushing the buttons off-window
- Strip host-terminal env vars from spawned agent shells
- Split header into sidebar/content columns; start all agents on open
- Two-column header layout, robust terminal font resolution
- Revert app-wide default back to Adamina; Manrope stays scoped
- Make font-panel Target type cross-platform
- Gemini adapter needs --skip-trust, fail closed on connect hang
- Stop discarding the adapter subprocess's stderr
- Show the user's own prompts in the Panel conversation
- Stop nil-agent MCP sessions from evicting each other
- Change default MCP port to 8767 to avoid Skwad conflict
- Avoid terminal args in ACP launches
- Target UI in OpenSpec config
- Scope permission shortcuts
- Wire knot MCP server into ACP sessions, drop dead CLI path
- Correct mcpServers schema, breaking opencode/gemini ACP
- Prefix request/response logs with adapter program name
- Read tool-call results from the content array
- Let the panel conversation scroll and wrap
- Repaint on panel phase changes and persist agent removal
- Name the tool schema field inputSchema, per MCP
- Stop the panel overflowing its pane and overlapping its text
- Size panel markdown by cascade, fixing overlapping text
- End the turn from the session/prompt response
- Give each config selector its own open state, and cache git stats
- Defer context-menu dialogs past the menu's own dismissal
- Auto-grow the prompt box instead of pinning its height
- Keep the agent header inside the pane at narrow widths
- Render tool output in the real mono font, and add panel affordances
- Order the app and its windows front on open
- Render the dialog layer, so confirmations actually appear
- Read the permission request's nested toolCallId
- Title the manager window so the Window menu lists it
- Fall back to any window when opening the About dialog
- Publish the panel slot before the registration turn
- Show the session when a sidebar row is picked, and centre the traffic lights
- Open the About dialog after the menu dispatch returns
- Show a refused prompt in the agent panel, not just on stderr
- Stop showing "Getting stats…" for a folder that isn't a repo
- Refresh personas in the agent editor and guard in-use deletes
- Name the MCP server the knot tools come from
- Give the settings window's MCP URL its /mcp path
- Keep "New Companion…" from creating a non-shell companion
- Put the dashboard at the top of the agent list
- Point the collaboration prompt outward
- Use a switch for the agent activation control


## [1.9.0] - 2026-04-03

### Added
- Cmd-Shift-P to add permissions to a folder (Claude only)

### Changed
- Use status for header title

### Fixed
- N/A

### Removed
- N/A


## [1.8.2] - 2026-03-25

### Added
- N/A

### Changed
- Check inbox prompt message

### Fixed
- Codex launch command fix

### Removed
- N/A


## [1.8.1] - 2026-03-16

### Added
- Cmd+` to cycle through workspaces

### Changed
- Allow editing agent type and persona on existing agents

### Fixed
- Conversation title no longer picks up "check your inbox" injected messages
- Cmd+1-9 now exits Command Center and switches to workspace
- Codex launch command no longer breaks on apostrophes in system prompt

### Removed
- N/A


## [1.8.0] - 2026-03-11

### Added
- Dashboard views (global and per-workspace) showing agent status, recent activity, and quick actions
- MCP tools for agents to list bench and deploy bench agents
- Restart workspace

### Changed
- N/A

### Fixed
- N/A

### Removed
- N/A


## [1.7.0] - 2026-03-04

**Personas** — Give your agents personality. Assign a persona when creating an agent to influence its coding style and philosophy. Want a TDD purist? Pick Kent Beck. Need raw performance focus? Go with John Carmack. Knot ships with 6 built-in personas from legendary engineering minds, and you can create your own. Agents can also attach personas when they create new agents via MCP. Personas work with any agent that supports system prompts (Claude, Codex).

**Bench** — Save your favorite agent configurations and deploy them instantly. Right-click any agent to add it to the bench, then drag it into any workspace to spin up a pre-configured agent in seconds. Great for reusable setups you reach for often.

### Added
- Bench
- Personas
- Codex: conversation history and resume/fork support
- Gemini: conversation history and resume support
- Copilot: conversation history and resume support

### Changed
- Improved text injection method
- Codex: system prompt injection via `developer_instructions`

### Fixed
- Voice input: transcript loss during long dictation (Apple speech recognizer silent resets)

### Removed
- Recent agents


## [1.6.0] - 2026-03-01

### Added
- Claude: detect when agent is prompting user. Shows new "Blocked" status (red dot on agent and workspace)
- Claude: add keep conversation option when forking an agent
- Claude: plugin with slash commands (/list-agents, /send, /check, /broadcast, /worktree, etc.)
- Claude: conversation history with resume capability
- Codex: hook handler for activity detection (working/idle status)
- Desktop notifications when an agent needs attention (with click-to-navigate)
- File finder (Cmd+P): fuzzy search to open files from the agent's working directory
- Markdown preview: two-phase panel — view mode (Approve/Review buttons) then review mode (comment popup on selection)
- Markdown preview: font size controls (A▼/A▲) in title bar 
- Mermaid diagrams: new `view-mermaid` MCP tool for agents to display flowcharts, state, sequence, class, and ER diagrams
- Autopilot: LLM-based tri-classification of agent messages (completed/binary/open) with configurable actions per category (mark, ask, auto-continue, custom prompt)

### Changed
- Sidebar/header: show agent's actual working directory (from hook-reported cwd) with branch indicator when it differs from base folder
- Claude: hook-based activity detection replaces terminal output parsing for more accurate status
- Claude: registration now uses hooks instead of MCP call on startup
- "Open In..." uses agent's actual working directory (Claude tracks working directory)
- Compact agent sidebar mode

### Fixed
- N/A

### Removed
- N/A


## [1.5.0] - 2026-02-12

### Added
- Markdown panel comment feature: select text and add comments that are injected into the agent terminal

### Changed
- Shell agents are now hidden from MCP list-agents results
- Companion agents are only visible to their owner in MCP list-agents
- Companion agents can only exchange messages with their owner

### Fixed
- Sending messages to shell agents now returns a clear error
- MCP server error in Codex

### Removed
- N/A


## [1.4.2] - 2026-02-10

### Added
- Option to relocate companions when change directory of main agent

### Changed
- N/A

### Fixed
- Startup time fix (especially when using shell agents/companions)

### Removed
- N/A


## [1.4.1] - 2026-02-08

### Added
- New Shell Companion menu item (Shift+Cmd+S) to quickly create a shell companion for the active agent
- Duplicate Agent (Cmd+D) and Fork Agent (Cmd+F) in Edit menu

### Changed
- Faster repository discovery
- Edit Agent now allows changing the folder/worktree (agent restarts automatically)

### Fixed
- Spurious activity detection on hidden terminals
- Auto-select first agent when switching to workspace with no active selection

### Removed
- Worktree creation from existing branch


## [1.4.0] - 2026-02-07

### Added
- Companion agents: create lightweight agents linked to an owner agent (automatically share screen with their owner)
- MCP tool `create-agent` supports companion agents with `companion` flag
- Markdown preview history per agent
- Cmd+W now closes the focused agent instead of the window
- Shell option in agent type picker for plain terminal without agent
- Optional shell command field when creating shell agents
- MCP tool `create-agent` supports optional `command` parameter for shell agent type
- Keep running in menu bar: hide to menu bar on Cmd+Q or close button, restore on click
- Drop indicator line in sidebar during agent drag and drop
- 3-pane layout: left half full-height + right side split top/bottom (auto-selected for 3 agents)

### Changed
- Reorganized menus to follow macOS conventions for single-window app

### Fixed
- Drag and drop agent reordering in sidebar (was moving wrong agent)
- Shell command now persisted and restored on app relaunch

### Removed
- N/A


## [1.3.0] - 2026-02-05

### Added
- File drop support: drag files onto terminal to inject their path
- MCP tool `close-agent` for agents to close agents they created
- Markdown panel auto-reloads when file changes on disk

### Changed
- Renamed MCP tool `show-markdown` to `display-markdown` with improved description
- Markdown panel is now per-agent: switching agents shows/hides the panel accordingly
- Inline registration for all supported agents (Claude, Codex, OpenCode, Gemini, Copilot)

### Fixed
- Context menu submenu flickering when terminal is active
- Markdown panel now reloads when file path changes
- Split pane now correctly collapses to single pane when removing an agent from a pane

### Removed
- N/A


## [1.2.0] - 2026-02-03

### Added
- Draggable split pane dividers for 2-pane and 4-pane layouts
- MCP tool `show-markdown` for agents to display markdown files in a panel

### Changed
- N/A

### Fixed
- N/A

### Removed
- N/A


## [1.1.0] - 2026-02-02

### Added
- Separate idle timeouts for terminal output (2s) and user input (10s)

### Changed
- N/A

### Fixed
- N/A

### Removed
- N/A


## [1.0.1] - 2026-01-31

### Added
- Agent recovery: help agents recover forgotten ID with folder matching
- Register agent context menu entry
- Move agent to workspace option in context menu

### Changed
- Improve send-message response to discourage polling
- Modernize to SOTA Swift patterns (view/logic separation)

### Fixed
- N/A

### Removed
- N/A


## [1.0.0] - 2026-01-28

### Added
- Workspace support for organizing agents
- Workspace-scoped MCP communication
- 4-pane grid layout mode
- Split vertical and horizontal layout modes
- Sparkle auto-update support
- Configurable default "open with" app and keyboard shortcut
- Comprehensive keyboard shortcuts
- Sidebar collapse toggle
- Broadcast message to all agents
- Close all agents option
- Clear agent keyboard shortcut (Shift+Cmd+C)
- Restart all menu option with confirmation
- Scroll wheel zoom in avatar editor
- Recent agent badges in empty state

### Changed
- Extended common source folder candidates list
- Extended avatar cropper zoom limits to 10%-2000%

### Fixed
- Focus pane when clicking visible agent instead of swapping
- Split pane implementation issues
- Settings organization
- Use zip instead of ditto to avoid resource fork corruption
- Notify terminal to resize when git panel toggles

### Removed
- N/A


## [0.9.0] - Initial Release

### Added
- Multi-agent terminal management with Ghostty and SwiftTerm engines
- Agent-to-agent communication via MCP server
- Git integration with status panel, staging, and commits
- Git worktree support for agent isolation
- Voice input with push-to-talk
- Custom agent avatars with image cropping
- Activity detection (working/idle status)
- Terminal state preservation when switching agents

### Changed
- N/A

### Fixed
- N/A

### Removed
- N/A
