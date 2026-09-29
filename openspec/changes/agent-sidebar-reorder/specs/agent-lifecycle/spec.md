# Spec Delta

## MODIFIED Requirements

### Requirement: Ordering and workspace placement

The system SHALL let an agent be reordered within its workspace and moved to
another workspace. A new agent inherits the workspace of its created-by or
insert-after source when one exists; otherwise it joins the current workspace.
If a workspace has no active agent, a newly added agent becomes its active
agent.

Reordering an agent SHALL move its companions with it, keeping them directly
after it in their existing order. A companion SHALL NOT be reordered on its
own, and no agent SHALL be placed between an owner and its companions. A
reorder that would leave the order unchanged SHALL change nothing and SHALL
NOT be saved.

#### Scenario: New agent inherits source workspace

- **WHEN** an agent is created with `insert_after = X` and `X` lives in
  workspace `W`
- **THEN** the new agent is added to `W`, not the current workspace

#### Scenario: An owner moves with its companions

- **WHEN** workspace `W` holds agents `A`, `A`'s companion `a`, `B` and `C`,
  in that order, and `A` is moved below `C`
- **THEN** `W`'s order is `B`, `C`, `A`, `a`

#### Scenario: An agent is not placed inside another's companion group

- **WHEN** workspace `W` holds `A`, `A`'s companion `a` and `B`, in that
  order, and `B` is moved to the place between `A` and `a`
- **THEN** the move is refused and `W`'s order stays `A`, `a`, `B`

#### Scenario: Moving an agent to its own place changes nothing

- **WHEN** an agent is moved to the place directly above or directly below
  itself
- **THEN** its workspace's order is unchanged and nothing is saved
