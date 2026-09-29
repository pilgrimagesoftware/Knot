# Design

## Decisions

### The URL is the registration

Knot hands each agent it launches `…/mcp?agent=<id>` (#544), so a connection on
that URL is Knot's own statement of who the agent is. Registering from it needs
no cooperation from the model, adds no turn to a resumed conversation, and covers
an agent whose earlier turn failed. The server calls `ToolCatalog::connected` on
every request that names an agent, not just `initialize`, because the registry
can lag the connection (#548). The call is idempotent and only takes the store
lock. `is_registered` isn't in the saved roster, so there's nothing to persist.

`connected` defaults to a no-op, so catalogs without a registry are unaffected.

### The fallback is per adapter, and says so

Only the adapters checked live to keep the query (`claude-agent-acp` 0.82.0,
`codex-acp`) are trusted with it. Every other type gets a one-line request after a
real resume (`connect_into` already tells a real resume from a fallback), and
only while MCP is on. The rule lives beside `supports_inline_registration`
rather than as an `AdapterConfig` field, which would have touched 29 struct
literals for one boolean. A type moves to the trusted list once it has been
checked.
