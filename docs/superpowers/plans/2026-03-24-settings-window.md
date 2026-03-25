# Settings Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the 360px slide-over settings panel with a dedicated macOS-native settings window (separate Tauri WebView) featuring sidebar category navigation.

**Architecture:** A second Tauri WebView window opened via `WebviewWindowBuilder`, sharing the Rust backend with the main editor window. Hash-based routing (`@solidjs/router`) distinguishes the two entry points within one SPA build. Config changes sync between windows via Tauri events.

**Tech Stack:** Tauri 2 multi-window, Solid.js, `@solidjs/router` (hash mode), existing CSS custom property theming

---

## File Map

### New Files

| File | Responsibility |
|------|---------------|
| `src/components/settings/SettingsWindow.tsx` | Root layout for settings window (sidebar + detail pane) |
| `src/components/settings/SettingsSidebar.tsx` | Category list with icons, active highlighting |
| `src/components/settings/categories/AppearanceSettings.tsx` | Theme, accent color, font size, editor font, line height |
| `src/components/settings/categories/EditorSettings.tsx` | Editor mode, navigation mode |
| `src/components/settings/categories/PreviewSettings.tsx` | 5 rendering toggles |
| `src/components/settings/categories/AdvancedSettings.tsx` | Reset onboarding button |
| `src/components/settings/shared/SettingRow.tsx` | Reusable row: label + description + control slot |
| `src/components/settings/shared/SettingToggle.tsx` | Toggle switch control |
| `src/components/settings/shared/SettingSelect.tsx` | Dropdown select control |
| `src/components/settings/shared/SettingSlider.tsx` | Range slider control |
| `src/components/settings/shared/SettingColorPicker.tsx` | Color input control |
| `src/styles/settings.css` | All settings window styles (sidebar, detail pane, controls) |

### Modified Files

| File | Changes |
|------|---------|
| `src-tauri/src/config.rs` | Add `editor_font: String` and `line_height: f64` fields to `AppConfig` |
| `src-tauri/src/commands/config.rs` | Add `AppHandle` param to `save_config_cmd`, emit `config-changed` event; add `open_settings` command |
| `src-tauri/src/lib.rs` | Register `open_settings` command in invoke handler |
| `src/lib/ipc.ts` | Add `editor_font` and `line_height` to `AppConfig` interface; add `openSettings()` function; add `onConfigChanged()` event listener |
| `src/lib/state.ts` | Remove `showSettings` signal |
| `src/index.tsx` | Add `@solidjs/router` `HashRouter` with `/` and `/settings` routes |
| `src/components/Toolbar.tsx` | Replace `setShowSettings(true)` with `openSettings()` IPC call |
| `src/App.tsx` | Add `config-changed` event listener that syncs signals + reapplies theme |
| `src/styles/base.css` | Remove settings panel CSS (`.settings-overlay`, `.settings-panel`, etc.) |
| `package.json` | Add `@solidjs/router` dependency |

### Deleted Files

| File | Reason |
|------|--------|
| `src/components/settings/SettingsPanel.tsx` | Replaced entirely by new settings window components |

---

## Task 1: Extend AppConfig with new fields (Rust)

**Files:**
- Modify: `src-tauri/src/config.rs` (lines 5-17, the `AppConfig` struct)

- [ ] **Step 1: Add `editor_font` and `line_height` fields to `AppConfig`**

In `src-tauri/src/config.rs`, add two fields to the `AppConfig` struct after `nav_mode`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub theme: ThemeConfig,
    #[serde(default = "default_editor_mode")]
    pub editor_mode: String,
    #[serde(default = "default_nav_mode")]
    pub nav_mode: String,
    #[serde(default)]
    pub editor_font: String,
    #[serde(default = "default_line_height")]
    pub line_height: f64,
    #[serde(default)]
    pub rendering: RenderingToggles,
    #[serde(default)]
    pub recent_folders: Vec<String>,
}
```

Add the default function near the other default functions (around line 53):

```rust
fn default_line_height() -> f64 {
    1.6
}
```

The `editor_font` field uses `#[serde(default)]` which gives `String::default()` (empty string), matching the spec.

Also update the manual `Default` impl for `AppConfig` (around line 56-66) to include the new fields:

```rust
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemeConfig::default(),
            editor_mode: default_editor_mode(),
            nav_mode: default_nav_mode(),
            editor_font: String::default(),
            line_height: default_line_height(),
            rendering: RenderingToggles::default(),
            recent_folders: Vec::new(),
        }
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors. Existing config files deserialize fine due to `#[serde(default)]`.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/config.rs
git commit -m "feat: add editor_font and line_height fields to AppConfig"
```

---

## Task 2: Add `open_settings` command and `config-changed` event (Rust)

**Files:**
- Modify: `src-tauri/src/commands/config.rs`
- Modify: `src-tauri/src/lib.rs` (line 42-46, invoke handler registration)

- [ ] **Step 1: Modify `save_config_cmd` to emit `config-changed` event**

In `src-tauri/src/commands/config.rs`, add `AppHandle` parameter and emit the event after saving:

```rust
use crate::config::{self, AppConfig};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager};

pub struct ConfigState {
    pub config: Mutex<AppConfig>,
}

#[tauri::command]
pub fn get_config(state: tauri::State<ConfigState>) -> AppConfig {
    state.config.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_config_cmd(
    app: AppHandle,
    config_data: AppConfig,
    state: tauri::State<ConfigState>,
) -> Result<(), String> {
    config::save_config(&config_data)?;
    *state.config.lock().unwrap() = config_data.clone();
    let _ = app.emit("config-changed", config_data);
    Ok(())
}

#[tauri::command]
pub fn open_settings(app: AppHandle) -> Result<(), String> {
    use tauri::WebviewWindowBuilder;

    // If settings window already exists, focus it
    if let Some(window) = app.get_webview_window("settings") {
        let _ = window.set_focus();
        return Ok(());
    }

    // Create new settings window
    WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("index.html#/settings".into()))
        .title("Settings")
        .inner_size(700.0, 500.0)
        .min_inner_size(500.0, 350.0)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}
```

- [ ] **Step 2: Register `open_settings` in the invoke handler**

In `src-tauri/src/lib.rs`, add `commands::config::open_settings` to the `.invoke_handler(tauri::generate_handler![...])` list (around line 42-46).

- [ ] **Step 3: Add `settings` window to Tauri capabilities**

In `src-tauri/capabilities/default.json`, add `"settings"` to the `windows` array so the settings window has permission to call IPC commands:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main and settings windows",
  "windows": ["main", "settings"],
  "permissions": [
    "core:default",
    "core:window:allow-show",
    "opener:default",
    "dialog:allow-open",
    "dialog:allow-save"
  ]
}
```

Without this, the settings window will render but all `invoke()` calls will fail.

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: compiles with no errors. The `Emitter` trait import enables `app.emit()`. The `WebviewWindowBuilder` and `get_webview_window` are from `tauri` directly.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/config.rs src-tauri/src/lib.rs src-tauri/capabilities/default.json
git commit -m "feat: add open_settings command, config-changed event, and settings window capability"
```

---

## Task 3: Install `@solidjs/router` and add hash routing

**Files:**
- Modify: `package.json` (add dependency)
- Modify: `src/index.tsx`

- [ ] **Step 1: Install the router**

Run: `pnpm add @solidjs/router`

- [ ] **Step 2: Add hash routing to `index.tsx`**

Replace the current `src/index.tsx` (4 lines) with:

```tsx
import { render } from "solid-js/web";
import { HashRouter, Route } from "@solidjs/router";
import App from "./App";
import { SettingsWindow } from "./components/settings/SettingsWindow";

render(
  () => (
    <HashRouter>
      <Route path="/" component={App} />
      <Route path="/settings" component={SettingsWindow} />
    </HashRouter>
  ),
  document.getElementById("root") as HTMLElement,
);
```

Note: `SettingsWindow` doesn't exist yet — this will have a TypeScript error until Task 7. That's fine; we're building bottom-up.

- [ ] **Step 3: Commit**

```bash
git add package.json pnpm-lock.yaml src/index.tsx
git commit -m "feat: add @solidjs/router with hash routing for multi-window support"
```

---

## Task 4: Update TypeScript IPC layer

**Files:**
- Modify: `src/lib/ipc.ts` (lines 57-63 for `AppConfig` interface, add new functions)

- [ ] **Step 1: Extend `AppConfig` interface and add new IPC functions**

In `src/lib/ipc.ts`:

Add `editor_font` and `line_height` to the `AppConfig` interface:

```typescript
export interface AppConfig {
  theme: {
    active: string;
    overrides: {
      colors: Record<string, string>;
      typography: Record<string, string>;
      spacing: Record<string, string>;
    };
  };
  editor_mode: string;
  nav_mode: string;
  editor_font: string;
  line_height: number;
  rendering: {
    render_math: boolean;
    render_diagrams: boolean;
    render_frontmatter: boolean;
    show_line_numbers: boolean;
    render_wikilinks: boolean;
  };
  recent_folders: string[];
}
```

Add `openSettings()` and `onConfigChanged()` functions after the existing exports:

```typescript
export async function openSettings(): Promise<void> {
  return invoke("open_settings");
}

export function onConfigChanged(
  callback: (config: AppConfig) => void,
): Promise<() => void> {
  return listen<AppConfig>("config-changed", (event) => {
    callback(event.payload);
  });
}
```

Make sure `listen` is imported at the top (alongside `invoke` from `@tauri-apps/api/core`):

```typescript
import { listen } from "@tauri-apps/api/event";
```

- [ ] **Step 2: Commit**

```bash
git add src/lib/ipc.ts
git commit -m "feat: extend AppConfig interface and add openSettings/onConfigChanged IPC"
```

---

## Task 5: Build shared setting controls

**Files:**
- Create: `src/components/settings/shared/SettingRow.tsx`
- Create: `src/components/settings/shared/SettingToggle.tsx`
- Create: `src/components/settings/shared/SettingSelect.tsx`
- Create: `src/components/settings/shared/SettingSlider.tsx`
- Create: `src/components/settings/shared/SettingColorPicker.tsx`

- [ ] **Step 1: Create `SettingRow.tsx`**

```tsx
import type { JSX } from "solid-js";

interface SettingRowProps {
  label: string;
  description?: string;
  children: JSX.Element;
}

export function SettingRow(props: SettingRowProps) {
  return (
    <div class="setting-row">
      <div class="setting-info">
        <div class="setting-label">{props.label}</div>
        {props.description && (
          <div class="setting-description">{props.description}</div>
        )}
      </div>
      <div class="setting-control">{props.children}</div>
    </div>
  );
}
```

- [ ] **Step 2: Create `SettingToggle.tsx`**

```tsx
interface SettingToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
}

export function SettingToggle(props: SettingToggleProps) {
  return (
    <button
      class={`toggle-switch ${props.checked ? "on" : ""}`}
      onClick={() => props.onChange(!props.checked)}
      role="switch"
      aria-checked={props.checked}
    >
      <span class="toggle-knob" />
    </button>
  );
}
```

- [ ] **Step 3: Create `SettingSelect.tsx`**

```tsx
interface SettingSelectOption {
  value: string;
  label: string;
}

interface SettingSelectProps {
  value: string;
  options: SettingSelectOption[];
  onChange: (value: string) => void;
}

export function SettingSelect(props: SettingSelectProps) {
  return (
    <select
      class="setting-select"
      value={props.value}
      onChange={(e) => props.onChange(e.currentTarget.value)}
    >
      {props.options.map((opt) => (
        <option value={opt.value}>{opt.label}</option>
      ))}
    </select>
  );
}
```

- [ ] **Step 4: Create `SettingSlider.tsx`**

```tsx
interface SettingSliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  onChange: (value: number) => void;
}

export function SettingSlider(props: SettingSliderProps) {
  return (
    <div class="setting-slider">
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step ?? 1}
        value={props.value}
        onInput={(e) => props.onChange(Number(e.currentTarget.value))}
      />
      <span class="setting-slider-value">
        {props.value}{props.unit ?? ""}
      </span>
    </div>
  );
}
```

- [ ] **Step 5: Create `SettingColorPicker.tsx`**

```tsx
interface SettingColorPickerProps {
  value: string;
  onChange: (color: string) => void;
}

export function SettingColorPicker(props: SettingColorPickerProps) {
  return (
    <input
      type="color"
      class="setting-color"
      value={props.value}
      onInput={(e) => props.onChange(e.currentTarget.value)}
    />
  );
}
```

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/shared/
git commit -m "feat: add shared settings control components"
```

---

## Task 6: Build category page components

**Files:**
- Create: `src/components/settings/categories/AppearanceSettings.tsx`
- Create: `src/components/settings/categories/EditorSettings.tsx`
- Create: `src/components/settings/categories/PreviewSettings.tsx`
- Create: `src/components/settings/categories/AdvancedSettings.tsx`

- [ ] **Step 1: Create `AppearanceSettings.tsx`**

```tsx
import { createSignal, onMount } from "solid-js";
import { getConfig, saveConfig, type AppConfig } from "../../../lib/ipc";
import { applyTheme, resolveTheme } from "../../../lib/theme";
import { SettingRow } from "../shared/SettingRow";
import { SettingSelect } from "../shared/SettingSelect";
import { SettingSlider } from "../shared/SettingSlider";
import { SettingColorPicker } from "../shared/SettingColorPicker";

export function AppearanceSettings() {
  const [config, setConfig] = createSignal<AppConfig | null>(null);

  onMount(async () => {
    setConfig(await getConfig());
  });

  async function update(updater: (c: AppConfig) => void) {
    const c = config();
    if (!c) return;
    const updated = structuredClone(c);
    updater(updated);
    setConfig(updated);
    await saveConfig(updated);
    applyTheme(resolveTheme(updated.theme.active), updated.theme.overrides);
  }

  return (
    <div class="settings-category">
      <h2 class="settings-category-title">Appearance</h2>
      <p class="settings-category-subtitle">Theme, colors, and typography</p>

      <SettingRow label="Theme" description="Follow system or choose manually">
        <SettingSelect
          value={config()?.theme.active ?? "auto"}
          options={[
            { value: "auto", label: "Auto (System)" },
            { value: "light", label: "Light" },
            { value: "dark", label: "Dark" },
          ]}
          onChange={(v) => update((c) => { c.theme.active = v; })}
        />
      </SettingRow>

      <SettingRow label="Accent Color" description="Used for links, selections, and highlights">
        <SettingColorPicker
          value={config()?.theme.overrides.colors.accent ?? "#89b4fa"}
          onChange={(v) =>
            update((c) => { c.theme.overrides.colors.accent = v; })
          }
        />
      </SettingRow>

      <SettingRow label="Font Size" description="Base size for editor content">
        <SettingSlider
          value={parseInt(config()?.theme.overrides.typography["body-size"] ?? "15")}
          min={12}
          max={24}
          unit="px"
          onChange={(v) =>
            update((c) => { c.theme.overrides.typography["body-size"] = `${v}px`; })
          }
        />
      </SettingRow>

      <SettingRow label="Editor Font" description="Font family for source editing (blank = system monospace)">
        <input
          type="text"
          class="setting-text-input"
          value={config()?.editor_font ?? ""}
          placeholder="System Default"
          onChange={(e) => update((c) => { c.editor_font = e.currentTarget.value; })}
        />
      </SettingRow>

      <SettingRow label="Line Height" description="Spacing between lines in the editor">
        <SettingSelect
          value={String(config()?.line_height ?? 1.6)}
          options={[
            { value: "1.2", label: "1.2 (Compact)" },
            { value: "1.4", label: "1.4 (Normal)" },
            { value: "1.6", label: "1.6 (Comfortable)" },
            { value: "1.8", label: "1.8 (Relaxed)" },
          ]}
          onChange={(v) => update((c) => { c.line_height = parseFloat(v); })}
        />
      </SettingRow>
    </div>
  );
}
```

- [ ] **Step 2: Create `EditorSettings.tsx`**

```tsx
import { createSignal, onMount } from "solid-js";
import { getConfig, saveConfig, type AppConfig } from "../../../lib/ipc";
import { SettingRow } from "../shared/SettingRow";
import { SettingSelect } from "../shared/SettingSelect";

export function EditorSettings() {
  const [config, setConfig] = createSignal<AppConfig | null>(null);

  onMount(async () => {
    setConfig(await getConfig());
  });

  async function update(updater: (c: AppConfig) => void) {
    const c = config();
    if (!c) return;
    const updated = structuredClone(c);
    updater(updated);
    setConfig(updated);
    await saveConfig(updated);
  }

  return (
    <div class="settings-category">
      <h2 class="settings-category-title">Editor</h2>
      <p class="settings-category-subtitle">Editing mode and navigation</p>

      <SettingRow label="Editor Mode" description="How you write and preview content">
        <SettingSelect
          value={config()?.editor_mode ?? "wysiwyg"}
          options={[
            { value: "wysiwyg", label: "Rich Text (WYSIWYG)" },
            { value: "source", label: "Markdown Source" },
            { value: "split", label: "Split View" },
            { value: "preview", label: "Preview Only" },
          ]}
          onChange={(v) => update((c) => { c.editor_mode = v; })}
        />
      </SettingRow>

      <SettingRow label="Navigation" description="How you browse files">
        <SettingSelect
          value={config()?.nav_mode ?? "sidebar"}
          options={[
            { value: "sidebar", label: "Sidebar Tree" },
            { value: "miller", label: "Miller Columns" },
            { value: "breadcrumb", label: "Breadcrumb Path" },
          ]}
          onChange={(v) => update((c) => { c.nav_mode = v; })}
        />
      </SettingRow>
    </div>
  );
}
```

- [ ] **Step 3: Create `PreviewSettings.tsx`**

```tsx
import { createSignal, onMount } from "solid-js";
import { getConfig, saveConfig, type AppConfig } from "../../../lib/ipc";
import { SettingRow } from "../shared/SettingRow";
import { SettingToggle } from "../shared/SettingToggle";

export function PreviewSettings() {
  const [config, setConfig] = createSignal<AppConfig | null>(null);

  onMount(async () => {
    setConfig(await getConfig());
  });

  async function update(updater: (c: AppConfig) => void) {
    const c = config();
    if (!c) return;
    const updated = structuredClone(c);
    updater(updated);
    setConfig(updated);
    await saveConfig(updated);
  }

  function toggleRendering(key: keyof AppConfig["rendering"]) {
    update((c) => { c.rendering[key] = !c.rendering[key]; });
  }

  return (
    <div class="settings-category">
      <h2 class="settings-category-title">Preview</h2>
      <p class="settings-category-subtitle">Markdown rendering options</p>

      <SettingRow label="Math Equations" description="Render LaTeX math with KaTeX">
        <SettingToggle
          checked={config()?.rendering.render_math ?? true}
          onChange={() => toggleRendering("render_math")}
        />
      </SettingRow>

      <SettingRow label="Diagrams" description="Render Mermaid diagrams">
        <SettingToggle
          checked={config()?.rendering.render_diagrams ?? true}
          onChange={() => toggleRendering("render_diagrams")}
        />
      </SettingRow>

      <SettingRow label="YAML Header" description="Show frontmatter metadata">
        <SettingToggle
          checked={config()?.rendering.render_frontmatter ?? true}
          onChange={() => toggleRendering("render_frontmatter")}
        />
      </SettingRow>

      <SettingRow label="Code Line Numbers" description="Show line numbers in code blocks">
        <SettingToggle
          checked={config()?.rendering.show_line_numbers ?? true}
          onChange={() => toggleRendering("show_line_numbers")}
        />
      </SettingRow>

      <SettingRow label="Wiki Links" description="Enable [[wikilink]] syntax">
        <SettingToggle
          checked={config()?.rendering.render_wikilinks ?? true}
          onChange={() => toggleRendering("render_wikilinks")}
        />
      </SettingRow>
    </div>
  );
}
```

- [ ] **Step 4: Create `AdvancedSettings.tsx`**

```tsx
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
```

- [ ] **Step 5: Commit**

```bash
git add src/components/settings/categories/
git commit -m "feat: add settings category page components"
```

---

## Task 7: Build SettingsSidebar and SettingsWindow

**Files:**
- Create: `src/components/settings/SettingsSidebar.tsx`
- Create: `src/components/settings/SettingsWindow.tsx`

- [ ] **Step 1: Create `SettingsSidebar.tsx`**

```tsx
import type { Component } from "solid-js";

export interface SettingsCategory {
  id: string;
  label: string;
  icon: string;
  component: Component;
}

interface SettingsSidebarProps {
  categories: SettingsCategory[];
  activeId: string;
  onSelect: (id: string) => void;
}

export function SettingsSidebar(props: SettingsSidebarProps) {
  return (
    <nav class="settings-sidebar">
      <div class="settings-sidebar-header">Settings</div>
      {props.categories.map((cat) => (
        <button
          class={`settings-sidebar-item ${props.activeId === cat.id ? "active" : ""}`}
          onClick={() => props.onSelect(cat.id)}
        >
          <span class="settings-sidebar-icon">{cat.icon}</span>
          <span class="settings-sidebar-label">{cat.label}</span>
        </button>
      ))}
    </nav>
  );
}
```

- [ ] **Step 2: Create `SettingsWindow.tsx`**

```tsx
import { createSignal, onMount } from "solid-js";
import { Dynamic } from "solid-js/web";
import { SettingsSidebar, type SettingsCategory } from "./SettingsSidebar";
import { AppearanceSettings } from "./categories/AppearanceSettings";
import { EditorSettings } from "./categories/EditorSettings";
import { PreviewSettings } from "./categories/PreviewSettings";
import { AdvancedSettings } from "./categories/AdvancedSettings";
import { getConfig } from "../../lib/ipc";
import { applyTheme, resolveTheme } from "../../lib/theme";
import "../../styles/settings.css";

const categories: SettingsCategory[] = [
  { id: "appearance", label: "Appearance", icon: "\u{1F3A8}", component: AppearanceSettings },
  { id: "editor", label: "Editor", icon: "\u{270F}\u{FE0F}", component: EditorSettings },
  { id: "preview", label: "Preview", icon: "\u{1F441}", component: PreviewSettings },
  { id: "advanced", label: "Advanced", icon: "\u{1F50C}", component: AdvancedSettings },
];

export function SettingsWindow() {
  const [activeCategory, setActiveCategory] = createSignal("appearance");

  onMount(async () => {
    // Load config and apply theme so settings window matches main window
    const config = await getConfig();
    applyTheme(resolveTheme(config.theme.active), config.theme.overrides);
  });

  const activeComponent = () =>
    categories.find((c) => c.id === activeCategory())?.component ?? AppearanceSettings;

  return (
    <div class="settings-window">
      <SettingsSidebar
        categories={categories}
        activeId={activeCategory()}
        onSelect={setActiveCategory}
      />
      <main class="settings-detail">
        <Dynamic component={activeComponent()} />
      </main>
    </div>
  );
}
```

- [ ] **Step 3: Commit**

```bash
git add src/components/settings/SettingsSidebar.tsx src/components/settings/SettingsWindow.tsx
git commit -m "feat: add SettingsWindow root layout with sidebar and category routing"
```

---

## Task 8: Add settings window styles

**Files:**
- Create: `src/styles/settings.css`

- [ ] **Step 1: Create `settings.css`**

```css
/* Settings Window Layout */
.settings-window {
  display: flex;
  height: 100vh;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: var(--font-body);
}

/* Sidebar */
.settings-sidebar {
  width: 200px;
  min-width: 200px;
  background: var(--bg-secondary);
  border-right: 1px solid var(--border);
  padding: var(--space-sm) 0;
  overflow-y: auto;
}

.settings-sidebar-header {
  padding: var(--space-sm) var(--space-md);
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: var(--space-xs);
}

.settings-sidebar-item {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  width: calc(100% - var(--space-md));
  margin: 2px var(--space-xs);
  padding: var(--space-xs) var(--space-md);
  border: none;
  border-radius: 6px;
  background: none;
  color: var(--text-secondary);
  font-size: 13px;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
}

.settings-sidebar-item:hover {
  background: var(--bg-tertiary);
}

.settings-sidebar-item.active {
  background: var(--bg-tertiary);
  color: var(--text-primary);
  font-weight: 500;
}

.settings-sidebar-icon {
  font-size: 15px;
  width: 20px;
  text-align: center;
}

.settings-sidebar-label {
  flex: 1;
}

/* Detail Pane */
.settings-detail {
  flex: 1;
  padding: var(--space-lg) var(--space-xl);
  overflow-y: auto;
}

/* Category Page */
.settings-category-title {
  font-size: 20px;
  font-weight: 600;
  margin: 0 0 var(--space-xs) 0;
  color: var(--text-primary);
}

.settings-category-subtitle {
  font-size: 12px;
  color: var(--text-muted);
  margin: 0 0 var(--space-lg) 0;
}

/* Setting Rows */
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-sm) 0;
  border-bottom: 1px solid var(--border);
}

.setting-info {
  flex: 1;
  min-width: 0;
}

.setting-label {
  font-weight: 500;
  color: var(--text-primary);
  font-size: 13px;
}

.setting-description {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}

.setting-control {
  flex-shrink: 0;
  margin-left: var(--space-md);
}

/* Controls */
.setting-select {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: var(--space-xs) var(--space-sm);
  color: var(--text-primary);
  font-size: 12px;
  min-width: 140px;
  font-family: inherit;
}

.setting-text-input {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: var(--space-xs) var(--space-sm);
  color: var(--text-primary);
  font-size: 12px;
  min-width: 140px;
  font-family: inherit;
}

.setting-text-input::placeholder {
  color: var(--text-muted);
}

.setting-slider {
  display: flex;
  align-items: center;
  gap: var(--space-xs);
}

.setting-slider input[type="range"] {
  width: 120px;
  accent-color: var(--accent);
}

.setting-slider-value {
  font-size: 12px;
  color: var(--text-secondary);
  min-width: 36px;
  text-align: right;
}

.setting-color {
  width: 32px;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 2px;
  cursor: pointer;
  background: none;
}

/* Toggle Switch (reused from existing design) */
.toggle-switch {
  position: relative;
  width: 40px;
  height: 22px;
  border-radius: 11px;
  border: none;
  background: var(--bg-tertiary);
  cursor: pointer;
  padding: 0;
  transition: background 0.2s;
}

.toggle-switch.on {
  background: var(--accent);
}

.toggle-knob {
  position: absolute;
  top: 3px;
  left: 3px;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: white;
  transition: transform 0.2s;
}

.toggle-switch.on .toggle-knob {
  transform: translateX(18px);
}

/* Reset Button */
.settings-reset-btn {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: var(--space-xs) var(--space-md);
  color: var(--text-primary);
  font-size: 12px;
  cursor: pointer;
  font-family: inherit;
}

.settings-reset-btn:hover {
  background: var(--bg-tertiary);
}
```

- [ ] **Step 2: Commit**

```bash
git add src/styles/settings.css
git commit -m "feat: add settings window styles"
```

---

## Task 9: Remove old settings panel, CSS, and signals

This task must be done as a unit to avoid compile errors — the panel, its CSS, its signal, and all references are removed together.

**Files:**
- Delete: `src/components/settings/SettingsPanel.tsx`
- Modify: `src/App.tsx` (remove SettingsPanel import and usage)
- Modify: `src/components/Toolbar.tsx` (remove `setShowSettings` usage)
- Modify: `src/lib/state.ts` (remove `showSettings` signal)
- Modify: `src/styles/base.css` (remove settings panel CSS)

- [ ] **Step 1: Remove SettingsPanel usage from App.tsx**

In `src/App.tsx`, remove the `<SettingsPanel />` component and its import. Remove the `<Show when={appState.showSettings()}>` wrapper if it exists. Also remove `showSettings` from the state destructuring if present.

- [ ] **Step 2: Remove `setShowSettings` from Toolbar.tsx**

In `src/components/Toolbar.tsx`, remove `setShowSettings` from the state destructuring (around line 7). Comment out or remove the settings button click handler temporarily — we'll rewire it in the next task.

- [ ] **Step 3: Remove `showSettings` signal from state.ts**

In `src/lib/state.ts`, remove the `showSettings` signal (line 16):
```typescript
// DELETE this line:
const [showSettings, setShowSettings] = createSignal(false);
```

And remove `showSettings` and `setShowSettings` from the returned object.

- [ ] **Step 4: Delete `SettingsPanel.tsx`**

```bash
rm src/components/settings/SettingsPanel.tsx
```

- [ ] **Step 5: Remove settings CSS from `base.css`**

In `src/styles/base.css`, remove the settings-related CSS block (approximately lines 664-814). This includes all rules for:
- `.settings-overlay`
- `.settings-panel`
- `.settings-header`
- `.settings-close`
- `.settings-body`
- `.settings-section`
- `.setting-row` (the old one — the new one lives in `settings.css`)
- `.settings-reset-btn` (the old one — new version in `settings.css`)
- `.toggle-switch` and `.toggle-knob` (old versions — new in `settings.css`)

Search for `/* Settings Panel */` or `.settings-overlay` to find the start of the block.

- [ ] **Step 6: Verify no stale references**

Run: `grep -r "settings-overlay\|settings-panel\|SettingsPanel\|showSettings\|setShowSettings" src/ --include="*.tsx" --include="*.ts"`

Expected: no matches.

- [ ] **Step 7: Commit**

```bash
git add -u src/components/settings/SettingsPanel.tsx src/App.tsx src/components/Toolbar.tsx src/lib/state.ts src/styles/base.css
git commit -m "refactor: remove old settings panel, its CSS, and showSettings signal"
```

---

## Task 10: Wire up toolbar, Cmd+, shortcut, and config-changed listener

**Files:**
- Modify: `src/components/Toolbar.tsx` (rewire settings button)
- Modify: `src/App.tsx` (add config-changed listener and Cmd+, shortcut)

- [ ] **Step 1: Update Toolbar to call `openSettings()`**

In `src/components/Toolbar.tsx`:

Add import at the top:
```tsx
import { openSettings } from "../lib/ipc";
```

Set the settings button click handler to:
```tsx
onClick={() => openSettings()}
```

- [ ] **Step 2: Add `config-changed` listener and Cmd+, shortcut in App.tsx**

In `src/App.tsx`, add these imports if not already present:

```tsx
import { onConfigChanged, openSettings, type AppConfig } from "./lib/ipc";
import { applyTheme, resolveTheme } from "./lib/theme";
```

Inside the `App` component, add to the `onMount` block (or extend the existing one):

```tsx
onMount(async () => {
  // ... existing onMount code ...

  // Listen for config changes from settings window
  await onConfigChanged((config) => {
    setAppConfig(config);
    setEditorMode(config.editor_mode as EditorMode);
    setNavMode(config.nav_mode as NavMode);
    applyTheme(resolveTheme(config.theme.active), config.theme.overrides);
  });

  // Cmd+, keyboard shortcut to open settings
  const handleKeydown = (e: KeyboardEvent) => {
    if (e.metaKey && e.key === ",") {
      e.preventDefault();
      openSettings();
    }
  };
  document.addEventListener("keydown", handleKeydown);
});
```

- [ ] **Step 3: Commit**

```bash
git add src/components/Toolbar.tsx src/App.tsx
git commit -m "feat: wire toolbar and Cmd+, shortcut to open settings, add config-changed listener"
```

---

## Task 11: Smoke test the full flow

- [ ] **Step 1: Start the dev server**

Run: `pnpm tauri dev`

- [ ] **Step 2: Verify settings window opens**

Click the settings gear icon in the toolbar. A separate "Settings" window should appear at 700x500px.

Verify:
- Window title is "Settings"
- Sidebar shows 4 categories: Appearance, Editor, Preview, Advanced
- Appearance is selected by default
- Theme, accent color, font size, editor font, and line height controls are visible

- [ ] **Step 3: Verify Cmd+, shortcut**

Press Cmd+, in the main window. The settings window should open (or focus if already open).

- [ ] **Step 4: Verify settings sync to main window**

In the settings window:
1. Change the theme from Auto to Dark — main window should update immediately
2. Change accent color — main window should reflect the new color
3. Change editor mode — main window should switch modes
4. Toggle a preview setting — the change should persist

- [ ] **Step 5: Verify singleton behavior**

Click the gear icon again while settings is open — should focus the existing window, not open a second one.

- [ ] **Step 6: Verify window close/reopen**

Close the settings window. Open it again. Verify it loads the current config values correctly.

- [ ] **Step 7: Commit any fixes, then final commit**

If everything works:
```bash
git add -A
git commit -m "feat: settings window complete — separate Tauri window with sidebar navigation"
```
