# Architecture Decision Records

Each ADR captures one architectural decision, the options weighed, and the
consequences. New records start `proposed` and flip to `accepted` once the
decision survives contact with the code. A reversed decision is a new ADR that
supersedes the old one; accepted records are not edited.

Scaffold a new record with `/adr "<title>"` (see `.claude/commands/adr.md`), or
copy `0000-template.md` by hand and add a row below.

| ADR | Title | Status | Decision | Date |
| --- | ----- | ------ | -------- | ---- |
| [0001](0001-git-flow.md) | Git-flow branching model | accepted | `develop` integration branch, `main` release-only | 2026-09-10 |
| [0002](0002-gpui-base-auto-grow-fork.md) | Fork gpui-base to allow auto-grow on any multi-line mode | accepted | `[patch.crates-io]` fork until upstream relaxes the bound | 2026-09-23 |
