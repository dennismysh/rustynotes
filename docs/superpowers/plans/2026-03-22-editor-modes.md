# Editor Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add WYSIWYG editing (TipTap), split-pane mode (source + preview), and a 4-way editor mode switcher.

**Architecture:** TipTap v3 mounted framework-agnostically via Solid.js onMount/onCleanup. The official @tiptap/markdown extension handles markdown<->TipTap conversion. Split-pane uses CSS grid with a draggable divider.

**Tech Stack:** @tiptap/core v3, @tiptap/starter-kit, @tiptap/markdown, @tiptap/extension-task-list, @tiptap/extension-task-item

**Spec:** `docs/superpowers/specs/2026-03-22-rustynotes-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `package.json` | Modify | Add TipTap dependencies |
| `src/lib/state.ts` | Modify | Expand EditorMode to 4 values |
| `src/components/Toolbar.tsx` | Modify | 4-segment mode switcher + keyboard shortcuts |
| `src/components/editor/WysiwygEditor.tsx` | Create | TipTap WYSIWYG wrapper |
| `src/components/editor/SplitPane.tsx` | Create | Source + preview side-by-side |
| `src/App.tsx` | Modify | Render all 4 editor modes |
| `src/styles/base.css` | Modify | Add WYSIWYG and split-pane styles |

---

## Task 1: Add TipTap Dependencies

**Files:**
- Modify: `package.json`

- [ ] **Step 1: Install TipTap packages**

```bash
cd "/Users/dennis/programming projects/rustynotes"
pnpm add @tiptap/core @tiptap/pm @tiptap/starter-kit @tiptap/markdown @tiptap/extension-task-list @tiptap/extension-task-item
```

- [ ] **Step 2: Verify TypeScript still compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml
git commit -m "feat: add TipTap v3 dependencies for WYSIWYG editing"
```

---

## Task 2: Expand Editor Mode State

**Files:**
- Modify: `src/lib/state.ts`

- [ ] **Step 1: Update EditorMode type**

Change the `EditorMode` type from `"source" | "preview"` to:

```typescript
export type EditorMode = "source" | "wysiwyg" | "split" | "preview";
```

No other changes needed — the signals already support this.

- [ ] **Step 2: Commit**

```bash
git add src/lib/state.ts
git commit -m "feat: expand EditorMode to support source, wysiwyg, split, preview"
```

---

## Task 3: WYSIWYG Editor Component

**Files:**
- Create: `src/components/editor/WysiwygEditor.tsx`

- [ ] **Step 1: Create the TipTap wrapper**

```tsx
import { Component, onMount, onCleanup, createEffect } from "solid-js";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { Markdown } from "@tiptap/markdown";
import TaskList from "@tiptap/extension-task-list";
import TaskItem from "@tiptap/extension-task-item";
import { appState } from "../../lib/state";
import { writeFile, parseMarkdown } from "../../lib/ipc";

const WysiwygEditor: Component = () => {
  let editorElement: HTMLDivElement | undefined;
  let editor: Editor | undefined;
  let skipUpdate = false;

  const {
    activeFileContent,
    setActiveFileContent,
    activeFilePath,
    setIsDirty,
    setRenderedHtml,
  } = appState;

  onMount(() => {
    if (!editorElement) return;

    editor = new Editor({
      element: editorElement,
      extensions: [
        StarterKit,
        Markdown.configure({
          markedOptions: { gfm: true },
        }),
        TaskList,
        TaskItem.configure({ nested: true }),
      ],
      content: activeFileContent(),
      contentType: "markdown",
      onUpdate({ editor: ed }) {
        if (skipUpdate) return;
        const md = ed.getMarkdown();
        setActiveFileContent(md);
        setIsDirty(true);
        parseMarkdown(md).then((html) => setRenderedHtml(html));
      },
    });
  });

  // Sync external content changes (e.g., file switch)
  createEffect(() => {
    const content = activeFileContent();
    if (editor && !editor.isDestroyed) {
      const currentMd = editor.getMarkdown();
      if (currentMd !== content) {
        skipUpdate = true;
        editor.commands.setContent(content, { contentType: "markdown" });
        skipUpdate = false;
      }
    }
  });

  // Cmd+S save
  const handleKeyDown = async (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      const path = activeFilePath();
      if (path && editor) {
        const md = editor.getMarkdown();
        await writeFile(path, md);
        setIsDirty(false);
      }
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });

  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
    editor?.destroy();
  });

  return <div ref={editorElement} class="wysiwyg-editor" />;
};

export default WysiwygEditor;
```

- [ ] **Step 2: Commit**

```bash
git add src/components/editor/WysiwygEditor.tsx
git commit -m "feat: add TipTap WYSIWYG editor component with markdown support"
```

---

## Task 4: Split-Pane Component

**Files:**
- Create: `src/components/editor/SplitPane.tsx`

- [ ] **Step 1: Create split-pane layout**

```tsx
import { Component } from "solid-js";
import SourceEditor from "./SourceEditor";
import Preview from "../preview/Preview";

const SplitPane: Component = () => {
  return (
    <div class="split-pane">
      <div class="split-pane-left">
        <SourceEditor />
      </div>
      <div class="split-pane-divider" />
      <div class="split-pane-right">
        <Preview />
      </div>
    </div>
  );
};

export default SplitPane;
```

- [ ] **Step 2: Commit**

```bash
git add src/components/editor/SplitPane.tsx
git commit -m "feat: add split-pane component for side-by-side source and preview"
```

---

## Task 5: Update Toolbar, App Shell, and Styles

**Files:**
- Modify: `src/components/Toolbar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/base.css`

- [ ] **Step 1: Update Toolbar with 4-segment switcher**

Replace the mode-switcher section in Toolbar.tsx to show all 4 modes:

```tsx
<div class="mode-switcher">
  <button classList={{ active: editorMode() === "source" }} onClick={() => setMode("source")}>Source</button>
  <button classList={{ active: editorMode() === "wysiwyg" }} onClick={() => setMode("wysiwyg")}>WYSIWYG</button>
  <button classList={{ active: editorMode() === "split" }} onClick={() => setMode("split")}>Split</button>
  <button classList={{ active: editorMode() === "preview" }} onClick={() => setMode("preview")}>Preview</button>
</div>
```

- [ ] **Step 2: Update App.tsx to render all 4 modes**

Import WysiwygEditor and SplitPane, then update the content area:

```tsx
import WysiwygEditor from "./components/editor/WysiwygEditor";
import SplitPane from "./components/editor/SplitPane";

// In the content area Show block:
<Show when={editorMode() === "source"}><SourceEditor /></Show>
<Show when={editorMode() === "wysiwyg"}><WysiwygEditor /></Show>
<Show when={editorMode() === "split"}><SplitPane /></Show>
<Show when={editorMode() === "preview"}><Preview /></Show>
```

- [ ] **Step 3: Add WYSIWYG and split-pane styles to base.css**

Append to `src/styles/base.css`:

```css
/* WYSIWYG editor styles */
.wysiwyg-editor {
  height: 100%;
  overflow-y: auto;
  padding: 24px 40px;
  max-width: 800px;
  margin: 0 auto;
}

.wysiwyg-editor .tiptap {
  outline: none;
  min-height: 100%;
  font-family: var(--font-body);
  font-size: var(--font-size);
  line-height: var(--line-height);
  color: var(--text-primary);
}

.wysiwyg-editor .tiptap h1 { font-size: 2em; margin: 0.67em 0; }
.wysiwyg-editor .tiptap h2 { font-size: 1.5em; margin: 0.75em 0; }
.wysiwyg-editor .tiptap h3 { font-size: 1.17em; margin: 0.83em 0; }
.wysiwyg-editor .tiptap p { margin: 1em 0; }
.wysiwyg-editor .tiptap code {
  background: var(--bg-tertiary);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 0.9em;
}
.wysiwyg-editor .tiptap pre {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px;
  overflow-x: auto;
  margin: 1em 0;
}
.wysiwyg-editor .tiptap pre code { background: none; padding: 0; }
.wysiwyg-editor .tiptap blockquote {
  border-left: 3px solid var(--accent);
  padding-left: 16px;
  color: var(--text-secondary);
  margin: 1em 0;
}
.wysiwyg-editor .tiptap a { color: var(--accent); text-decoration: none; }
.wysiwyg-editor .tiptap a:hover { text-decoration: underline; }
.wysiwyg-editor .tiptap ul[data-type="taskList"] {
  list-style: none;
  padding-left: 0;
}
.wysiwyg-editor .tiptap ul[data-type="taskList"] li {
  display: flex;
  align-items: flex-start;
  gap: 8px;
}
.wysiwyg-editor .tiptap ul[data-type="taskList"] li input[type="checkbox"] {
  margin-top: 4px;
}
.wysiwyg-editor .tiptap img { max-width: 100%; }

/* Split pane styles */
.split-pane {
  display: grid;
  grid-template-columns: 1fr 1px 1fr;
  height: 100%;
  overflow: hidden;
}

.split-pane-left {
  overflow: hidden;
}

.split-pane-divider {
  background: var(--border);
  cursor: col-resize;
  width: 1px;
}

.split-pane-right {
  overflow-y: auto;
}
```

- [ ] **Step 4: Verify TypeScript compiles**

```bash
npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git add src/components/Toolbar.tsx src/App.tsx src/styles/base.css
git commit -m "feat: wire up 4-way editor mode switching with WYSIWYG and split pane"
```

---

## Task 6: Keyboard Shortcuts

**Files:**
- Modify: `src/App.tsx` (add global keyboard handler)

- [ ] **Step 1: Add keyboard shortcut handler**

Add a global keydown listener in App.tsx that handles:
- `Cmd+E` — cycle through editor modes (source -> wysiwyg -> split -> preview -> source)
- `Cmd+P` — jump to preview mode

```tsx
import { onMount, onCleanup } from "solid-js";

// Inside App component, before the return:
const modes: EditorMode[] = ["source", "wysiwyg", "split", "preview"];

const handleKeyDown = (e: KeyboardEvent) => {
  if ((e.metaKey || e.ctrlKey) && e.key === "e") {
    e.preventDefault();
    const current = editorMode();
    const idx = modes.indexOf(current);
    setEditorMode(modes[(idx + 1) % modes.length]);
  }
  if ((e.metaKey || e.ctrlKey) && e.key === "p") {
    e.preventDefault();
    setEditorMode("preview");
  }
};

onMount(() => document.addEventListener("keydown", handleKeyDown));
onCleanup(() => document.removeEventListener("keydown", handleKeyDown));
```

- [ ] **Step 2: Commit**

```bash
git add src/App.tsx
git commit -m "feat: add Cmd+E and Cmd+P keyboard shortcuts for editor mode switching"
```
