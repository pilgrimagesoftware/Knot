## Why

The Open In… submenu offers VS Code, Xcode, Finder and Terminal, ported from
the Swift reference's fixed list. Zed is missing, and it is an editor this
project's own author works in - it is running on this machine now.

## What Changes

- Add **Zed** to the Open In… submenu, in the editors group after VS Code.
- This is an addition beyond the Swift reference, not a port gap. The
  reference's `OpenWithProvider` has no Zed entry; nothing here is being
  brought into line with it, and a later parity pass should not remove this.

## Capabilities

### Modified Capabilities
- `agent-list-ui`: the Open In… submenu's contents.

### New Capabilities
(none)

## Impact

- `crates/knot`: one entry in `open_in.rs`'s submenu list and one arm in its
  `open` argument mapping.
- **Sequencing:** this delta modifies a requirement introduced by
  `agent-context-menu-parity`, which is implemented and merged but not
  archived. `openspec validate` already reports that archive would refuse
  this delta until that change is archived first. Nothing else blocks the
  implementation.
