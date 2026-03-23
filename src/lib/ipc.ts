import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

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
