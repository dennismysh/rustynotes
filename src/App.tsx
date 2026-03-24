import { Component, Show, onMount, onCleanup } from "solid-js";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import MillerColumns from "./components/navigation/MillerColumns";
import Breadcrumb from "./components/navigation/Breadcrumb";
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
  const { activeFilePath, editorMode, setEditorMode, setAppConfig, navMode, setNavMode, showSearch, setShowSearch, setSearchQuery } = appState;

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
    if ((e.metaKey || e.ctrlKey) && e.key === "1") {
      e.preventDefault();
      setNavMode("sidebar");
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "2") {
      e.preventDefault();
      setNavMode("miller");
    }
    if ((e.metaKey || e.ctrlKey) && e.key === "3") {
      e.preventDefault();
      setNavMode("breadcrumb");
    }
    // Cmd+K / Ctrl+K toggles search
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      const next = !showSearch();
      setShowSearch(next);
      if (!next) setSearchQuery("");
    }
  };

  onMount(async () => {
    document.addEventListener("keydown", handleKeyDown);

    try {
      const config = await getConfig();
      setAppConfig(config);
      applyTheme(resolveTheme(config.theme.active), config.theme.overrides);
      document.getElementById("root")?.classList.add("ready");

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
        onCleanup(() => mediaQuery.removeEventListener("change", handler));
      }
    } catch (e) {
      console.error("Failed to load config:", e);
      document.getElementById("root")?.classList.add("ready");
    }
  });

  onCleanup(() => document.removeEventListener("keydown", handleKeyDown));

  const isMac = navigator.platform.includes("Mac");
  const mod = isMac ? "\u2318" : "Ctrl+";

  return (
    <div
      class="app-shell"
      classList={{
        "nav-sidebar": navMode() === "sidebar",
        "nav-miller": navMode() === "miller",
        "nav-breadcrumb": navMode() === "breadcrumb",
      }}
    >
      <Toolbar />
      <Show when={navMode() === "sidebar"}>
        <Sidebar />
      </Show>
      <Show when={navMode() === "miller"}>
        <MillerColumns />
      </Show>
      <Show when={navMode() === "breadcrumb"}>
        <Breadcrumb />
      </Show>
      <div class="content-area">
        <Show
          when={activeFilePath()}
          fallback={
            <div class="empty-state">
              <h1 class="empty-state-title">Open a folder to get started</h1>
              <p class="hint">Click "Open Folder" in the toolbar, then select a markdown file.</p>
              <div class="empty-state-shortcuts">
                <kbd>{mod}E</kbd> Cycle editor modes
                <span class="empty-state-sep">&middot;</span>
                <kbd>{mod}S</kbd> Save
                <span class="empty-state-sep">&middot;</span>
                <kbd>{mod}K</kbd> Search files
              </div>
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
