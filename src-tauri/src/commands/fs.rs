use super::CommandError;
use crate::fs_ops::{self, FileEntry};

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
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, CommandError> {
    Ok(fs_ops::list_directory(std::path::Path::new(&path))?)
}

#[tauri::command]
pub fn resolve_wikilink(root: String, name: String) -> Option<String> {
    fs_ops::find_file_by_name(std::path::Path::new(&root), &name)
        .map(|p| p.display().to_string())
}
