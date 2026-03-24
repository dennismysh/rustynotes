import { Component, Show } from "solid-js";
import { appState, type EditorMode, type NavMode } from "../lib/state";
import { openFolderDialog, listDirectory, watchFolder, exportFile, showSaveDialog } from "../lib/ipc";

const Toolbar: Component = () => {
  const { editorMode, setEditorMode, setCurrentFolder, setFileTree, setShowSettings, navMode, setNavMode, activeFileContent, activeFilePath, isDirty } = appState;

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

  const handleExport = async () => {
    const content = activeFileContent();
    const filePath = activeFilePath();
    if (!filePath) return;

    const fileName = filePath.split("/").pop()?.replace(/\.[^.]+$/, "") ?? "document";
    const defaultName = `${fileName}.html`;

    const savePath = await showSaveDialog(defaultName, "html");
    if (savePath) {
      await exportFile(content, savePath, "html", true);
    }
  };

  return (
    <div class="toolbar">
      <button onClick={handleOpenFolder}>Open Folder</button>
      <div class="nav-switcher">
        <button
          classList={{ active: navMode() === "sidebar" }}
          onClick={() => setNav("sidebar")}
        >
          Tree
        </button>
        <button
          classList={{ active: navMode() === "miller" }}
          onClick={() => setNav("miller")}
        >
          Columns
        </button>
        <button
          classList={{ active: navMode() === "breadcrumb" }}
          onClick={() => setNav("breadcrumb")}
        >
          Crumbs
        </button>
      </div>
      <div class="spacer" />
      <Show when={isDirty()}>
        <span class="dirty-indicator" title="Unsaved changes" aria-label="Unsaved changes" />
      </Show>
      <div class="mode-switcher">
        <button
          classList={{ active: editorMode() === "source" }}
          onClick={() => setMode("source")}
        >
          Source
        </button>
        <button
          classList={{ active: editorMode() === "wysiwyg" }}
          onClick={() => setMode("wysiwyg")}
        >
          WYSIWYG
        </button>
        <button
          classList={{ active: editorMode() === "split" }}
          onClick={() => setMode("split")}
        >
          Split
        </button>
        <button
          classList={{ active: editorMode() === "preview" }}
          onClick={() => setMode("preview")}
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
