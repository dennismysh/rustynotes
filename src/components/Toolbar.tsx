import { Component, Show, createSignal, createMemo } from "solid-js";
import { appState } from "../lib/state";
import { openFolderDialog, listDirectory, watchFolder, exportFile, showSaveDialog } from "../lib/ipc";

const Toolbar: Component = () => {
  const {
    setCurrentFolder, setFileTree,
    activeFileContent, activeFilePath, isDirty,
    showSearch, setShowSearch,
  } = appState;
  const [exportStatus, setExportStatus] = createSignal<string | null>(null);

  const handleOpenFolder = async () => {
    const folder = await openFolderDialog();
    if (folder) {
      setCurrentFolder(folder);
      const tree = await listDirectory(folder);
      setFileTree(tree);
      await watchFolder(folder);
    }
  };

  const showStatus = (msg: string) => {
    setExportStatus(msg);
    setTimeout(() => setExportStatus(null), 2000);
  };

  const handleExport = async () => {
    const content = activeFileContent();
    const filePath = activeFilePath();
    if (!filePath) return;

    const fileName = filePath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "document";
    const defaultName = `${fileName}.html`;

    const savePath = await showSaveDialog(defaultName, "html");
    if (!savePath) return;

    try {
      await exportFile(content, savePath, "html", true);
      const savedName = savePath.split("/").pop() ?? "file";
      showStatus(`Saved ${savedName}`);
    } catch (e) {
      showStatus("Could not export");
      console.error("Export failed:", e);
    }
  };

  const activeFileName = createMemo(() => {
    const path = activeFilePath();
    if (!path) return null;
    return path.split("/").pop() ?? null;
  });

  const isMac = navigator.platform.includes("Mac");

  return (
    <div class="toolbar">
      <button onClick={handleOpenFolder}>Open Folder</button>
      <div class="spacer" />
      <Show when={activeFileName()}>
        <div class="toolbar-filename">
          <Show when={isDirty()}>
            <span class="dirty-indicator" aria-label="Unsaved changes" />
          </Show>
          <span class="toolbar-filename-text" title={activeFilePath()!}>
            {activeFileName()}
          </span>
        </div>
      </Show>
      <div class="spacer" />
      <Show when={exportStatus()}>
        <span class="toolbar-status">{exportStatus()}</span>
      </Show>
      <button
        class="toolbar-icon-btn"
        onClick={() => setShowSearch(!showSearch())}
        classList={{ active: showSearch() }}
        title={`Search files (${isMac ? "\u2318" : "Ctrl+"}K)`}
      >
        &#x2315;
      </button>
      <button class="toolbar-icon-btn" onClick={handleExport} title="Export as HTML">
        &#x21E5;
      </button>
      <button class="toolbar-icon-btn" onClick={() => {}} title="Settings">
        &#x2699;
      </button>
    </div>
  );
};

export default Toolbar;
