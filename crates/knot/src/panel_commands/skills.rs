//! Skills the selected agent can reach, read from `SKILL.md` frontmatter.
//!
//! A skill root holds one directory per skill, each with a `SKILL.md` whose
//! YAML frontmatter names it and says what it is for. This module reads
//! those two fields and nothing else: a skill is offered as text to
//! complete, never loaded, run, or written back.
//!
//! A root that is missing or unreadable yields no entries rather than an
//! error, per the spec's "a skill root that cannot be read is not an
//! error" - the lookup lists what it could read.

use std::path::Path;
use std::path::PathBuf;

use super::LookupEntry;
use super::LookupSource;

/// Where skills live under a home or project directory.
const SKILLS_SUBDIR: &[&str] = &[".claude", "skills"];

/// The per-skill file holding the frontmatter this module reads.
const SKILL_FILE: &str = "SKILL.md";

/// The skills half of the registry: a list of roots to scan.
pub(super) struct SkillRoots {
    roots: Vec<PathBuf>,
}

impl SkillRoots {
    /// The roots an agent working in `folder` can reach: the user's global
    /// skills, then the project's own. A project skill of the same name
    /// does not displace the global one - `LookupRegistry::from_sources`
    /// keeps whichever token it saw first.
    pub(super) fn for_agent(folder: Option<&Path>) -> Self {
        let mut roots = Vec::new();
        // `HOME`, not a home-directory crate: a GUI process keeps `HOME`
        // even without a shell environment, which is what
        // `knot-agent-launch`'s adapter lookup already relies on.
        if let Ok(home) = std::env::var("HOME")
           && !home.is_empty()
        {
            roots.push(skills_dir(Path::new(&home)));
        }
        if let Some(folder) = folder {
            let project = skills_dir(folder);
            if !roots.contains(&project) {
                roots.push(project);
            }
        }
        Self { roots }
    }

    #[cfg(test)]
    pub(super) fn from_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

impl LookupSource for SkillRoots {
    fn entries(&self) -> Vec<LookupEntry> {
        let mut entries: Vec<LookupEntry> =
            self.roots.iter().flat_map(|root| scan_root(root)).collect();
        entries.sort_by(|left, right| left.token.cmp(&right.token));
        entries
    }
}

fn skills_dir(base: &Path) -> PathBuf {
    SKILLS_SUBDIR.iter()
                 .fold(base.to_path_buf(), |path, part| path.join(part))
}

/// Every readable skill directly under `root`. An unreadable root, or a
/// skill whose `SKILL.md` cannot be read or names nothing, is skipped.
fn scan_root(root: &Path) -> Vec<LookupEntry> {
    let Ok(dir) = std::fs::read_dir(root)
    else {
        return Vec::new();
    };
    dir.flatten()
       .filter_map(|entry| {
           let manifest = entry.path().join(SKILL_FILE);
           let text = std::fs::read_to_string(&manifest).ok()?;
           skill_entry(&text)
       })
       .collect()
}

/// The entry a `SKILL.md`'s frontmatter describes, or `None` when it names
/// no skill.
fn skill_entry(text: &str) -> Option<LookupEntry> {
    let frontmatter = frontmatter(text)?;
    let name = field(frontmatter, "name")?;
    if name.is_empty() {
        return None;
    }
    let description = field(frontmatter, "description").unwrap_or_default();
    Some(LookupEntry::new(name, description))
}

/// The lines between the opening and closing `---` of a leading YAML
/// frontmatter block.
fn frontmatter(text: &str) -> Option<&str> {
    let rest = text.strip_prefix("---\n")
                   .or_else(|| text.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// The value of top-level scalar `key`, folded onto one line.
///
/// Handles the two shapes skills actually use: a value on the key's own
/// line, and a block scalar (`>-`, `|`) continued on the indented lines
/// below it. Anything else yields whatever sat on the key's line.
fn field(frontmatter: &str, key: &str) -> Option<String> {
    let mut lines = frontmatter.lines();
    let prefix = format!("{key}:");
    let first = lines.by_ref()
                     .find(|line| line.starts_with(&prefix) && !line.starts_with(' '))?;
    let value = first[prefix.len()..].trim();
    let mut value = strip_quotes(value.trim_start_matches(['>', '|', '-']).trim()).to_string();
    for line in lines {
        if !line.starts_with(' ') && !line.starts_with('\t') {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !value.is_empty() {
            value.push(' ');
        }
        value.push_str(line);
    }
    Some(value)
}

fn strip_quotes(value: &str) -> &str {
    value.strip_prefix('"')
         .and_then(|value| value.strip_suffix('"'))
         .or_else(|| {
             value.strip_prefix('\'')
                  .and_then(|value| value.strip_suffix('\''))
         })
         .unwrap_or(value)
}
