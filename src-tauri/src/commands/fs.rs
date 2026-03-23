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
