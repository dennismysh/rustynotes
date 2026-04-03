# RustyNotes

A local-first markdown editor built with **Tauri 2** (Rust backend) and **Leptos** (Rust/WASM frontend). Rich rendering via KaTeX, Mermaid, and Syntect. Editing via CodeMirror 6 and TipTap 3.

No cloud. No accounts. Your files, your machine.

## Features

- **Four editor modes** — Source (CodeMirror), WYSIWYG (TipTap), Split, and Preview
- **Rich markdown rendering** — GFM tables, task lists, footnotes, alerts, math ($LaTeX$), Mermaid diagrams, syntax-highlighted code blocks
- **Three navigation modes** — Sidebar tree, Miller columns, breadcrumb
- **Theming** — Dark/light themes via CSS custom properties with full color override support
- **Configurable saving** — Manual (Cmd+S), auto-save after delay, or save on focus loss
- **Local-first** — Files stay on disk. Config at `~/.config/rustynotes/config.json`
- **Keyboard-driven** — Cmd+S save, Cmd+N new file, Cmd+1/2/3/4 editor modes, Cmd+K search
- **Custom title bar** — Integrated traffic lights, draggable, theme-matched
- **Window & folder persistence** — Remembers size, position, and last opened folder

## Architecture

```
Tauri 2 (Rust)              Leptos (Rust/WASM)           JS Bridge
+-----------------+         +------------------+         +-------------+
| File I/O        |  IPC    | App state        |  FFI    | CodeMirror 6|
| Markdown parser |<------->| Reactive signals |<------->| TipTap 3    |
| Syntax highlight|         | Components       |         | KaTeX       |
| Config system   |         | Theme engine     |         | Mermaid     |
| File watcher    |         | Save logic       |         +-------------+
+-----------------+         +------------------+
```

The backend handles markdown parsing (Comrak + Syntect) and file operations. The frontend is pure Rust compiled to WASM. A thin JS bridge provides CodeMirror and TipTap editors since the rich text editing ecosystem only exists in JavaScript.

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`
- [Trunk](https://trunkrs.dev/): `cargo install trunk`
- [Node.js](https://nodejs.org/) 18+ with [pnpm](https://pnpm.io/)
- [Tauri CLI](https://v2.tauri.app/): `cargo install tauri-cli`

## Getting Started

```bash
# Install dependencies
pnpm install

# Run in development mode
pnpm tauri dev

# Build for production
pnpm tauri build
```

The production build outputs to `target/release/bundle/`.

## Project Structure

```
src-tauri/              Tauri backend (Rust)
  src/
    lib.rs              App entry, plugin registration
    markdown_parser.rs  Comrak + Syntect rendering
    fs_ops.rs           File I/O, directory listing, search
    config.rs           Config load/save (~/.config/rustynotes/)
    watcher.rs          File system watcher
    commands/           Tauri IPC command handlers
    export/             HTML export

crates/
  rustynotes-frontend/  Leptos frontend (Rust -> WASM)
    src/
      app.rs            Router, main view, settings view
      state.rs          Reactive signals (AppState)
      save.rs           Save logic, file loading, folder opening
      bridge.rs         JS bridge FFI wrappers
      tauri_ipc.rs      Tauri IPC bindings
      theme.rs          Theme resolution + CSS variable injection
      components/
        toolbar.rs      Title bar + toolbar (traffic lights, file ops)
        titlebar.rs     Reusable title bar for secondary windows
        editor/         Source, WYSIWYG, Split editors
        preview/        Markdown preview (IPC-rendered)
        navigation/     Sidebar, Miller columns, breadcrumb
        settings/       Settings window + categories
        onboarding/     Welcome screen

  rustynotes-common/    Shared types (AppConfig, FileNode, enums)

styles/                 CSS
  base.css              Main stylesheet + theme tokens
  settings.css          Settings window styles
  katex.min.css         KaTeX math rendering
  themes/               Theme JSON files (dark/light)

js/
  bridge-src.js         JS bridge source (CodeMirror, TipTap, KaTeX, Mermaid)
  bridge.bundle.js      Bundled output (esbuild)
  bundle-vendor.sh      Build script for JS bundle
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | Tauri 2 |
| Frontend | Leptos 0.7 (CSR, WASM) |
| Source editor | CodeMirror 6 |
| WYSIWYG editor | TipTap 3 |
| Markdown parser | Comrak |
| Syntax highlighting | Syntect |
| Math rendering | KaTeX |
| Diagrams | Mermaid |
| Build (WASM) | Trunk |
| Build (JS) | esbuild |
| Package manager | pnpm |

## License

MIT
