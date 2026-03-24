import { Component, For, Show, createSignal, createMemo } from "solid-js";
import { appState } from "../../lib/state";
import { readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

function filterMdEntries(entries: FileEntry[]): FileEntry[] {
  return entries
    .map((entry) => {
      if (entry.is_dir) {
        const filtered = entry.children ? filterMdEntries(entry.children) : [];
        if (filtered.length === 0) return null;
        return { ...entry, children: filtered };
      }
      return entry.name.endsWith(".md") ? entry : null;
    })
    .filter((e): e is FileEntry => e !== null);
}

function filterByQuery(entries: FileEntry[], query: string): FileEntry[] {
  const q = query.toLowerCase();
  return entries
    .map((entry) => {
      if (entry.is_dir) {
        const filtered = entry.children ? filterByQuery(entry.children, query) : [];
        if (filtered.length === 0) return null;
        return { ...entry, children: filtered };
      }
      return entry.name.toLowerCase().includes(q) ? entry : null;
    })
    .filter((e): e is FileEntry => e !== null);
}

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

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick();
    }
    if (e.key === "ArrowRight" && props.entry.is_dir && !expanded()) {
      e.preventDefault();
      setExpanded(true);
    }
    if (e.key === "ArrowLeft" && props.entry.is_dir && expanded()) {
      e.preventDefault();
      setExpanded(false);
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = (e.currentTarget as HTMLElement).closest("[role='treeitem']")
        ?.nextElementSibling?.querySelector("[role='treeitem'] > [tabindex]") as HTMLElement
        ?? (e.currentTarget as HTMLElement).parentElement
          ?.querySelector(".tree-children [tabindex]") as HTMLElement;
      next?.focus();
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const items = Array.from(
        (e.currentTarget as HTMLElement).closest("[role='tree'], .tree-children")
          ?.querySelectorAll("[tabindex='0']") ?? []
      ) as HTMLElement[];
      const idx = items.indexOf(e.currentTarget as HTMLElement);
      if (idx > 0) items[idx - 1].focus();
    }
  };

  const isActive = () => activeFilePath() === props.entry.path;

  return (
    <div role="treeitem" aria-expanded={props.entry.is_dir ? expanded() : undefined}>
      <div
        class="tree-item"
        classList={{ active: isActive() }}
        style={{ "padding-left": `${12 + props.depth * 16}px` }}
        onClick={handleClick}
        onKeyDown={handleKeyDown}
        tabIndex={0}
        role="button"
        aria-label={`${props.entry.is_dir ? "Folder" : "File"}: ${props.entry.name}`}
      >
        <span class="icon" aria-hidden="true">
          {props.entry.is_dir ? (expanded() ? "\u25BE" : "\u25B8") : "\u2013"}
        </span>
        <span class="name">{props.entry.name}</span>
      </div>
      <Show when={props.entry.is_dir && expanded() && props.entry.children}>
        <div class="tree-children" role="group">
          <For each={props.entry.children!}>
            {(child) => <TreeNode entry={child} depth={props.depth + 1} />}
          </For>
        </div>
      </Show>
    </div>
  );
};

const Sidebar: Component = () => {
  const { fileTree, currentFolder, searchQuery, setSearchQuery, showSearch } = appState;

  const filteredTree = createMemo(() => {
    let tree = filterMdEntries(fileTree());
    const q = searchQuery();
    if (q.length > 0) {
      tree = filterByQuery(tree, q);
    }
    return tree;
  });

  return (
    <div class="sidebar">
      <Show when={showSearch()}>
        <div class="sidebar-search">
          <input
            type="text"
            placeholder="Filter files..."
            value={searchQuery()}
            onInput={(e) => setSearchQuery(e.currentTarget.value)}
            autofocus
          />
        </div>
      </Show>
      <div role="tree" aria-label="File tree">
        <Show
          when={currentFolder()}
          fallback={
            <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
              No folder open
            </div>
          }
        >
          <Show
            when={filteredTree().length > 0}
            fallback={
              <div style="padding: 16px; color: var(--text-muted); font-size: 13px; text-align: center;">
                {searchQuery() ? "No matching files" : "No markdown files"}
              </div>
            }
          >
            <For each={filteredTree()}>
              {(entry) => <TreeNode entry={entry} depth={0} />}
            </For>
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default Sidebar;
