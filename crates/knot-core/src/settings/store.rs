//! Durable configuration store: scalar settings plus the serialized
//! collections (saved agents, workspaces, personas, bench templates, recent
//! repos), with decode-tolerant migration, first-launch source-folder
//! detection, and a bounded recent-repos MRU.
//!
//! Contract: `openspec/specs/settings-persistence/spec.md`.
//!
//! The whole store is one [`Settings`] value serialized as a single JSON
//! document. Every mutating helper writes the document immediately. A
//! collection blob that fails to decode yields an empty collection rather than
//! failing the load, so the app always starts.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::{BaseDirs, ProjectDirs};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use uuid::Uuid;

pub use super::records::{BenchAgent, Persona, PersonaState, PersonaType, SavedAgent, Workspace};
use crate::consts::{
    AI_PROVIDER_DEFAULT, APP_NAME, APPEARANCE_MODE_DEFAULT, AUTOPILOT_ACTION_DEFAULT,
    DEFAULT_PERSONAS, MARKDOWN_FONT_SIZE_DEFAULT, MCP_PORT_DEFAULT, MERMAID_THEME_DEFAULT,
    ORG_NAME, ORG_QUALIFIER, RECENT_REPOS_MAX, SETTINGS_FILE, SETTINGS_TEMP_EXTENSION,
    SOURCE_FOLDER_CANDIDATES, TERMINAL_FONT_DEFAULT, TERMINAL_FONT_SIZE_DEFAULT,
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
    pub appearance_mode:                String,
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
    pub autopilot_enabled:              bool,
    pub ai_provider:                    String,
    pub ai_api_key:                     String,
    pub autopilot_action:               String,
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

    #[serde(deserialize_with = "de_tolerant_vec")]
    pub saved_agents:     Vec<SavedAgent>,
    #[serde(deserialize_with = "de_tolerant_vec")]
    pub saved_workspaces: Vec<Workspace>,
    #[serde(deserialize_with = "de_tolerant_vec")]
    pub personas:         Vec<Persona>,
    #[serde(deserialize_with = "de_tolerant_vec")]
    pub bench_agents:     Vec<BenchAgent>,
    #[serde(deserialize_with = "de_tolerant_vec")]
    pub recent_repos:     Vec<String>,

    #[serde(skip)]
    store_path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { appearance_mode:                APPEARANCE_MODE_DEFAULT.to_string(),
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
               autopilot_enabled:              false,
               ai_provider:                    AI_PROVIDER_DEFAULT.to_string(),
               ai_api_key:                     String::new(),
               autopilot_action:               AUTOPILOT_ACTION_DEFAULT.to_string(),
               autopilot_custom_prompt:        String::new(),
               voice_enabled:                  false,
               voice_engine:                   VOICE_ENGINE_DEFAULT.to_string(),
               voice_push_to_talk_key:         VOICE_PUSH_TO_TALK_KEY_DEFAULT,
               voice_auto_insert:              true,
               agent_panel_shift_enter_sends:  false,
               agent_panel_compact_tool_calls: false,
               saved_agents:                   Vec::new(),
               saved_workspaces:               Vec::new(),
               personas:                       Vec::new(),
               bench_agents:                   Vec::new(),
               recent_repos:                   Vec::new(),
               store_path:                     None, }
    }
}

impl Settings {
    /// Load from the platform config directory. A missing file, an unreadable
    /// blob, or a non-object document all yield defaults with `Ok` so the app
    /// still starts. Only an I/O error other than "not found" is returned as
    /// `Err`.
    pub fn load() -> Result<Self> {
        match Self::platform_store_path() {
            Some(path) => Self::load_at(&path, Some(path.clone())),
            None => Ok(Self::default()),
        }
    }

    /// Load from an explicit path (tests, alternate profiles). Same tolerance
    /// rules as [`Settings::load`].
    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        Self::load_at(path, Some(path.to_path_buf()))
    }

    /// An empty settings value bound to an explicit store path.
    pub fn with_store_path(path: impl Into<PathBuf>) -> Self {
        Self { store_path: Some(path.into()),
               ..Self::default() }
    }

    fn load_at(path: &Path, store: Option<PathBuf>) -> Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(Self::bound(store));
            }
            Err(err) => return Err(err.into()),
        };

        let value: Value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) if value.is_object() => value,
            _ => return Ok(Self::bound(store)),
        };

        let mut settings: Self = serde_json::from_value(value).unwrap_or_default();
        settings.store_path = store;
        // "SF Mono" was the terminal font default before JetBrains Mono
        // replaced it; a persisted document from before that change still
        // carries the old value, and SF Mono isn't reliably resolvable
        // through GPUI's font lookup (unlike AppKit, which special-cases
        // it), silently falling back to the UI font. Upgrade it once,
        // the same way a never-customized document already would default.
        if settings.terminal_font_name == "SF Mono" {
            settings.terminal_font_name = TERMINAL_FONT_DEFAULT.to_string();
        }
        Ok(settings)
    }

    fn bound(store: Option<PathBuf>) -> Self {
        Self { store_path: store,
               ..Self::default() }
    }

    fn platform_store_path() -> Option<PathBuf> {
        ProjectDirs::from(ORG_QUALIFIER, ORG_NAME, APP_NAME).map(|dirs| {
                                                                dirs.config_dir()
                                                                    .join(SETTINGS_FILE)
                                                            })
    }

    /// The resolved path this value writes to.
    pub fn store_path(&self) -> Option<PathBuf> {
        self.store_path.clone().or_else(Self::platform_store_path)
    }

    /// Write the document to [`Settings::store_path`], creating the parent
    /// directory as needed.
    ///
    /// Written to a temporary file in the same directory and renamed into
    /// place, so the settings file is never observed half-written. Nearly
    /// every mutating helper on this type persists immediately, so this runs
    /// on most user actions; a truncating write interrupted by a crash or a
    /// power loss would leave unparseable JSON, and the next launch would
    /// silently fall back to defaults - losing every agent, workspace and
    /// persona with no way back. `rename` within one directory is atomic on
    /// macOS and Linux, so a reader sees either the old document or the new
    /// one.
    pub fn persist(&self) -> Result<()> {
        let path = self.store_path()
                       .ok_or_else(|| Error::Config("no config directory available".to_string()))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        // Same directory as the target: `rename` is only atomic within one
        // filesystem, and a temp dir may be on another.
        let temporary = path.with_extension(SETTINGS_TEMP_EXTENSION);
        fs::write(&temporary, json)?;
        match fs::rename(&temporary, &path) {
            Ok(()) => Ok(()),
            Err(error) => {
                // Leaving the temp file behind would shadow the next
                // attempt's write with a stale document.
                let _ = fs::remove_file(&temporary);
                Err(error.into())
            }
        }
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
        self.persist()
    }

    /// Move `name` to the front of the recent-repos list, de-duplicating, and
    /// cap the list length.
    pub fn add_recent_repo(&mut self, name: impl Into<String>) -> Result<()> {
        let name = name.into();
        self.recent_repos.retain(|entry| entry != &name);
        self.recent_repos.insert(0, name);
        self.recent_repos.truncate(RECENT_REPOS_MAX);
        self.persist()
    }

    /// Add a bench template, replacing any existing entry for the same folder.
    pub fn add_bench_agent(&mut self, entry: BenchAgent) -> Result<()> {
        self.bench_agents.retain(|b| b.folder != entry.folder);
        self.bench_agents.push(entry);
        self.persist()
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
            self.persist()?;
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
        self.persist()?;
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
        self.persist()
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
        self.persist()
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
        self.persist()
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

fn de_tolerant_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
    where D: Deserializer<'de>,
          T: DeserializeOwned {
    let value = Value::deserialize(deserializer)?;
    Ok(serde_json::from_value(value).unwrap_or_default())
}

#[cfg(test)]
mod tests;
