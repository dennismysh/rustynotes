import { createSignal, createRoot } from "solid-js";
import type { FileEntry, AppConfig } from "./ipc";

export type EditorMode = "source" | "wysiwyg" | "split" | "preview";
export type NavMode = "sidebar" | "miller" | "breadcrumb";

function createAppState() {
  const [currentFolder, setCurrentFolder] = createSignal<string | null>(null);
  const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
  const [activeFilePath, setActiveFilePath] = createSignal<string | null>(null);
  const [activeFileContent, setActiveFileContent] = createSignal<string>("");
  const [editorMode, setEditorMode] = createSignal<EditorMode>("wysiwyg");
  const [isDirty, setIsDirty] = createSignal(false);
  const [renderedHtml, setRenderedHtml] = createSignal<string>("");
  const [appConfig, setAppConfig] = createSignal<AppConfig | null>(null);
  const [showSettings, setShowSettings] = createSignal(false);
  const [navMode, setNavMode] = createSignal<NavMode>("sidebar");
  const [searchQuery, setSearchQuery] = createSignal("");
  const [showSearch, setShowSearch] = createSignal(false);

  return {
    currentFolder,
    setCurrentFolder,
    fileTree,
    setFileTree,
    activeFilePath,
    setActiveFilePath,
    activeFileContent,
    setActiveFileContent,
    editorMode,
    setEditorMode,
    isDirty,
    setIsDirty,
    renderedHtml,
    setRenderedHtml,
    appConfig,
    setAppConfig,
    showSettings,
    setShowSettings,
    navMode,
    setNavMode,
    searchQuery,
    setSearchQuery,
    showSearch,
    setShowSearch,
  };
}

export const appState = createRoot(createAppState);
