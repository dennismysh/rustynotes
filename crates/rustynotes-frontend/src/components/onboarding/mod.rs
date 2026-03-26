mod feature_tip;
mod welcome;

pub use feature_tip::FeatureTip;
pub use welcome::WelcomeEmptyState;

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Onboarding state (localStorage-backed, matching the TS version)
// ---------------------------------------------------------------------------

const STORAGE_KEY: &str = "rustynotes:onboarding";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OnboardingState {
    welcomed: bool,
    tips_seen: Vec<String>,
}

impl Default for OnboardingState {
    fn default() -> Self {
        Self {
            welcomed: false,
            tips_seen: Vec::new(),
        }
    }
}

fn load_onboarding() -> OnboardingState {
    LocalStorage::get::<OnboardingState>(STORAGE_KEY).unwrap_or_default()
}

fn persist_onboarding(state: &OnboardingState) {
    let _ = LocalStorage::set(STORAGE_KEY, state);
}

/// Reactive signal tracking whether this is the first run (welcome not yet dismissed).
/// Initialised once from localStorage.
pub fn create_is_first_run() -> (ReadSignal<bool>, WriteSignal<bool>) {
    let initial = load_onboarding();
    signal(!initial.welcomed)
}

/// Reactive signal tracking which tip IDs have been dismissed.
pub fn create_tips_seen() -> (ReadSignal<Vec<String>>, WriteSignal<Vec<String>>) {
    let initial = load_onboarding();
    signal(initial.tips_seen)
}

/// Mark the welcome screen as dismissed (persists to localStorage).
pub fn mark_welcomed(set_first_run: WriteSignal<bool>) {
    set_first_run.set(false);
    let mut state = load_onboarding();
    state.welcomed = true;
    persist_onboarding(&state);
}

/// Dismiss a specific feature tip (persists to localStorage).
pub fn dismiss_tip(tip_id: &str, set_tips_seen: WriteSignal<Vec<String>>) {
    let id = tip_id.to_string();
    set_tips_seen.update(|tips| {
        if !tips.contains(&id) {
            tips.push(id.clone());
        }
    });
    let mut state = load_onboarding();
    if !state.tips_seen.contains(&id) {
        state.tips_seen.push(id);
        persist_onboarding(&state);
    }
}

/// Check if a tip has been seen.
pub fn is_tip_seen(tip_id: &str, tips_seen: ReadSignal<Vec<String>>) -> bool {
    tips_seen.with(|tips| tips.contains(&tip_id.to_string()))
}
