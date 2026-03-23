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
        <div class="miller-columns">
          <For each={columns()}>
            {(column, colIndex) => (
              <div class="miller-column">
                <For each={column}>
                  {(entry) => (
                    <div
                      class="miller-item"
                      classList={{
                        active:
                          selectedPaths()[colIndex()] === entry.path ||
                          activeFilePath() === entry.path,
                      }}
                      onClick={() => handleClick(entry, colIndex())}
                    >
                      <span>{entry.name}</span>
                      <Show when={entry.is_dir}>
                        <span class="chevron">&#9656;</span>
                      </Show>
                    </div>
                  )}
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
