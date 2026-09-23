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
# `cargo clippy --all-targets` passes too, and for the same reason: clippy sees
# only what the crate graph reaches. So an undeclared file also accumulates
# dead code, unused imports and lint violations invisibly, and the bill arrives
# as a wall of errors for whoever finally declares it. A dropped test count is
# the gentler half of this.
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
# ---------------------------------------------------------------------------
# Why this is one grep and not a pipeline
# ---------------------------------------------------------------------------
#
# Comments were once stripped with `sed 's,//.*,,' "$owner" | grep -q ...`.
# Under `set -o pipefail` that is a race, not a filter: `grep -q` exits at its
# first match, `sed` is still writing, `sed` takes SIGPIPE and exits 141, and
# pipefail promotes 141 to the pipeline's status - so a FOUND declaration reads
# as a missing one.
#
# The signature is backwards from every intuition about a flaky check, which is
# how it survived review: EARLY matches fail and LATE matches pass, because a
# late match leaves the producer with nothing left to write and therefore no
# signal to take. Measured on this repo, on the file that exposed it:
#
#     store.rs, `mod documents` (line 54 of 678)   26 failures / 150
#     store.rs, `mod tests`     (line 678 of 678)   0 failures / 150
#
# It is invisible on small inputs, so it survives any test written against a
# short fixture. Forced with a match on line 1 of a 200k-line file it is not
# rare at all - 40 failures out of 40.
#
# The rule generalises past this script: `set -o pipefail` plus ANY
# early-exiting consumer - `grep -q`, `head`, `grep -m N` - is a latent race.
# Fix it by removing the pipe. Retrying, or unsetting pipefail around it, both
# leave the race and only change how often you notice.
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
    # One grep, no pipeline, deliberately.
    #
    # This was `sed 's,//.*,,' "$owner" | grep -q ...`, which is a latent flake
    # under `set -o pipefail`: grep -q exits at the first match, sed is still
    # writing, sed takes SIGPIPE, and the pipeline reports 141 even though the
    # declaration was found. Whether that happens depends on file size, pipe
    # buffer and scheduling, so it passed on one machine and failed on another
    # - CI reported four declared modules in knot-core/settings/store.rs as
    # missing while the fifth, declared 620 lines further down, passed, because
    # by then sed had almost nothing left to write.
    #
    # `^[^/]*` is what replaces stripping comments: the prefix cannot contain a
    # slash, so `// mod foo;` and `use a::b; // mod foo;` are both rejected,
    # while every declaration form in the tree is accepted - bare `mod foo;`
    # (281), `pub mod foo;` (52), `pub(crate) mod foo;` (10), `pub(super) mod
    # foo;` (9), and `#[cfg(test)] mod foo;` on one line.
    if ! grep -qE "^[^/]*[[:space:]]*mod[[:space:]]+${base}[[:space:]]*;" "$owner"; then
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
