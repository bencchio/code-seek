# Code Seek MCP Server

## What it is

`code-seek mcp` starts a Model Context Protocol server over stdio. AI tools (Claude Code, OpenCode, etc.) can use it to get a structured entity index of any codebase — without reading raw source files.

**Purpose**: give agents fast, low-token access to code structure for navigation, refactor detection, and code review. Instead of loading entire files into context, ask code-seek for the entity tree of a directory and drill down with filters.

## Setup

### Claude Code

Register the server with the `claude mcp` CLI:

```bash
# Current project only (default, stored in project-local config):
claude mcp add code-seek -- code-seek mcp

# Shared with everyone working on the repo (writes .mcp.json at the repo root):
claude mcp add --scope project code-seek -- code-seek mcp

# Available in all your projects:
claude mcp add --scope user code-seek -- code-seek mcp
```

Or write `.mcp.json` at the project root by hand:

```json
{
  "mcpServers": {
    "code-seek": {
      "command": "code-seek",
      "args": ["mcp"]
    }
  }
}
```

Note: MCP servers do **not** go in `.claude/settings.json` — Claude Code reads them from `.mcp.json` (project scope) or its own user config, both managed by `claude mcp add`.

If `code-seek` is not on `PATH`, use the full path to the binary:

```bash
claude mcp add code-seek -- /home/you/.cargo/bin/code-seek mcp
```

After `cargo install --path .`, the binary is at `~/.cargo/bin/code-seek`. If Claude Code reports that it cannot find the command, that directory is probably not on `PATH`. Verify with:

```bash
which code-seek          # should print /usr/local/bin/code-seek or ~/.cargo/bin/code-seek
echo $PATH | tr : '\n' | grep cargo   # should print ~/.cargo/bin
```

Restart Claude Code and confirm with `/mcp` that `code-seek` is listed as connected.

### OpenCode

Add to `.opencode.json` in the project root:

```json
{
  "mcp": {
    "code-seek": {
      "type": "stdio",
      "command": "code-seek",
      "args": ["mcp"]
    }
  }
}
```

## Tools

### `scan`

Returns the entity structure of a directory or file as JSON. Same schema as `code-seek scan --format json`.

| Argument    | Type    | Required | Description |
|-------------|---------|----------|-------------|
| `path`      | string  | yes      | Directory or file path to scan |
| `lang`      | string  | no       | Comma-separated language filter (any name or alias): `c`, `cpp`, `c++`, `go`, `golang`, `js`, `javascript`, `python`, `py`, `qml`, `rust`, `ts`, `typescript` |
| `match`     | string  | no       | Case-insensitive substring filter on entity name; parents with matching children are preserved |
| `max_depth` | integer | no       | Hard depth ceiling: `0` = file headers only, `1` = root entities, `2` = roots + direct children |
| `ignore`    | string  | no       | Comma-separated directory names to skip (e.g. `"target,node_modules"`) |
| `info`      | string  | no       | Entity info filter: `"all"` (default), `"no-tests"` (exclude test modules), `"tests-only"` (show only test modules) |
| `no_gitignore` | boolean | no    | Include files git is told to ignore. Default `false`: inside a git repository, ignored files and directories are hidden |

See `docs/reference/SPECS.md` for the full JSON schema.

### `entity`

Look up a single entity by its canonical `entity_path`. Scans only the file referenced in the path — efficient for drill-down.

| Argument      | Type   | Required | Description |
|---------------|--------|----------|-------------|
| `entity_path` | string | yes      | Full entity path as produced by `scan`, e.g. `"src/main.rs > impl Foo > bar"` |

Returns a JSON object with `name`, `entity_type`, `loc`, `start_line`, `end_line`, `children`. Returns JSON-RPC error `-32000` when the entity is not found.

### `summary`

Returns aggregate counts for a path — total files, LOC, and entities, grouped by language and entity type.

| Argument | Type   | Required | Description |
|----------|--------|----------|-------------|
| `path`   | string | yes      | Directory or file path to scan |
| `lang`   | string | no       | Comma-separated language filter (any name or alias): `c`, `cpp`, `c++`, `go`, `golang`, `js`, `javascript`, `python`, `py`, `qml`, `rust`, `ts`, `typescript` |

Response fields: `scan_path`, `total_files`, `total_loc`, `total_entities`, `by_language` (alphabetical), `by_entity_type` (alphabetical).

### `locate`

Search for an entity by name across a directory. Returns all matches with their location — use when you know the name but not the file.

| Argument | Type   | Required | Description |
|----------|--------|----------|-------------|
| `name`   | string | yes      | Entity name to search for (case-insensitive) |
| `path`   | string | yes      | Directory or file path to search in |
| `lang`   | string | no       | Comma-separated language filter (any name or alias): `c`, `cpp`, `c++`, `go`, `golang`, `js`, `javascript`, `python`, `py`, `qml`, `rust`, `ts`, `typescript` |

Response fields: `name`, `matches[]` with `entity_path`, `entity_type`, `loc`, `start_line`, `end_line` per match.

## Raw session example

Each line is a complete JSON-RPC 2.0 message. `→` = client sends, `←` = server responds.
Notifications have no `id` and receive no response.

```
→ {"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"claude-code","version":"1.0"}}}
← {"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"code-seek","version":"<code-seek-version>"}}}

→ {"jsonrpc":"2.0","method":"notifications/initialized"}
  (no response)

→ {"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}
← {"jsonrpc":"2.0","id":2,"result":{"tools":[
  {"name":"scan","description":"Scan a directory or file...","inputSchema":{...}},
  {"name":"entity","description":"Look up a single entity...","inputSchema":{...}},
  {"name":"summary","description":"Return aggregate counts...","inputSchema":{...}},
  {"name":"locate","description":"Search for an entity by name...","inputSchema":{...}}
]}}

→ {"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"scan","arguments":{"path":"src/","lang":"rust","match":"parse"}}}
← {"jsonrpc":"2.0","id":3,"result":{"content":[{"type":"text","text":"{\"version\":\"<code-seek-version>\",\"scan_path\":\"src/\",\"total_files\":6,\"total_entities\":12,...}"}]}}

→ {"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"entity","arguments":{"entity_path":"src/scan.rs > filter_entities"}}}
← {"jsonrpc":"2.0","id":4,"result":{"content":[{"type":"text","text":"{\"name\":\"filter_entities\",\"entity_type\":\"Function\",\"loc\":20,\"start_line\":285,\"end_line\":304,\"children\":[]}"}]}}

→ {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"summary","arguments":{"path":"src/","lang":"rust"}}}
← {"jsonrpc":"2.0","id":5,"result":{"content":[{"type":"text","text":"{\"scan_path\":\"src/\",\"total_files\":8,\"total_loc\":450,\"total_entities\":95,...}"}]}}

→ {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"locate","arguments":{"name":"parse","path":"src/"}}}
← {"jsonrpc":"2.0","id":6,"result":{"content":[{"type":"text","text":"{\"name\":\"parse\",\"matches\":[{\"entity_path\":\"src/lang/mod.rs > parse_source\",...}]}"}]}}
```

### Filtering example

To get a fast overview of a large codebase before drilling down:

```
→ {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"scan","arguments":{"path":"src/","max_depth":1}}}
```

Returns only root-level entities (no nested methods/children), one line per function/struct/class/impl. Ideal for spotting where refactors are needed without loading full trees.

# MCP Rules

## Security

- Reject path traversal (`..` components) in all tool arguments
- Validate all inputs before processing — return `-32602` on invalid input
- Avoid command injection — never pass user input to shell commands

## Output

- Responses must be valid JSON-RPC 2.0
- Use compact JSON (not pretty-printed) for tool responses
- Deterministic ordering in all list fields (alphabetical by default)
- Error responses use `error.code` and `error.message`, not `isError`

## Error Codes

| Code   | Meaning                        | When                          |
|--------|--------------------------------|-------------------------------|
| -32601 | Method not found               | Unknown method in dispatch    |
| -32602 | Invalid params                 | Missing args, path traversal  |
| -32000 | Server error                   | Scan failure, entity not found |

## Tool Design

- Each tool has a dedicated handler function (`handle_scan`, `handle_entity`, etc.)
- Tool definitions are registered in `tools/list` via dedicated functions (`scan_tool_def()`, etc.)
- All tools log the incoming method name via `crate::log::info` on every request
- Notifications (no `id` field) are logged and silently skipped — no response sent
- Tool argument extraction uses `serde_json::Value::get()` with safe unwrap fallbacks

## Protocol

- Protocol version is negotiated with the client on `initialize` — echoes the client's version when supported, falls back to `"2024-11-05"`
- `serverInfo.version` uses `env!("CARGO_PKG_VERSION")` — always reflects the binary version
- Empty or invalid JSON lines are logged as warnings and skipped

## Troubleshooting

### Test the server manually

Pipe a single JSON-RPC `initialize` message to the binary. The server responds on stdout and logs to stderr:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' \
  | code-seek mcp
```

Expected response (one JSON line on stdout):

```json
{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{"tools":{}},"serverInfo":{"name":"code-seek","version":"<code-seek-version>"}}}
```

If you see no output, the binary is not found. If you see an error, check stderr.

### MCP server not recognized by Claude Code

1. Confirm `code-seek` is on `PATH`: `which code-seek`
2. Restart Claude Code after editing `settings.json` — MCP servers are loaded at startup
3. Use the absolute binary path in the config if PATH is not set in Claude Code's environment:
   ```json
   { "command": "/home/you/.cargo/bin/code-seek", "args": ["mcp"] }
   ```
4. All code-seek logs go to **stderr**. Claude Code captures stderr separately — check its MCP log output if available.

### `code-seek init` is not required for MCP

`code-seek mcp` works without a prior `init`. The first scan creates the XDG state slot and cache.
