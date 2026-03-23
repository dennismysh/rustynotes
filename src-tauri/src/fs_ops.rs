use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub children: Option<Vec<FileEntry>>,
}

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Path not found: {0}")]
    NotFound(String),
}

pub fn read_file_content(path: &Path) -> Result<String, FsError> {
    if !path.exists() {
        return Err(FsError::NotFound(path.display().to_string()));
    }
    Ok(std::fs::read_to_string(path)?)
}

pub fn write_file_content(path: &Path, content: &str) -> Result<(), FsError> {
    Ok(std::fs::write(path, content)?)
}

pub fn list_directory(path: &Path) -> Result<Vec<FileEntry>, FsError> {
    if !path.exists() {
        return Err(FsError::NotFound(path.display().to_string()));
    }
    let mut entries: Vec<FileEntry> = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.flatten() {
            let file_type = entry.file_type();
            let is_dir = file_type.map(|ft| ft.is_dir()).unwrap_or(false);
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files/dirs
            if name.starts_with('.') {
                continue;
            }

            // For files, only include .md files
            if !is_dir && !name.ends_with(".md") {
                continue;
            }

            let children = if is_dir {
                Some(list_directory(&entry.path()).unwrap_or_default())
            } else {
                None
            };

            entries.push(FileEntry {
                name,
                path: entry.path(),
                is_dir,
                children,
            });
        }
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then(a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path();

        fs::create_dir_all(base.join("subfolder")).unwrap();
        fs::write(base.join("hello.md"), "# Hello\nWorld").unwrap();
        fs::write(base.join("notes.md"), "Some notes").unwrap();
        fs::write(base.join("subfolder/nested.md"), "Nested content").unwrap();
        fs::write(base.join("readme.txt"), "not markdown").unwrap();
        fs::write(base.join(".hidden"), "hidden file").unwrap();

        dir
    }

    #[test]
    fn test_read_file_content() {
        let dir = setup_test_dir();
        let content = read_file_content(&dir.path().join("hello.md")).unwrap();
        assert_eq!(content, "# Hello\nWorld");
    }

    #[test]
    fn test_read_file_not_found() {
        let result = read_file_content(Path::new("/nonexistent/file.md"));
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FsError::NotFound(_)));
    }

    #[test]
    fn test_write_file_content() {
        let dir = setup_test_dir();
        let path = dir.path().join("new.md");
        write_file_content(&path, "New content").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "New content");
    }

    #[test]
    fn test_list_directory() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].name, "subfolder");
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].name, "hello.md");
        assert_eq!(entries[2].name, "notes.md");
    }

    #[test]
    fn test_list_directory_skips_hidden() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        assert!(!entries.iter().any(|e| e.name.starts_with('.')));
    }

    #[test]
    fn test_list_directory_only_markdown_files() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        for entry in &entries {
            if !entry.is_dir {
                assert!(entry.name.ends_with(".md"));
            }
        }
    }

    #[test]
    fn test_list_directory_nested() {
        let dir = setup_test_dir();
        let entries = list_directory(dir.path()).unwrap();
        let subfolder = &entries[0];
        let children = subfolder.children.as_ref().unwrap();
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "nested.md");
    }
}
