<!-- qwert README for public repository -->

<div align="center">

# qwert

**A small, fast, AI-agent-friendly Markdown notes app.**

*"Just a small tool in the garage — hook and MCP as spare keys."*

![status](https://img.shields.io/badge/status-early%20development-orange)
![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)
![built with](https://img.shields.io/badge/built%20with-Rust%20%2B%20Tauri%202.0-000000)

</div>

---

qwert is a lightweight local Markdown notes application built with **Tauri 2.0 + SolidJS**.
It targets a sub-1-second cold start and under 100 MB of memory by avoiding Electron entirely
and keeping the frontend as thin as possible.

It is **not** trying to be "everything in one app." If you want the full plugin ecosystem,
graph views, and limitless customization, Obsidian or VS Code are the right tools. qwert is
deliberately the opposite: a small, sharply-scoped tool for people (and AI agents) who write
and operate **spec-driven-development documents** in plain local Markdown files.

> ⚠️ **Status:** qwert is in active early development. Many features below are planned or
> partially implemented. See [Roadmap](#roadmap) for what is real today vs. what is coming.

## Why qwert?

Most note apps are designed for humans first and treat automation as an afterthought.
qwert flips that around: it is built for **AI agents and the humans who drive them**, while
still letting non-AI users reach every feature through a plain CLI.

Three things set it apart from general-purpose PKM tools:

1. **Deterministic vault operations for AI agents** — a Noun-Verb CLI with semantic exit
   codes and structured (JSON) output, plus a wikilink-aware Revision system driven by real
   AST parsing rather than fuzzy text replacement.
2. **Invisible-character detection** — structural detection of characters that have no business
   being in a Markdown file (Unicode Tag chars, null bytes, C0/C1 control chars), useful as
   raw material for spotting indirect prompt-injection payloads.
3. **A WCAG-compliant Markdown reader** — readability treated as a tool-quality property
   (WCAG 2.2 Level AA required, AAA partial), not as decoration.

## Highlights

- **Fast & light** — Tauri 2.0 (no Electron), Rust core, no virtual DOM. Goal: <1 s start, <100 MB RAM.
- **Local-first** — only touches local `.md` files. No accounts, no cloud, no telemetry.
- **CodeMirror 6 editor** — Markdown syntax highlighting, optional Vim bindings, auto-save,
  external-change detection.
- **Split preview** — CommonMark + GFM, three view modes (Editor / Split / Preview), with
  scroll sync.
- **Wikilinks** — `[[note]]`, `[[note|alias]]`, `[[note#heading]]`, backlinks, autocomplete.
- **Mermaid & KaTeX** — diagrams and math rendered inside fenced code blocks only, lazy-loaded
  so notes without them pay zero cost.
- **Revision system** — rename a note and have every `[[wikilink]]` across the vault updated
  atomically, with `--dry-run` and unified-diff preview.
- **Agent-friendly CLI + MCP server** — every GUI action is also a scriptable command and an
  MCP tool, all backed by the same Rust core.
- **Sync-agnostic** — no built-in sync. Pair it with [Syncthing](https://syncthing.net/)
  (or anything else) for PC ↔ phone file sync.

## Installation

> qwert currently targets **Linux desktop** (developed on Pop!_OS 24 LTS / Ubuntu 24 LTS,
> X11 and Wayland). Android is planned. There are no prebuilt binaries yet — build from source.

**Prerequisites**

```bash
# Rust + Node toolchains (mise recommended)
mise use rust@latest
mise use node@lts
corepack enable && corepack prepare pnpm@latest --activate
cargo install tauri-cli --version "^2"

# Linux system dependencies (Pop!_OS / Ubuntu)
sudo apt install -y \
  libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**Build & run**

```bash
git clone https://github.com/sanmal/qwert.git
cd qwert
pnpm install

cargo tauri dev      # run the desktop app (hot reload)
cargo tauri build    # produce a release build
```

## Quick start

1. Launch qwert and open any folder as your **vault**.
2. Browse `.md` files in the sidebar, edit in the CodeMirror pane, preview on the right.
3. Toggle Vim mode and syntax highlighting in settings.
4. Everything you do is also available from the command line (below).

## CLI usage

qwert ships a single binary that runs as a GUI, a CLI, or an MCP server. The CLI uses a
predictable **Noun-Verb** grammar so an agent can guess the next command from the pattern.

```bash
# file — raw file operations
qwert file read  <path>            # print a file to stdout
qwert file write <path>            # write stdin to a file (atomic)
qwert file list  [--tree]          # list .md files in the vault

# note — Markdown-aware operations
qwert note render    <path>        # Markdown -> HTML
qwert note backlinks <path>        # list backlinks
qwert note revision  <path>        # rename + update every wikilink
qwert note scan      <path>        # detect invisible characters

# vault — whole-vault operations
qwert vault search <query>         # full-text search
qwert vault status                 # report sync conflicts, pending state, etc.
qwert vault scan                   # scan the whole vault for invisible chars

# appearance — visual settings
qwert appearance contrast --fg "#1a1a1a" --bg "#ffffff"   # check WCAG contrast
qwert appearance set --fg "#3a2418" --bg "#fdf6e3"        # write a validated theme
qwert appearance status                                    # show the effective config
```

Short aliases exist for humans (`qwert read`, `qwert write`, `qwert search`, …); the
canonical Noun-Verb forms are the source of truth for agents and documentation.

**Composable by design**

```bash
# grep every note for TODO
qwert file list --format path | xargs grep -l "TODO"

# preview a revision as a diff and dry-apply it
qwert note revision old.md --dry-run --format diff | patch -p1 --dry-run

# extract fields with jq
qwert vault search "TODO" --format json | jq -r '.hits[].path'
```

Output format is selectable per command via `--format` (`json` / `path` / `text` / `raw` /
`diff`), and exit codes are **semantic** (`0` success, `2` usage, `3` not found, `4` conflict,
`5` validation) so scripts and agents can branch on `$?`.

## MCP integration

The MCP server is a thin wrapper over the same core, so an AI agent gets exactly the same
deterministic operations as the CLI.

```bash
qwert mcp --vault /home/user/notes
```

Example Claude Desktop config:

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

## Configuration

Settings are split into two files (XDG paths):

- `~/.config/qwert/config.toml` — behaviour (editor, preview, search, revision, sanitize…).
- `~/.config/qwert/appearance.toml` — visuals (theme, text spacing, colors). A per-vault
  override at `vault/.qwert/appearance.toml` is reflected immediately (hot reload).

There is **no color-picker UI**. Instead you get two paths to customize appearance:

- **CLI path** — `qwert appearance contrast` / `qwert appearance set` validate and write a
  WCAG-checked `fg`/`bg` pair.
- **AI path** — a machine-readable instruction template embedded as comments in
  `appearance.toml`, so an agent can turn "warmer tones" or "easier on the eyes at night"
  into a WCAG-compliant color pair.

All appearance values pass through a Rust-side gatekeeper that sanitizes them before they
ever reach the frontend (one-way data flow), so the worst case of a malicious write is an
ugly-but-valid theme — never code execution or external resource loading.

## Accessibility

"Show well" means **"readable in every environment," not "beautiful in the best one."**

- WCAG 2.2 **Level AA required**, **AAA partial**.
- Four preset themes: `default`, `high-contrast`, `dark`, `dark-high-contrast`.
- Follows OS `prefers-color-scheme` and `prefers-contrast`.
- Default syntax-highlight palette is Protanopia/Deuteranopia friendly (distinguished by
  lightness, not just hue). Highlighting is an on/off toggle — no per-token color UI.
- KaTeX is chosen over MathML specifically because it renders identically across WebView
  engines, with hidden MathML retained for screen readers.

## Security model

qwert reduces its attack surface by **not having** the features that usually need defending:

- No external-URL image fetching, no `<iframe>`, no external scripts/styles/fonts.
- No `javascript:` URIs, no arbitrary code execution, no `<script>` tags.
- No plugin system (so no plugin can load arbitrary code).
- No custom CSS injection beyond the sanitized appearance values.
- `shell` capability fully denied; `fs` scoped to the vault; `http` disabled.

Because these are absent by construction, indirect prompt-injection has no functional path
to act on — an agent reading a malicious note still cannot exfiltrate data or run commands.
Invisible-character detection sits in front of this boundary to *tell* you (and the agent)
when a note contains characters that shouldn't be there.

## Obsidian compatibility

| Compatible | Not supported |
|---|---|
| `.md` files | `.obsidian/` plugins & themes |
| `[[wikilink]]` syntax | Obsidian-specific metadata (frontmatter is display-only) |
| Folder structure | Plugins (Dataview, Templater, …) |
| CommonMark / GFM | Canvas |
| Mermaid code blocks | Obsidian Sync / Publish (use Syncthing instead) |

## Philosophy & non-goals

qwert intentionally stays small and pushes extension out to external tools (Syncthing, jj,
neovim, Figma, …) and to separate "spare-key" binaries (e.g. `qwert-figma-preview`,
`qwert-canvas-bridge`, `qwert-pdf-renderer`) rather than growing the core.

**Out of scope (on purpose):** built-in sync, plugin marketplace, graph view, pixel-perfect
UI design, freehand drawing, and native rendering of PDF/Canvas/video. The visual ceiling is
Mermaid and KaTeX inside code blocks; anything richer is delegated to external services and,
when a snapshot is needed, kept as an exported image inside the vault.

## Tech stack

Tauri 2.0 · Rust (`qwert-core`) · SolidJS + TypeScript · CodeMirror 6 · pulldown-cmark ·
Mermaid · KaTeX · Vite · pnpm.

## Roadmap

- **Phase 1 — Desktop MVP:** file tree, editor (Vim toggle), split preview, auto-save,
  external-change detection, visual-settings foundation.
- **Phase 2 — Core + CLI:** wikilinks & backlinks, full-text search, Revision system,
  Mermaid/KaTeX, the CLI (Noun-Verb, JSON, exit codes), invisible-char detection (layer 1),
  security boundary.
- **Phase 3 — Quality + integrations:** MCP server, per-vault appearance hot reload,
  drag-and-drop, performance work, `qwert describe`.
- **Phase 4 — Android:** Tauri Android build, responsive/touch UI, Syncthing testing.
- **Phase 5 — Optional:** tags, templates, export, advanced Japanese search, and more.

## Contributing

Issues and PRs are welcome. Because the project's value is in staying small, feature requests
are evaluated against the non-goals above — many are better served as a `hook`/MCP integration
or a separate companion binary than as core additions.

## License

Dual-licensed under either **MIT** or **Apache-2.0**, at your option.
