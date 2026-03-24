import { Component, Show, createSignal } from "solid-js";
import { appState, type EditorMode, type NavMode } from "../lib/state";
import { openFolderDialog, listDirectory, watchFolder, exportFile, showSaveDialog } from "../lib/ipc";

const Toolbar: Component = () => {
  const { editorMode, setEditorMode, setCurrentFolder, setFileTree, setShowSettings, navMode, setNavMode, activeFileContent, activeFilePath, isDirty } = appState;
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

  const setMode = (mode: EditorMode) => {
    setEditorMode(mode);
  };

  const setNav = (mode: NavMode) => {
    setNavMode(mode);
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
      showStatus("Exported");
    } catch (e) {
      showStatus("Export failed");
      console.error("Export failed:", e);
    }
  };

  const isMac = navigator.platform.includes("Mac");
  const mod = isMac ? "\u2318" : "Ctrl+";

  return (
    <div class="toolbar">
      <button onClick={handleOpenFolder}>Open Folder</button>
      <div class="nav-switcher">
        <button
          classList={{ active: navMode() === "sidebar" }}
          onClick={() => setNav("sidebar")}
          title={`Tree view (${mod}1)`}
        >
          Tree
        </button>
        <button
          classList={{ active: navMode() === "miller" }}
          onClick={() => setNav("miller")}
          title={`Column view (${mod}2)`}
        >
          Columns
        </button>
        <button
          classList={{ active: navMode() === "breadcrumb" }}
          onClick={() => setNav("breadcrumb")}
          title={`Breadcrumb view (${mod}3)`}
        >
          Crumbs
        </button>
      </div>
      <div class="spacer" />
      <Show when={exportStatus()}>
        <span class="toolbar-status">{exportStatus()}</span>
      </Show>
      <Show when={isDirty()}>
        <span class="dirty-indicator" title="Unsaved changes" aria-label="Unsaved changes" />
      </Show>
      <div class="mode-switcher">
        <button
          classList={{ active: editorMode() === "source" }}
          onClick={() => setMode("source")}
          title={`Source editor (${mod}E to cycle)`}
        >
          Source
        </button>
        <button
          classList={{ active: editorMode() === "wysiwyg" }}
          onClick={() => setMode("wysiwyg")}
          title={`WYSIWYG editor (${mod}E to cycle)`}
        >
          WYSIWYG
        </button>
        <button
          classList={{ active: editorMode() === "split" }}
          onClick={() => setMode("split")}
          title={`Split view (${mod}E to cycle)`}
        >
          Split
        </button>
        <button
          classList={{ active: editorMode() === "preview" }}
          onClick={() => setMode("preview")}
          title={`Preview (${mod}P)`}
        >
          Preview
        </button>
      </div>
      <button class="export-btn" onClick={handleExport} title="Export to HTML">
        Export
      </button>
      <button class="settings-btn" onClick={() => setShowSettings(true)} title="Settings">
        &#9881;
      </button>
    </div>
  );
};

export default Toolbar;
