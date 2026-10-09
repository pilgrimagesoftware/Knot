# Knot

AI coding agents, tied together.

Knot is a family of repositories. This one gathers them as git submodules so
you can check out everything in one go.

| Repository | What it is | License |
| --- | --- | --- |
| [Knot-App](https://github.com/pilgrimagesoftware/Knot-App) | The macOS app: a team of AI coding agents, each in an embedded terminal, coordinating over MCP | AGPL-3.0 |
| [Knot-MCP](https://github.com/pilgrimagesoftware/Knot-MCP) | An MCP server shared across Knot installations and users (planned) | MIT |
| [Knot-Library](https://github.com/pilgrimagesoftware/Knot-Library) | Agent personas and other shareable data to import into Knot | CC BY 4.0 |

## Clone

```bash
git clone --recurse-submodules https://github.com/pilgrimagesoftware/Knot.git
```

In an existing clone, `git submodule update --init` fetches the submodules,
and `git submodule update --remote` moves each one to the tip of the branch it
tracks: `develop` for Knot-App and Knot-MCP, `master` for Knot-Library.

## Issues and contributions

Each project has its own issues, pull requests and releases. File bugs and
ideas in the repository they belong to - app bugs go to
[Knot-App](https://github.com/pilgrimagesoftware/Knot-App/issues). This
repository only tracks which commit of each project belongs together.

## History

This repository used to hold the app itself. Its code, branches, tags and
issues now live in Knot-App; its past pull requests remain here for
reference.
