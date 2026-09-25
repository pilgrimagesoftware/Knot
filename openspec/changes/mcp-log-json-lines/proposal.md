# Proposal

Issue: #484

## Why

Every Knot instance that runs with the same `HOME` appends to the same MCP
log file: the installed app, a dev build launched from a worktree, a
verification build. Nothing on a line says which process wrote it.

On 2026-09-25 that made an MCP outage undiagnosable. The file held a
`lifecycle stopped`, fifteen minutes of silence and a run of
`Address already in use` retries, and none of it could be attributed to the
installed app rather than to one of the test instances writing beside it.
Heartbeats from several processes landed with the same timestamp and read as
one heartbeat task duplicated.

The line format also has to be split by hand to be read by a tool, and its
escaping is a local invention that has to be proven reversible. JSON Lines is
a standard, structured, and parseable by anything.

## What Changes

- **Each log-file entry is one JSON object on one line**, with the fields
  `time`, `pid`, `level`, `subject`, `message`, in that order.
- **`pid` is new**: the ID of the process that wrote the entry.
- **The file is renamed** from `knot-mcp.log` to `knot-mcp.jsonl`, rolled
  files becoming `knot-mcp.jsonl.1` and so on. Appending JSON to the old file
  would leave it half text and half JSON, parseable by neither reader. The
  old files are left where they are and are no longer written.
- **The hand-rolled escaping goes.** JSON escapes every control character in
  a string, which is what kept one entry on one line.
- Standard error is unchanged. It is written separately, already belongs to
  one process, and the `mcp-server` spec requires it to stay as it was.

The messages themselves are unchanged strings (`bound 127.0.0.1:8767`,
`tools/call send-message keys=[...] bytes=134`). Splitting them into typed
fields per event is a larger change and not part of this one.

## Capabilities

### Modified Capabilities

- `mcp-server`: "Each log entry is one self-describing line" becomes a JSON
  object with a fixed field set, including the process ID.

## Impact

- `crates/knot-mcp/src/log/entry.rs`: `Entry` records the process ID;
  `render` serializes the entry; `escape` is removed.
- `crates/knot-mcp/src/consts.rs`: `LOG_FILE_NAME` becomes `knot-mcp.jsonl`.
- Tests that matched the text format parse the lines instead:
  `log/entry/tests.rs`, `log/logger/tests.rs`, `log/writer/tests.rs`,
  `tests/logging.rs`.
- Anyone reading the old file by hand finds new entries in the new one.
