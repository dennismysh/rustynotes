import { createSignal, createRoot } from "solid-js";
import type { FileEntry } from "./ipc";

export type EditorMode = "source" | "preview";

function createAppState() {
  const [currentFolder, setCurrentFolder] = createSignal<string | null>(null);
  const [fileTree, setFileTree] = createSignal<FileEntry[]>([]);
  const [activeFilePath, setActiveFilePath] = createSignal<string | null>(null);
  const [activeFileContent, setActiveFileContent] = createSignal<string>("");
  const [editorMode, setEditorMode] = createSignal<EditorMode>("source");
  const [isDirty, setIsDirty] = createSignal(false);
  const [renderedHtml, setRenderedHtml] = createSignal<string>("");

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
  };
}

export const appState = createRoot(createAppState);
