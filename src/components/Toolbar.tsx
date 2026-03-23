import { Component } from "solid-js";
import { appState, EditorMode } from "../lib/state";
import { openFolderDialog, listDirectory, watchFolder } from "../lib/ipc";

const Toolbar: Component = () => {
  const { editorMode, setEditorMode, setCurrentFolder, setFileTree } = appState;

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

  return (
    <div class="toolbar">
      <button onClick={handleOpenFolder}>Open Folder</button>
      <div class="spacer" />
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
    </div>
  );
};

export default Toolbar;
