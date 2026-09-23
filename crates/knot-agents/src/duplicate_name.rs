//! Naming a duplicated agent.
//!
//! `format!("{name} (copy)")` at the menu was two defects in one line: it
//! stacked - `Foo (copy) (copy) (copy)` after three duplications - and it was
//! an English literal in a codebase where every other user-facing string goes
//! through the catalog.
//!
//! Numbering instead, from the stem rather than from whatever the source
//! happened to be called: duplicating `Foo 2` gives `Foo 3`, not `Foo 2 2`.

use knot_core::l10n;

/// The first number a duplicate can take. `Foo` duplicates to `Foo 2`,
/// because `Foo` is already the first of its series.
const FIRST_DUPLICATE: usize = 2;

/// What the catalog puts between a stem and its number.
///
/// Derived by rendering the entry with markers and taking what lands between
/// them, so the catalog stays the one place that decides the shape. Splitting
/// on this rather than on a literal space is what keeps a catalog spelling it
/// `%{name} #%{number}` from reading `Agent 7` as stem `Agent`.
fn separator() -> String {
    const STEM: char = '\u{1}';
    const NUMBER: char = '\u{2}';

    let rendered = render(&STEM.to_string(), &NUMBER.to_string());
    rendered.strip_prefix(STEM)
            .and_then(|rest| rest.strip_suffix(NUMBER))
            .filter(|separator| !separator.is_empty())
            .unwrap_or(" ")
            .to_string()
}

fn render(stem: &str, number: &str) -> String {
    l10n::t_with("agent.duplicate_name",
                 &[("name", stem), ("number", number)])
}

/// `name` with a trailing series number removed, if it has one.
///
/// Any trailing integer counts. `Catch 22` therefore stems to `Catch`, which
/// is wrong but harmless - the alternative is guessing which trailing numbers
/// are part of the name, and a duplicate called `Catch 2` is a better failure
/// than one called `Catch 22 2`.
fn stem(name: &str) -> &str {
    let separator = separator();

    match name.rsplit_once(separator.as_str()) {
        Some((stem, tail)) if !stem.is_empty() && tail.parse::<usize>().is_ok() => stem,
        _ => name,
    }
}

/// A name for a duplicate of `source` that none of `taken` already uses.
///
/// Numbers from the stem, so duplicating from the middle of a series extends
/// that series rather than nesting a new one. Picks the lowest free number
/// rather than one past the source's: duplicating `Foo 5` after `Foo 2` was
/// deleted reuses `Foo 2`, which is what a user looking at the gap expects.
///
/// `taken` is every existing agent name. The store does not require names to
/// be unique, so this is a courtesy rather than a constraint - it cannot
/// fail, and its worst case is repeating a name the user typed by hand.
pub fn duplicate_name<'a>(source: &str, taken: impl IntoIterator<Item = &'a str>) -> String {
    let taken: Vec<&str> = taken.into_iter().collect();
    let stem = stem(source);

    (FIRST_DUPLICATE..).map(|number| render(stem, &number.to_string()))
                       .find(|candidate| !taken.contains(&candidate.as_str()))
                       // `(FIRST_DUPLICATE..)` is unbounded and every step
                       // either collides or returns, so reaching this would
                       // need more agents than the machine has memory for.
                       .unwrap_or_else(|| render(stem, &FIRST_DUPLICATE.to_string()))
}

#[cfg(test)]
mod tests;
