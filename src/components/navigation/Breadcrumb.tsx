import { Component, For, Show, createSignal, createMemo } from "solid-js";
import { appState } from "../../lib/state";
import { listDirectory, readFile, parseMarkdown } from "../../lib/ipc";
import type { FileEntry } from "../../lib/ipc";

const Breadcrumb: Component = () => {
  const {
    currentFolder,
    activeFilePath,
    setActiveFilePath,
    setActiveFileContent,
    setRenderedHtml,
    setIsDirty,
  } = appState;

  const [dropdownItems, setDropdownItems] = createSignal<FileEntry[]>([]);
  const [dropdownIndex, setDropdownIndex] = createSignal<number | null>(null);

  // Derive path segments from activeFilePath relative to currentFolder
  const pathSegments = createMemo(() => {
    const folder = currentFolder();
    const filePath = activeFilePath();
    if (!folder || !filePath) return [];

    const relative = filePath.startsWith(folder)
      ? filePath.slice(folder.length).replace(/^\//, "")
      : filePath;

    return relative.split("/").filter((s) => s.length > 0);
  });

  const closeDropdown = () => {
    setDropdownIndex(null);
    setDropdownItems([]);
  };

  const handleSegmentClick = async (segmentIndex: number) => {
    const folder = currentFolder();
    if (!folder) return;

    // Build the parent directory path for this segment
    const segments = pathSegments();
    let parentPath: string;

    if (segmentIndex === 0) {
      // Root level — list the root folder
      parentPath = folder;
    } else {
      // List the directory that contains this segment
      parentPath = folder + "/" + segments.slice(0, segmentIndex).join("/");
    }

    try {
      const entries = await listDirectory(parentPath);
      setDropdownItems(entries);
      setDropdownIndex(segmentIndex);
    } catch (e) {
      console.error("Failed to list directory for breadcrumb:", e);
    }
  };

  const handleRootClick = async () => {
    const folder = currentFolder();
    if (!folder) return;

    try {
      const entries = await listDirectory(folder);
      setDropdownItems(entries);
      setDropdownIndex(-1);
    } catch (e) {
      console.error("Failed to list root directory:", e);
    }
  };

  const handleDropdownItemClick = async (entry: FileEntry) => {
    closeDropdown();

    if (entry.is_dir) {
      // Navigate into directory — list its contents
      try {
        const entries = await listDirectory(entry.path);
        setDropdownItems(entries);
        setDropdownIndex(dropdownIndex());
      } catch (e) {
        console.error("Failed to list directory:", e);
      }
    } else {
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
    <div class="breadcrumb-bar">
      <Show when={currentFolder()}>
        <span class="breadcrumb-root" onClick={handleRootClick}>
          {currentFolder()!.split("/").pop() || currentFolder()}
        </span>

        <For each={pathSegments()}>
          {(segment, index) => (
            <>
              <span class="breadcrumb-separator">/</span>
              <span
                class="breadcrumb-segment"
                classList={{ active: index() === pathSegments().length - 1 }}
                onClick={() => handleSegmentClick(index())}
              >
                {segment}
              </span>
            </>
          )}
        </For>
      </Show>

      <Show when={!currentFolder()}>
        <span style="color: var(--text-muted); font-size: 13px;">No folder open</span>
      </Show>

      <Show when={dropdownIndex() !== null}>
        <div class="breadcrumb-dropdown-overlay" onClick={closeDropdown} />
        <div class="breadcrumb-dropdown">
          <For each={dropdownItems()}>
            {(entry) => (
              <div
                class="breadcrumb-dropdown-item"
                onClick={() => handleDropdownItemClick(entry)}
              >
                <span>{entry.is_dir ? "\u{1F4C1}" : "\u{1F4C4}"}</span>
                <span>{entry.name}</span>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
};

export default Breadcrumb;
