use leptos::prelude::*;
use rustynotes_common::{AppConfig, EditorMode, FileNode, NavMode};

#[derive(Clone, Debug, PartialEq)]
pub enum SaveStatus {
    Idle,
    Saving,
    Saved,
    Error(String),
}

#[derive(Clone)]
pub struct AppState {
    pub current_folder: RwSignal<Option<String>>,
    pub file_tree: RwSignal<Vec<FileNode>>,
    pub active_file_path: RwSignal<Option<String>>,
    pub active_file_content: RwSignal<String>,
    pub editor_mode: RwSignal<EditorMode>,
    pub is_dirty: RwSignal<bool>,
    pub rendered_html: RwSignal<String>,
    pub app_config: RwSignal<Option<AppConfig>>,
    pub nav_mode: RwSignal<NavMode>,
    pub search_query: RwSignal<String>,
    pub show_search: RwSignal<bool>,
    // Save-related state
    pub save_status: RwSignal<SaveStatus>,
    pub last_save_timestamp: RwSignal<Option<f64>>,
    pub pending_file_switch: RwSignal<Option<String>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            current_folder: RwSignal::new(None),
            file_tree: RwSignal::new(Vec::new()),
            active_file_path: RwSignal::new(None),
            active_file_content: RwSignal::new(String::new()),
            editor_mode: RwSignal::new(EditorMode::Wysiwyg),
            is_dirty: RwSignal::new(false),
            rendered_html: RwSignal::new(String::new()),
            app_config: RwSignal::new(None),
            nav_mode: RwSignal::new(NavMode::Sidebar),
            search_query: RwSignal::new(String::new()),
            show_search: RwSignal::new(false),
            save_status: RwSignal::new(SaveStatus::Idle),
            last_save_timestamp: RwSignal::new(None),
            pending_file_switch: RwSignal::new(None),
        }
    }
}

pub fn provide_app_state() {
    provide_context(AppState::new());
}

pub fn use_app_state() -> AppState {
    expect_context::<AppState>()
}
