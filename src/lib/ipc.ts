import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open, save } from "@tauri-apps/plugin-dialog";

export interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
  children: FileEntry[] | null;
}

export async function readFile(path: string): Promise<string> {
  return invoke<string>("read_file", { path });
}

export async function writeFile(
  path: string,
  content: string
): Promise<void> {
  return invoke<void>("write_file", { path, content });
}

export async function listDirectory(path: string): Promise<FileEntry[]> {
  return invoke<FileEntry[]>("list_directory", { path });
}

export async function parseMarkdown(content: string): Promise<string> {
  return invoke<string>("parse_markdown", { content });
}

export async function openFolderDialog(): Promise<string | null> {
  const selected = await open({ directory: true, multiple: false });
  return selected as string | null;
}

export interface FileChangeEvent {
  paths: string[];
  kind: string;
}

export async function watchFolder(path: string): Promise<void> {
  return invoke<void>("watch_folder", { path });
}

export async function onFileChanged(
  callback: (event: FileChangeEvent) => void
): Promise<() => void> {
  return listen<FileChangeEvent>("file-changed", (event) => {
    callback(event.payload);
  });
}

export async function resolveWikilink(root: string, name: string): Promise<string | null> {
  return invoke<string | null>("resolve_wikilink", { root, name });
}

export interface AppConfig {
  theme: { active: string; overrides: { colors: Record<string, string>; typography: Record<string, string>; spacing: Record<string, string> } };
  editor_mode: string;
  nav_mode: string;
  editor_font: string;
  line_height: number;
  rendering: { render_math: boolean; render_diagrams: boolean; render_frontmatter: boolean; show_line_numbers: boolean; render_wikilinks: boolean };
  recent_folders: string[];
}

export async function getConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_config");
}

export async function saveConfig(config: AppConfig): Promise<void> {
  return invoke<void>("save_config_cmd", { configData: config });
}

export async function exportFile(
  markdown: string,
  outputPath: string,
  format: string,
  includeTheme: boolean,
): Promise<void> {
  return invoke<void>("export_file", { markdown, outputPath, format, includeTheme });
}

export async function showSaveDialog(defaultName: string, extension: string): Promise<string | null> {
  const path = await save({
    defaultPath: defaultName,
    filters: [{ name: extension.toUpperCase(), extensions: [extension] }],
  });
  return path as string | null;
}

export interface SearchResult {
  path: string;
  name: string;
  context: string;
}

export async function searchFiles(root: string, query: string): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search_files", { root, query });
}

export async function openSettings(): Promise<void> {
  return invoke("open_settings");
}

export function onConfigChanged(
  callback: (config: AppConfig) => void,
): Promise<() => void> {
  return listen<AppConfig>("config-changed", (event) => {
    callback(event.payload);
  });
}
