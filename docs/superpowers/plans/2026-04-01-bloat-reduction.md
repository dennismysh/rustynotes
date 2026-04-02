# Bloat Reduction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce WASM binary (~2-3MB) and JS bundle (~5MB initial) by removing dead code, consolidating comrak/syntect into the backend, and splitting the JS bridge bundle.

**Architecture:** Move all markdown parsing and syntax highlighting from the WASM frontend to the native Rust backend (accessed via Tauri IPC). Split the monolithic JS bridge bundle into eagerly-loaded editor code and lazily-loaded rendering libraries. Remove Solid.js migration leftovers.

**Tech Stack:** Tauri 2 IPC, comrak, syntect, esbuild code splitting, Leptos reactive signals

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `crates/rustynotes-frontend/src/bridge.rs` | Remove 5 unused bridge functions |
| Modify | `crates/rustynotes-frontend/src/tauri_ipc.rs` | Remove `listen_file_changed`, add `parse_markdown` IPC |
| Delete | `crates/rustynotes-frontend/src/components/preview/markdown.rs` | Frontend markdown rendering (moving to backend) |
| Modify | `crates/rustynotes-frontend/src/components/preview/mod.rs` | Remove `pub mod markdown` |
| Modify | `crates/rustynotes-frontend/src/components/preview/preview.rs` | Use IPC instead of local rendering |
| Modify | `crates/rustynotes-frontend/src/components/editor/source_editor.rs` | Use IPC for preview sync |
| Modify | `crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs` | Use IPC for preview sync |
| Modify | `crates/rustynotes-frontend/Cargo.toml` | Remove comrak, syntect, regex-lite, html-escape, once_cell |
| Modify | `src-tauri/src/lib.rs` | Register `parse_markdown` command |
| Modify | `src-tauri/src/markdown_parser.rs` | Add syntect highlighting post-processing |
| Modify | `src-tauri/Cargo.toml` | Add syntect, regex, html-escape, once_cell |
| Modify | `package.json` | Remove Solid.js deps, Vite scripts |
| Modify | `styles/base.css` | Remove dead CSS vars, add missing ones |
| Modify | `styles/themes/default-dark.json` | Add surface/text/error tokens |
| Modify | `styles/themes/default-light.json` | Add surface/text/error tokens |
| Modify | `js/bridge-src.js` | Lazy-load KaTeX and Mermaid |
| Modify | `js/bundle-vendor.sh` | Enable esbuild code splitting |
| Modify | `index.html` | Update Trunk directives for chunked JS |

---

### Task 1: Record Baseline Sizes

**Files:**
- None (measurement only)

- [ ] **Step 1: Measure WASM binary size**

Run: `ls -lh dist/rustynotes-frontend-*_bg.wasm`

Record the size (expected ~6.2MB). If dist/ is stale, rebuild first:

Run: `cd "/Users/dennis/programming projects/rustynotes" && trunk build --release 2>&1 | tail -5`

- [ ] **Step 2: Measure JS bundle size**

Run: `ls -lh js/bridge.bundle.js`

Record the size (expected ~8.2MB).

- [ ] **Step 3: Commit baseline notes**

Create a temporary file to track sizes:

```bash
cat > docs/superpowers/plans/bloat-reduction-sizes.md << 'EOF'
# Bloat Reduction Size Tracking

## Baseline
- WASM: [fill in]
- JS bundle: [fill in]

## After cleanup
- WASM: [fill in]
- JS bundle: [fill in]
EOF
```

```bash
git add docs/superpowers/plans/bloat-reduction-sizes.md
git commit -m "docs: add bloat reduction baseline tracking"
```

---

### Task 2: Remove Unused Rust Functions (Frontend)

**Files:**
- Modify: `crates/rustynotes-frontend/src/bridge.rs`
- Modify: `crates/rustynotes-frontend/src/tauri_ipc.rs`

- [ ] **Step 1: Remove 5 unused functions from bridge.rs**

In `crates/rustynotes-frontend/src/bridge.rs`, delete these functions:

1. `focus_code_mirror` (lines 61-63)
2. `get_tiptap_markdown` (lines 96-101)
3. `focus_tiptap` (lines 104-106)
4. `render_katex` (lines 115-127)
5. `render_mermaid` (lines 131-143)

Also remove the `use wasm_bindgen_futures::JsFuture;` import (line 3) since render_katex and render_mermaid were the only async functions using it.

The file should retain: `get_bridge`, `call_bridge`, `mount_code_mirror`, `update_code_mirror`, `destroy_code_mirror`, `mount_tiptap`, `update_tiptap`, `destroy_tiptap`.

- [ ] **Step 2: Remove listen_file_changed from tauri_ipc.rs**

In `crates/rustynotes-frontend/src/tauri_ipc.rs`, delete the `listen_file_changed` function (lines 248-260).

Keep `listen_config_changed` and `listen_event` — they're actively used.

- [ ] **Step 3: Verify it compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -10`

Expected: Build succeeds with no errors. Warnings about unused imports are acceptable and will be fixed in later tasks.

- [ ] **Step 4: Commit**

```bash
git add crates/rustynotes-frontend/src/bridge.rs crates/rustynotes-frontend/src/tauri_ipc.rs
git commit -m "refactor: remove unused bridge and IPC functions"
```

---

### Task 3: Clean Up npm and CSS Dead Code

**Files:**
- Modify: `package.json`
- Modify: `styles/base.css`
- Modify: `styles/themes/default-dark.json`
- Modify: `styles/themes/default-light.json`

- [ ] **Step 1: Remove Solid.js dependencies and Vite scripts from package.json**

In `package.json`, remove these entries:

From `dependencies`:
- `"@solidjs/router": "^0.16.1",`
- `"solid-js": "^1.9.3"`

From `devDependencies`:
- `"vite": "^6.0.3",`
- `"vite-plugin-solid": "^2.11.0"`

From `scripts`, remove all Vite scripts:
- `"start": "vite",`
- `"dev": "vite",`
- `"build": "vite build",`
- `"serve": "vite preview",`

Keep the `"tauri"` script.

- [ ] **Step 2: Reinstall node_modules**

Run: `cd "/Users/dennis/programming projects/rustynotes" && pnpm install 2>&1 | tail -5`

Expected: Clean install without Solid.js packages. pnpm-lock.yaml regenerated.

- [ ] **Step 3: Remove dead CSS custom properties from base.css**

In `styles/base.css`, in the light-mode `:root` block (lines 1-30):
- Remove line 17: `--overlay-bg: rgba(0, 0, 0, 0.4);`
- Remove line 25: `--toolbar-height: 44px;`

In the dark-mode `@media (prefers-color-scheme: dark)` `:root` block (lines 32-49):
- Remove line 48: `--overlay-bg: rgba(0, 0, 0, 0.6);`

- [ ] **Step 4: Add missing CSS custom properties for modal**

In `styles/base.css`, in the light-mode `:root` block, add after the `--border` line:

```css
  --surface: #ffffff;
  --surface-hover: #f0f0f2;
  --text: #1d1d1f;
  --error: #e74c3c;
```

In the dark-mode `:root` block, add after the `--border` line:

```css
    --surface: #1e1e2e;
    --surface-hover: #313244;
    --text: #cdd6f4;
    --error: #f38ba8;
```

- [ ] **Step 5: Add missing tokens to theme JSON files**

In `styles/themes/default-dark.json`, add to the `"colors"` object:

```json
    "surface": "#1e1e2e",
    "surface-hover": "#313244",
    "text": "#cdd6f4",
    "error": "#f38ba8"
```

In `styles/themes/default-light.json`, add to the `"colors"` object:

```json
    "surface": "#ffffff",
    "surface-hover": "#f0f0f2",
    "text": "#1d1d1f",
    "error": "#e74c3c"
```

- [ ] **Step 6: Verify the Trunk build still works**

Run: `cd "/Users/dennis/programming projects/rustynotes" && trunk build 2>&1 | tail -10`

Expected: Build succeeds.

- [ ] **Step 7: Commit**

```bash
git add package.json pnpm-lock.yaml styles/base.css styles/themes/default-dark.json styles/themes/default-light.json
git commit -m "chore: remove Solid.js leftovers, fix dead/missing CSS properties"
```

---

### Task 4: Add Syntax Highlighting to Backend Markdown Parser

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/markdown_parser.rs`

- [ ] **Step 1: Add syntect and supporting deps to backend Cargo.toml**

In `src-tauri/Cargo.toml`, add to `[dependencies]`:

```toml
syntect = { version = "5", default-features = false, features = ["default-fancy", "html"] }
regex = "1"
html-escape = "0.2"
once_cell = "1"
```

- [ ] **Step 2: Add syntax highlighting to markdown_parser.rs**

Replace the contents of `src-tauri/src/markdown_parser.rs` with:

```rust
use comrak::{markdown_to_html, Options};
use once_cell::sync::Lazy;
use syntect::highlighting::ThemeSet;
use syntect::html::highlighted_html_for_string;
use syntect::parsing::SyntaxSet;

static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEME_SET: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

pub struct MarkdownParser;

impl MarkdownParser {
    pub fn parse(input: &str) -> String {
        let html = markdown_to_html(input, &Self::default_options());
        highlight_code_blocks(&html)
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

fn highlight_code_blocks(html: &str) -> String {
    let re = regex::Regex::new(
        r#"<pre><code class="language-(\w+)">([\s\S]*?)</code></pre>"#,
    )
    .unwrap();

    re.replace_all(html, |caps: &regex::Captures| {
        let lang = &caps[1];
        let code = html_escape::decode_html_entities(&caps[2]);

        // Preserve mermaid blocks for client-side rendering
        if lang == "mermaid" {
            return caps[0].to_string();
        }

        let syntax = SYNTAX_SET
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        let theme = &THEME_SET.themes["base16-ocean.dark"];
        match highlighted_html_for_string(&code, &SYNTAX_SET, syntax, theme) {
            Ok(highlighted) => {
                format!(r#"<div class="shiki-wrapper">{}</div>"#, highlighted)
            }
            Err(_) => caps[0].to_string(),
        }
    })
    .to_string()
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
    fn test_fenced_code_block_highlighted() {
        let md = "```rust\nfn main() {}\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("shiki-wrapper"));
    }

    #[test]
    fn test_mermaid_block_preserved() {
        let md = "```mermaid\ngraph LR\nA-->B\n```";
        let html = MarkdownParser::parse(md);
        assert!(html.contains("language-mermaid"));
        assert!(!html.contains("shiki-wrapper"));
    }

    #[test]
    fn test_math_passthrough() {
        let md = "Inline $x^2$ and block:\n\n$$\nE = mc^2\n$$";
        let html = MarkdownParser::parse(md);
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

    #[test]
    fn test_unknown_language_fallback() {
        let md = "```obscurelang\nsome code\n```";
        let html = MarkdownParser::parse(md);
        // Should still produce output (falls back to plain text highlighting)
        assert!(html.contains("shiki-wrapper") || html.contains("<pre"));
    }
}
```

- [ ] **Step 3: Run backend tests**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo test -p rustynotes 2>&1 | tail -20`

Expected: All tests pass, including the new `test_fenced_code_block_highlighted` and `test_mermaid_block_preserved` tests.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/markdown_parser.rs
git commit -m "feat: add syntect syntax highlighting to backend markdown parser"
```

---

### Task 5: Register parse_markdown in Tauri Handler

**Files:**
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add parse_markdown to the invoke handler**

In `src-tauri/src/lib.rs`, add `commands::markdown::parse_markdown` to the `generate_handler!` macro (after line 47):

Change:
```rust
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::fs::resolve_wikilink,
            commands::fs::search_files,
            commands::config::get_config,
            commands::config::save_config_cmd,
            commands::config::open_settings,
            commands::export::export_file,
            watch_folder,
        ])
```

To:
```rust
        .invoke_handler(tauri::generate_handler![
            commands::fs::read_file,
            commands::fs::write_file,
            commands::fs::list_directory,
            commands::fs::resolve_wikilink,
            commands::fs::search_files,
            commands::config::get_config,
            commands::config::save_config_cmd,
            commands::config::open_settings,
            commands::export::export_file,
            commands::markdown::parse_markdown,
            watch_folder,
        ])
```

- [ ] **Step 2: Verify backend compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -10`

Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/lib.rs
git commit -m "feat: register parse_markdown in Tauri command handler"
```

---

### Task 6: Add parse_markdown IPC to Frontend

**Files:**
- Modify: `crates/rustynotes-frontend/src/tauri_ipc.rs`

- [ ] **Step 1: Add parse_markdown IPC function**

In `crates/rustynotes-frontend/src/tauri_ipc.rs`, add after the `export_file` function (after line 166):

```rust
// ---------------------------------------------------------------------------
// Markdown command
// ---------------------------------------------------------------------------

pub async fn parse_markdown(content: &str) -> Result<String, String> {
    #[derive(Serialize)]
    struct Args<'a> {
        content: &'a str,
    }
    let val = tauri_invoke("parse_markdown", &Args { content }).await?;
    val.as_string()
        .ok_or_else(|| "parse_markdown: expected string result".to_string())
}
```

- [ ] **Step 2: Verify frontend compiles**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -10`

Expected: Build succeeds (with warnings about unused markdown module — that's fine, removed next task).

- [ ] **Step 3: Commit**

```bash
git add crates/rustynotes-frontend/src/tauri_ipc.rs
git commit -m "feat: add parse_markdown IPC binding to frontend"
```

---

### Task 7: Switch Frontend to Backend Markdown Rendering

**Files:**
- Modify: `crates/rustynotes-frontend/src/components/preview/preview.rs`
- Modify: `crates/rustynotes-frontend/src/components/editor/source_editor.rs`
- Modify: `crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs`
- Modify: `crates/rustynotes-frontend/src/components/preview/mod.rs`
- Delete: `crates/rustynotes-frontend/src/components/preview/markdown.rs`

- [ ] **Step 1: Rewrite preview.rs to use IPC**

Replace the contents of `crates/rustynotes-frontend/src/components/preview/preview.rs` with:

```rust
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::state::use_app_state;
use crate::tauri_ipc;

#[component]
pub fn Preview() -> impl IntoView {
    let state = use_app_state();

    Effect::new(move |_| {
        let content = state.active_file_content.get();
        let rendered_html = state.rendered_html;
        if content.is_empty() {
            rendered_html.set(String::new());
        } else {
            spawn_local(async move {
                match tauri_ipc::parse_markdown(&content).await {
                    Ok(html) => rendered_html.set(html),
                    Err(e) => {
                        web_sys::console::error_1(&format!("parse_markdown: {e}").into());
                    }
                }
            });
        }
    });

    view! {
        <div class="preview-container" inner_html=move || state.rendered_html.get() />
    }
}
```

- [ ] **Step 2: Update source_editor.rs to use IPC for preview sync**

In `crates/rustynotes-frontend/src/components/editor/source_editor.rs`, replace the onChange closure (lines 34-47) with one that uses IPC:

Replace:
```rust
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                // Skip if content matches what we already have (programmatic update).
                if new_content == content_signal.get_untracked() {
                    return;
                }
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Also update rendered HTML for preview sync
                let html = crate::components::preview::markdown::render_markdown(&new_content);
                rendered_html_signal.set(html);
            }) as Box<dyn Fn(String)>);
```

With:
```rust
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                // Skip if content matches what we already have (programmatic update).
                if new_content == content_signal.get_untracked() {
                    return;
                }
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Update rendered HTML for preview sync via backend IPC
                let content_for_render = new_content;
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(html) = crate::tauri_ipc::parse_markdown(&content_for_render).await {
                        rendered_html_signal.set(html);
                    }
                });
            }) as Box<dyn Fn(String)>);
```

- [ ] **Step 3: Update wysiwyg_editor.rs to use IPC for preview sync**

In `crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs`, replace the onChange closure (lines 30-39) with:

Replace:
```rust
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Also update rendered HTML for preview sync
                let html = crate::components::preview::markdown::render_markdown(&new_content);
                rendered_html_signal.set(html);
            }) as Box<dyn Fn(String)>);
```

With:
```rust
            let cb = Closure::wrap(Box::new(move |new_content: String| {
                content_signal.set(new_content.clone());
                if !suppress_dirty.get_untracked() {
                    dirty_signal.set(true);
                }

                // Update rendered HTML for preview sync via backend IPC
                let content_for_render = new_content;
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(html) = crate::tauri_ipc::parse_markdown(&content_for_render).await {
                        rendered_html_signal.set(html);
                    }
                });
            }) as Box<dyn Fn(String)>);
```

- [ ] **Step 4: Remove the frontend markdown module**

In `crates/rustynotes-frontend/src/components/preview/mod.rs`, change:

```rust
pub mod markdown;
pub mod preview;
```

To:

```rust
pub mod preview;
```

Then delete the file: `crates/rustynotes-frontend/src/components/preview/markdown.rs`

- [ ] **Step 5: Remove heavy deps from frontend Cargo.toml**

In `crates/rustynotes-frontend/Cargo.toml`, remove these lines:

```toml
comrak = "0.51"
syntect = { version = "5", default-features = false, features = ["default-fancy", "html"] }
regex-lite = "0.1"
html-escape = "0.2"
once_cell = "1"
```

- [ ] **Step 6: Verify full build**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes-frontend --target wasm32-unknown-unknown 2>&1 | tail -10`

Expected: Build succeeds with no errors.

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo build -p rustynotes 2>&1 | tail -10`

Expected: Backend build also succeeds.

- [ ] **Step 7: Commit**

```bash
git add crates/rustynotes-frontend/src/components/preview/preview.rs \
       crates/rustynotes-frontend/src/components/editor/source_editor.rs \
       crates/rustynotes-frontend/src/components/editor/wysiwyg_editor.rs \
       crates/rustynotes-frontend/src/components/preview/mod.rs \
       crates/rustynotes-frontend/Cargo.toml
git rm crates/rustynotes-frontend/src/components/preview/markdown.rs
git commit -m "refactor: move markdown rendering to backend, remove comrak/syntect from WASM"
```

---

### Task 8: Split JS Bridge Bundle with Code Splitting

**Files:**
- Modify: `js/bridge-src.js`
- Modify: `js/bundle-vendor.sh`
- Modify: `index.html`

- [ ] **Step 1: Convert KaTeX and Mermaid to lazy imports in bridge-src.js**

Replace the contents of `js/bridge-src.js` with:

```javascript
// js/bridge-src.js
import { EditorView, basicSetup } from 'codemirror';
import { EditorState } from '@codemirror/state';
import { markdown } from '@codemirror/lang-markdown';
import { oneDark } from '@codemirror/theme-one-dark';
import { keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { searchKeymap } from '@codemirror/search';
import { Editor } from '@tiptap/core';
import StarterKit from '@tiptap/starter-kit';
import TaskList from '@tiptap/extension-task-list';
import TaskItem from '@tiptap/extension-task-item';
import { Markdown } from '@tiptap/markdown';

let katexModule = null;
let mermaidModule = null;

window.RustyNotesBridge = {
  mountCodeMirror(element, content, options, onChange) {
    const extensions = [
      basicSetup,
      markdown(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap]),
      history(),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) onChange(update.state.doc.toString());
      }),
    ];
    if (options.theme === 'dark') extensions.push(oneDark);
    const state = EditorState.create({ doc: content, extensions });
    return { view: new EditorView({ state, parent: element }) };
  },

  updateCodeMirror(handle, content) {
    const cur = handle.view.state.doc.toString();
    if (cur !== content) {
      handle.view.dispatch({
        changes: { from: 0, to: cur.length, insert: content },
      });
    }
  },

  focusCodeMirror(handle) {
    handle.view.focus();
  },

  destroyCodeMirror(handle) {
    handle.view.destroy();
  },

  mountTipTap(element, content, options, onChange) {
    const exts = [
      StarterKit.configure({ codeBlock: false }),
      Markdown,
    ];
    if (options.taskLists !== false) {
      exts.push(TaskList, TaskItem.configure({ nested: true }));
    }
    const editor = new Editor({
      element,
      extensions: exts,
      content,
      contentType: 'markdown',
      onUpdate: ({ editor }) => onChange(editor.getMarkdown()),
    });
    return { editor };
  },

  updateTipTap(handle, content) {
    if (handle.editor.getMarkdown() !== content) {
      handle.editor.commands.setContent(content, { contentType: 'markdown' });
    }
  },

  getTipTapMarkdown(handle) {
    return handle.editor.getMarkdown();
  },

  focusTipTap(handle) {
    handle.editor.commands.focus();
  },

  destroyTipTap(handle) {
    handle.editor.destroy();
  },

  async renderKatex(element, latex, displayMode) {
    if (!katexModule) katexModule = await import('katex');
    element.innerHTML = katexModule.default.renderToString(latex, {
      throwOnError: false,
      displayMode,
    });
  },

  async renderMermaid(element, code, theme) {
    if (!mermaidModule) {
      mermaidModule = await import('mermaid');
      mermaidModule.default.initialize({
        startOnLoad: false,
        theme: theme === 'dark' ? 'dark' : 'default',
        securityLevel: 'loose',
      });
    }
    const id = `mermaid-${Date.now()}-${Math.random().toString(36).slice(2)}`;
    const { svg } = await mermaidModule.default.render(id, code);
    element.innerHTML = svg;
  },
};
```

Note: The bridge-src.js content is nearly identical — KaTeX and Mermaid already use lazy `import()`. The key change is in the esbuild configuration (next step) which enables code splitting so those dynamic imports actually produce separate chunks.

- [ ] **Step 2: Update bundle-vendor.sh for code splitting**

Replace the contents of `js/bundle-vendor.sh` with:

```bash
#!/bin/bash
set -e
cd "$(dirname "$0")/.."

pnpm install

npx esbuild js/bridge-src.js \
  --bundle --splitting --format=esm --outdir=js/dist/

# Vendor KaTeX CSS locally
cp node_modules/katex/dist/katex.min.css styles/katex.min.css

echo "Done: js/dist/ (chunked) + styles/katex.min.css"
```

- [ ] **Step 3: Run the new bundle script**

Run: `cd "/Users/dennis/programming projects/rustynotes" && bash js/bundle-vendor.sh 2>&1 | tail -10`

Expected: Creates `js/dist/` directory with a main chunk and separate chunks for KaTeX and Mermaid.

Run: `ls -lh js/dist/`

Verify: Multiple .js files exist. The main file (bridge-src.js) should be significantly smaller than the old 8.2MB bundle.

- [ ] **Step 4: Update index.html for chunked JS**

Replace the Trunk directives in `index.html`:

Change:
```html
    <link data-trunk rel="copy-file" href="js/bridge.bundle.js" />
    <script type="module" src="bridge.bundle.js"></script>
```

To:
```html
    <link data-trunk rel="copy-dir" href="js/dist" />
    <script type="module" src="dist/bridge-src.js"></script>
```

- [ ] **Step 5: Verify Trunk build works with chunked JS**

Run: `cd "/Users/dennis/programming projects/rustynotes" && trunk build 2>&1 | tail -10`

Expected: Build succeeds. Check that dist/ contains the js chunks:

Run: `ls dist/dist/`

Expected: The chunked JS files are copied into the Trunk output.

- [ ] **Step 6: Commit**

```bash
git add js/bridge-src.js js/bundle-vendor.sh js/dist/ index.html
git rm js/bridge.bundle.js
git commit -m "perf: split JS bridge bundle with lazy KaTeX/Mermaid loading"
```

---

### Task 9: Measure Final Sizes and Verify

**Files:**
- Modify: `docs/superpowers/plans/bloat-reduction-sizes.md`

- [ ] **Step 1: Rebuild everything fresh**

Run: `cd "/Users/dennis/programming projects/rustynotes" && trunk build --release 2>&1 | tail -10`

Expected: Clean release build succeeds.

- [ ] **Step 2: Measure final WASM size**

Run: `ls -lh dist/rustynotes-frontend-*_bg.wasm`

Record the size. Expected: ~3.5-4.5MB (down from ~6.2MB).

- [ ] **Step 3: Measure final JS bundle sizes**

Run: `ls -lh js/dist/`

Record the main chunk size. Expected: ~2-3MB for the main chunk (down from 8.2MB monolithic).

- [ ] **Step 4: Update size tracking doc**

Fill in the "After cleanup" section in `docs/superpowers/plans/bloat-reduction-sizes.md` with actual measured values.

- [ ] **Step 5: Run all backend tests**

Run: `cd "/Users/dennis/programming projects/rustynotes" && cargo test -p rustynotes 2>&1 | tail -20`

Expected: All tests pass.

- [ ] **Step 6: Run Tauri dev to smoke-test the app**

Run: `cd "/Users/dennis/programming projects/rustynotes" && pnpm tauri dev 2>&1 | head -30`

Manual verification checklist:
- App launches without console errors
- Source editor (CodeMirror) works
- WYSIWYG editor (TipTap) works
- Preview mode renders markdown with syntax-highlighted code blocks
- Split mode shows live preview
- Math rendering works (type `$x^2$` in source mode, check preview)
- Mermaid rendering works (type a mermaid code block, check preview)

- [ ] **Step 7: Final commit**

```bash
git add docs/superpowers/plans/bloat-reduction-sizes.md
git commit -m "docs: record final bloat reduction measurements"
```
