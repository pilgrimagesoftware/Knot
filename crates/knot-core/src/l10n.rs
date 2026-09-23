//! Localization, backed by `rust-i18n` (locale catalogs under `locales/`,
//! `en` authoritative). Mirrors the `pilgrimagesoftware/dtrpg-app` Rust
//! app's approach - a real catalog + macro, not a hand-rolled lookup table.
//!
//! `rust_i18n::t!` expands using a `_rust_i18n_t` helper the `i18n!` macro
//! generates at the invoking crate's root (see `lib.rs`), so it can't be
//! called directly from other crates - these functions are the crate
//! boundary, wrapping the macro so callers keep a plain function API.

pub fn t(key: &str) -> String {
    rust_i18n::t!(key).to_string()
}

/// [`t`] with `%{name}` placeholders replaced by the matching `args` value.
///
/// A sentence that embeds a value has to stay one catalog entry: the word
/// order around the value is the translator's to choose, so the call site
/// must not assemble it from fragments. `rust_i18n::t!`'s own argument
/// form needs the names as literals at the macro call, which this crate's
/// function boundary cannot pass through - hence the substitution here. A
/// placeholder with no matching arg is left as written, so a missing value
/// shows up in the copy instead of silently emptying it.
#[must_use]
pub fn t_with(key: &str, args: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in args {
        text = text.replace(&format!("%{{{name}}}"), value);
    }
    text
}

/// Formats `count` with the correctly localized noun form.
///
/// `singular_key` and `plural_key` are translation keys (e.g. `"count.file"`
/// / `"count.files"`), not literal words, so the noun form - and its
/// position relative to the number - comes from the locale catalog rather
/// than being assumed at the call site. Returns `"{count} {noun}"` where
/// `noun` is the localized value of `singular_key` when `count == 1`,
/// otherwise the localized value of `plural_key`.
#[must_use]
pub fn pluralize(count: u64, singular_key: &str, plural_key: &str) -> String {
    format!("{count} {}", plural_noun(count, singular_key, plural_key))
}

/// The localized noun alone, without the count in front of it, for callers
/// that render the two separately - a diff stat that colors the number but
/// not the word it counts, say. Same key-not-word contract as
/// [`pluralize`], which is built on this.
#[must_use]
pub fn plural_noun(count: u64, singular_key: &str, plural_key: &str) -> String {
    t(if count == 1 { singular_key } else { plural_key })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_resolves() {
        assert_eq!(t("app.name"), "Knot");
    }

    #[test]
    fn context_usage_key_resolves() {
        assert_eq!(t("panel.context_usage"), "Context usage");
    }

    /// Three-level keys resolve, which the git panel's section titles use.
    #[test]
    fn git_panel_keys_resolve() {
        for key in ["git_panel.title",
                    "git_panel.clean",
                    "git_panel.not_a_repository",
                    "git_panel.select_a_file",
                    "git_panel.binary_file",
                    "git_panel.no_changes",
                    "git_panel.commit",
                    "git_panel.section.staged",
                    "git_panel.section.unstaged",
                    "git_panel.section.untracked",
                    "git_panel.section.conflicted",
                    "git_panel.error.path_not_utf8"]
        {
            assert_ne!(t(key), key, "{key} did not resolve");
        }
    }

    /// Every sentence that embeds a value stays one entry, substituted
    /// rather than assembled at the call site.
    #[test]
    fn git_panel_substitutions_resolve() {
        assert!(t_with("git_panel.discard_title", &[("path", "src/f.rs")]).contains("src/f.rs"));
        assert!(t_with("git_panel.status_failed", &[("reason", "boom")]).contains("boom"));
        assert!(t_with("git_panel.ahead", &[("count", "3")]).contains('3'));
    }

    /// The code-block copy control's pair. Asserted as resolving rather than
    /// as its English copy, per the catalogue-key contract: the renderer that
    /// reads them needs a `Window` and an `App`, so a key missing from the
    /// catalogue would otherwise surface only as the raw key drawn in a
    /// tooltip.
    #[test]
    fn code_block_copy_keys_resolve() {
        for key in ["panel.copy_code", "panel.copied_code"] {
            assert_ne!(t(key), key, "{key} should resolve to its localized copy");
        }
    }

    #[test]
    fn unknown_key_falls_back_to_key_string() {
        assert_eq!(t("does.not.exist"), "does.not.exist");
    }

    #[test]
    fn t_with_substitutes_a_named_placeholder() {
        assert_eq!(t_with("quit.working_message_many", &[("count", "3")]),
                   "3 agents are still working. Quitting now ends their sessions, and any work in \
                    progress is lost.");
    }

    #[test]
    fn t_with_leaves_an_unmatched_placeholder_visible() {
        // Better a literal `%{count}` in the copy than a sentence that
        // silently lost its number.
        assert!(t_with("quit.working_message_many", &[]).contains("%{count}"));
    }

    #[test]
    fn zero_uses_plural() {
        assert_eq!(pluralize(0, "count.file", "count.files"), "0 files");
    }

    #[test]
    fn one_uses_singular() {
        assert_eq!(pluralize(1, "count.file", "count.files"), "1 file");
    }

    #[test]
    fn many_uses_plural() {
        assert_eq!(pluralize(3, "count.file", "count.files"), "3 files");
    }

    #[test]
    fn plural_noun_omits_the_count_but_keeps_its_form() {
        assert_eq!(plural_noun(0, "count.file", "count.files"), "files");
        assert_eq!(plural_noun(1, "count.file", "count.files"), "file");
        assert_eq!(plural_noun(3, "count.file", "count.files"), "files");
    }
}
