# Spec Delta

## MODIFIED Requirements

### Requirement: Benching an agent

Benching an agent SHALL save it to the bench exactly as Save to Bench does -
the same fields, replacing only its own earlier entry (the same name and
folder) - and then remove it per "Agent removal". The bench entry SHALL be written before the agent is
removed, and if writing it fails the agent SHALL NOT be removed.

Only an agent that is not a companion SHALL be benchable. Benching an owner
SHALL remove its companions with it, as removal does; the companions are not
put on the bench.

The benched agent's conversation SHALL NOT be kept. A bench entry is a
template, and deploying it starts a fresh session.

The Swift app has Save to Bench only; benching is new to the Rust port.

#### Scenario: Benching saves then removes

- **WHEN** an agent is benched
- **THEN** a bench entry with its name and folder holds its fields, and the agent is
  gone from its workspace and the master agent list

#### Scenario: A failed bench write keeps the agent

- **WHEN** an agent is benched and the bench document cannot be written
- **THEN** the agent is still in its workspace, running

#### Scenario: Benching an owner removes its companions

- **WHEN** an owner with one companion is benched
- **THEN** the bench holds one entry, for the owner, and both agents are
  removed
