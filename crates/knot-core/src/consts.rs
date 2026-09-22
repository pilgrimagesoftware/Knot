pub const APP_NAME: &str = "Knot";

pub const ORG_QUALIFIER: &str = "com";
pub const ORG_NAME: &str = "Pilgrimage Software";

/// The preferences document, holding every scalar setting. Lives in the
/// platform's user-preferences directory, apart from the collections below.
pub const PREFERENCES_FILE: &str = "preferences.json";

/// The saved-agents collection, one of the durable data documents.
pub const AGENTS_FILE: &str = "agents.json";

/// The saved-workspaces collection.
pub const WORKSPACES_FILE: &str = "workspaces.json";

/// The personas collection.
pub const PERSONAS_FILE: &str = "personas.json";

/// The bench-templates collection.
pub const BENCH_FILE: &str = "bench.json";

/// The recent-repositories collection.
pub const RECENT_REPOS_FILE: &str = "recent-repos.json";

/// The single document every setting used to live in, read once on load and
/// renamed to [`LEGACY_MIGRATED_EXTENSION`] after its values have been
/// distributed across the documents above.
pub const LEGACY_SETTINGS_FILE: &str = "settings.json";

/// Extension the migrated legacy document is renamed to, so it is never read
/// again but stays recoverable by hand.
pub const LEGACY_MIGRATED_EXTENSION: &str = "json.migrated";

/// Extension for the temporary file each document is written to before being
/// renamed into place, so an interrupted write never leaves a real document
/// truncated.
pub const DOCUMENT_TEMP_EXTENSION: &str = "json.tmp";

/// Distinct from Skwad's default (8766) so a Knot instance doesn't fight a
/// running Skwad instance over the same port.
pub const MCP_PORT_DEFAULT: u16 = 8767;

pub const RECENT_REPOS_MAX: usize = 5;

pub const DEFAULT_AVATAR: &str = "\u{1f916}";

pub const DEFAULT_AGENT_TYPE: &str = "claude";

pub const TERMINAL_FONT_DEFAULT: &str = "JetBrains Mono";

pub const TERMINAL_FONT_SIZE_DEFAULT: f64 = 13.0;

/// The application's default proportional family: the face the interface and
/// all body text is drawn in, Markdown body text included.
pub const UI_FONT_DEFAULT: &str = "Adamina";

/// The application's default text size, paired with [`UI_FONT_DEFAULT`].
pub const UI_FONT_SIZE_DEFAULT: f64 = 16.0;

/// The family for titles and headers: the workspace header, the sidebar's
/// secondary cell text, the About window's credit lines, the panel input and
/// Markdown headers.
pub const TITLE_FONT_DEFAULT: &str = "Manrope";

/// The text size for titles and headers, paired with [`TITLE_FONT_DEFAULT`].
pub const TITLE_FONT_SIZE_DEFAULT: f64 = 14.0;

/// Version the current font-role arrangement is recorded under in a persisted
/// settings document. Version `0` - a document carrying no `settingsVersion`
/// key at all - means the pre-swap roles, where `uiFontName` held the title
/// face and `titleFontName` held the application-wide default.
pub const SETTINGS_VERSION_CURRENT: u32 = 1;

pub const APPEARANCE_MODE_DEFAULT: &str = "auto";

pub const MERMAID_THEME_DEFAULT: &str = "auto";

pub const MARKDOWN_FONT_SIZE_DEFAULT: i32 = 14;

/// The workspace sidebar's width when nothing has been persisted, matching the
/// Swift reference's `sidebarWidth` default.
pub const SIDEBAR_WIDTH_DEFAULT: f64 = 250.0;

/// The narrowest the sidebar may be dragged. Above the Swift reference's 80px
/// because the Rust window puts the title bar - and so the traffic lights -
/// inside the sidebar column, and the toolkit reserves 80px of left padding
/// for them; 120px leaves the application icon visible beside them and fits
/// the compact row's avatar. Revisit this floor if that reservation changes.
pub const SIDEBAR_WIDTH_MIN: f64 = 120.0;

/// The widest the sidebar may be dragged, as in the Swift reference.
pub const SIDEBAR_WIDTH_MAX: f64 = 400.0;

/// Below this width the sidebar drops its text and draws the compact layout -
/// avatar-only rows, an icon-only dashboard row, no application-name label and
/// an icon-only new-agent button. Ported from `SidebarView.swift`'s
/// `isCompact`.
pub const SIDEBAR_COMPACT_BREAKPOINT: f64 = 160.0;

pub const SOURCE_FOLDER_CANDIDATES: [&str; 3] = ["~/src", "~/source", "~/sources"];

pub const AI_PROVIDER_DEFAULT: &str = "openai";

pub const AUTOPILOT_ACTION_DEFAULT: &str = "mark";

pub const VOICE_ENGINE_DEFAULT: &str = "apple";

/// `ModifierKeyCode.rightCommand` in the Swift reference.
pub const VOICE_PUSH_TO_TALK_KEY_DEFAULT: i32 = 54;

/// Shipped system personas: (fixed id, name, instructions). Fixed ids let the
/// same persona be matched across installs and updates.
///
/// The first six describe *how to write code*. "Orchestrator" is a
/// different kind - it describes *how to coordinate other agents* - but it
/// rides the same mechanism, since both are instruction text injected as a
/// system prompt. It deliberately names no teammate: a roster written into
/// a prompt is a copy of state that goes stale the first time the team
/// changes, which is what the agent registry exists to prevent. See
/// `openspec/specs/agent-registry/spec.md`.
pub const DEFAULT_PERSONAS: [(&str, &str, &str); 7] = [("A1000001-0000-0000-0000-000000000001",
                                                        "Kent Beck",
                                                        "Write the simplest code that could possibly work, then refactor. Practice TDD religiously: red, green, refactor. Favor small steps and continuous feedback. Design emerges from refactoring, not upfront planning. Value communication, simplicity, and courage. When in doubt, write a test first."),
                                                       ("A1000001-0000-0000-0000-000000000002",
                                                        "Martin Fowler",
                                                        "Prioritize code readability above all - code is read far more than it is written. Apply established design patterns where they clarify intent. Refactor continuously to improve internal structure without changing behavior. Name things precisely. Favor clear abstractions and well-defined interfaces. Avoid clever code; prefer obvious code."),
                                                       ("A1000001-0000-0000-0000-000000000003",
                                                        "Linus Torvalds",
                                                        "Keep it simple and stupid. Performance matters - think about what the machine actually does. Reject unnecessary abstraction layers. Good taste in code means seeing the simple solution. Be direct and opinionated about bad design. Prefer pragmatic solutions over theoretically elegant ones. Data structures matter more than algorithms."),
                                                       ("A1000001-0000-0000-0000-000000000004",
                                                        "Uncle Bob",
                                                        "Follow SOLID principles strictly. Functions should do one thing and do it well. Keep them small - extract until you can't extract anymore. Clean code reads like well-written prose. Names should reveal intent. Dependencies point inward. Discipline and professionalism are non-negotiable. Leave the code cleaner than you found it."),
                                                       ("A1000001-0000-0000-0000-000000000005",
                                                        "John Carmack",
                                                        "Focus deeply on the technical problem at hand. Optimize ruthlessly where it matters - understand the hardware and the data. Prefer straightforward, linear code over complex abstractions. Static analysis and assertions catch bugs early. Write code that is easy to reason about locally. Pragmatism over dogma. Ship working software and iterate."),
                                                       ("A1000001-0000-0000-0000-000000000006",
                                                        "Dave Farley",
                                                        "Design for continuous delivery: every change should be deployable. Write tests at every level - unit, integration, acceptance. Work in small, incremental steps that keep the system always releasable. Decouple components to enable independent deployment. Automate everything that can be automated. Favor evolutionary design over big upfront architecture. Fast feedback loops are essential."),
                                                       ("A1000001-0000-0000-0000-000000000007",
                                                        "Orchestrator",
                                                        "You coordinate other agents. Before dispatching work that spans more than one agent or more than one task, find out who is available and commit a plan.\n\n1. Call describe-agents to see who can do what. Ask by capability, never by name: the team changes, and the registry is the only current record of it. Do not assume a teammate exists.\n2. Call plan-tasks with a small graph - one task per unit of work, each naming what it depends on. Assign a task to an agent, or to the capabilities an agent must carry, or leave it unassigned until you know.\n3. Call dispatch-task as each task becomes ready. It refuses a task whose dependencies are unfinished and tells you what it is waiting for.\n4. Call complete-task once an outcome is known, so the tasks behind it unblock. Call task-status to see where the plan stands.\n\nWork that is one task for one agent needs no plan; send it with send-message.\n\nPrefer the cheapest agent that can start now - describe-agents already ranks candidates that way. Let the plan be the record of what you intend, rather than describing it in prose.")];

// ---------------------------------------------------------------------------
// Data import
// ---------------------------------------------------------------------------

/// Where Claude Code keeps its subagent definitions, relative to the home
/// directory for the user-level set and to a project folder for its own.
pub const CLAUDE_AGENTS_SUBPATH: &str = ".claude/agents";

/// The only file extension a subagent definition is read from.
pub const SUBAGENT_DEFINITION_EXTENSION: &str = "md";

/// The line that opens and closes a definition's frontmatter block.
pub const FRONTMATTER_DELIMITER: &str = "---";

/// The one frontmatter key a definition is read for. Every other key is
/// dropped, per `openspec/specs/data-import/spec.md`.
pub const SUBAGENT_NAME_KEY: &str = "name:";

/// The macOS preferences domain Skwad, Knot's predecessor, stores its
/// collections under.
pub const SKWAD_PREFERENCES_DOMAIN: &str = "com.kochava.skwad";

/// Skwad's preference keys, each holding a JSON document stored as data, in
/// the record shapes Knot's own settings already read.
pub const SKWAD_WORKSPACES_KEY: &str = "savedWorkspacesData";
pub const SKWAD_AGENTS_KEY: &str = "savedAgentsData";
pub const SKWAD_PERSONAS_KEY: &str = "personasData";
pub const SKWAD_BENCH_AGENTS_KEY: &str = "benchAgentsData";

/// The scheme a pull request URL is recognized under. GitHub serves nothing
/// over plain HTTP, so a `http://` link is not one Knot records.
pub const PULL_REQUEST_URL_SCHEME: &str = "https://";

/// The host Knot recognizes without being told. Enterprise hosts are
/// supplied by the forge layer, which is the only part that knows which ones
/// are configured.
pub const PULL_REQUEST_HOST_GITHUB: &str = "github.com";

/// The longest a pull request URL can be and still parse, and so the most a
/// stream scanner needs to carry across a chunk boundary: a 253-byte host, a
/// 39-byte owner, a 100-byte repository, the scheme, the separators and the
/// number, rounded up.
pub const MAX_PULL_REQUEST_URL_LEN: usize = 512;
