import { Component, Show, onMount, onCleanup } from "solid-js";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import SourceEditor from "./components/editor/SourceEditor";
import WysiwygEditor from "./components/editor/WysiwygEditor";
import SplitPane from "./components/editor/SplitPane";
import Preview from "./components/preview/Preview";
import SettingsPanel from "./components/settings/SettingsPanel";
import { appState, type EditorMode } from "./lib/state";
import "./styles/base.css";

const modes: EditorMode[] = ["source", "wysiwyg", "split", "preview"];

const App: Component = () => {
  const { activeFilePath, editorMode, setEditorMode } = appState;

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "e") {
      e.preventDefault();
      const currentIndex = modes.indexOf(editorMode());
      const nextIndex = (currentIndex + 1) % modes.length;
      setEditorMode(modes[nextIndex]);
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "p") {
      e.preventDefault();
      setEditorMode("preview");
    }
  };

  onMount(() => document.addEventListener("keydown", handleKeyDown));
  onCleanup(() => document.removeEventListener("keydown", handleKeyDown));

  return (
    <div class="app-shell">
      <Toolbar />
      <Sidebar />
      <div class="content-area">
        <Show
          when={activeFilePath()}
          fallback={
            <div class="empty-state">
              <div style="font-size: 32px">Open a folder to get started</div>
              <div class="hint">Click "Open Folder" in the toolbar</div>
            </div>
          }
        >
          <Show when={editorMode() === "source"}>
            <SourceEditor />
          </Show>
          <Show when={editorMode() === "wysiwyg"}>
            <WysiwygEditor />
          </Show>
          <Show when={editorMode() === "split"}>
            <SplitPane />
          </Show>
          <Show when={editorMode() === "preview"}>
            <Preview />
          </Show>
        </Show>
      </div>
      <SettingsPanel />
    </div>
  );
};

export default App;
