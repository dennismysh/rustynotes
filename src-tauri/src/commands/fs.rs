use super::CommandError;
use crate::fs_ops::{self, FileNode};
use rustynotes_common::SearchResult;
use std::path::Path;
use walkdir::WalkDir;

#[tauri::command]
pub fn read_file(path: String) -> Result<String, CommandError> {
    Ok(fs_ops::read_file_content(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), CommandError> {
    Ok(fs_ops::write_file_content(
        std::path::Path::new(&path),
        &content,
    )?)
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileNode>, CommandError> {
    Ok(fs_ops::list_directory(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn resolve_wikilink(root: String, name: String) -> Option<String> {
    fs_ops::find_file_by_name(std::path::Path::new(&root), &name)
        .map(|p| p.display().to_string())
}

#[tauri::command]
pub fn search_files(root: String, query: String) -> Result<Vec<SearchResult>, CommandError> {
    let root_path = Path::new(&root);
    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    for entry in WalkDir::new(root_path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext == "md")
        })
    {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let name_match = name.to_lowercase().contains(&query_lower);

        if name_match {
            results.push(SearchResult {
                path: path.display().to_string(),
                name,
                context: String::new(),
            });
        } else if let Ok(content) = std::fs::read_to_string(path) {
            if let Some(line) = content
                .lines()
                .find(|l| l.to_lowercase().contains(&query_lower))
            {
                results.push(SearchResult {
                    path: path.display().to_string(),
                    name,
                    context: line.trim().chars().take(120).collect(),
                });
            }
        }

        if results.len() >= 50 {
            break;
        }
    }

    Ok(results)
}
