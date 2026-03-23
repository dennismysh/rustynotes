import { Component, Show } from "solid-js";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import SourceEditor from "./components/editor/SourceEditor";
import Preview from "./components/preview/Preview";
import { appState } from "./lib/state";
import "./styles/base.css";

const App: Component = () => {
  const { activeFilePath, editorMode } = appState;

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
          <Show when={editorMode() === "preview"}>
            <Preview />
          </Show>
        </Show>
      </div>
    </div>
  );
};

export default App;
