use leptos::prelude::*;

use super::source_editor::SourceEditor;
use crate::components::preview::preview::Preview;

/// Split pane component that renders a SourceEditor on the left and a Preview on the right,
/// separated by a visual divider.
#[component]
pub fn SplitPane() -> impl IntoView {
    view! {
        <div class="split-pane">
            <div class="split-pane-left">
                <SourceEditor />
            </div>
            <div class="split-pane-divider" />
            <div class="split-pane-right">
                <Preview />
            </div>
        </div>
    }
}
