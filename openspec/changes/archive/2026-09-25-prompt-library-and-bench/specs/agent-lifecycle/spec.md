# Spec Delta

## MODIFIED Requirements

### Requirement: Bench deployment

Deploying a bench agent SHALL verify the target folder exists and is a
directory. If the check fails, the bench entry SHALL be removed and no agent
created. If it succeeds, an agent is created from the bench entry's folder,
name, avatar, agent type, shell command, persona, and startup prompt, in the
form the entry holds it.

Deploying an entry SHALL NOT remove it from the bench; one entry can be
deployed any number of times.

#### Scenario: Stale bench entry is pruned

- **WHEN** a bench agent whose folder no longer exists is deployed
- **THEN** no agent is created and the bench entry is removed

#### Scenario: The startup prompt is deployed with the entry

- **WHEN** a bench entry whose startup prompt references library prompt P is
  deployed
- **THEN** the created agent's startup prompt references P, and its first
  session sends P's text after the initialization prompt

#### Scenario: An entry stays on the bench after deployment

- **WHEN** a bench entry is deployed twice
- **THEN** two agents are created and the entry is still on the bench

### Requirement: Edit triggers restart only for launch-affecting changes

Editing an agent's name, avatar, description, capabilities, cost tier, or
startup prompt SHALL NOT restart it: none of them changes how the session
runs, and re-tagging an agent mid-job would otherwise throw away the work it
is doing. A changed startup prompt takes effect on the agent's next fresh
session. Changing its folder, agent type, or persona SHALL restart it. When
the folder changes and companion relocation is requested, each companion that
shared the old folder SHALL be moved to the new folder and restarted.

#### Scenario: Rename does not restart

- **WHEN** only the agent's name changes
- **THEN** the terminal session is not recreated

#### Scenario: Folder change restarts and can relocate companions

- **WHEN** the agent's folder changes with relocate-companions requested
- **THEN** the agent restarts, and each companion at the old folder moves to the
  new folder and restarts

#### Scenario: Re-tagging a working agent does not interrupt it

- **WHEN** a capability tag is added to an agent that is Working
- **THEN** its session is not recreated and it keeps working

#### Scenario: A new startup prompt waits for the next fresh session

- **WHEN** a running agent's startup prompt is changed
- **THEN** its session is not recreated, nothing is sent to it, and the new
  prompt is sent the next time it starts a fresh session

## ADDED Requirements

### Requirement: Benching an agent

Benching an agent SHALL save it to the bench exactly as Save to Bench does -
the same fields, replacing any entry for the same folder - and then remove it
per "Agent removal". The bench entry SHALL be written before the agent is
removed, and if writing it fails the agent SHALL NOT be removed.

Only an agent that is not a companion SHALL be benchable. Benching an owner
SHALL remove its companions with it, as removal does; the companions are not
put on the bench.

The benched agent's conversation SHALL NOT be kept. A bench entry is a
template, and deploying it starts a fresh session.

The Swift app has Save to Bench only; benching is new to the Rust port.

#### Scenario: Benching saves then removes

- **WHEN** an agent is benched
- **THEN** a bench entry for its folder holds its fields, and the agent is
  gone from its workspace and the master agent list

#### Scenario: A failed bench write keeps the agent

- **WHEN** an agent is benched and the bench document cannot be written
- **THEN** the agent is still in its workspace, running

#### Scenario: Benching an owner removes its companions

- **WHEN** an owner with one companion is benched
- **THEN** the bench holds one entry, for the owner, and both agents are
  removed
