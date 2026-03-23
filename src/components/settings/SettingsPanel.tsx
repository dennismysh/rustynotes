import { Component, Show } from "solid-js";
import { appState } from "../../lib/state";
import { saveConfig, type AppConfig } from "../../lib/ipc";
import { applyTheme, resolveTheme } from "../../lib/theme";

const SettingsPanel: Component = () => {
  const { appConfig, setAppConfig, showSettings, setShowSettings } = appState;

  const updateConfig = async (updater: (config: AppConfig) => AppConfig) => {
    const current = appConfig();
    if (!current) return;
    const updated = updater({ ...current });
    setAppConfig(updated);
    applyTheme(resolveTheme(updated.theme.active), updated.theme.overrides);
    try {
      await saveConfig(updated);
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  };

  const setThemeActive = (active: string) => {
    updateConfig((c) => ({
      ...c,
      theme: { ...c.theme, active },
    }));
  };

  const toggleRendering = (key: keyof AppConfig["rendering"]) => {
    updateConfig((c) => ({
      ...c,
      rendering: { ...c.rendering, [key]: !c.rendering[key] },
    }));
  };

  const setFontSize = (size: string) => {
    updateConfig((c) => ({
      ...c,
      theme: {
        ...c.theme,
        overrides: {
          ...c.theme.overrides,
          typography: { ...c.theme.overrides.typography, "body-size": `${size}px` },
        },
      },
    }));
  };

  const setAccentColor = (color: string) => {
    updateConfig((c) => ({
      ...c,
      theme: {
        ...c.theme,
        overrides: {
          ...c.theme.overrides,
          colors: { ...c.theme.overrides.colors, accent: color },
        },
      },
    }));
  };

  const currentFontSize = () => {
    const config = appConfig();
    if (!config) return 15;
    const sizeStr = config.theme.overrides.typography?.["body-size"];
    if (sizeStr) return parseInt(sizeStr, 10);
    return 15;
  };

  const currentAccent = () => {
    const config = appConfig();
    if (!config) return "#007aff";
    return config.theme.overrides.colors?.accent || "#007aff";
  };

  return (
    <Show when={showSettings()}>
      <div class="settings-overlay" onClick={() => setShowSettings(false)}>
        <div class="settings-panel" onClick={(e) => e.stopPropagation()}>
          <div class="settings-header">
            <h2>Settings</h2>
            <button class="settings-close" onClick={() => setShowSettings(false)}>
              &times;
            </button>
          </div>
          <div class="settings-body">
            <div class="settings-section">
              <h3>Appearance</h3>
              <div class="setting-row">
                <label>Theme</label>
                <select
                  value={appConfig()?.theme.active || "auto"}
                  onChange={(e) => setThemeActive(e.currentTarget.value)}
                >
                  <option value="auto">Auto (System)</option>
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                </select>
              </div>
              <div class="setting-row">
                <label>Font Size: {currentFontSize()}px</label>
                <input
                  type="range"
                  min="12"
                  max="24"
                  value={currentFontSize()}
                  onInput={(e) => setFontSize(e.currentTarget.value)}
                />
              </div>
              <div class="setting-row">
                <label>Accent Color</label>
                <input
                  type="color"
                  value={currentAccent()}
                  onInput={(e) => setAccentColor(e.currentTarget.value)}
                />
              </div>
            </div>
            <div class="settings-section">
              <h3>Rendering</h3>
              <div class="setting-row">
                <label>Math (LaTeX)</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_math ? "on" : ""}`}
                  onClick={() => toggleRendering("render_math")}
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label>Diagrams (Mermaid)</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_diagrams ? "on" : ""}`}
                  onClick={() => toggleRendering("render_diagrams")}
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label>Frontmatter</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_frontmatter ? "on" : ""}`}
                  onClick={() => toggleRendering("render_frontmatter")}
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label>Line Numbers</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.show_line_numbers ? "on" : ""}`}
                  onClick={() => toggleRendering("show_line_numbers")}
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label>Wiki-links</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_wikilinks ? "on" : ""}`}
                  onClick={() => toggleRendering("render_wikilinks")}
                >
                  <span class="toggle-knob" />
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Show>
  );
};

export default SettingsPanel;
