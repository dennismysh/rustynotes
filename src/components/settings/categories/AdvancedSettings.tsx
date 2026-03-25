import { resetOnboarding } from "../../../lib/onboarding";

export function AdvancedSettings() {
  return (
    <div class="settings-category">
      <h2 class="settings-category-title">Advanced</h2>
      <p class="settings-category-subtitle">Developer options and resets</p>

      <div class="setting-row">
        <div class="setting-info">
          <div class="setting-label">Onboarding Tips</div>
          <div class="setting-description">Show the welcome tips and feature highlights again</div>
        </div>
        <button class="settings-reset-btn" onClick={() => resetOnboarding()}>
          Reset Tips
        </button>
      </div>
    </div>
  );
}
