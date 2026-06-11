# qwert MCP Server — Setup Guide

qwert can run as an **MCP (Model Context Protocol) stdio server**, giving Claude and other
AI agents access to the same deterministic vault operations as the CLI.

## Prerequisites

- qwert binary built and available on `$PATH` (or an absolute path known to you).
- A local vault directory (a folder containing `.md` files).

## Build the binary

```bash
git clone https://github.com/sanmal/qwert.git
cd qwert
pnpm install
cargo tauri build          # release binary
```

The release binary lands at:

```
src-tauri/target/release/qwert
```

Optionally copy it to a stable location:

```bash
cp src-tauri/target/release/qwert ~/.local/bin/qwert
# or
sudo cp src-tauri/target/release/qwert /usr/local/bin/qwert
```

During development you can also use the debug binary:

```bash
cargo build
# → src-tauri/target/debug/qwert
```

## Verify the binary

```bash
qwert --version
qwert mcp --help
```

Test that the MCP server starts and responds to a JSON-RPC `initialize` request:

```bash
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}' \
  | qwert mcp --vault /path/to/your/notes
```

You should receive a JSON response with `"result"` containing `"serverInfo"` and the
list of capabilities. Press `Ctrl-C` to stop.

## Claude Desktop configuration

Open or create `~/.config/claude/claude_desktop_config.json` and add a `qwert` entry
inside `mcpServers`:

```json
{
  "mcpServers": {
    "qwert": {
      "command": "/home/user/.cargo/bin/qwert",
      "args": ["mcp", "--vault", "/home/user/notes"]
    }
  }
}
```

Replace the paths with your actual binary location and vault directory.

**Path checklist:**

| Field | What to put | Example |
|---|---|---|
| `command` | Absolute path to the qwert binary | `/home/alice/.local/bin/qwert` |
| `args[2]` | Absolute path to your vault | `/home/alice/notes` |

Relative paths and `~` are **not** expanded by Claude Desktop — always use absolute paths.

After editing the file, restart Claude Desktop. The `qwert` server will appear in the
MCP tools list.

## Available tools

Once connected, Claude can call the following tools:

| Tool | Description |
|---|---|
| `file_read` | Read a file; returns content, mtime, invisible-char warnings, and `editing` flag |
| `file_write` | Write a file atomically; supports optimistic concurrency via `if_match` |
| `file_list` | List `.md` files; optional directory tree view |
| `note_render` | Render Markdown to HTML |
| `note_backlinks` | List all notes that link to a given note |
| `note_revision` | Rename a note and update every wikilink (dry-run by default) |
| `note_scan` | Detect invisible characters in a note |
| `vault_search` | Full-text or regex search across the vault |
| `vault_status` | Report sync conflicts, pending renames, appearance warnings |
| `vault_scan` | Scan every note in the vault for invisible characters |

All tools return a top-level JSON envelope:

```json
{ "schema_version": "v1", "kind": "<type>", ...fields }
```

No `data` wrapper. Error responses use `"kind": "error"` with `category`, `exit_code`,
`message`, and `next_step` fields.

## Editing-state hint (`editing` field)

`file_read`, `file_write`, and `note_revision` include an `editing` boolean:

- `true` — the GUI currently has this file open with unsaved changes.
- `false` — the file is saved or not open.

When `editing` is `true`, `file_write` and `note_revision` also include an `editing_note`
string explaining the situation. Claude should read the note first and coordinate with the
user before overwriting.

## Troubleshooting

**`error: mcp requires --vault <path>`**

The `--vault` argument is mandatory for `mcp` mode. Verify the `args` array in
`claude_desktop_config.json` includes `"--vault"` followed by an absolute path.

**Server appears in the list but tools call fail immediately**

Check that the vault path exists and is readable:

```bash
ls /path/to/your/notes
```

**`No such file or directory` for the binary**

Claude Desktop uses the PATH of a non-login shell. Using the full absolute path for
`command` avoids any PATH lookup issues.

**JSON-RPC `initialize` returns no response**

Make sure you are running a release or debug build with the `server`, `transport-io`,
and `macros` features compiled in (they are included in the default Cargo setup).

**Verify the server manually**

```bash
qwert mcp --vault /path/to/notes
# Type JSON-RPC messages on stdin; press Ctrl-C to exit.
```

You can also pipe a quick session:

```bash
printf '%s\n%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' \
  | qwert mcp --vault /path/to/notes
```

The second response should contain all 10 tool definitions.
