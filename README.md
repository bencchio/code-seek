# Code Seek

Explore the functional structure of a repository from the CLI.

- **Structural tree**: classes, methods, functions, structs, traits, enums,
  components, with LOC and line numbers.
- **Multi-language**: C, C++, Elixir, Go, JavaScript, Python, Rust, TypeScript, QML.
- **Error-tolerant**: partially parses even with invalid code.
- **Honors `.gitignore`**: inside a git repository, ignored files never reach the output.

## Status

**v0.6.0** — published: releases ship the Arch package, installable from source, cargo, or the AUR recipe.

See [`docs/rules/mcp.md`](docs/rules/mcp.md) for MCP setup and
[`docs/CHANGELOG.md`](docs/CHANGELOG.md) for release history.

## Installation

**Requirements:** Rust toolchain (stable). If you don't have Rust installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# Then restart your shell or run: source ~/.cargo/env
```

**Option A — the install script:**

```bash
git clone https://github.com/bencchio/code-seek
cd code-seek
./install.sh
```

It builds the release binary and installs it to `~/.local/bin`. Pass
`--prefix <dir>` to install somewhere else, for example
`sudo ./install.sh --prefix /usr/local/bin`. The script tells you if the
prefix is not on your `PATH`.

**Option B — cargo:**

```bash
cargo install --path .
```

This installs to `~/.cargo/bin`, which must be on your `PATH`. If
`code-seek --version` fails afterwards, add it:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc  # or ~/.zshrc
source ~/.bashrc
```

**Option C — Arch Linux:**

Every release ships the built package as an asset. Download the
`.pkg.tar.zst` from the [latest release](https://github.com/bencchio/code-seek/releases/latest)
and install it:

```bash
sudo pacman -U code-seek-*-x86_64.pkg.tar.zst
```

To build it yourself instead, the recipe lives in
[`packaging/aur/PKGBUILD`](packaging/aur/PKGBUILD):

```bash
cd packaging/aur && makepkg -si
```

Verify any of them with:

```bash
code-seek --version
```

## Quick start

```bash
# Initialize Code Seek in the current directory
code-seek init

# Analyze the codebase
code-seek scan .
```

## Using with AI agents

There are two ways to give an AI agent access to code-seek: instruction files (teach the agent to run the CLI) and MCP (give the agent a structured tool).

### Instruction files — CLAUDE.md / AGENTS.md

Add this to your project's `CLAUDE.md` (Claude Code) or `AGENTS.md` (Codex / OpenAI):

```markdown
## Code structure

Use `code-seek scan <path>` to explore the structure of any file or directory before reading source files.

- Overview of the whole repo: `code-seek scan .`
- JSON output for structured data: `code-seek scan . --format json`
- Filter by language: `code-seek scan . --lang rust`
- Find an entity by name: `code-seek scan . --match parse`
- Limit tree depth: `code-seek scan . --max-depth 1`
- Exclude test modules: `code-seek scan . --info no-tests`
- Only test modules: `code-seek scan . --info tests-only`
- Include files git ignores: `code-seek scan . --no-gitignore`

Prefer `--max-depth 1` for a fast first pass on large directories.
```

This works with any agent that can run shell commands. No MCP setup required.

### MCP — programmatic tool access

`code-seek mcp` starts a JSON-RPC 2.0 server over stdio with 4 tools:
`scan`, `entity`, `summary`, `locate`.

See [`docs/rules/mcp.md`](docs/rules/mcp.md) for setup instructions (Claude Code, OpenCode) and tool reference.

## Sample output

```
󱘗 src/main.rs  [Rust]          38 LOC  3 entities
├─ 󰠱 Cli                        4 LOC  [ 14- 17]
├─ 󰒻 Command                   11 LOC  [ 20- 30]
└─ 󰊕 main                      11 LOC  [ 32- 42]

󱘗 src/model.rs  [Rust]         28 LOC  3 entities
├─ 󰠱 Entity                     8 LOC  [ 16- 23]
├─ 󰒻 EntityType                10 LOC  [  4- 13]
└─ 󰠱 FileResult                 6 LOC  [ 26- 31]

2 files  6 entities
```

## Documentation

- [`docs/reference/SPECS.md`](docs/reference/SPECS.md) — Full specification

## Supported languages

C · C++ · Elixir · Go · JavaScript · Python · Rust · TypeScript · QML

## License

MIT — see [`LICENSE`](LICENSE).
