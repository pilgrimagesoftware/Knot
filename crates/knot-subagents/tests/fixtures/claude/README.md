# Claude Code ACP fixtures

What `@agentclientprotocol/claude-agent-acp` **v0.81.1** puts on the wire for a
delegation, and for a tool call that is not one.

These are built from the adapter's own emit code rather than captured from one
session: the generator, not a sample of its output. Each file names the
function it mirrors, so a version bump is checked by re-reading that function
rather than by hoping a recorded session covered the case.

| File | Mirrors |
|---|---|
| `dispatch.json` | `toolCallNotification()` non-refine path + `claudeCodeMetaFromToolUse()`, for `toolUse.name === "Task"` |
| `not_a_delegation.json` | the same two, for `toolUse.name === "Bash"` |
| `completion.json` | the `tool_call_update` built at `acp-agent.js:7912-7928` |
| `completion_failed.json` | the same, with `chunk.is_error` true |

Each file is the whole `session/update` params object, envelope included, so a
test exercises the same decode path a live notification takes.

## Three details worth keeping in view

**The completion does not carry `subagent`.** `claudeCodeMetaFromToolUse` is
what stamps `subagent: true`, and the completion path does not call it — it
builds its meta inline as `{ toolName, ...nonExecution }`. So a recognizer that
keyed on `subagent` alone would see every dispatch and no completion. Keying on
`toolName` being `Task` or `Agent` is what covers both.

**`_meta.claudeCode.title` is not set for a delegation.** It is populated only
when `toolUse.name` is `Bash` or `PowerShell`, from that tool's own
`input.description`. A delegation's description reaches the wire as the tool
call's top-level `title` and inside `rawInput`, not there.

**`kind` is `"think"`, not a delegation-specific value.** It is shared with
ordinary reasoning calls, which is why nothing here keys on it.

## Regenerating

```sh
npm pack @agentclientprotocol/claude-agent-acp
tar xzf agentclientprotocol-claude-agent-acp-*.tgz
# then read, in package/dist/:
#   acp-agent.js  -> toolCallNotification, claudeCodeMetaFromToolUse
#   tools.js      -> toolInfoFromToolUse, the "Agent"/"Task" case
```
