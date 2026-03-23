# RustyNotes Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working Tauri + Solid.js app that opens a folder, browses markdown files in a sidebar tree, edits them in a CodeMirror source editor, and previews rendered markdown.

**Architecture:** Tauri v2 Rust backend handles file I/O and markdown parsing (comrak). Solid.js TypeScript frontend renders the UI with a sidebar file tree, CodeMirror 6 source editor, and markdown preview pane. Communication via Tauri's typed IPC commands.

**Tech Stack:** Tauri 2.x, Solid.js, TypeScript, comrak 0.51, CodeMirror 6, notify (file watching), Vite

**Spec:** `docs/superpowers/specs/2026-03-22-rustynotes-design.md`

---

## File Map

### Rust Backend (`src-tauri/`)

| File | Responsibility |
|------|---------------|
| `src/main.rs` | Desktop entry point (generated, delegates to lib) |
| `src/lib.rs` | Tauri builder setup, command registration, state management |
| `src/commands/mod.rs` | Module declarations for IPC commands |
| `src/commands/fs.rs` | File system IPC commands: read, write, list directory |
| `src/commands/markdown.rs` | Markdown parsing IPC command |
| `src/fs_ops.rs` | Core file system operations (testable, no Tauri dependency) |
| `src/markdown_parser.rs` | comrak wrapper with GFM + extension config |
| `src/watcher.rs` | File system watcher using notify crate |
| `Cargo.toml` | Rust dependencies |

### TypeScript Frontend (`src/`)

| File | Responsibility |
|------|---------------|
| `index.html` | HTML entry point |
| `index.tsx` | Solid.js mount |
| `App.tsx` | Main app shell — toolbar + sidebar + content area layout |
| `components/navigation/Sidebar.tsx` | Recursive file tree with expand/collapse |
| `components/editor/SourceEditor.tsx` | CodeMirror 6 wrapper for markdown editing |
| `components/preview/Preview.tsx` | Rendered markdown HTML view |
| `components/Toolbar.tsx` | Top bar with editor mode switcher (source/preview for now) |
| `lib/ipc.ts` | Typed wrappers for all Tauri commands |
| `lib/state.ts` | Solid.js signals: current folder, file tree, active file, content, editor mode |
| `styles/base.css` | CSS custom properties, adaptive light/dark, layout grid |

---

## Task 1: Project Scaffolding

**Files:**
- Create: entire project via `create-tauri-app`
- Modify: `src-tauri/Cargo.toml` (add dependencies)
- Modify: `package.json` (add dependencies)
- Create: `.gitignore`

- [ ] **Step 1: Scaffold Tauri + Solid.js project**

```bash
cd "/Users/dennis/programming projects"
pnpm create tauri-app rustynotes-scaffold --template template-solid-ts
```

Select: TypeScript, pnpm, Solid. Then copy the generated files into the existing `rustynotes/` directory (which already has the docs/ folder and git history):

```bash
cp -r rustynotes-scaffold/* rustynotes/
cp rustynotes-scaffold/.gitignore rustynotes/
rm -rf rustynotes-scaffold
```

- [ ] **Step 2: Add Rust dependencies**

Edit `src-tauri/Cargo.toml` — add under `[dependencies]`:

```toml
comrak = "0.51"
notify = "8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
walkdir = "2"
thiserror = "2"
```

- [ ] **Step 3: Add frontend dependencies**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm add @codemirror/view @codemirror/state @codemirror/lang-markdown @codemirror/language @codemirror/commands @codemirror/search
pnpm add -D @tauri-apps/cli@latest
```

- [ ] **Step 4: Update .gitignore**

Append to `.gitignore`:

```
.superpowers/
```

- [ ] **Step 5: Verify it builds and shows a window**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm install
pnpm tauri dev
```

Expected: A window opens showing the default Solid.js Tauri welcome page.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat: scaffold Tauri v2 + Solid.js project with dependencies"
```

---

## Task 2: Rust File System Operations

**Files:**
- Create: `src-tauri/src/fs_ops.rs`
- Modify: `src-tauri/src/lib.rs` (add module declaration)

- [ ] **Step 1: Write failing tests for fs_ops**

Create `src-tauri/src/fs_ops.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
}

pub fn read_file_content(path: &Path) -> Result<String, FsError> {
    if !path.exists() {
        return Err(FsError::NotFound(path.display().to_string()));
    }
    Ok(std::fs::read_to_string(path)?)
}

pub fn write_file_content(path: &Path, content: &str) -> Result<(), FsError> {
    Ok(std::fs::write(path, content)?)
}

pub fn list_directory(path: &Path) -> Result<Vec<FileEntry>, FsError> {
    if !path.exists() {
        return Err(FsError::NotFound(path.display().to_string()));
    }
    let mut entries: Vec<FileEntry> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let file_type = entry.file_type();
            let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files/dirs
            if name.starts_with('.') {
                continue;
            }

            // For files, only include .md files
            if !is_dir && !name.ends_with(".md") {
                continue;
            }

            let children = if is_dir {
                Some(list_directory(&entry.path()).unwrap_or_default())
            } else {
                None
            };

            entries.push(FileEntry {
                name,
                path: entry.path(),
                is_dir,
                children,
            });
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        // Create test structure
        fs::create_dir_all(base.join("subfolder")).unwrap();
        fs::write(base.join("hello.md"), "# Hello\nWorld").unwrap();
        fs::write(base.join("notes.md"), "Some notes").unwrap();
        fs::write(base.join("subfolder/nested.md"), "Nested content").unwrap();
        fs::write(base.join("readme.txt"), "not markdown").unwrap();
        fs::write(base.join(".hidden"), "hidden file").unwrap();

        dir
    }

    #[test]
    fn test_read_file_content() {
        let dir = setup_test_dir();
        let content = read_file_content(&dir.path().join("hello.md")).unwrap();
        assert_eq!(content, "# Hello\nWorld");
    }

    #[test]
    fn test_read_file_not_found() {
        let result = read_file_content(Path::new("/nonexistent/file.md"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::NotFound(_)));
    }

    #[test]
    fn test_write_file_content() {
        let dir = setup_test_dir();
        let path = dir.path().join("new.md");
        write_file_content(&path, "New content").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "New content");
    }

    #[test]
    fn test_list_directory() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();

        // Should have: subfolder, hello.md, notes.md (no .hidden, no readme.txt)
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "subfolder");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "hello.md");
        assert_eq!(entries[2].name, "notes.md");
    }

    #[test]
    fn test_list_directory_skips_hidden() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        assert!(!entries.iter().any(|e| e.name.starts_with('.')));
    }

    #[test]
    fn test_list_directory_only_markdown_files() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        for entry in &entries {
            if !entry.is_dir {
                assert!(entry.name.ends_with(".md"));
            }
        }
    }

    #[test]
    fn test_list_directory_nested() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        let subfolder = &entries[0];
        let children = subfolder.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "nested.md");
    }
}
```

- [ ] **Step 2: Add module to lib.rs and tempfile dev-dependency**

Add to `src-tauri/Cargo.toml` under `[dev-dependencies]`:

```toml
tempfile = "3"
```

Add to `src-tauri/src/lib.rs`:

```rust
mod fs_ops;
```

- [ ] **Step 3: Run tests to verify they pass**

```bash
cd "/Users/dennis/programming projects/rustynotes/src-tauri"
cargo test fs_ops -- --nocapture
```

Expected: All 6 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/fs_ops.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: add file system operations module with tests"
```

---

## Task 3: Rust Markdown Parser

**Files:**
- Create: `src-tauri/src/markdown_parser.rs`
- Modify: `src-tauri/src/lib.rs` (add module)

- [ ] **Step 1: Write parser module with tests**

Create `src-tauri/src/markdown_parser.rs`:

```rust
use comrak::{markdown_to_html, Options};

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str) -> String {
        let options = Self::default_options();
        markdown_to_html(input, &options)
    }

    fn default_options() -> Options<'static> {
        let mut options = Options::default();

        // GFM extensions
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
        options.extension.footnotes = true;
        options.extension.description_lists = true;

        // GitHub-style alerts (admonitions)
        options.extension.alerts = true;

        // Math support
        options.extension.math_dollars = true;

        // Heading IDs for anchor links
        options.extension.header_ids = Some(String::new());

        // Front matter
        options.extension.front_matter_delimiter = Some("---".to_string());

        // Wiki-links
        options.extension.wikilinks_title_after_pipe = true;

        // Allow raw HTML passthrough
        options.render.r#unsafe = true;

        options
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_markdown() {
        let html = MarkdownParser::parse("# Hello\n\nWorld");
        assert!(html.contains("<h1"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<p>World</p>"));
    }

    #[test]
    fn test_bold_italic() {
        let html = MarkdownParser::parse("**bold** and *italic*");
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn test_gfm_strikethrough() {
        let html = MarkdownParser::parse("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_gfm_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn test_gfm_tasklist() {
        let md = "- [x] Done\n- [ ] Todo";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("checked"));
        assert!(html.contains("type=\"checkbox\""));
    }

    #[test]
    fn test_fenced_code_block() {
        let md = "```rust\nfn main() {}\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("<code class=\"language-rust\">"));
    }

    #[test]
    fn test_math_passthrough() {
        let md = "Inline $x^2$ and block:\n\n$$\nE = mc^2\n$$";
        let html = MarkdownParser::parse(md);
        // comrak with math_dollars wraps math in specific elements
        assert!(html.contains("x^2") || html.contains("math"));
    }

    #[test]
    fn test_footnotes() {
        let md = "Text[^1]\n\n[^1]: Footnote content";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("footnote"));
    }

    #[test]
    fn test_autolink() {
        let html = MarkdownParser::parse("Visit https://example.com");
        assert!(html.contains("<a href=\"https://example.com\">"));
    }
}
```

- [ ] **Step 2: Add module declaration**

Add to `src-tauri/src/lib.rs`:

```rust
mod markdown_parser;
```

- [ ] **Step 3: Run tests**

```bash
cd "/Users/dennis/programming projects/rustynotes/src-tauri"
cargo test markdown_parser -- --nocapture
```

Expected: All 9 tests pass.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/markdown_parser.rs src-tauri/src/lib.rs
git commit -m "feat: add markdown parser module wrapping comrak with GFM extensions"
```

---

## Task 4: Tauri IPC Commands

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/fs.rs`
- Create: `src-tauri/src/commands/markdown.rs`
- Modify: `src-tauri/src/lib.rs` (register commands)
- Modify: `src-tauri/capabilities/default.json` (if needed for fs dialog)

- [ ] **Step 1: Create error type for commands**

Create `src-tauri/src/commands/mod.rs`:

```rust
pub mod fs;
pub mod markdown;

use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Fs(#[from] crate::fs_ops::FsError),
    #[error("{0}")]
    Generic(String),
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_str(self.to_string().as_ref())
    }
}
```

- [ ] **Step 2: Create file system commands**

Create `src-tauri/src/commands/fs.rs`:

```rust
use super::CommandError;
use crate::fs_ops::{self, FileEntry};

#[tauri::command]
pub fn read_file(path: String) -> Result<String, CommandError> {
    Ok(fs_ops::read_file_content(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), CommandError> {
    Ok(fs_ops::write_file_content(
        std::path::Path::new(&path),
        &content,
    )?)
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, CommandError> {
    Ok(fs_ops::list_directory(std::path::Path::new(&path))?)
}
```

- [ ] **Step 3: Create markdown command**

Create `src-tauri/src/commands/markdown.rs`:

```rust
use crate::markdown_parser::MarkdownParser;

#[tauri::command]
pub fn parse_markdown(content: String) -> String {
    MarkdownParser::parse(&content)
}
```

- [ ] **Step 4: Register commands in lib.rs**

Replace `src-tauri/src/lib.rs` content:

```rust
mod commands;
mod fs_ops;
mod markdown_parser;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::markdown::parse_markdown,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: Add dialog plugin dependency**

Add to `src-tauri/Cargo.toml` under `[dependencies]`:

```toml
tauri-plugin-dialog = "2"
```

And in the frontend:

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm add @tauri-apps/plugin-dialog
```

Also update `src-tauri/capabilities/default.json` to add dialog permissions. Add `"dialog:allow-open"` to the `"permissions"` array:

```json
{
  ...
  "permissions": [
    "core:default",
    "opener:default",
    "dialog:allow-open"
  ]
}
```

- [ ] **Step 6: Verify it compiles**

```bash
cd "/Users/dennis/programming projects/rustynotes/src-tauri"
cargo check
```

Expected: Compiles without errors.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/ src-tauri/src/lib.rs src-tauri/Cargo.toml package.json pnpm-lock.yaml
git commit -m "feat: add Tauri IPC commands for file ops and markdown parsing"
```

---

## Task 5: Frontend State & IPC Layer

**Files:**
- Create: `src/lib/ipc.ts`
- Create: `src/lib/state.ts`

- [ ] **Step 1: Create typed IPC wrappers**

Create `src/lib/ipc.ts`:

```typescript
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileEntry[] | null;
}

export async function readFile(path: string): Promise<string> {
  return invoke<string>("read_file", { path });
}

export async function writeFile(
  path: string,
  content: string
): Promise<void> {
  return invoke<void>("write_file", { path, content });
}

export async function listDirectory(path: string): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("list_directory", { path });
}

export async function parseMarkdown(content: string): Promise<string> {
  return invoke<string>("parse_markdown", { content });
}

export async function openFolderDialog(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return selected as string | null;
}
```

- [ ] **Step 2: Create app state with Solid.js signals**

Create `src/lib/state.ts`:

```typescript
import { createSignal, createRoot } from "solid-js";
import type { FileEntry } from "./ipc";

export type EditorMode = "source" | "preview";

function createAppState() {
  const [currentFolder, setCurrentFolder] = createSignal<string | null>(null);
  const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
  const [activeFilePath, setActiveFilePath] = createSignal<string | null>(null);
  const [activeFileContent, setActiveFileContent] = createSignal<string>("");
  const [editorMode, setEditorMode] = createSignal<EditorMode>("source");
  const [isDirty, setIsDirty] = createSignal(false);
  const [renderedHtml, setRenderedHtml] = createSignal<string>("");

  return {
    currentFolder,
    setCurrentFolder,
    fileTree,
    setFileTree,
    activeFilePath,
    setActiveFilePath,
    activeFileContent,
    setActiveFileContent,
    editorMode,
    setEditorMode,
    isDirty,
    setIsDirty,
    renderedHtml,
    setRenderedHtml,
  };
}

export const appState = createRoot(createAppState);
```

- [ ] **Step 3: Commit**

```bash
git add src/lib/
git commit -m "feat: add typed IPC wrappers and Solid.js app state"
```

---

## Task 6: Base Styles & App Shell

**Files:**
- Create: `src/styles/base.css`
- Create: `src/components/Toolbar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/index.tsx`
- Modify: `index.html`

- [ ] **Step 1: Create base CSS with adaptive theme**

Create `src/styles/base.css`:

```css
:root {
  /* Light theme (default) */
  --bg-primary: #ffffff;
  --bg-secondary: #f5f5f7;
  --bg-tertiary: #e8e8ed;
  --text-primary: #1d1d1f;
  --text-secondary: #6e6e73;
  --text-muted: #aeaeb2;
  --accent: #007aff;
  --border: #d2d2d7;
  --sidebar-width: 250px;
  --toolbar-height: 40px;

  /* Typography */
  --font-body: -apple-system, BlinkMacSystemFont, "Segoe UI", system-ui, sans-serif;
  --font-mono: "SF Mono", "JetBrains Mono", "Fira Code", monospace;
  --font-size: 15px;
  --line-height: 1.6;
}

@media (prefers-color-scheme: dark) {
  :root {
    --bg-primary: #1e1e2e;
    --bg-secondary: #181825;
    --bg-tertiary: #313244;
    --text-primary: #cdd6f4;
    --text-secondary: #a6adc8;
    --text-muted: #6c7086;
    --accent: #89b4fa;
    --border: #45475a;
  }
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

html, body, #root {
  height: 100%;
  overflow: hidden;
  font-family: var(--font-body);
  font-size: var(--font-size);
  line-height: var(--line-height);
  color: var(--text-primary);
  background: var(--bg-primary);
}

.app-shell {
  display: grid;
  grid-template-rows: var(--toolbar-height) 1fr;
  grid-template-columns: var(--sidebar-width) 1fr;
  height: 100%;
}

.toolbar {
  grid-column: 1 / -1;
  grid-row: 1;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 12px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  -webkit-app-region: drag;
  user-select: none;
}

.toolbar button {
  -webkit-app-region: no-drag;
  background: none;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  padding: 4px 10px;
  font-size: 12px;
  cursor: pointer;
  font-family: var(--font-body);
}

.toolbar button:hover {
  background: var(--bg-tertiary);
}

.toolbar button.active {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.toolbar .spacer {
  flex: 1;
}

.toolbar .mode-switcher {
  display: flex;
  gap: 0;
}

.toolbar .mode-switcher button {
  border-radius: 0;
  border-right-width: 0;
}

.toolbar .mode-switcher button:first-child {
  border-radius: 6px 0 0 6px;
}

.toolbar .mode-switcher button:last-child {
  border-radius: 0 6px 6px 0;
  border-right-width: 1px;
}

.sidebar {
  grid-row: 2;
  grid-column: 1;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  padding: 8px 0;
}

.content-area {
  grid-row: 2;
  grid-column: 2;
  overflow: hidden;
  position: relative;
}

/* Sidebar tree styles */
.tree-item {
  display: flex;
  align-items: center;
  padding: 4px 12px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
  gap: 6px;
  user-select: none;
}

.tree-item:hover {
  background: var(--bg-tertiary);
}

.tree-item.active {
  background: var(--accent);
  color: white;
}

.tree-item .icon {
  font-size: 14px;
  width: 16px;
  text-align: center;
  flex-shrink: 0;
}

.tree-item .name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.tree-children {
  padding-left: 16px;
}

/* Preview styles */
.preview-container {
  padding: 24px 40px;
  max-width: 800px;
  margin: 0 auto;
  overflow-y: auto;
  height: 100%;
}

.preview-container h1 { font-size: 2em; margin: 0.67em 0; }
.preview-container h2 { font-size: 1.5em; margin: 0.75em 0; }
.preview-container h3 { font-size: 1.17em; margin: 0.83em 0; }
.preview-container p { margin: 1em 0; }
.preview-container code {
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 0.9em;
}
.preview-container pre {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
  margin: 1em 0;
}
.preview-container pre code {
  background: none;
  padding: 0;
}
.preview-container blockquote {
  border-left: 3px solid var(--accent);
  padding-left: 16px;
  color: var(--text-secondary);
  margin: 1em 0;
}
.preview-container a {
  color: var(--accent);
  text-decoration: none;
}
.preview-container a:hover {
  text-decoration: underline;
}
.preview-container table {
  border-collapse: collapse;
  width: 100%;
  margin: 1em 0;
}
.preview-container th, .preview-container td {
  border: 1px solid var(--border);
  padding: 8px 12px;
  text-align: left;
}
.preview-container th {
  background: var(--bg-secondary);
  font-weight: 600;
}
.preview-container img {
  max-width: 100%;
}

/* Empty state */
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-muted);
  gap: 12px;
}

.empty-state .hint {
  font-size: 13px;
}

/* CodeMirror overrides for theme integration */
.cm-editor {
  height: 100%;
  font-family: var(--font-mono);
  font-size: 14px;
}

.cm-editor .cm-content {
  padding: 16px 24px;
}

.cm-editor .cm-gutters {
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  color: var(--text-muted);
}
```

- [ ] **Step 2: Create Toolbar component**

Create `src/components/Toolbar.tsx`:

```tsx
import { Component } from "solid-js";
import { appState, EditorMode } from "../lib/state";
import { openFolderDialog, listDirectory } from "../lib/ipc";

const Toolbar: Component = () => {
  const { editorMode, setEditorMode, setCurrentFolder, setFileTree } = appState;

  const handleOpenFolder = async () => {
    const folder = await openFolderDialog();
    if (folder) {
      setCurrentFolder(folder);
      const tree = await listDirectory(folder);
      setFileTree(tree);
    }
  };

  const setMode = (mode: EditorMode) => {
    setEditorMode(mode);
  };

  return (
    <div class="toolbar">
      <button onClick={handleOpenFolder}>Open Folder</button>
      <div class="spacer" />
      <div class="mode-switcher">
        <button
          classList={{ active: editorMode() === "source" }}
          onClick={() => setMode("source")}
        >
          Source
        </button>
        <button
          classList={{ active: editorMode() === "preview" }}
          onClick={() => setMode("preview")}
        >
          Preview
        </button>
      </div>
    </div>
  );
};

export default Toolbar;
```

- [ ] **Step 3: Create App shell**

Replace `src/App.tsx`:

```tsx
import { Component, Show } from "solid-js";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import SourceEditor from "./components/editor/SourceEditor";
import Preview from "./components/preview/Preview";
import { appState } from "./lib/state";
import "./styles/base.css";

const App: Component = () => {
  const { activeFilePath, editorMode } = appState;

  return (
    <div class="app-shell">
      <Toolbar />
      <Sidebar />
      <div class="content-area">
        <Show
          when={activeFilePath()}
          fallback={
            <div class="empty-state">
              <div style="font-size: 32px">&#128221;</div>
              <div>Open a folder to get started</div>
              <div class="hint">Cmd+O or click "Open Folder"</div>
            </div>
          }
        >
          <Show when={editorMode() === "source"}>
            <SourceEditor />
          </Show>
          <Show when={editorMode() === "preview"}>
            <Preview />
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default App;
```

- [ ] **Step 4: Update index.tsx**

Replace `src/index.tsx`:

```tsx
import { render } from "solid-js/web";
import App from "./App";

render(() => <App />, document.getElementById("root") as HTMLElement);
```

- [ ] **Step 5: Commit**

```bash
git add src/styles/ src/components/Toolbar.tsx src/App.tsx src/index.tsx
git commit -m "feat: add base CSS with adaptive theme and app shell layout"
```

---

## Task 7: Sidebar File Tree

**Files:**
- Create: `src/components/navigation/Sidebar.tsx`

- [ ] **Step 1: Create Sidebar component**

Create `src/components/navigation/Sidebar.tsx`:

```tsx
import { Component, For, Show, createSignal } from "solid-js";
import { appState } from "../../lib/state";
import { readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const TreeNode: Component<{ entry: FileEntry; depth: number }> = (props) => {
  const [expanded, setExpanded] = createSignal(false);
  const { activeFilePath, setActiveFilePath, setActiveFileContent, setRenderedHtml, setIsDirty } = appState;

  const handleClick = async () => {
    if (props.entry.is_dir) {
      setExpanded(!expanded());
    } else {
      setActiveFilePath(props.entry.path);
      const content = await readFile(props.entry.path);
      setActiveFileContent(content);
      setIsDirty(false);

      const html = await parseMarkdown(content);
      setRenderedHtml(html);
    }
  };

  const isActive = () => activeFilePath() === props.entry.path;

  return (
    <div>
      <div
        class="tree-item"
        classList={{ active: isActive() }}
        style={{ "padding-left": `${12 + props.depth * 16}px` }}
        onClick={handleClick}
      >
        <span class="icon">
          {props.entry.is_dir ? (expanded() ? "&#9660;" : "&#9654;") : "&#128196;"}
        </span>
        <span class="name">{props.entry.name}</span>
      </div>
      <Show when={props.entry.is_dir && expanded() && props.entry.children}>
        <div class="tree-children">
          <For each={props.entry.children!}>
            {(child) => <TreeNode entry={child} depth={props.depth + 1} />}
          </For>
        </div>
      </Show>
    </div>
  );
};

const Sidebar: Component = () => {
  const { fileTree, currentFolder } = appState;

  return (
    <div class="sidebar">
      <Show
        when={currentFolder()}
        fallback={
          <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
            No folder open
          </div>
        }
      >
        <For each={fileTree()}>
          {(entry) => <TreeNode entry={entry} depth={0} />}
        </For>
      </Show>
    </div>
  );
};

export default Sidebar;
```

- [ ] **Step 2: Commit**

```bash
git add src/components/navigation/
git commit -m "feat: add sidebar file tree with expand/collapse and file selection"
```

---

## Task 8: CodeMirror Source Editor

**Files:**
- Create: `src/components/editor/SourceEditor.tsx`

- [ ] **Step 1: Create CodeMirror wrapper**

Create `src/components/editor/SourceEditor.tsx`:

```tsx
import { Component, onMount, onCleanup, createEffect } from "solid-js";
import { EditorView, keymap, lineNumbers, highlightActiveLine } from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { markdown } from "@codemirror/lang-markdown";
import { defaultKeymap, history, historyKeymap } from "@codemirror/commands";
import { searchKeymap, highlightSelectionMatches } from "@codemirror/search";
import { appState } from "../../lib/state";
import { writeFile, parseMarkdown } from "../../lib/ipc";

const SourceEditor: Component = () => {
  let containerRef: HTMLDivElement | undefined;
  let view: EditorView | undefined;
  let skipUpdate = false;

  const {
    activeFileContent,
    setActiveFileContent,
    activeFilePath,
    setIsDirty,
    setRenderedHtml,
  } = appState;

  // Theme that adapts to CSS variables
  const theme = EditorView.theme({
    "&": {
      height: "100%",
      backgroundColor: "var(--bg-primary)",
      color: "var(--text-primary)",
    },
    ".cm-content": {
      fontFamily: "var(--font-mono)",
      fontSize: "14px",
      padding: "16px 24px",
      caretColor: "var(--accent)",
    },
    ".cm-cursor": {
      borderLeftColor: "var(--accent)",
    },
    ".cm-gutters": {
      backgroundColor: "var(--bg-secondary)",
      borderRight: "1px solid var(--border)",
      color: "var(--text-muted)",
    },
    ".cm-activeLineGutter": {
      backgroundColor: "var(--bg-tertiary)",
    },
    ".cm-activeLine": {
      backgroundColor: "var(--bg-secondary)",
    },
    ".cm-selectionMatch": {
      backgroundColor: "var(--bg-tertiary)",
    },
    "&.cm-focused .cm-selectionBackground, ::selection": {
      backgroundColor: "var(--accent)",
      opacity: "0.3",
    },
  });

  const saveFile = async () => {
    const path = activeFilePath();
    if (path && view) {
      const content = view.state.doc.toString();
      await writeFile(path, content);
      setIsDirty(false);
    }
  };

  const saveKeymap = keymap.of([
    {
      key: "Mod-s",
      run: () => {
        saveFile();
        return true;
      },
    },
  ]);

  onMount(() => {
    if (!containerRef) return;

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && !skipUpdate) {
        const content = update.state.doc.toString();
        setActiveFileContent(content);
        setIsDirty(true);

        // Debounced preview update
        parseMarkdown(content).then((html) => {
          setRenderedHtml(html);
        });
      }
    });

    const state = EditorState.create({
      doc: activeFileContent(),
      extensions: [
        lineNumbers(),
        highlightActiveLine(),
        highlightSelectionMatches(),
        history(),
        markdown(),
        theme,
        keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
        saveKeymap,
        updateListener,
      ],
    });

    view = new EditorView({
      state,
      parent: containerRef,
    });
  });

  // Sync external content changes into editor
  createEffect(() => {
    const content = activeFileContent();
    if (view && view.state.doc.toString() !== content) {
      skipUpdate = true;
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: content,
        },
      });
      skipUpdate = false;
    }
  });

  onCleanup(() => {
    view?.destroy();
  });

  return <div ref={containerRef} style={{ height: "100%", overflow: "hidden" }} />;
};

export default SourceEditor;
```

- [ ] **Step 2: Verify it compiles with the frontend build**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm build
```

Expected: TypeScript compiles without errors (Tauri build may need `tauri build`, but `pnpm build` should check TS).

- [ ] **Step 3: Commit**

```bash
git add src/components/editor/
git commit -m "feat: add CodeMirror 6 source editor with markdown highlighting and save"
```

---

## Task 9: Markdown Preview

**Files:**
- Create: `src/components/preview/Preview.tsx`

- [ ] **Step 1: Create Preview component**

Create `src/components/preview/Preview.tsx`:

```tsx
import { Component } from "solid-js";
import { appState } from "../../lib/state";

const Preview: Component = () => {
  const { renderedHtml } = appState;

  return (
    <div class="preview-container" innerHTML={renderedHtml()} />
  );
};

export default Preview;
```

- [ ] **Step 2: Commit**

```bash
git add src/components/preview/
git commit -m "feat: add markdown preview component rendering comrak HTML output"
```

---

## Task 10: File Watcher

**Files:**
- Create: `src-tauri/src/watcher.rs`
- Modify: `src-tauri/src/lib.rs` (add watcher setup)

- [ ] **Step 1: Create watcher module**

Create `src-tauri/src/watcher.rs`:

```rust
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc;
use tauri::Emitter;

#[derive(Clone, serde::Serialize)]
pub struct FileChangeEvent {
    pub paths: Vec<String>,
    pub kind: String,
}

pub fn start_watcher(
    app_handle: tauri::AppHandle,
    path: &Path,
) -> Result<RecommendedWatcher, notify::Error> {
    let (tx, rx) = mpsc::channel::<Result<Event, notify::Error>>();

    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
    watcher.watch(path, RecursiveMode::Recursive)?;

    let handle = app_handle.clone();
    std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            if let Ok(event) = event {
                let paths: Vec<String> = event
                    .paths
                    .iter()
                    .filter(|p| {
                        p.extension()
                            .map(|ext| ext == "md")
                            .unwrap_or(false)
                    })
                    .map(|p| p.display().to_string())
                    .collect();

                if !paths.is_empty() {
                    let kind = format!("{:?}", event.kind);
                    let _ = handle.emit(
                        "file-changed",
                        FileChangeEvent { paths, kind },
                    );
                }
            }
        }
    });

    Ok(watcher)
}
```

- [ ] **Step 2: Add watcher command and state to lib.rs**

Update `src-tauri/src/lib.rs`:

```rust
mod commands;
mod fs_ops;
mod markdown_parser;
mod watcher;

use std::sync::Mutex;

struct WatcherState {
    _watcher: Mutex<Option<notify::RecommendedWatcher>>,
}

#[tauri::command]
fn watch_folder(path: String, app_handle: tauri::AppHandle, state: tauri::State<WatcherState>) -> Result<(), String> {
    let watcher = watcher::start_watcher(app_handle, std::path::Path::new(&path))
        .map_err(|e| e.to_string())?;
    *state._watcher.lock().unwrap() = Some(watcher);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(WatcherState {
            _watcher: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::markdown::parse_markdown,
            watch_folder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: Add file-changed listener to frontend IPC**

Append to `src/lib/ipc.ts`:

```typescript
import { listen } from "@tauri-apps/api/event";

export interface FileChangeEvent {
  paths: string[];
  kind: string;
}

export async function watchFolder(path: string): Promise<void> {
  return invoke<void>("watch_folder", { path });
}

export async function onFileChanged(
  callback: (event: FileChangeEvent) => void
): Promise<() => void> {
  return listen<FileChangeEvent>("file-changed", (event) => {
    callback(event.payload);
  });
}
```

- [ ] **Step 4: Wire watcher into Toolbar folder open**

Update the `handleOpenFolder` function in `src/components/Toolbar.tsx` to also start watching:

```typescript
import { openFolderDialog, listDirectory, watchFolder } from "../lib/ipc";

// Inside handleOpenFolder:
const handleOpenFolder = async () => {
    const folder = await openFolderDialog();
    if (folder) {
      setCurrentFolder(folder);
      const tree = await listDirectory(folder);
      setFileTree(tree);
      await watchFolder(folder);
    }
  };
```

- [ ] **Step 5: Verify compilation**

```bash
cd "/Users/dennis/programming projects/rustynotes/src-tauri"
cargo check
```

Expected: Compiles without errors.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/watcher.rs src-tauri/src/lib.rs src/lib/ipc.ts src/components/Toolbar.tsx
git commit -m "feat: add file watcher with Tauri events for live folder monitoring"
```

---

## Task 11: Integration Smoke Test

**Files:**
- No new files — verify everything works end-to-end

- [ ] **Step 1: Create a test markdown folder**

```bash
mkdir -p /tmp/rustynotes-test
cat > /tmp/rustynotes-test/welcome.md << 'EOF'
# Welcome to RustyNotes

This is a **test document** with some markdown features.

## Features

- [x] File tree navigation
- [x] Source editing
- [x] Preview rendering
- [ ] WYSIWYG editing (coming soon)

## Code Example

```rust
fn main() {
    println!("Hello from RustyNotes!");
}
```

## Table

| Feature | Status |
|---------|--------|
| Sidebar | Done |
| Editor  | Done |
| Preview | Done |

> This is a blockquote for testing.

Visit https://github.com for ~~nothing~~ everything.
EOF

mkdir -p /tmp/rustynotes-test/subfolder
cat > /tmp/rustynotes-test/subfolder/notes.md << 'EOF'
# Nested Notes

This file tests nested folder navigation.
EOF
```

- [ ] **Step 2: Launch the app and test**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm tauri dev
```

**Manual verification checklist:**
1. Window opens with empty state ("Open a folder to get started")
2. Click "Open Folder" -> navigate to `/tmp/rustynotes-test` -> OK
3. Sidebar shows: subfolder/, welcome.md
4. Click welcome.md -> source editor shows markdown content
5. Click "Preview" button -> rendered HTML with headings, code, table, checkboxes
6. Click "Source" -> back to CodeMirror editor
7. Edit text in source -> switch to preview -> changes reflected
8. Cmd+S saves the file
9. Expand subfolder/ -> click notes.md -> content loads

- [ ] **Step 3: Fix any issues found during testing**

Address any bugs discovered during the smoke test.

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "fix: address integration issues from smoke testing"
```

(Only if there were fixes. Skip if everything works.)
