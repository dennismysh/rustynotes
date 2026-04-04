//! Watch the running binary for external changes (e.g., user drags new .app
//! to /Applications). On change, debounce 500ms then relaunch.

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const DEBOUNCE_MS: u64 = 500;

pub struct BinaryWatcher {
    _watcher: RecommendedWatcher,
    rx: mpsc::Receiver<PathBuf>,
    exe_path: PathBuf,
    changed_at: Option<Instant>,
}

impl BinaryWatcher {
    pub fn start() -> Option<Self> {
        let exe_path = std::env::current_exe().ok()?.canonicalize().ok()?;
        let parent = exe_path.parent()?.to_path_buf();

        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: Result<Event, _>| {
            if let Ok(event) = res {
                for path in event.paths {
                    let _ = tx.send(path);
                }
            }
        })
        .ok()?;

        watcher.watch(&parent, RecursiveMode::NonRecursive).ok()?;

        Some(Self {
            _watcher: watcher,
            rx,
            exe_path,
            changed_at: None,
        })
    }

    pub fn poll(&mut self) -> bool {
        while let Ok(path) = self.rx.try_recv() {
            if path == self.exe_path {
                self.changed_at = Some(Instant::now());
            }
        }

        if let Some(changed_at) = self.changed_at {
            if changed_at.elapsed() >= Duration::from_millis(DEBOUNCE_MS) {
                self.changed_at = None;
                return true;
            }
        }

        false
    }
}
