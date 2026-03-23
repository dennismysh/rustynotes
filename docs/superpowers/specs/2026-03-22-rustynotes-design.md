# RustyNotes Design Spec

A minimalist macOS markdown editor and viewer built with Tauri (Rust backend) and a TypeScript frontend.

## Goals

- View and edit markdown files with a clean, adaptive UI
- Support extended markdown (GFM, math, diagrams, wiki-links, footnotes, admonitions, definition lists)
- Offer multiple navigation and editing modes for flexibility
- Style customization for every markdown element
- Future export to HTML, PDF, DOCX, LaTeX, and note app formats
- Plain markdown files on disk — no lock-in, no proprietary format

## Architecture

### Two-Layer Split

- **Rust backend (Tauri):** File system operations, markdown parsing (comrak), file watching (notify), search/indexing, export pipeline, config persistence, wiki-link resolution, backlink graph
- **TypeScript frontend:** UI shell, navigation panels, editor integration (CodeMirror 6 + TipTap), rendered markdown view, theme/style engine, settings UI

### IPC Boundary

Tauri `#[tauri::command]` for typed Rust<->TS communication. The frontend never touches the filesystem directly — all file ops go through Tauri commands.

### Key Data Flows

- **Open file:** Frontend requests path -> Rust reads file -> returns content + metadata -> Frontend renders in active editor mode
- **Save file:** Frontend sends content -> Rust writes to disk
- **Live preview:** Frontend sends markdown string -> Rust parses via comrak -> returns HTML string -> Frontend renders in preview pane
- **File watching:** Rust watches folder via `notify` -> emits events to frontend when external changes detected
- **Search:** Frontend sends query -> Rust searches file contents/names -> returns results

### Frontend Framework

Solid.js — chosen for fine-grained reactivity (signals), small bundle size (~7KB), and familiar JSX syntax without a virtual DOM.

## Navigation & Layout

### Navigation Modes (switchable via toolbar or Cmd+1/2/3)

1. **Sidebar tree** — collapsible folder/file tree on the left (~250px default, resizable). Icons for folders/files, drag-to-reorder. Click file to open in content area.
2. **Miller columns** — 2-3 scrollable columns that drill into folders left-to-right. Selecting a file in the rightmost column opens it in the content area.
3. **Breadcrumb** — no persistent nav panel. Breadcrumb bar at the top shows current path. Click any segment to see its children in a dropdown. Content area gets full width.

### Editor Modes (switchable via toolbar segmented control or Cmd+E)

1. **Source** — CodeMirror 6 with markdown syntax highlighting, optional line numbers
2. **WYSIWYG** — TipTap editor rendering markdown as rich text, edit in-place
3. **Split** — source on left, live-rendered preview on right (synchronized scroll)
4. **Preview** — read-only rendered markdown, switchable back to any edit mode

### Toolbar

Minimal top bar containing:
- Navigation mode switcher (3 icons)
- Editor mode switcher (4-segment control)
- Search (magnifying glass)
- Settings gear

### Keyboard Shortcuts

- `Cmd+1/2/3` — switch navigation mode
- `Cmd+E` — cycle editor modes
- `Cmd+P` — preview mode
- `Cmd+Shift+F` — search across files

## Markdown Engine & Extensions

### Parser

comrak (Rust) in the Tauri backend. Parses markdown to HTML which is sent to the frontend for rendering.

### Extension Support (all toggleable in settings)

| Extension | Parser | Renderer (frontend) |
|---|---|---|
| GFM (tables, task lists, strikethrough, autolinks) | comrak built-in | CSS styling |
| Footnotes | comrak built-in | CSS styling + scroll-to anchor |
| Math (`$...$`, `$$...$$`) | comrak raw pass-through (custom node) | KaTeX |
| Diagrams (` ```mermaid ` blocks) | comrak raw pass-through (fenced code) | Mermaid.js |
| Wiki-links (`[[page]]`) | Custom comrak plugin or post-parse regex | Custom resolver — Rust resolves link targets, frontend renders as clickable links |
| Admonitions (`> [!NOTE]`, etc.) | Post-parse transform in Rust | CSS styled callout boxes |
| Definition lists | comrak built-in (with extension flag) | CSS styling |
| Syntax highlighting | comrak outputs fenced code blocks | Shiki (WASM-based, same TextMate grammars as VS Code, build-time compatible) |
| Frontmatter (YAML) | Rust strips + parses to struct | Optionally rendered as metadata header, toggleable |

### Wiki-Links & Backlinks

Rust maintains an in-memory index of all `[[link]]` targets across the opened folder. When a file is opened, the backend resolves wiki-links to actual file paths and provides a backlinks list (files that link to the current file). Rebuilt on file change events.

### WYSIWYG Mapping

TipTap extensions mirror each markdown extension — a TipTap node type for each comrak AST node.

**Conversion flow:** Markdown is the source of truth, not the comrak AST or ProseMirror document.

- **Markdown -> WYSIWYG:** When entering WYSIWYG mode, the raw markdown string is sent to the Rust backend which parses it via comrak and returns a JSON AST (comrak nodes serialized via serde). The frontend walks this JSON AST and builds the corresponding ProseMirror/TipTap document nodes.
- **WYSIWYG -> Markdown:** When saving or switching to source mode, the frontend serializes the TipTap document back to markdown using TipTap's `tiptap-markdown` extension (or a custom serializer that walks the ProseMirror doc and emits markdown syntax). This produces a markdown string that gets written to disk.
- **Round-trip integrity:** The markdown string is always what gets persisted. Switching modes does: `markdown -> (parse) -> editor model -> (serialize) -> markdown`. Any extension not representable in TipTap falls back to an opaque HTML block node that preserves the raw markdown source.

## Theming & Style Customization

### Adaptive Base

Detects macOS light/dark mode via `prefers-color-scheme` and Tauri's `theme()` API. Ships with default light and dark themes.

### Theme System

CSS custom properties drive all visual styling. A theme is a JSON file mapping semantic tokens to values:

```json
{
  "name": "Default Dark",
  "colors": {
    "bg-primary": "#1e1e2e",
    "bg-secondary": "#181825",
    "text-primary": "#cdd6f4",
    "text-muted": "#6c7086",
    "accent": "#89b4fa",
    "border": "#313244"
  },
  "typography": {
    "body-font": "Inter, system-ui",
    "body-size": "16px",
    "mono-font": "JetBrains Mono, monospace",
    "heading-font": "Inter, system-ui",
    "line-height": 1.6
  },
  "spacing": {
    "content-width": "720px",
    "paragraph-gap": "1em"
  }
}
```

### Per-Element Customization

Users can override individual markdown element styles — heading sizes/weights, code block background, blockquote border color, link color, table styling. Stored as overrides on top of the active theme.

### Settings UI

Visual settings panel with live preview. Sliders for sizes, color pickers, font dropdowns. Changes apply instantly via CSS variable updates — no reload needed.

### Rendering Toggles

Independent switches for:
- Show/hide frontmatter
- Render math or show raw LaTeX
- Render diagrams or show raw code
- Show/hide line numbers in code blocks
- Wiki-link display (show as link text vs `[[raw]]`)

## Export Pipeline

### Architecture

All export lives in Rust. The comrak AST is the canonical intermediate representation — every export format reads from the same AST.

### Export Formats (future, architecture designed now)

| Format | Approach |
|---|---|
| HTML | comrak `format_html` + theme CSS inlined |
| PDF | HTML export piped through `weasyprint` (CLI) or headless WebView print |
| DOCX | Custom AST walker generating Open XML (or pandoc) |
| LaTeX | Custom AST walker emitting `.tex` |
| Obsidian vault | Direct — markdown files with `[[wiki-link]]` syntax are already compatible |
| Notion | Notion API — map AST nodes to Notion block types |
| Bear / Apple Notes | Export as HTML, import via app URL schemes |

### Exporter Trait

```rust
trait Exporter {
    fn export(&self, ast: &comrak::nodes::AstNode, options: &ExportOptions) -> Result<Vec<u8>>;
}
```

Each format implements this trait. New formats added without touching existing code.

## Data Model

- **Markdown files** — plain `.md` files on disk. The app never modifies file format or adds proprietary metadata.
- **Config** — `~/.config/rustynotes/config.json` — theme, active nav mode, editor mode, rendering toggles, recent folders
- **Wiki-link index** — cached in-memory, rebuilt on folder open (not persisted)
- **Theme overrides** — stored alongside config as JSON

**Key principle:** The app is a view layer over markdown files. No database, no proprietary format, no lock-in.

## Project Structure

```
rustynotes/
├── src-tauri/           # Rust backend
│   ├── src/
│   │   ├── main.rs      # Tauri entry point
│   │   ├── commands/    # Tauri IPC command handlers
│   │   ├── markdown/    # comrak parsing, extensions, wiki-links
│   │   ├── fs/          # File operations, watcher, search
│   │   ├── export/      # Exporter trait + format implementations
│   │   └── config/      # Config persistence, theme loading
│   └── Cargo.toml
├── src/                  # TypeScript frontend
│   ├── app.ts           # App shell, routing
│   ├── components/
│   │   ├── navigation/  # Sidebar, MillerColumns, Breadcrumb
│   │   ├── editor/      # CodeMirror wrapper, TipTap wrapper, SplitPane
│   │   ├── preview/     # Rendered markdown view
│   │   └── settings/    # Theme editor, rendering toggles
│   ├── styles/
│   │   ├── themes/      # Default light/dark theme JSON
│   │   └── base.css     # CSS variable definitions, adaptive styling
│   └── lib/
│       ├── ipc.ts       # Typed Tauri command wrappers
│       └── state.ts     # Frontend state management
├── package.json
└── tauri.conf.json
```

## Key Dependencies

### Rust Crates
- `comrak` — markdown parsing
- `notify` — filesystem watching
- `serde` / `serde_json` — serialization
- `syntect` — server-side syntax highlighting for exports
- `walkdir` — directory traversal

### TypeScript
- `@codemirror/view` + `@codemirror/lang-markdown` — source editor
- `@tiptap/core` + extensions — WYSIWYG editor
- `katex` — math rendering
- `mermaid` — diagram rendering
- `shiki` — code syntax highlighting (WASM-based, TextMate grammars)
- `solid-js` — reactive UI framework
