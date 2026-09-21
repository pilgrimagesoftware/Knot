#!/usr/bin/env bash
#
# Fails when a Rust source file in the workspace exceeds the line limit.
#
# The limit is a forcing function, not a style rule: a file nobody can hold in
# their head stops getting reviewed properly, and the only reliable moment to
# split one is before it is large. It was applied once by hand (1f52a76,
# "split oversized modules into submodules for the file-size limit") and then
# regressed - workspace_window/mod.rs came out of that commit at 2216 lines and
# reached 4578 four days later - because nothing enforced it.
#
# Usage: scripts/check-file-size.sh [limit]

set -euo pipefail

LIMIT="${1:-700}"

if [ "$LIMIT" -lt 1 ]; then
    printf 'check-file-size: limit must be a positive integer, got %s\n' "$LIMIT" >&2
    exit 2
fi

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

# Only the Rust port. The Swift tree is the behavioral reference and is not
# being restructured here; `target/` is build output.
#
# A plain while-read rather than `mapfile`: macOS ships bash 3.2, which has no
# `mapfile`, and this has to run the same locally and on both CI runners.
total=0
over=0
while IFS= read -r file; do
    total=$((total + 1))
    lines=$(wc -l < "$file" | tr -d ' ')
    if [ "$lines" -gt "$LIMIT" ]; then
        printf '%s: %s lines (limit %s)\n' "$file" "$lines" "$LIMIT" >&2
        over=$((over + 1))
    fi
done <<EOF
$(find crates -name '*.rs' -not -path '*/target/*' | sort)
EOF

if [ "$total" -eq 0 ]; then
    printf 'check-file-size: found no Rust sources under crates/\n' >&2
    exit 2
fi

if [ "$over" -gt 0 ]; then
    cat >&2 <<EOF

$over file(s) over the $LIMIT-line limit.

Split by concern, not by line count: pull out the group of items that answer
one question, give the new module a doc comment saying what it owns, and widen
visibility only as far as the move needs. Inherent methods can live in several
\`impl\` blocks across files, so a large \`impl\` splits without changing any
call site.

If a file genuinely cannot be split - a generated table, one long match over a
protocol's variants - say so in this script rather than raising the limit.
EOF
    exit 1
fi

printf 'check-file-size: %s Rust files, none over %s lines\n' "$total" "$LIMIT"
