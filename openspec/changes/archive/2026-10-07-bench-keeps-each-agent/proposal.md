# Proposal

## Why

Issue #500: saving an agent to the bench overwrites an agent already benched
from the same folder. The bench's "replace any entry with the same folder"
rule comes from the Swift reference, where an agent was effectively one per
folder. In Knot, several agents commonly work in one repository folder, so
saving a second one silently loses the first template.

## What Changes

- A new bench entry SHALL replace an existing one only when both name and
  folder are the same, meaning the same agent was saved again. A different
  agent from the same folder is added beside it.
- Save to Bench and Bench Agent follow the same rule.

This deliberately departs from the Swift reference.

## Capabilities

### Modified Capabilities

- `settings-persistence`: "Bench templates" keys replacement on name and
  folder.
- `agent-lifecycle`: "Benching an agent" refers to that rule.

## Impact

- `crates/knot-core/src/settings/store/bench.rs` (`add_bench_agent`) and its
  tests.
- No stored data changes. Existing bench entries load as before.
