//! Durable configuration store: scalar settings plus the serialized
//! collections (saved agents, workspaces, personas, bench templates, recent
//! repos, recorded pull requests), with decode-tolerant migration,
//! first-launch source-folder detection, and a bounded recent-repos MRU.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! One [`Settings`] value is one settings surface over eight documents, whose
//! locations [`StorePaths`] derives:
//!
//! | Document | Holds | Directory |
//! |---|---|---|
//! | `preferences.json` | every scalar setting | user preferences |
//! | `agents.json` | saved agents | application data |
//! | `workspaces.json` | saved workspaces | application data |
//! | `workspace-ui-state.json` | per-workspace UI state | application data |
//! | `personas.json` | personas | application data |
//! | `bench.json` | bench templates | application data |
//! | `recent-repos.json` | recent repositories | application data |
//! | `pull-requests.json` | recorded pull requests | application data |
//!
//! Three kinds, distinguished by what the values are rather than by which
//! screen edits them: the scalars the user tunes, the collections the user
//! built, and - in `workspace-ui-state.json` - what the application recorded
//! about how its own windows were arranged, which the user never entered.
//! That last one is separate so that moving a window, the most frequent write
//! in the store and the least valuable, does not rewrite the roster.
//!
//! Every mutating helper writes immediately, and writes only the document it
//! changed: [`Settings::persist`] is the whole surface, while
//! [`Settings::persist_preferences`] and the per-collection writers are what
//! the helpers actually call. Each document loads on its own, so one that
//! fails to decode costs only what it held and the app still starts.
//!
//! Two one-way migrations run on load, and compose: an installation still
//! holding the single `settings.json` is migrated off it - see [`legacy`] -
//! and a `workspaces.json` whose records still carry UI state is split - see
//! [`workspace_split`]. The legacy migration's own output is combined, so the
//! split runs on its result as well as on a pre-existing document.
//!
//! Two upgrades run on the preferences document. The terminal font's
//! `"SF Mono"` default is replaced value-for-value, and a document written
//! before the two proportional fonts swapped roles has them exchanged once,
//! gated on the `settingsVersion` marker - see [`migrate_font_roles`].

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

mod documents;
mod legacy;
mod paths;
mod refresh;
mod workspace_split;

pub use paths::StorePaths;

pub use super::records::{
    BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, SavedPullRequest, Workspace,
    WorkspaceUiState,
};
use super::vocabulary::{AiProvider, AppearanceMode, AutopilotAction, UnknownVariant};
use crate::consts::{
    DEFAULT_PERSONAS, MARKDOWN_FONT_SIZE_DEFAULT, MCP_PORT_DEFAULT, MERMAID_THEME_DEFAULT,
    RECENT_REPOS_MAX, SETTINGS_VERSION_CURRENT, SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX,
    SIDEBAR_WIDTH_MIN, SOURCE_FOLDER_CANDIDATES, TERMINAL_FONT_DEFAULT, TERMINAL_FONT_SIZE_DEFAULT,
    TITLE_FONT_DEFAULT, TITLE_FONT_SIZE_DEFAULT, UI_FONT_DEFAULT, UI_FONT_SIZE_DEFAULT,
    VOICE_ENGINE_DEFAULT, VOICE_PUSH_TO_TALK_KEY_DEFAULT,
};
use crate::error::{Error, Result};

/// The whole persisted configuration surface. Load with [`Settings::load`],
/// mutate through the helpers (each persists), or set fields directly and call
/// [`Settings::persist`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Which arrangement the persisted document was written under, so a
    /// load-time migration runs exactly once. See [`SETTINGS_VERSION_CURRENT`]
    /// and [`migrate_font_roles`].
    ///
    /// Its own serde default, rather than the container's: a document with no
    /// `settingsVersion` key predates the marker and must read as `0`, while a
    /// fresh [`Settings::default`] is already current and must not be
    /// migrated.
    #[serde(default = "de_legacy_settings_version")]
    pub settings_version:               u32,
    pub appearance_mode:                AppearanceMode,
    pub restore_layout_on_launch:       bool,
    pub restore_conversation_on_launch: bool,
    pub keep_in_menu_bar:               bool,
    pub mcp_server_enabled:             bool,
    pub mcp_server_port:                u16,
    pub source_base_folder:             String,
    #[serde(rename = "sourceBaseFolderInitialized")]
    pub source_folder_detected:         bool,
    pub desktop_notifications_enabled:  bool,
    pub markdown_font_size:             i32,
    pub mermaid_theme:                  String,
    pub mermaid_scale:                  f64,
    pub agent_commands:                 BTreeMap<String, String>,
    pub agent_options:                  BTreeMap<String, String>,
    pub terminal_font_name:             String,
    pub terminal_font_size:             f64,
    pub ui_font_name:                   String,
    pub ui_font_size:                   f64,
    pub title_font_name:                String,
    pub title_font_size:                f64,
    /// The workspace sidebar's width in pixels, shared by every workspace
    /// window. Clamped into [`SIDEBAR_WIDTH_MIN`]..=[`SIDEBAR_WIDTH_MAX`] on
    /// load, so a reader never has to clamp it again. See
    /// `openspec/specs/agent-list-ui/spec.md`.
    pub sidebar_width:                  f64,
    pub autopilot_enabled:              bool,
    pub ai_provider:                    AiProvider,
    pub ai_api_key:                     String,
    pub autopilot_action:               AutopilotAction,
    pub autopilot_custom_prompt:        String,
    pub voice_enabled:                  bool,
    pub voice_engine:                   String,
    pub voice_push_to_talk_key:         i32,
    pub voice_auto_insert:              bool,
    /// Which chord sends a Panel-mode prompt: `false` (default) is Enter to
    /// send / Shift+Enter for a newline; `true` swaps them.
    pub agent_panel_shift_enter_sends:  bool,
    /// Collapse a Panel turn's contiguous tool calls into one summary line
    /// instead of a card per call. Off by default, so existing installs keep
    /// the per-call rendering. See
    /// `openspec/specs/collapsed-tool-call-summary/spec.md`.
    pub agent_panel_compact_tool_calls: bool,

    /// The durable collections, each persisted as its own document rather
    /// than as a key of the preferences document - hence `skip`, which also
    /// makes a stray collection key in a hand-edited preferences document
    /// ignored on read and never written back.
    #[serde(skip)]
    pub saved_agents:     Vec<SavedAgent>,
    #[serde(skip)]
    pub saved_workspaces: Vec<Workspace>,
    /// How each workspace's window was last arranged, keyed by workspace id.
    ///
    /// Beside `saved_workspaces` rather than inside it: the two have
    /// different lifetimes - a workspace's configuration outlives any window,
    /// and its arrangement is meaningless without one - and only this one is
    /// rewritten by a pointer drag. Written by
    /// [`Settings::persist_workspace_ui`] alone.
    #[serde(skip)]
    pub workspace_ui:     BTreeMap<Uuid, WorkspaceUiState>,
    #[serde(skip)]
    pub personas:         Vec<Persona>,
    #[serde(skip)]
    pub bench_agents:     Vec<BenchAgent>,
    #[serde(skip)]
    pub recent_repos:     Vec<String>,
    #[serde(skip)]
    pub pull_requests:    Vec<SavedPullRequest>,

    #[serde(skip)]
    paths: Option<StorePaths>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { settings_version:               SETTINGS_VERSION_CURRENT,
               appearance_mode:                AppearanceMode::default(),
               restore_layout_on_launch:       true,
               restore_conversation_on_launch: false,
               keep_in_menu_bar:               false,
               mcp_server_enabled:             true,
               mcp_server_port:                MCP_PORT_DEFAULT,
               source_base_folder:             String::new(),
               source_folder_detected:         false,
               desktop_notifications_enabled:  true,
               markdown_font_size:             MARKDOWN_FONT_SIZE_DEFAULT,
               mermaid_theme:                  MERMAID_THEME_DEFAULT.to_string(),
               mermaid_scale:                  1.0,
               agent_commands:                 BTreeMap::new(),
               agent_options:                  BTreeMap::new(),
               terminal_font_name:             TERMINAL_FONT_DEFAULT.to_string(),
               terminal_font_size:             TERMINAL_FONT_SIZE_DEFAULT,
               ui_font_name:                   UI_FONT_DEFAULT.to_string(),
               ui_font_size:                   UI_FONT_SIZE_DEFAULT,
               title_font_name:                TITLE_FONT_DEFAULT.to_string(),
               title_font_size:                TITLE_FONT_SIZE_DEFAULT,
               sidebar_width:                  SIDEBAR_WIDTH_DEFAULT,
               autopilot_enabled:              false,
               ai_provider:                    AiProvider::default(),
               ai_api_key:                     String::new(),
               autopilot_action:               AutopilotAction::default(),
               autopilot_custom_prompt:        String::new(),
               voice_enabled:                  false,
               voice_engine:                   VOICE_ENGINE_DEFAULT.to_string(),
               voice_push_to_talk_key:         VOICE_PUSH_TO_TALK_KEY_DEFAULT,
               voice_auto_insert:              true,
               agent_panel_shift_enter_sends:  false,
               agent_panel_compact_tool_calls: false,
               saved_agents:                   Vec::new(),
               saved_workspaces:               Vec::new(),
               workspace_ui:                   BTreeMap::new(),
               personas:                       Vec::new(),
               bench_agents:                   Vec::new(),
               recent_repos:                   Vec::new(),
               pull_requests:                  Vec::new(),
               paths:                          None, }
    }
}

impl Settings {
    /// Load from the platform's preferences and application-data
    /// directories, migrating a legacy single document first if one is
    /// present. A missing, unreadable or malformed document yields that
    /// document's defaults with `Ok` so the app still starts.
    pub fn load() -> Result<Self> {
        match StorePaths::platform() {
            Some(paths) => Self::load_with(paths),
            None => Ok(Self::default()),
        }
    }

    /// Load every document from one directory (tests, alternate profiles).
    /// Same tolerance rules as [`Settings::load`].
    pub fn load_from_root(dir: impl AsRef<Path>) -> Result<Self> {
        Self::load_with(StorePaths::rooted(dir.as_ref()))
    }

    /// An empty settings value whose documents live under `dir`.
    pub fn with_store_root(dir: impl Into<PathBuf>) -> Self {
        Self { paths: Some(StorePaths::rooted(dir)),
               ..Self::default() }
    }

    fn load_with(paths: StorePaths) -> Result<Self> {
        // The legacy migration's own output is a combined workspaces
        // document, so the split has to run on its result too - hence the
        // split inside `read_documents` rather than only on the branch that
        // skipped the legacy migration.
        if let Some(migrated) = legacy::migrate(&paths)? {
            return Ok(migrated);
        }
        Ok(Self::read_documents(paths))
    }

    /// Read each document on its own, so one that cannot be read costs only
    /// what it held.
    fn read_documents(paths: StorePaths) -> Self {
        let mut settings = match documents::read_object(&paths.preferences()) {
            Some(value) => Self::from_preferences(value),
            None => Self::default(),
        };
        settings.saved_agents = documents::read_collection(&paths.agents());
        let workspaces = workspace_split::read(&paths);
        settings.saved_workspaces = workspaces.workspaces;
        settings.workspace_ui = workspaces.ui_state;
        settings.personas = documents::read_collection(&paths.personas());
        settings.bench_agents = documents::read_collection(&paths.bench());
        settings.recent_repos = documents::read_collection(&paths.recent_repos());
        settings.pull_requests = documents::read_collection(&paths.pull_requests());
        settings.prune_workspace_ui();
        settings.paths = Some(paths);
        if workspaces.needs_write {
            // Best effort: a read-only store still loads, it simply splits
            // again next launch. The values are already correct in memory.
            let _ = settings.persist_workspace_ui();
            let _ = settings.persist_workspaces();
        }
        settings
    }

    /// Drop UI state for a workspace that no longer exists.
    ///
    /// On load rather than on deletion, and in this one place. Deleting a
    /// workspace has several paths - the sidebar, the command centre, an
    /// import that replaces the roster - and a missed one would leak an entry
    /// silently and forever. Load is the single funnel every path's result
    /// goes through, so pruning here cannot be bypassed by a new one.
    fn prune_workspace_ui(&mut self) {
        let live: std::collections::BTreeSet<Uuid> =
            self.saved_workspaces.iter().map(|w| w.id).collect();
        self.workspace_ui.retain(|id, _| live.contains(id));
    }

    /// Decode a preferences object and apply the load-time upgrades. The
    /// collections are left empty; the caller fills them from their own
    /// documents.
    ///
    /// Also the path a legacy document takes once its collection keys have
    /// been lifted out, so an upgrading installation gets exactly the same
    /// upgrades as a document already in the new arrangement.
    fn from_preferences(mut value: Value) -> Self {
        migrate_font_roles(&mut value);
        report_unreadable_vocabularies(&value);

        let mut settings: Self = serde_json::from_value(value).unwrap_or_default();
        // "SF Mono" was the terminal font default before JetBrains Mono
        // replaced it; a persisted document from before that change still
        // carries the old value, and SF Mono isn't reliably resolvable
        // through GPUI's font lookup (unlike AppKit, which special-cases
        // it), silently falling back to the UI font. Upgrade it once,
        // the same way a never-customized document already would default.
        if settings.terminal_font_name == "SF Mono" {
            settings.terminal_font_name = TERMINAL_FONT_DEFAULT.to_string();
        }
        // A hand-edited or future-written width outside the divider's range
        // describes an intent the window cannot honour; the nearest legal
        // width is closer to it than the default is. Clamping here rather
        // than in the window keeps every reader of the setting free of the
        // bound.
        settings.sidebar_width = settings.sidebar_width
                                         .clamp(SIDEBAR_WIDTH_MIN, SIDEBAR_WIDTH_MAX);
        settings
    }

    /// Where this value's documents live, falling back to the platform
    /// directories when it was not bound to an explicit root.
    fn resolved_paths(&self) -> Result<StorePaths> {
        self.paths
            .clone()
            .or_else(StorePaths::platform)
            .ok_or_else(|| Error::Config("no config directory available".to_string()))
    }

    /// Write every document.
    ///
    /// The whole surface at once, for a caller that changed more than one
    /// kind of value or does not know which. A caller that does know should
    /// use the writer for what it changed: it then does not rewrite five
    /// documents to record one edit, and - the reason that matters - does not
    /// write back its own stale copy of what another window persisted in the
    /// meantime.
    pub fn persist(&self) -> Result<()> {
        self.persist_preferences()?;
        self.persist_roster()?;
        self.persist_workspace_ui()?;
        self.persist_personas()?;
        self.persist_bench()?;
        self.persist_recent_repos()?;
        self.persist_pull_requests()
    }

    /// Write the preferences document: every scalar setting, and nothing
    /// else.
    pub fn persist_preferences(&self) -> Result<()> {
        documents::write(&self.resolved_paths()?.preferences(),
                         &serde_json::to_string_pretty(self)?)
    }

    /// Write the saved-agents and saved-workspaces documents.
    ///
    /// The two together because that is the unit every caller changes: an
    /// agent belongs to a workspace, so the roster is rebuilt from the agent
    /// store as a pair or not at all.
    pub fn persist_roster(&self) -> Result<()> {
        self.persist_agents()?;
        self.persist_workspaces()
    }

    fn persist_agents(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.agents(), &self.saved_agents)
    }

    fn persist_workspaces(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.workspaces(), &self.saved_workspaces)
    }

    /// Write the per-workspace UI-state document, and nothing else.
    ///
    /// The point of the whole arrangement: a window move, resize, split or
    /// detach persists through here, so it costs one small document rather
    /// than a rewrite of the saved agents and saved workspaces the user
    /// built. See `openspec/specs/settings-persistence/spec.md`.
    pub fn persist_workspace_ui(&self) -> Result<()> {
        documents::write_map(&self.resolved_paths()?.workspace_ui_state(),
                             &self.workspace_ui)
    }

    /// The UI state recorded for `workspace`, or the defaults if none was.
    ///
    /// An absent entry is a workspace whose window has not been arranged yet,
    /// which is exactly the default arrangement - so a reader never has to
    /// distinguish "missing" from "never moved".
    pub fn workspace_ui(&self, workspace: Uuid) -> WorkspaceUiState {
        self.workspace_ui
            .get(&workspace)
            .cloned()
            .unwrap_or_default()
    }

    /// Change `workspace`'s UI state and persist the UI-state document.
    ///
    /// Creates the entry if there is none, so a caller does not have to.
    pub fn update_workspace_ui(&mut self, workspace: Uuid,
                               edit: impl FnOnce(&mut WorkspaceUiState))
                               -> Result<()> {
        edit(self.workspace_ui.entry(workspace).or_default());
        self.persist_workspace_ui()
    }

    fn persist_personas(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.personas(), &self.personas)
    }

    fn persist_bench(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.bench(), &self.bench_agents)
    }

    fn persist_recent_repos(&self) -> Result<()> {
        documents::write_collection(&self.resolved_paths()?.recent_repos(), &self.recent_repos)
    }

    /// Write the recorded-pull-requests document.
    ///
    /// Skipped while there is nothing to record and no document already
    /// exists, so an installation that has never seen a pull request grows no
    /// file for the feature. Once the document exists it is always written,
    /// including when the last record is removed - otherwise a removal would
    /// not survive a restart, which is the one thing the spec says it must.
    pub fn persist_pull_requests(&self) -> Result<()> {
        let path = self.resolved_paths()?.pull_requests();
        if self.pull_requests.is_empty() && !path.exists() {
            return Ok(());
        }
        documents::write_collection(&path, &self.pull_requests)
    }

    /// On first launch with no source folder set, adopt the first existing
    /// directory among the common source locations and mark detection done so
    /// it never runs again.
    pub fn init_source_folder(&mut self) -> Result<()> {
        if self.source_folder_detected {
            return Ok(());
        }
        let expanded: Vec<PathBuf> = SOURCE_FOLDER_CANDIDATES.iter()
                                                             .map(|c| expand_tilde(c))
                                                             .collect();
        let refs: Vec<&Path> = expanded.iter().map(PathBuf::as_path).collect();
        if let Some(found) = detect_source_base_folder(&refs) {
            self.source_base_folder = found.to_string_lossy().into_owned();
        }
        self.source_folder_detected = true;
        self.persist_preferences()
    }

    /// Move `name` to the front of the recent-repos list, de-duplicating, and
    /// cap the list length.
    pub fn add_recent_repo(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.recent_repos.retain(|entry| entry != &name);
        self.recent_repos.insert(0, name);
        self.recent_repos.truncate(RECENT_REPOS_MAX);
        self.persist_recent_repos()
    }

    /// Add a bench template, replacing any existing entry for the same folder.
    pub fn add_bench_agent(&mut self, entry: BenchAgent) -> Result<()> {
        self.bench_agents.retain(|b| b.folder != entry.folder);
        self.bench_agents.push(entry);
        self.persist_bench()
    }

    /// Personas excluding soft-deleted ones, sorted case-insensitively by name.
    pub fn active_personas(&self) -> Vec<&Persona> {
        let mut personas: Vec<&Persona> = self.personas
                                              .iter()
                                              .filter(|p| p.state != PersonaState::Deleted)
                                              .collect();
        personas.sort_by_key(|p| p.name.to_lowercase());
        personas
    }

    /// Install any shipped default persona whose id is not already present
    /// (deleted entries count as present, so a removed default stays removed).
    pub fn install_default_personas(&mut self) -> Result<()> {
        let mut changed = false;
        for persona in default_personas() {
            if self.personas.iter().any(|p| p.id == persona.id) {
                continue;
            }
            self.personas.push(persona);
            changed = true;
        }
        if changed {
            self.persist_personas()?;
        }
        Ok(())
    }

    /// Add a new user persona, enabled by default.
    pub fn add_persona(&mut self, name: impl Into<String>, instructions: impl Into<String>)
                       -> Result<&Persona> {
        let persona = Persona { id:           Uuid::new_v4(),
                                name:         name.into(),
                                instructions: instructions.into(),
                                persona_type: PersonaType::User,
                                state:        PersonaState::Enabled, };
        self.personas.push(persona);
        self.persist_personas()?;
        Ok(self.personas.last().expect("just pushed"))
    }

    /// Rewrite name/instructions for an existing persona of any type. A no-op
    /// if `id` is not present.
    pub fn update_persona(&mut self, id: Uuid, name: impl Into<String>,
                          instructions: impl Into<String>)
                          -> Result<()> {
        let Some(persona) = self.personas.iter_mut().find(|p| p.id == id)
        else {
            return Ok(());
        };
        persona.name = name.into();
        persona.instructions = instructions.into();
        self.persist_personas()
    }

    /// Look up a persona by id, restricted to the active (non-deleted) list.
    pub fn persona(&self, id: Uuid) -> Option<&Persona> {
        self.active_personas().into_iter().find(|p| p.id == id)
    }

    /// Remove a persona: soft delete (state becomes `deleted`, record kept)
    /// for a system persona, hard delete (record removed) for a user persona.
    /// A no-op if `id` is not present.
    pub fn remove_persona(&mut self, id: Uuid) -> Result<()> {
        let Some(index) = self.personas.iter().position(|p| p.id == id)
        else {
            return Ok(());
        };
        if self.personas[index].persona_type == PersonaType::System {
            self.personas[index].state = PersonaState::Deleted;
        }
        else {
            self.personas.remove(index);
        }
        self.persist_personas()
    }

    /// Reset every shipped default persona already present (matched by id,
    /// including soft-deleted ones) to its shipped name, instructions, type,
    /// and state; append any shipped default that is entirely missing. User
    /// personas are untouched.
    pub fn restore_default_personas(&mut self) -> Result<()> {
        for default in default_personas() {
            match self.personas.iter_mut().find(|p| p.id == default.id) {
                Some(existing) => *existing = default,
                None => self.personas.push(default),
            }
        }
        self.persist_personas()
    }
}

/// Return the first candidate that is an existing directory.
pub fn detect_source_base_folder(candidates: &[&Path]) -> Option<PathBuf> {
    candidates.iter()
              .find(|path| path.is_dir())
              .map(|path| path.to_path_buf())
}

/// The six personas shipped with the app, keyed by fixed ids.
fn default_personas() -> Vec<Persona> {
    DEFAULT_PERSONAS.iter()
                    .map(|(id, name, instructions)| {
                        Persona {
            id: Uuid::parse_str(id).expect("default persona id is a valid uuid"),
            name: (*name).to_string(),
            instructions: (*instructions).to_string(),
            persona_type: PersonaType::System,
            state: PersonaState::Enabled,
        }
                    })
                    .collect()
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/")
       && let Some(base) = BaseDirs::new()
    {
        return base.home_dir().join(rest);
    }
    PathBuf::from(path)
}

/// The version a document carrying no `settingsVersion` key was written under:
/// the pre-swap font roles.
fn de_legacy_settings_version() -> u32 {
    0
}

/// Exchange the two proportional font settings in a pre-swap document.
///
/// Before the roles were swapped, `uiFontName` / `uiFontSize` held the title
/// face and `titleFontName` / `titleFontSize` held the application-wide
/// default. Exchanging the values keeps a user's customized fonts on the text
/// they were chosen for.
///
/// Runs on the raw document because only the raw document says which keys the
/// user actually customized: after deserializing, an absent key is
/// indistinguishable from one holding the default, so exchanging both would
/// invert the defaults for every user who never picked a font. An absent key
/// therefore stays absent and takes the new default.
///
/// Gated on `settingsVersion`, so it runs exactly once. The migrated value is
/// not written eagerly; the next persist records it, the same as the
/// `"SF Mono"` upgrade.
/// Says so, once, when a stored vocabulary value could not be read.
///
/// Deserialization is deliberately tolerant - one bad field must not take the
/// whole document down - so without this the substitution is invisible and a
/// corrupt value behaves exactly like the real default. That
/// indistinguishability is what issue #224 was about.
fn report_unreadable_vocabularies(value: &Value) {
    fn check<T: std::str::FromStr<Err = UnknownVariant> + Default + std::fmt::Display>(value: &Value,
                                                                                       field: &str)
    {
        let Some(stored) = value.get(field).and_then(Value::as_str)
        else {
            return;
        };
        if let (_, Some(unknown)) = {
            let parsed = stored.parse::<T>();
            match parsed {
                Ok(v) => (v, None),
                Err(e) => (T::default(), Some(e)),
            }
        } {
            eprintln!("knot-core: settings field `{field}`: {unknown}; using {}",
                      T::default());
        }
    }

    check::<AppearanceMode>(value, "appearanceMode");
    check::<AiProvider>(value, "aiProvider");
    check::<AutopilotAction>(value, "autopilotAction");
}

fn migrate_font_roles(document: &mut Value) {
    let Some(object) = document.as_object_mut()
    else {
        return;
    };
    let version = object.get("settingsVersion")
                        .and_then(Value::as_u64)
                        .unwrap_or_else(|| u64::from(de_legacy_settings_version()));
    if version >= u64::from(SETTINGS_VERSION_CURRENT) {
        return;
    }

    exchange_entries(object, "uiFontName", "titleFontName");
    exchange_entries(object, "uiFontSize", "titleFontSize");

    object.insert("settingsVersion".to_string(),
                  Value::from(SETTINGS_VERSION_CURRENT));
}

/// Move the values under `left` and `right` past each other. A key the
/// document does not carry is left absent rather than created, so a setting
/// the user never customized keeps its default instead of inheriting the
/// other's value.
fn exchange_entries(object: &mut serde_json::Map<String, Value>, left: &str, right: &str) {
    match (object.remove(left), object.remove(right)) {
        (Some(was_left), Some(was_right)) => {
            object.insert(left.to_string(), was_right);
            object.insert(right.to_string(), was_left);
        }
        (Some(was_left), None) => {
            object.insert(right.to_string(), was_left);
        }
        (None, Some(was_right)) => {
            object.insert(left.to_string(), was_right);
        }
        (None, None) => {}
    }
}

#[cfg(test)]
mod tests;
