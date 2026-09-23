#!/usr/bin/env bash
#
# Fails when a Rust source file sits beside a `mod.rs` that never declares it.
#
# An undeclared file is not compiled, and nothing says so: `cargo build`
# succeeds, `cargo test` succeeds, and the only trace is a test count that
# dropped. So the suite goes green having silently stopped running whatever was
# in that file - the worst shape a regression can take, because it looks like
# success.
#
# The moment this happens is a merge. Two branches that each add a test module
# insert into the same sorted run of `mod` lines in the same `mod.rs`, git
# reports a conflict rather than merging them, and resolving it by keeping one
# side is both easy and invisible (#355; the near miss was #350 and #354, whose
# `mod permission_keybindings;` and `mod panel_scroll;` land three lines apart).
#
# This pairs with the `mod.rs` rule in AGENTS.md - "declares; it does not
# implement" - which is what makes a plain grep for the declaration sufficient:
# if every `mod.rs` is declarations and re-exports, a missing declaration is
# the only way a sibling goes uncompiled.
#
# Usage: scripts/check-declared-modules.sh

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Only the Rust port; `target/` is build output. See check-file-size.sh for why
# this avoids `mapfile` - macOS ships bash 3.2, and this runs on both runners.
checked=0
missing=0
while IFS= read -r modfile; do
    dir="$(dirname "$modfile")"
    while IFS= read -r sibling; do
        base="$(basename "$sibling" .rs)"
        [ "$base" = "mod" ] && continue
        checked=$((checked + 1))
        # `mod foo;`, `pub mod foo;`, `pub(crate) mod foo;`, `#[cfg(test)] mod foo;`
        # on one line - all of which the workspace uses.
        if ! grep -qE "(^|[[:space:]])mod[[:space:]]+${base}[[:space:]]*;" "$modfile"; then
            printf '%s: not declared in %s\n' "$sibling" "$modfile" >&2
            missing=$((missing + 1))
        fi
    done <<INNER
$(find "$dir" -maxdepth 1 -name '*.rs' | sort)
INNER
done <<EOF
$(find crates -name 'mod.rs' -not -path '*/target/*' | sort)
EOF

if [ "$checked" -eq 0 ]; then
    printf 'check-declared-modules: found no modules to check under crates/\n' >&2
    exit 2
fi

if [ "$missing" -gt 0 ]; then
    cat >&2 <<EOF

$missing file(s) sit beside a mod.rs that does not declare them, so they are
not compiled and anything they test is not running.

If the file is wanted, add its declaration to the mod.rs above, keeping the
existing sorted order. If it is genuinely dead, delete it - leaving it
undeclared reads as an oversight to the next person either way.

If you hit this while resolving a merge conflict in a run of \`mod\` lines,
the resolution is almost always to keep every line from both sides.
EOF
    exit 1
fi

printf 'check-declared-modules: %s modules, all declared\n' "$checked"
