#!/usr/bin/env bash
#
# Fails when a Rust source file under crates/*/src/ is never declared by the
# module that owns it.
#
# An undeclared file is not compiled, and nothing says so: `cargo build`
# succeeds, `cargo test` succeeds, and the only trace is a test count that
# dropped. So the suite goes green having silently stopped running whatever was
# in that file - the worst shape a regression can take, because it looks like
# success.
#
# The moment this happens is a merge. Two branches that each add a test module
# insert into the same sorted run of `mod` lines in the same declaring file,
# git reports a conflict rather than merging them, and resolving it by keeping
# one side is both easy and invisible (#355; the near miss was #350 and #354,
# whose `mod permission_keybindings;` and `mod panel_scroll;` land three lines
# apart).
#
# Which file declares which is the language's own rule, so this covers every
# layout the workspace uses rather than just the test directories:
#
#   src/a/b.rs   -> src/a/mod.rs if it exists, else src/a.rs
#   src/a.rs     -> the crate root, src/lib.rs or src/main.rs
#
# That is what catches the three shapes we actually have: a sibling
# `<thing>/tests.rs` declared from `<thing>.rs`, a `tests/` directory with its
# own `mod.rs`, and a nested `<thing>/tests/*.rs` declared from the sibling
# `<thing>/tests.rs` one level up. An earlier draft walked only directories
# containing a `mod.rs` and missed the first and third entirely - 92 files,
# caught in review rather than by the check.
#
# Usage: scripts/check-declared-modules.sh

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Only files under a crate's src/. Integration tests at crate root
# (crates/*/tests/*.rs) are discovered by cargo itself - no `mod` line exists
# or is wanted for those, and walking them would report five false positives
# immediately. `target/` is build output.
#
# A plain while-read rather than `mapfile`: macOS ships bash 3.2, which has no
# `mapfile`, and this has to run the same locally and on both CI runners.
checked=0
missing=0
while IFS= read -r file; do
    base="$(basename "$file" .rs)"
    dir="$(dirname "$file")"

    # Crate roots and mod.rs declare; they are not themselves declared here.
    case "$base" in
        lib | main | mod) continue ;;
    esac

    if [ -f "$dir/mod.rs" ]; then
        owner="$dir/mod.rs"
    elif [ -f "$dir.rs" ]; then
        # src/a/b.rs where a/ has no mod.rs: a.rs owns it.
        owner="$dir.rs"
    elif [ -f "$dir/lib.rs" ]; then
        owner="$dir/lib.rs"
    elif [ -f "$dir/main.rs" ]; then
        owner="$dir/main.rs"
    else
        printf '%s: no module could own this file (no mod.rs, %s.rs, lib.rs or main.rs)\n' \
            "$file" "$dir" >&2
        missing=$((missing + 1))
        continue
    fi

    checked=$((checked + 1))
    # `mod foo;`, `pub mod foo;`, `pub(crate) mod foo;`, `#[cfg(test)] mod foo;`
    # on one line - all of which the workspace uses.
    if ! grep -qE "(^|[[:space:]])mod[[:space:]]+${base}[[:space:]]*;" "$owner"; then
        printf '%s: not declared in %s\n' "$file" "$owner" >&2
        missing=$((missing + 1))
    fi
done <<EOF
$(find crates -path '*/src/*' -name '*.rs' -not -path '*/target/*' | sort)
EOF

if [ "$checked" -eq 0 ]; then
    printf 'check-declared-modules: found no modules to check under crates/\n' >&2
    exit 2
fi

if [ "$missing" -gt 0 ]; then
    cat >&2 <<EOF

$missing file(s) are not declared by the module that owns them, so they are
not compiled and anything they test is not running.

If the file is wanted, add its declaration to the owning file above, keeping
the existing sorted order. If it is genuinely dead, delete it - leaving it
undeclared reads as an oversight to the next person either way.

If you hit this while resolving a merge conflict in a run of \`mod\` lines,
the resolution is almost always to keep every line from both sides.
EOF
    exit 1
fi

printf 'check-declared-modules: %s modules, all declared\n' "$checked"
