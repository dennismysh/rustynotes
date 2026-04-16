use std::path::{Path, PathBuf};

/// Push `path` to the front of `list`, dedup by equality, cap at `cap`
/// entries. Returns true if the list changed.
pub fn push_recent(list: &mut Vec<String>, path: String, cap: usize) -> bool {
    if list.first().map(|s| s == &path).unwrap_or(false) {
        return false;
    }
    list.retain(|p| p != &path);
    list.insert(0, path);
    if list.len() > cap {
        list.truncate(cap);
    }
    true
}

/// Remove entries from `list` whose paths no longer exist on disk.
/// Returns true if the list changed.
pub fn prune_missing(list: &mut Vec<String>) -> bool {
    let before = list.len();
    list.retain(|p| Path::new(p).exists());
    list.len() != before
}

/// Canonicalize (resolve symlinks + make absolute). Falls back to the
/// input if the path doesn't exist or can't be canonicalized.
pub fn canonicalize_or_same(path: &str) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| PathBuf::from(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_recent_adds_new_entry_at_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/c".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/c", "/a", "/b"]);
    }

    #[test]
    fn push_recent_moves_existing_entry_to_front() {
        let mut list = vec!["/a".to_string(), "/b".to_string(), "/c".to_string()];
        let changed = push_recent(&mut list, "/b".to_string(), 10);
        assert!(changed);
        assert_eq!(list, vec!["/b", "/a", "/c"]);
    }

    #[test]
    fn push_recent_no_op_if_already_first() {
        let mut list = vec!["/a".to_string(), "/b".to_string()];
        let changed = push_recent(&mut list, "/a".to_string(), 10);
        assert!(!changed);
        assert_eq!(list, vec!["/a", "/b"]);
    }

    #[test]
    fn push_recent_caps_length() {
        let mut list: Vec<String> = (0..10).map(|i| format!("/{i}")).collect();
        push_recent(&mut list, "/new".to_string(), 10);
        assert_eq!(list.len(), 10);
        assert_eq!(list[0], "/new");
        assert_eq!(list[9], "/8");
    }

    #[test]
    fn prune_missing_removes_nonexistent() {
        let mut list = vec!["/definitely/not/a/real/path.md".to_string()];
        let changed = prune_missing(&mut list);
        assert!(changed);
        assert!(list.is_empty());
    }

    #[test]
    fn prune_missing_keeps_existing() {
        let tmp = std::env::temp_dir();
        let mut list = vec![tmp.to_string_lossy().into_owned()];
        let changed = prune_missing(&mut list);
        assert!(!changed);
        assert_eq!(list.len(), 1);
    }
}
