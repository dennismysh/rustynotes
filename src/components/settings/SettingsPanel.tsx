import { Component, Show, onMount, onCleanup } from "solid-js";
import { appState, type EditorMode, type NavMode } from "../../lib/state";
import { saveConfig, type AppConfig } from "../../lib/ipc";
import { applyTheme, resolveTheme } from "../../lib/theme";

const SettingsPanel: Component = () => {
  const { appConfig, setAppConfig, showSettings, setShowSettings, editorMode, setEditorMode, navMode, setNavMode } = appState;

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

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      setShowSettings(false);
    }
  };

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
  });
  onCleanup(() => {
    document.removeEventListener("keydown", handleKeyDown);
  });

  return (
    <Show when={showSettings()}>
      <div class="settings-overlay" onClick={() => setShowSettings(false)}>
        <div
          class="settings-panel"
          onClick={(e) => e.stopPropagation()}
          role="dialog"
          aria-modal="true"
          aria-label="Settings"
        >
          <div class="settings-header">
            <h2>Settings</h2>
            <button
              class="settings-close"
              onClick={() => setShowSettings(false)}
              aria-label="Close settings"
            >
              &times;
            </button>
          </div>
          <div class="settings-body">
            <div class="settings-section">
              <h3>Editor</h3>
              <div class="setting-row">
                <label for="settings-editor-mode">Mode</label>
                <select
                  id="settings-editor-mode"
                  value={editorMode()}
                  onChange={(e) => setEditorMode(e.currentTarget.value as EditorMode)}
                >
                  <option value="wysiwyg">WYSIWYG</option>
                  <option value="source">Source</option>
                  <option value="split">Split</option>
                  <option value="preview">Preview</option>
                </select>
              </div>
              <div class="setting-row">
                <label for="settings-nav-mode">Navigation</label>
                <select
                  id="settings-nav-mode"
                  value={navMode()}
                  onChange={(e) => setNavMode(e.currentTarget.value as NavMode)}
                >
                  <option value="sidebar">Tree</option>
                  <option value="miller">Miller Columns</option>
                  <option value="breadcrumb">Breadcrumb</option>
                </select>
              </div>
            </div>
            <div class="settings-section">
              <h3>Appearance</h3>
              <div class="setting-row">
                <label for="settings-theme">Theme</label>
                <select
                  id="settings-theme"
                  value={appConfig()?.theme.active || "auto"}
                  onChange={(e) => setThemeActive(e.currentTarget.value)}
                >
                  <option value="auto">Auto (System)</option>
                  <option value="light">Light</option>
                  <option value="dark">Dark</option>
                </select>
              </div>
              <div class="setting-row">
                <label for="settings-font-size">Font Size: {currentFontSize()}px</label>
                <input
                  id="settings-font-size"
                  type="range"
                  min="12"
                  max="24"
                  value={currentFontSize()}
                  onInput={(e) => setFontSize(e.currentTarget.value)}
                />
              </div>
              <div class="setting-row">
                <label for="settings-accent">Accent Color</label>
                <input
                  id="settings-accent"
                  type="color"
                  value={currentAccent()}
                  onInput={(e) => setAccentColor(e.currentTarget.value)}
                />
              </div>
            </div>
            <div class="settings-section">
              <h3>Rendering</h3>
              <div class="setting-row">
                <label id="label-math">Math (LaTeX)</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_math ? "on" : ""}`}
                  onClick={() => toggleRendering("render_math")}
                  role="switch"
                  aria-checked={appConfig()?.rendering.render_math ?? false}
                  aria-labelledby="label-math"
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label id="label-diagrams">Diagrams (Mermaid)</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_diagrams ? "on" : ""}`}
                  onClick={() => toggleRendering("render_diagrams")}
                  role="switch"
                  aria-checked={appConfig()?.rendering.render_diagrams ?? false}
                  aria-labelledby="label-diagrams"
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label id="label-frontmatter">Frontmatter</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_frontmatter ? "on" : ""}`}
                  onClick={() => toggleRendering("render_frontmatter")}
                  role="switch"
                  aria-checked={appConfig()?.rendering.render_frontmatter ?? false}
                  aria-labelledby="label-frontmatter"
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label id="label-linenums">Line Numbers</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.show_line_numbers ? "on" : ""}`}
                  onClick={() => toggleRendering("show_line_numbers")}
                  role="switch"
                  aria-checked={appConfig()?.rendering.show_line_numbers ?? false}
                  aria-labelledby="label-linenums"
                >
                  <span class="toggle-knob" />
                </button>
              </div>
              <div class="setting-row">
                <label id="label-wikilinks">Wiki-links</label>
                <button
                  class={`toggle-switch ${appConfig()?.rendering.render_wikilinks ? "on" : ""}`}
                  onClick={() => toggleRendering("render_wikilinks")}
                  role="switch"
                  aria-checked={appConfig()?.rendering.render_wikilinks ?? false}
                  aria-labelledby="label-wikilinks"
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
