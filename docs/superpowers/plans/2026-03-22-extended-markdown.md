# Extended Markdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enhance the preview and WYSIWYG rendering with KaTeX math, Mermaid diagrams, Shiki syntax highlighting, styled admonitions, and wiki-link navigation.

**Architecture:** comrak already outputs HTML with math, alert, wiki-link, and fenced-code elements. A frontend post-processor enhances this HTML after it's injected into the preview DOM — running KaTeX on math elements, Mermaid on diagram blocks, and Shiki on code blocks. Wiki-link resolution happens in Rust (index of files in the opened folder).

**Tech Stack:** KaTeX, Mermaid.js, Shiki, comrak (already configured)

**Spec:** `docs/superpowers/specs/2026-03-22-rustynotes-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `package.json` | Modify | Add katex, mermaid, shiki |
| `src/lib/postprocess.ts` | Create | DOM post-processor: math, diagrams, code highlighting |
| `src/components/preview/Preview.tsx` | Modify | Run post-processor after HTML inject |
| `src/styles/base.css` | Modify | Admonition/alert styles, wiki-link styles |
| `src-tauri/src/commands/fs.rs` | Modify | Add search_files command |
| `src-tauri/src/fs_ops.rs` | Modify | Add search function |

---

## Task 1: Add Frontend Dependencies

**Files:** `package.json`

- [ ] **Step 1: Install packages**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm add katex mermaid shiki
```

- [ ] **Step 2: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "feat: add KaTeX, Mermaid, and Shiki dependencies"
```

---

## Task 2: Post-Processor Module

**Files:** Create `src/lib/postprocess.ts`

This module takes a DOM element containing comrak HTML output and enhances it:

- [ ] **Step 1: Create the post-processor**

```typescript
import katex from "katex";
import "katex/dist/katex.min.css";
import mermaid from "mermaid";
import { codeToHtml } from "shiki";

let mermaidInitialized = false;

function initMermaid(isDark: boolean) {
  mermaid.initialize({
    startOnLoad: false,
    theme: isDark ? "dark" : "default",
    securityLevel: "loose",
  });
  mermaidInitialized = true;
}

export async function postProcessPreview(container: HTMLElement): Promise<void> {
  const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;

  await Promise.all([
    renderMath(container),
    renderDiagrams(container, isDark),
    highlightCode(container, isDark),
  ]);
}

function renderMath(container: HTMLElement): void {
  // comrak with math_dollars wraps inline math in <code class="math-inline">
  // and block math in <code class="math-display">
  // It may also use <span> or <div> with data-math attributes depending on version

  // Handle inline math
  container.querySelectorAll('code.math-inline, [data-math-style="inline"]').forEach((el) => {
    try {
      const tex = el.textContent || "";
      const rendered = katex.renderToString(tex, { throwOnError: false, displayMode: false });
      const span = document.createElement("span");
      span.innerHTML = rendered;
      span.className = "math-rendered";
      el.replaceWith(span);
    } catch (e) {
      // Leave raw on error
    }
  });

  // Handle block/display math
  container.querySelectorAll('code.math-display, [data-math-style="display"]').forEach((el) => {
    try {
      const tex = el.textContent || "";
      const rendered = katex.renderToString(tex, { throwOnError: false, displayMode: true });
      const div = document.createElement("div");
      div.innerHTML = rendered;
      div.className = "math-rendered math-block";
      // Replace the parent <pre> if it exists, otherwise replace the element
      const parent = el.closest("pre") || el;
      parent.replaceWith(div);
    } catch (e) {
      // Leave raw on error
    }
  });
}

async function renderDiagrams(container: HTMLElement, isDark: boolean): Promise<void> {
  if (!mermaidInitialized) {
    initMermaid(isDark);
  }

  const mermaidBlocks = container.querySelectorAll('code.language-mermaid');

  for (let i = 0; i < mermaidBlocks.length; i++) {
    const el = mermaidBlocks[i];
    const code = el.textContent || "";
    const pre = el.closest("pre");
    if (!pre) continue;

    try {
      const id = `mermaid-${Date.now()}-${i}`;
      const { svg } = await mermaid.render(id, code);
      const div = document.createElement("div");
      div.className = "mermaid-diagram";
      div.innerHTML = svg;
      pre.replaceWith(div);
    } catch (e) {
      // Leave as code block on error
    }
  }
}

async function highlightCode(container: HTMLElement, isDark: boolean): Promise<void> {
  const codeBlocks = container.querySelectorAll("pre > code[class*='language-']");

  for (const el of codeBlocks) {
    const classAttr = el.className;
    const langMatch = classAttr.match(/language-(\w+)/);
    if (!langMatch) continue;

    const lang = langMatch[1];

    // Skip mermaid blocks (handled separately)
    if (lang === "mermaid") continue;

    const code = el.textContent || "";
    const pre = el.closest("pre");
    if (!pre) continue;

    try {
      const html = await codeToHtml(code, {
        lang,
        theme: isDark ? "github-dark" : "github-light",
      });
      const wrapper = document.createElement("div");
      wrapper.className = "shiki-wrapper";
      wrapper.innerHTML = html;
      pre.replaceWith(wrapper);
    } catch (e) {
      // Leave unhighlighted on error (unsupported language, etc.)
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/postprocess.ts
git commit -m "feat: add post-processor for math, diagrams, and syntax highlighting"
```

---

## Task 3: Wire Post-Processor into Preview

**Files:** Modify `src/components/preview/Preview.tsx`

- [ ] **Step 1: Update Preview to run post-processor**

The Preview component needs to call `postProcessPreview` after the HTML is injected. Use a ref and createEffect:

```tsx
import { Component, createEffect, onMount } from "solid-js";
import { appState } from "../../lib/state";
import { postProcessPreview } from "../../lib/postprocess";

const Preview: Component = () => {
  let containerRef: HTMLDivElement | undefined;
  const { renderedHtml } = appState;

  createEffect(async () => {
    const html = renderedHtml();
    if (containerRef) {
      containerRef.innerHTML = html;
      await postProcessPreview(containerRef);
    }
  });

  return <div ref={containerRef} class="preview-container" />;
};

export default Preview;
```

Note: This replaces the previous `innerHTML={renderedHtml()}` approach with a ref-based approach so we can post-process the DOM after injection.

- [ ] **Step 2: Commit**

```bash
git add src/components/preview/Preview.tsx
git commit -m "feat: wire post-processor into preview for math, diagrams, code highlighting"
```

---

## Task 4: Admonition and Extended Styles

**Files:** Modify `src/styles/base.css`

- [ ] **Step 1: Add styles for admonitions, math, diagrams, wiki-links**

Append to base.css:

```css
/* Admonitions / GitHub-style alerts */
.preview-container .markdown-alert {
  padding: 12px 16px;
  margin: 1em 0;
  border-left: 4px solid;
  border-radius: 4px;
  background: var(--bg-secondary);
}

.preview-container .markdown-alert-note { border-left-color: var(--accent); }
.preview-container .markdown-alert-tip { border-left-color: #2da44e; }
.preview-container .markdown-alert-important { border-left-color: #8957e5; }
.preview-container .markdown-alert-warning { border-left-color: #d29922; }
.preview-container .markdown-alert-caution { border-left-color: #e74c3c; }

.preview-container .markdown-alert .markdown-alert-title {
  font-weight: 600;
  margin-bottom: 4px;
  text-transform: capitalize;
}

/* Math rendering */
.math-rendered { display: inline; }
.math-block {
  display: block;
  text-align: center;
  margin: 1em 0;
  overflow-x: auto;
}

/* Mermaid diagrams */
.mermaid-diagram {
  display: flex;
  justify-content: center;
  margin: 1em 0;
  overflow-x: auto;
}

.mermaid-diagram svg {
  max-width: 100%;
  height: auto;
}

/* Shiki code blocks */
.shiki-wrapper pre {
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
  margin: 1em 0;
  border: 1px solid var(--border);
}

.shiki-wrapper code {
  background: none;
  padding: 0;
  font-family: var(--font-mono);
  font-size: 0.9em;
}

/* Wiki-links */
.preview-container a.wikilink {
  color: var(--accent);
  text-decoration: none;
  border-bottom: 1px dashed var(--accent);
}

.preview-container a.wikilink:hover {
  text-decoration: none;
  border-bottom-style: solid;
}

/* Footnotes */
.preview-container .footnotes {
  margin-top: 2em;
  padding-top: 1em;
  border-top: 1px solid var(--border);
  font-size: 0.9em;
}

.preview-container .footnote-ref a,
.preview-container .footnote-backref {
  color: var(--accent);
  text-decoration: none;
}

/* Definition lists */
.preview-container dt {
  font-weight: 600;
  margin-top: 1em;
}

.preview-container dd {
  margin-left: 2em;
  margin-bottom: 0.5em;
}
```

- [ ] **Step 2: Commit**

```bash
git add src/styles/base.css
git commit -m "feat: add styles for admonitions, math, diagrams, wiki-links, footnotes"
```

---

## Task 5: Wiki-Link Navigation

**Files:**
- Modify: `src-tauri/src/fs_ops.rs` (add file search)
- Modify: `src-tauri/src/commands/fs.rs` (add resolve_wikilink command)
- Modify: `src-tauri/src/lib.rs` (register command)
- Modify: `src/lib/ipc.ts` (add wikilink resolver)
- Modify: `src/lib/postprocess.ts` (add wiki-link click handling)

- [ ] **Step 1: Add file search to fs_ops**

Add to `src-tauri/src/fs_ops.rs`:

```rust
/// Find a markdown file by name (without .md extension) in a directory tree.
/// Returns the full path if found.
pub fn find_file_by_name(root: &Path, name: &str) -> Option<PathBuf> {
    let target = if name.ends_with(".md") {
        name.to_string()
    } else {
        format!("{}.md", name)
    };

    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            if let Some(file_name) = entry.path().file_name() {
                if file_name.to_string_lossy().eq_ignore_ascii_case(&target) {
                    return Some(entry.path().to_path_buf());
                }
            }
        }
    }
    None
}
```

Add a test:

```rust
#[test]
fn test_find_file_by_name() {
    let dir = setup_test_dir();
    let result = find_file_by_name(dir.path(), "hello");
    assert!(result.is_some());
    assert!(result.unwrap().ends_with("hello.md"));
}

#[test]
fn test_find_file_by_name_nested() {
    let dir = setup_test_dir();
    let result = find_file_by_name(dir.path(), "nested");
    assert!(result.is_some());
}

#[test]
fn test_find_file_by_name_not_found() {
    let dir = setup_test_dir();
    let result = find_file_by_name(dir.path(), "nonexistent");
    assert!(result.is_none());
}
```

- [ ] **Step 2: Add resolve_wikilink command**

Add to `src-tauri/src/commands/fs.rs`:

```rust
#[tauri::command]
pub fn resolve_wikilink(root: String, name: String) -> Option<String> {
    fs_ops::find_file_by_name(std::path::Path::new(&root), &name)
        .map(|p| p.display().to_string())
}
```

Register in `lib.rs` generate_handler.

- [ ] **Step 3: Add frontend IPC and click handling**

Add to `src/lib/ipc.ts`:

```typescript
export async function resolveWikilink(root: string, name: string): Promise<string | null> {
  return invoke<string | null>("resolve_wikilink", { root, name });
}
```

Add wiki-link click handling to postprocess.ts — when a wiki-link is clicked, resolve it via IPC and open the file.

- [ ] **Step 4: Run tests**

```bash
cd "/Users/dennis/programming projects/rustynotes/src-tauri" && cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/fs_ops.rs src-tauri/src/commands/fs.rs src-tauri/src/lib.rs src/lib/ipc.ts src/lib/postprocess.ts
git commit -m "feat: add wiki-link resolution and click-to-navigate"
```
