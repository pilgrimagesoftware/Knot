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
    let key = if count == 1 { singular_key } else { plural_key };
    format!("{count} {}", t(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_key_resolves() {
        assert_eq!(t("app.name"), "Knot");
    }

    #[test]
    fn unknown_key_falls_back_to_key_string() {
        assert_eq!(t("does.not.exist"), "does.not.exist");
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
}
