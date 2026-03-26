use leptos::prelude::*;

use super::{create_tips_seen, dismiss_tip, is_tip_seen};

/// A contextual hint with an optional keyboard shortcut, dismissible by the user.
///
/// Props:
/// - `tip_id` — unique identifier, persisted to localStorage when dismissed.
/// - `message` — the tip text.
/// - `shortcut` — optional keyboard shortcut shown in a `<kbd>`.
#[component]
pub fn FeatureTip(
    #[prop(into)] tip_id: String,
    #[prop(into)] message: String,
    #[prop(optional, into)] shortcut: Option<String>,
) -> impl IntoView {
    let (tips_seen, set_tips_seen) = create_tips_seen();

    // Store all owned data in signals so they're Copy-friendly inside <Show>.
    let tip_id_sig = RwSignal::new(tip_id.clone());
    let message_sig = RwSignal::new(message.clone());
    let shortcut_sig = RwSignal::new(shortcut);
    let dismiss_label_sig = RwSignal::new(format!("Dismiss tip: {message}"));

    let visible = {
        let tip_id = tip_id.clone();
        move || !is_tip_seen(&tip_id, tips_seen)
    };

    let handle_dismiss = move |_| {
        dismiss_tip(&tip_id_sig.get_untracked(), set_tips_seen);
    };

    view! {
        <Show when=visible>
            <div class="feature-tip" role="status">
                <span class="feature-tip-icon" aria-hidden="true">
                    "\u{2139}"
                </span>
                <span class="feature-tip-message">
                    {move || message_sig.get()}
                    {move || {
                        shortcut_sig.get().map(|s| view! { " " <kbd>{s}</kbd> })
                    }}
                </span>
                <button
                    class="feature-tip-dismiss"
                    on:click=handle_dismiss
                    aria-label=move || dismiss_label_sig.get()
                >
                    "\u{00D7}"
                </button>
            </div>
        </Show>
    }
}
