# Navigation Modes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Miller columns and breadcrumb navigation modes, plus a 3-way nav mode switcher with Cmd+1/2/3 shortcuts.

**Architecture:** Three navigation components share the same file tree data from state. A NavMode signal controls which one renders. The app shell CSS grid adjusts sidebar width based on the active mode (Miller columns need more width, breadcrumb needs none).

**Tech Stack:** Solid.js components, CSS grid

**Spec:** `docs/superpowers/specs/2026-03-22-rustynotes-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/lib/state.ts` | Modify | Add NavMode type and signal |
| `src/components/navigation/MillerColumns.tsx` | Create | 2-3 scrollable drill-down columns |
| `src/components/navigation/Breadcrumb.tsx` | Create | Breadcrumb bar with dropdown children |
| `src/components/Toolbar.tsx` | Modify | Add 3-icon nav mode switcher |
| `src/App.tsx` | Modify | Render active nav mode, adjust grid, add Cmd+1/2/3 |
| `src/styles/base.css` | Modify | Miller columns and breadcrumb styles |

---

## Task 1: NavMode State

**Files:** Modify `src/lib/state.ts`

- [ ] **Step 1: Add NavMode type and signal**

Add to state.ts:
```typescript
export type NavMode = "sidebar" | "miller" | "breadcrumb";
```

Add signal:
```typescript
const [navMode, setNavMode] = createSignal<NavMode>("sidebar");
```

Export from createAppState.

- [ ] **Step 2: Commit**

```bash
git commit -m "feat: add NavMode type and signal to app state"
```

---

## Task 2: Miller Columns Component

**Files:** Create `src/components/navigation/MillerColumns.tsx`

- [ ] **Step 1: Create Miller columns**

Miller columns show 2-3 scrollable columns. Clicking a folder in column N populates column N+1 with its children. Clicking a file opens it.

```tsx
import { Component, For, Show, createSignal } from "solid-js";
import { appState } from "../../lib/state";
import { readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const MillerColumns: Component = () => {
  const { fileTree, currentFolder, activeFilePath, setActiveFilePath, setActiveFileContent, setRenderedHtml, setIsDirty } = appState;

  // columns[0] = root entries, columns[1] = children of selected in col 0, etc.
  const [columns, setColumns] = createSignal<FileEntry[][]>([]);
  const [selectedPaths, setSelectedPaths] = createSignal<(string | null)[]>([]);

  // Initialize columns from file tree
  const initColumns = () => {
    setColumns([fileTree()]);
    setSelectedPaths([null]);
  };

  // Watch for file tree changes
  import { createEffect } from "solid-js";
  createEffect(() => {
    if (fileTree().length > 0) initColumns();
  });

  const handleClick = async (entry: FileEntry, colIndex: number) => {
    if (entry.is_dir && entry.children) {
      // Update columns: keep columns up to colIndex, add new column
      const newColumns = columns().slice(0, colIndex + 1);
      newColumns.push(entry.children);
      setColumns(newColumns);

      const newSelected = selectedPaths().slice(0, colIndex);
      newSelected[colIndex] = entry.path;
      setSelectedPaths(newSelected);
    } else {
      // Open file
      setActiveFilePath(entry.path);
      const content = await readFile(entry.path);
      setActiveFileContent(content);
      setIsDirty(false);
      const html = await parseMarkdown(content);
      setRenderedHtml(html);

      const newSelected = selectedPaths().slice(0, colIndex);
      newSelected[colIndex] = entry.path;
      setSelectedPaths(newSelected);
    }
  };

  return (
    <div class="miller-columns">
      <Show when={currentFolder()} fallback={
        <div style="padding: 16px; color: var(--text-muted); font-size: 13px;">No folder open</div>
      }>
        <For each={columns()}>
          {(entries, colIndex) => (
            <div class="miller-column">
              <For each={entries}>
                {(entry) => (
                  <div
                    class="miller-item"
                    classList={{
                      active: selectedPaths()[colIndex()] === entry.path,
                      "is-dir": entry.is_dir,
                    }}
                    onClick={() => handleClick(entry, colIndex())}
                  >
                    <span class="name">{entry.name}</span>
                    <Show when={entry.is_dir}>
                      <span class="chevron">&#9654;</span>
                    </Show>
                  </div>
                )}
              </For>
            </div>
          )}
        </For>
      </Show>
    </div>
  );
};

export default MillerColumns;
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat: add Miller columns navigation component"
```

---

## Task 3: Breadcrumb Component

**Files:** Create `src/components/navigation/Breadcrumb.tsx`

- [ ] **Step 1: Create breadcrumb bar**

Shows the current file's path as clickable segments. Clicking a segment shows a dropdown of its children. No persistent sidebar.

```tsx
import { Component, For, Show, createSignal, createMemo } from "solid-js";
import { appState } from "../../lib/state";
import { listDirectory, readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const Breadcrumb: Component = () => {
  const { currentFolder, activeFilePath, setActiveFilePath, setActiveFileContent, setRenderedHtml, setIsDirty, fileTree } = appState;

  const [dropdownEntries, setDropdownEntries] = createSignal<FileEntry[]>([]);
  const [dropdownIndex, setDropdownIndex] = createSignal<number | null>(null);

  const pathSegments = createMemo(() => {
    const folder = currentFolder();
    const file = activeFilePath();
    if (!folder || !file) return [];

    const relative = file.startsWith(folder) ? file.slice(folder.length) : file;
    const parts = relative.split("/").filter(Boolean);

    return parts.map((part, i) => ({
      name: part,
      fullPath: folder + "/" + parts.slice(0, i + 1).join("/"),
      isLast: i === parts.length - 1,
    }));
  });

  const handleSegmentClick = async (index: number, fullPath: string) => {
    // Show dropdown with siblings
    const parentPath = fullPath.split("/").slice(0, -1).join("/");
    try {
      const entries = await listDirectory(parentPath);
      setDropdownEntries(entries);
      setDropdownIndex(index);
    } catch {
      setDropdownIndex(null);
    }
  };

  const handleDropdownSelect = async (entry: FileEntry) => {
    setDropdownIndex(null);
    if (!entry.is_dir) {
      setActiveFilePath(entry.path);
      const content = await readFile(entry.path);
      setActiveFileContent(content);
      setIsDirty(false);
      const html = await parseMarkdown(content);
      setRenderedHtml(html);
    }
  };

  const closeDropdown = () => setDropdownIndex(null);

  return (
    <div class="breadcrumb-bar">
      <Show when={currentFolder()}>
        <span class="breadcrumb-root" onClick={() => handleSegmentClick(-1, currentFolder()!)}>
          {currentFolder()!.split("/").pop()}
        </span>
        <For each={pathSegments()}>
          {(segment, i) => (
            <>
              <span class="breadcrumb-separator">/</span>
              <span
                class="breadcrumb-segment"
                classList={{ active: segment.isLast }}
                onClick={() => handleSegmentClick(i(), segment.fullPath)}
              >
                {segment.name}
              </span>
            </>
          )}
        </For>
      </Show>
      <Show when={dropdownIndex() !== null}>
        <div class="breadcrumb-dropdown-overlay" onClick={closeDropdown} />
        <div class="breadcrumb-dropdown">
          <For each={dropdownEntries()}>
            {(entry) => (
              <div class="breadcrumb-dropdown-item" onClick={() => handleDropdownSelect(entry)}>
                <span class="icon">{entry.is_dir ? "\u25B6" : "\uD83D\uDCC4"}</span>
                {entry.name}
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default Breadcrumb;
```

- [ ] **Step 2: Commit**

```bash
git commit -m "feat: add breadcrumb navigation component with dropdown"
```

---

## Task 4: Wire Navigation Modes into App Shell

**Files:**
- Modify: `src/components/Toolbar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/styles/base.css`

- [ ] **Step 1: Add nav mode switcher to Toolbar**

Add 3 icon buttons before the spacer (Sidebar, Miller, Breadcrumb) that toggle navMode. Import navMode/setNavMode from appState.

- [ ] **Step 2: Update App.tsx**

Import MillerColumns and Breadcrumb. Conditionally render the active nav component based on navMode().

For breadcrumb mode: render the Breadcrumb component above the content area (not as a sidebar), and adjust the grid to have no sidebar column.

For Miller columns: render in the sidebar area but with a wider default width.

Add keyboard shortcuts: Cmd+1 = sidebar, Cmd+2 = miller, Cmd+3 = breadcrumb.

- [ ] **Step 3: Add CSS for Miller columns and breadcrumb**

Append to base.css:

```css
/* Miller columns */
.miller-columns {
  display: flex;
  height: 100%;
  overflow-x: auto;
}

.miller-column {
  min-width: 200px;
  max-width: 250px;
  border-right: 1px solid var(--border);
  overflow-y: auto;
  flex-shrink: 0;
}

.miller-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
  user-select: none;
}

.miller-item:hover { background: var(--bg-tertiary); }
.miller-item.active { background: var(--accent); color: white; }
.miller-item .chevron { font-size: 10px; color: var(--text-muted); }
.miller-item.active .chevron { color: white; }

/* Breadcrumb */
.breadcrumb-bar {
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px 16px;
  background: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  font-size: 13px;
  position: relative;
  flex-wrap: wrap;
}

.breadcrumb-root,
.breadcrumb-segment {
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 4px;
  color: var(--text-secondary);
}

.breadcrumb-root:hover,
.breadcrumb-segment:hover { background: var(--bg-tertiary); color: var(--text-primary); }
.breadcrumb-segment.active { color: var(--text-primary); font-weight: 500; }
.breadcrumb-separator { color: var(--text-muted); }

.breadcrumb-dropdown-overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
}

.breadcrumb-dropdown {
  position: absolute;
  top: 100%;
  left: 16px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px 0;
  min-width: 200px;
  max-height: 300px;
  overflow-y: auto;
  z-index: 51;
  box-shadow: 0 4px 12px rgba(0,0,0,0.15);
}

.breadcrumb-dropdown-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 13px;
  color: var(--text-primary);
}

.breadcrumb-dropdown-item:hover { background: var(--bg-tertiary); }

/* App shell adjustments for nav modes */
.app-shell.nav-miller { grid-template-columns: minmax(400px, 500px) 1fr; }
.app-shell.nav-breadcrumb { grid-template-columns: 1fr; }
.app-shell.nav-breadcrumb .content-area { grid-column: 1; }
```

- [ ] **Step 4: Verify**

```bash
npx tsc --noEmit
```

- [ ] **Step 5: Commit**

```bash
git commit -m "feat: wire up 3-way navigation mode switching with Cmd+1/2/3"
```
