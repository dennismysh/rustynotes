import { Component, For, Show, createSignal, createMemo, onCleanup } from "solid-js";
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

  // Close dropdown on Escape
  const handleGlobalKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape" && dropdownIndex() !== null) {
      e.preventDefault();
      closeDropdown();
    }
  };

  document.addEventListener("keydown", handleGlobalKeyDown);
  onCleanup(() => document.removeEventListener("keydown", handleGlobalKeyDown));

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

  const handleDropdownKeyDown = (e: KeyboardEvent, entry: FileEntry) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      handleDropdownItemClick(entry);
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      const next = (e.currentTarget as HTMLElement).nextElementSibling as HTMLElement;
      next?.focus();
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      const prev = (e.currentTarget as HTMLElement).previousElementSibling as HTMLElement;
      prev?.focus();
    }
  };

  return (
    <nav class="breadcrumb-bar" aria-label="File path">
      <Show when={currentFolder()}>
        <button
          class="breadcrumb-root"
          onClick={handleRootClick}
          aria-label={`Root folder: ${currentFolder()!.split("/").pop() || currentFolder()}`}
        >
          {currentFolder()!.split("/").pop() || currentFolder()}
        </button>

        <For each={pathSegments()}>
          {(segment, index) => (
            <>
              <span class="breadcrumb-separator" aria-hidden="true">/</span>
              <button
                class="breadcrumb-segment"
                classList={{ active: index() === pathSegments().length - 1 }}
                onClick={() => handleSegmentClick(index())}
                aria-current={index() === pathSegments().length - 1 ? "page" : undefined}
              >
                {segment}
              </button>
            </>
          )}
        </For>
      </Show>

      <Show when={!currentFolder()}>
        <span style="color: var(--text-muted); font-size: 13px;">No folder open</span>
      </Show>

      <Show when={dropdownIndex() !== null}>
        <div class="breadcrumb-dropdown-overlay" onClick={closeDropdown} />
        <div class="breadcrumb-dropdown" role="listbox" aria-label="Directory contents">
          <For each={dropdownItems()}>
            {(entry) => (
              <div
                class="breadcrumb-dropdown-item"
                onClick={() => handleDropdownItemClick(entry)}
                onKeyDown={(e) => handleDropdownKeyDown(e, entry)}
                tabIndex={0}
                role="option"
                aria-label={`${entry.is_dir ? "Folder" : "File"}: ${entry.name}`}
              >
                <span aria-hidden="true">{entry.is_dir ? "\u{1F4C1}" : "\u{1F4C4}"}</span>
                <span>{entry.name}</span>
              </div>
            )}
          </For>
        </div>
      </Show>
    </nav>
  );
};

export default Breadcrumb;
