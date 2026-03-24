import { Component, For, Show, createSignal, createEffect } from "solid-js";
import { appState } from "../../lib/state";
import { readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const MillerColumns: Component = () => {
  const {
    fileTree,
    currentFolder,
    activeFilePath,
    setActiveFilePath,
    setActiveFileContent,
    setRenderedHtml,
    setIsDirty,
  } = appState;

  const [columns, setColumns] = createSignal<FileEntry[][]>([]);
  const [selectedPaths, setSelectedPaths] = createSignal<(string | null)[]>([]);

  // Re-initialize columns when the file tree changes
  createEffect(() => {
    const tree = fileTree();
    if (tree.length > 0) {
      setColumns([tree]);
      setSelectedPaths([null]);
    } else {
      setColumns([]);
      setSelectedPaths([]);
    }
  });

  const handleClick = async (entry: FileEntry, colIndex: number) => {
    if (entry.is_dir) {
      // Keep columns 0..colIndex, add children as next column
      const newColumns = columns().slice(0, colIndex + 1);
      const newSelected = selectedPaths().slice(0, colIndex + 1);

      newSelected[colIndex] = entry.path;

      if (entry.children && entry.children.length > 0) {
        newColumns.push(entry.children);
        newSelected.push(null);
      }

      setColumns(newColumns);
      setSelectedPaths(newSelected);
    } else {
      // Select this file in the current column
      const newSelected = selectedPaths().slice(0, colIndex + 1);
      newSelected[colIndex] = entry.path;
      setSelectedPaths(newSelected);

      // Trim any columns after this one
      setColumns(columns().slice(0, colIndex + 1));

      // Open the file
      setActiveFilePath(entry.path);
      const content = await readFile(entry.path);
      setActiveFileContent(content);
      setIsDirty(false);
      const html = await parseMarkdown(content);
      setRenderedHtml(html);
    }
  };

  const handleItemKeyDown = (e: KeyboardEvent, entry: FileEntry, colIndex: number) => {
    const target = e.currentTarget as HTMLElement;

    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleClick(entry, colIndex);
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = target.nextElementSibling as HTMLElement;
      next?.focus();
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const prev = target.previousElementSibling as HTMLElement;
      prev?.focus();
    }
    if (e.key === "ArrowRight" && entry.is_dir) {
      e.preventDefault();
      handleClick(entry, colIndex);
      // Focus first item in next column after render
      requestAnimationFrame(() => {
        const cols = document.querySelectorAll(".miller-column");
        const nextCol = cols[colIndex + 1];
        const firstItem = nextCol?.querySelector("[tabindex='0']") as HTMLElement;
        firstItem?.focus();
      });
    }
    if (e.key === "ArrowLeft" && colIndex > 0) {
      e.preventDefault();
      // Focus the selected item in the previous column
      const cols = document.querySelectorAll(".miller-column");
      const prevCol = cols[colIndex - 1];
      const activeItem = prevCol?.querySelector(".miller-item.active") as HTMLElement
        ?? prevCol?.querySelector("[tabindex='0']") as HTMLElement;
      activeItem?.focus();
    }
  };

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
        <div class="miller-columns" role="group" aria-label="Miller column file browser">
          <For each={columns()}>
            {(column, colIndex) => (
              <div class="miller-column" role="listbox" aria-label={`Column ${colIndex() + 1}`}>
                <For each={column}>
                  {(entry) => {
                    const isActive = () =>
                      selectedPaths()[colIndex()] === entry.path ||
                      activeFilePath() === entry.path;

                    return (
                      <div
                        class="miller-item"
                        classList={{ active: isActive() }}
                        onClick={() => handleClick(entry, colIndex())}
                        onKeyDown={(e) => handleItemKeyDown(e, entry, colIndex())}
                        tabIndex={0}
                        role="option"
                        aria-selected={isActive()}
                        aria-label={`${entry.is_dir ? "Folder" : "File"}: ${entry.name}`}
                      >
                        <span>{entry.name}</span>
                        <Show when={entry.is_dir}>
                          <span class="chevron" aria-hidden="true">&#9656;</span>
                        </Show>
                      </div>
                    );
                  }}
                </For>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default MillerColumns;
