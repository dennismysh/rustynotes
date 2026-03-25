import { Component, Show, onMount, onCleanup } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Toolbar from "./components/Toolbar";
import Sidebar from "./components/navigation/Sidebar";
import MillerColumns from "./components/navigation/MillerColumns";
import Breadcrumb from "./components/navigation/Breadcrumb";
import SourceEditor from "./components/editor/SourceEditor";
import WysiwygEditor from "./components/editor/WysiwygEditor";
import SplitPane from "./components/editor/SplitPane";
import Preview from "./components/preview/Preview";
import WelcomeEmptyState from "./components/onboarding/WelcomeEmptyState";
import FeatureTip from "./components/onboarding/FeatureTip";
import { appState, type EditorMode, type NavMode } from "./lib/state";
import { getConfig, saveConfig, openFolderDialog, listDirectory, watchFolder, onConfigChanged, openSettings, type AppConfig } from "./lib/ipc";
import { applyTheme, resolveTheme } from "./lib/theme";
import { markWelcomed } from "./lib/onboarding";
import "./styles/base.css";

const modes: EditorMode[] = ["source", "wysiwyg", "split", "preview"];

const App: Component = () => {
  const { activeFilePath, editorMode, setEditorMode, setAppConfig, navMode, setNavMode, showSearch, setShowSearch, setSearchQuery, currentFolder, setCurrentFolder, setFileTree } = appState;

  const openFolder = async (folder: string) => {
    setCurrentFolder(folder);
    const tree = await listDirectory(folder);
    setFileTree(tree);
    await watchFolder(folder);

    // Persist to recent_folders
    const config = appState.appConfig();
    if (config) {
      const recent = [folder, ...config.recent_folders.filter((f) => f !== folder)].slice(0, 10);
      const updated: AppConfig = { ...config, recent_folders: recent };
      setAppConfig(updated);
      saveConfig(updated).catch((e) => console.error("Failed to save config:", e));
    }
  };

  const handleOpenFolder = async () => {
    const folder = await openFolderDialog();
    if (folder) await openFolder(folder);
  };

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
      document.documentElement.classList.add("ready");
      await getCurrentWindow().show();

      // Set editor mode from config
      const mode = config.editor_mode as EditorMode;
      if (modes.includes(mode)) {
        setEditorMode(mode);
      }

      // Reopen last folder (returning user)
      if (config.recent_folders.length > 0) {
        markWelcomed();
        try {
          await openFolder(config.recent_folders[0]);
        } catch (e) {
          console.error("Failed to reopen folder:", e);
        }
      }

      // Listen for config changes from settings window
      await onConfigChanged((config) => {
        setAppConfig(config);
        setEditorMode(config.editor_mode as EditorMode);
        setNavMode(config.nav_mode as NavMode);
        applyTheme(resolveTheme(config.theme.active), config.theme.overrides);
      });

      // Cmd+, keyboard shortcut to open settings
      const handleKeydown = (e: KeyboardEvent) => {
        if (e.metaKey && e.key === ",") {
          e.preventDefault();
          openSettings();
        }
      };
      document.addEventListener("keydown", handleKeydown);

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
      document.documentElement.classList.add("ready");
      await getCurrentWindow().show();
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
              <Show
                when={currentFolder()}
                fallback={
                  <WelcomeEmptyState
                    onOpenFolder={handleOpenFolder}
                    onOpenRecent={openFolder}
                  />
                }
              >
                <h1 class="empty-state-title">Select a file</h1>
                <p class="hint">
                  Choose a markdown file from the sidebar, or press <kbd>{mod}K</kbd> to search.
                </p>
                <FeatureTip
                  id="nav-modes"
                  message="Try different navigation styles."
                  shortcut={`${mod}1/2/3`}
                />
              </Show>
            </div>
          }
        >
          <FeatureTip
            id="editor-modes"
            message="Cycle editor modes: rich text, source, split, preview."
            shortcut={`${mod}E`}
          />
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
    </div>
  );
};

export default App;
