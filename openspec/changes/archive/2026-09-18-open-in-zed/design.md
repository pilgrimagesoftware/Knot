## Context

See proposal.md — Why. `open_in.rs` holds a fixed list of applications and a
mapping from each to the arguments `/usr/bin/open` needs. Both are small and
already tested; this change adds one entry to each.

## Goals / Non-Goals

**Goals:**

- One more editor in the list, behaving exactly like the ones already there.

**Non-Goals:**

- Discovering installed editors at runtime. The list is deliberately fixed,
  as it is in the Swift reference: a menu whose contents depend on what is
  installed is a menu that changes shape between machines, and the existing
  "fail quietly if it is not installed" rule already covers the absent case.
- Making the list user-configurable. Worth considering once there are enough
  entries to argue about; four was not enough and five is not either.

## Decisions

### `-b dev.zed.Zed`, read from the installed application

Zed is launched by bundle identifier, like VS Code, rather than by name like
Xcode. The identifier was read from `/Applications/Zed.app/Contents/Info.plist`
rather than recalled or inferred.

That distinction matters more than it looks: `open -b` takes a bundle id and
`open -a` takes an application name, they are silently interchangeable at the
call site, and a wrong one fails exactly like an application that is not
installed - which this menu is specified to swallow. `open_in.rs` already has
a test pinning each application's arguments for that reason, and Zed joins it.

### After VS Code, before Xcode

The submenu's first group is editors and its second is the system pair
(Finder, Terminal), separated by a divider. Zed belongs in the first, next to
the other cross-platform editor rather than after the Apple one.

## Risks / Trade-offs

- **The delta cannot archive yet** - it modifies a requirement that
  `agent-context-menu-parity` introduces, and that change is merged but
  unarchived → archive that one first; `openspec validate` reports this
  plainly, so it cannot be missed at archive time.

## Open Questions

None.
