import { Component, Show, onMount, onCleanup } from "solid-js";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import SourceEditor from "./components/editor/SourceEditor";
import WysiwygEditor from "./components/editor/WysiwygEditor";
import SplitPane from "./components/editor/SplitPane";
import Preview from "./components/preview/Preview";
import SettingsPanel from "./components/settings/SettingsPanel";
import { appState, type EditorMode } from "./lib/state";
import { getConfig } from "./lib/ipc";
import { applyTheme, resolveTheme } from "./lib/theme";
import "./styles/base.css";

const modes: EditorMode[] = ["source", "wysiwyg", "split", "preview"];

const App: Component = () => {
  const { activeFilePath, editorMode, setEditorMode, setAppConfig } = appState;

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

  onMount(async () => {
    document.addEventListener("keydown", handleKeyDown);

    try {
      const config = await getConfig();
      setAppConfig(config);
      applyTheme(resolveTheme(config.theme.active), config.theme.overrides);

      // Set editor mode from config
      const mode = config.editor_mode as EditorMode;
      if (modes.includes(mode)) {
        setEditorMode(mode);
      }

      // Listen for OS theme changes when active === "auto"
      if (config.theme.active === "auto") {
        const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
        const handler = () => {
          const currentConfig = appState.appConfig();
          if (currentConfig && currentConfig.theme.active === "auto") {
            applyTheme(resolveTheme("auto"), currentConfig.theme.overrides);
          }
        };
        mediaQuery.addEventListener("change", handler);
      }
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  });

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
