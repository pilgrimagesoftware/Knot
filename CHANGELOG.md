
## 1.17.0 - 2026-09-24

### Added
- Scroll and search the panel model picker
- Drop merged pull requests from the list after a day
- Artifact panel layout arithmetic and per-agent arrangement
- The artifact panel container


### Changed
- Keep the selector fields inside the struct's column
- Bind the store lock before persisting expired records


### Documentation
- State the timing rule as a property, not a shape
- Generalize the off-thread rule, and drop a note that dated itself


### Fixed
- Route ACP panel status through the activity tracker
- Land the tracker's store write on a frame, and dedupe on what was reported
- Removing a pull request no longer opens it
- Stop a teammate's request preempting an agent's task
- Schedule a frame when a pull request passes the window



## 1.16.0 - 2026-09-23

### Added
- Build the panel composer on Editor, not Textarea
- Add the composer scanner
- Paint the composer's styled runs
- Match and enumerate files for @ mentions
- Complete @ file mentions in the composer
- Give an attachment a place in the prompt
- Share the settings surface copy-on-write
- Add a streaming, cancellable shell runner
- Run a shell command from the panel with a ! prefix
- Add the typed MCP server inventory
- Run and read an agent's MCP listing
- Record each agent type's MCP list and handover commands
- Schedule MCP probes and land their results on a frame
- Draw the MCP servers section in a Panel agent's pane
- Hand the user their agent's own MCP flow
- Pair the MCP and processes sections as an accordion
- Add the typed subagent record


### Changed
- Split the panel composer by concern
- The settings window edits the shared surface
- The workspace window reads the shared surface
- The last snapshots read the shared surface


### Fixed
- Resolve the terminal font family once per configured name
- Focus the terminal surface when a shell agent is selected
- Drop recorded pull requests whose agent is gone
- Imports write the current settings, not a caller's copy
- Read a listing that a CLI writes to stderr



## 1.15.0 - 2026-09-23

### Added
- Swap the turn-in-progress animation for the pulse artwork
- Let the application name the git it runs
- Number a duplicated agent instead of suffixing "(copy)"
- Show the bound keystroke on the permission prompt buttons
- Add path-scoped file_diff
- Add git panel state and section grouping
- Add the git panel's work-tree relevance filter
- Add git panel reads, actions and watch lifecycle
- Render the git panel and wire it into the workspace window
- Apply the chosen appearance mode
- Name a new agent after its folder, and ask for the folder sooner


### Changed
- Host the workspace name dialog in the shared dialog layer
- Drop the panel's vestigial 120ms indicator repaint
- Make the claim-to-spawn gap empty, not merely safe
- Move the folder-name rule out of the agent store


### Documentation
- Correct why gh is resolved before it is spawned
- Record the off-thread-results rule and its four breaks


### Fixed
- Locate gh outside the launchd PATH
- Decide the scroll-to-latest control from the tail row
- Deliver a preference change to the windows already open
- Summarize the processes section instead of counting forever
- Take the gh availability probe off the render path
- Repaint when an unselected panel agent changes state
- Stop the grid scan benchmark failing the gate at random
- An unbound Settings can no longer write to the real store
- Release the process sampler's claim on unwind
- Seed the diff list with a uniform row height
- Draw diff line text from theme tokens, not the fixed palette
- Use the computed line box for the diff row height hint
- Resolve a pending slot even when the work unwinds
- Stop the watch tests counting their own setup events
- Stop requiring an optional ACP field to find a config option



## 1.14.0 - 2026-09-23

### Added
- Recognize pull request URLs in agent output
- Read pull request state through the gh CLI
- Persist recorded pull requests as a seventh document
- Watch both agent kinds for pull request URLs
- Add the Pull Requests view and its state cache
- Colour each pull request row by its state
- One window per workspace, Command Center and manager
- Name the workspace in its window's title bar
- Supervise the server with state, probing and backoff
- Show and announce the MCP server's supervised state
- Add the animated working-indicator asset and its recipe
- Animate the app icon as the conversation's working row
- Add the log entry model, writer task and rotation
- Log requests, tool calls and the server's own lifecycle
- Write a periodic heartbeat carrying the server's vitals
- Resolve the MCP log directory and hand it to the server
- Reveal the prompt copy control on hover
- Read an agent's descendant tree and terminate one process
- Resolve an agent to the process Knot spawned for its session
- Sample an agent's process tree off the render path
- Show an agent's processes in its pane, and act on one
- Split workspace UI state into its own document
- Write workspace arrangement without rewriting the roster


### Changed
- Split agent_editor's mod.rs into fields, window, submit and pickers
- Split panel_state's mod.rs into message, state and fold
- Split panel_view's mod.rs into style, callbacks and render
- Split settings_window's mod.rs into tab, window and render
- Split about_window into build_info, window and pane
- Split workspace_window's mod.rs into named siblings


### Documentation
- Make "mod.rs declares, it does not implement" a standing rule
- Treat the six legacy mod.rs files as a backlog, not exceptions
- Record the mod.rs backlog as done, and what splitting it taught


### Fixed
- Keep a restored window on an attached display
- One Enter Full Screen, and a reachable Window menu
- Scroll the Command Center's grid instead of clipping it
- Scroll the workspace window's dashboard too
- Focus the prompt input when a Panel-mode agent is selected
- Compare the log directory's name case-insensitively
- Cap the prompt cluster, not the bubble inside it
- Confirm a process copy, and name the terminate button
- Paste into the terminal pane with cmd-v
- Keep the historical workspace UI-state defaults



## 1.13.0 - 2026-09-22

### Added
- Route the settings window's text through the catalog
- Route the menus, tabs and settings title through the catalog
- Route the dialogs' text through the catalog
- Route the last of the app's text through the catalog
- Add a copy button to rendered code blocks
- Activate a deactivated agent on a direct message
- Add the panel slash lookup registry
- Complete slash commands in the panel prompt
- Read subagent definitions and Skwad preferences
- Add the Import tab to the settings window
- Add registry fields to agents and bench templates
- Project live agents and bench templates into a registry
- Add the task graph, its state machine and dispatch gate
- Widen list-agents and add describe-agents
- Add plan-tasks, dispatch-task, complete-task, task-status
- Add description, capabilities and cost controls to the agent editor
- Draw the committed plan as a diagram in the agent panel
- Ship an Orchestrator persona describing the method
- Split the settings store into per-collection documents
- Give the menu bar its platform key equivalents
- Take the Agents menu's shortcuts from the Swift reference


### Changed
- Confirm workspace deletion through the shared alert dialog
- Report the errors these actions were swallowing
- One confirmation builder, one workspace lookup, one count type
- Validate the agent editor's form in one place
- Name the dashboard callbacks, the folder label and the sh fixture
- Write the transcript scan once
- One picker builder for the settings panes
- Split the agent editor's render into its rows
- Make the Open In… applications an enum
- Split the workspace manager's render from its state
- Split the personas pane's row out of its render
- Split the last two long renders
- One roster for the agent types, read from six crates
- Give the repaint poll a name and split its side effects
- Split the workspace window's render into its parts
- Split the application's bootstrap into its three jobs
- Move Import out of settings into its own window
- Move the tool catalogue and dispatch to catalog.rs
- Collapse the one-line setters through update()
- Build every window from two shared builders
- Parse provider entries with typed Deserialize


### Documentation
- Bring the structure rules back in line with the code


### Fixed
- Remember settings, associate agent with workspace
- Repaint the panel when a queued prompt is picked up
- Self-heal on unknown MCP session id
- Render declared single-line rows as one line
- Stop the command center running git on the render path
- Refresh settings snapshot after agent editor closes
- Put imports into the live store and always report the outcome
- Redraw every window when an import completes
- Move Fork Agent off cmd-f, leaving the find key free



## 1.12.0 - 2026-09-22

### Added
- Add an Agents menu to the application menu bar
- Show the working indicator on the sidebar row and dashboard card
- Delete queued messages and show retry as an icon
- Edit a queued message in the composer
- Adopt the macOS system palette for app chrome
- Add the sidebar background context menu
- Add t_with for localized text with a value in it
- Warn before quitting while agents are working
- Operate the workspace name dialog from the keyboard
- Preview each font picker in the family it names
- Replace the About alert dialog with a window
- Copy build details by clicking them, and credit Skwad
- Swap the UI and title font roles, with a load-time migration
- Render Markdown headers in the title font
- Persist per-agent Panel session setup
- Collapse a turn's tool calls into one summary line
- Actually raise the desktop notifications the spec requires
- Make the workspace sidebar resizable


### Changed
- Delete orphaned source files and dead diff classifier
- Split the tests grab-bag by subject
- Split workspace_window into per-concern modules
- Split settings_window into one module per tab
- Enforce the file-size limit, drop the crate-wide dead-code allow
- Replace std Mutex with parking_lot to delete the poisoning split
- Cut the registration prompt to what agents act on
- Give every module explicit imports, drop the glob prelude
- Make the settings vocabularies enums, not strings


### Documentation
- Base PRs on develop and merge with a merge commit
- Derive the worktree path from the checkout root
- State the worktree convention in AGENTS.md and the .agents skills
- Update every remaining reference to the renamed make targets
- Correct the embedded-font doc comment for the swapped roles


### Fixed
- Scroll the persona picker and drop deleted personas
- Focus the workspace window root so the Agents menu enables
- Drop the working indicator from the sidebar agent row
- Deliver the font panel's choice to the settings window
- Stop leaking adapter processes, memory and git subprocesses
- Gate the macOS-only palette math by cfg, not by a blanket allow
- Gate the Rgba import with the code that uses it
- Queue the inbox nudge instead of prompting over a live turn
- Gate the macOS-only imports the explicit-import pass left bare



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
