# Spec Delta

## Purpose

Defines the catalogue the system keeps of every agent it could put to work —
each one's description, capability tags, reachable tools, declared cost tier
and live status — how a caller finds candidates by capability, and how an
orchestrating agent's roster is derived from that catalogue instead of being
written into its prompt.

## ADDED Requirements

### Requirement: Registry entry fields

Every agent and every bench template SHALL carry three durable descriptive
fields: a one-line `description` (possibly empty), a set of `capabilities`
(tags), and a `cost_tier` of `low`, `medium` or `high`, defaulting to
`medium`. A record written before these fields existed SHALL load with an
empty description, no capability tags, and cost tier `medium`.

Each registry entry SHALL additionally report two derived fields the caller
does not set: `tools`, being the entry's agent type together with the tool
surface its ACP adapter declares, and `status`. For a live agent `status`
SHALL be its automatic state, its agent-set status text, and its registered
flag; for a bench template `status` SHALL be reported as `template`, since a
template has no session.

Capability tags SHALL be opaque to the system: matched by exact
case-insensitive equality, never parsed or interpreted, and never restricted
to a fixed vocabulary. A new kind of agent — research, infrastructure,
review, testing, or one not yet imagined — is added by tagging it, not by
changing the system.

Divergence from the Swift reference: the Swift app has no registry. Its
agents carry a name, folder and type only, and any notion of what an agent is
for lives in the persona text a human wrote.

#### Scenario: Defaults for a record that predates the fields

- **WHEN** an agent record saved before registry fields existed is loaded
- **THEN** it loads with an empty description, no capability tags, and cost
  tier `medium`, and is otherwise unchanged

#### Scenario: An unrecognised tag is still a valid tag

- **WHEN** an agent is given the capability tag `terraform-review`, which the
  system has never seen
- **THEN** the tag is stored and matched as written, and no error is raised

#### Scenario: A template reports no live status

- **WHEN** the registry is read and it contains a bench template
- **THEN** that entry's status is `template`, with no automatic state and no
  registered flag

### Requirement: Capability query returns ranked candidates

The registry SHALL answer a query for a set of capability tags with the
entries that carry **every** requested tag and that the caller is permitted
to see. Visibility SHALL follow the rules already governing agent listing:
the caller's own workspace, and companion agents only where the caller owns
them. A query with no tags SHALL return every visible entry.

Results SHALL be ordered so the cheapest agent able to start soonest comes
first: live agents that are Idle, then live agents in any other state, then
deployable bench templates; within each group by ascending cost tier (`low`,
`medium`, `high`), and then by name. Ordering is a recommendation, not an
assignment — the caller still chooses.

A query naming a tag no entry carries SHALL return an empty list and SHALL
NOT be an error, so an orchestrator discovering it has no reviewer gets an
answer it can act on rather than a failure.

#### Scenario: Cheapest idle candidate ranks first

- **WHEN** an agent queries for the tag `code-review` and two idle agents
  carry it, one at cost tier `low` and one at `high`
- **THEN** the `low` agent is listed before the `high` one

#### Scenario: A busy agent ranks below an idle one regardless of cost

- **WHEN** a `high` cost tier agent carrying the tag is Idle and a `low` one
  carrying the same tag is Working
- **THEN** the idle `high` agent is listed first

#### Scenario: All requested tags must match

- **WHEN** the query asks for both `rust` and `testing`, and an agent carries
  only `rust`
- **THEN** that agent is not among the candidates

#### Scenario: No candidate is an empty answer, not a failure

- **WHEN** the query asks for a tag no visible entry carries
- **THEN** the result is an empty candidate list and the call succeeds

#### Scenario: Companion visibility is unchanged

- **WHEN** an agent queries for a tag carried by a companion it does not own
- **THEN** that companion is not among the candidates

### Requirement: A roster is obtained by query, never stored as text

An agent that coordinates others SHALL learn its team by querying the
registry, and the system SHALL NOT require any stored instruction text to
enumerate teammates. A query answer SHALL carry, for each visible entry, its
identifier, description, capability tags, cost tier and status — everything a
caller needs to choose, so nothing about the team has to be written down
somewhere else to be usable.

Adding, removing, retagging or re-pricing an agent SHALL change what the next
query returns, with no edit to any stored text. Shipped instruction text
SHALL describe the method — query, plan, then dispatch — and SHALL NOT name
individual teammates, because a named roster in stored text is a copy of
state that goes stale the first time the team changes.

#### Scenario: A new agent is visible to the next query

- **WHEN** an agent tagged `infrastructure` is created and an existing agent
  then queries the registry for that tag
- **THEN** the new agent is among the candidates, and no stored instruction
  text was changed

#### Scenario: A removed agent leaves the answer

- **WHEN** a tagged agent is removed and the registry is queried again for
  that tag
- **THEN** the removed agent is absent from the candidates

#### Scenario: A query answer is self-sufficient

- **WHEN** a caller receives registry candidates
- **THEN** each candidate carries its identifier, description, capability
  tags, cost tier and status, without a second call to interpret it

### Requirement: Registry metadata survives the bench round trip

Saving an agent to the bench SHALL carry its description, capability tags and
cost tier onto the bench entry, and deploying that bench entry SHALL restore
all three onto the created agent. A template is otherwise a record of a
folder, which is not enough to choose it from a registry.

#### Scenario: Save and redeploy preserves the role

- **WHEN** an agent described as "Reviews Rust diffs" with tags
  `rust`, `code-review` and cost tier `high` is saved to the bench and later
  deployed
- **THEN** the deployed agent carries the same description, the same two
  tags, and cost tier `high`
