# Settings Window Design

Replace the current 360px right-sliding settings panel with a dedicated macOS-native settings window — a separate Tauri WebView window with sidebar category navigation and a detail pane.

## Motivation

As RustyNotes expands with more features, the narrow slide-over panel can't accommodate the growing number of settings categories. A separate window follows macOS conventions (Cmd+, to open), gives settings the space they need, and lets users tweak settings while viewing the editor side-by-side.

## Window Structure

- **Separate Tauri WebView window**, 700x500px default, resizable, minimum 500x350
- **Singleton**: only one settings window at a time. Cmd+, opens or focuses it.
- **Two-column layout**: 200px sidebar (category list) + flexible detail pane (settings content)
- **Standard macOS window chrome**. Title: "Settings"
- **Closing destroys the window** (no hidden window lingering). Reopening creates fresh.
- **Sidebar only shows implemented categories** (initially: Appearance, Editor, Preview, Advanced)

## Layout

```
┌──────────────────────────────────────────────┐
│  Settings                          (window)  │
├──────────────┬───────────────────────────────┤
│              │                               │
│  🎨 Appear.  │  Appearance                   │
│  ✏️ Editor   │  Theme, colors, and typography │
│  👁 Preview  │                               │
│  🔌 Advanced │  ┌─────────────┬────────────┐ │
│              │  │ Theme       │ Auto ▾     │ │
│              │  ├─────────────┼────────────┤ │
│              │  │ Accent Color│ ● ● ● ● ● │ │
│              │  ├─────────────┼────────────┤ │
│              │  │ Font Size   │ A ─●── A   │ │
│              │  └─────────────┴────────────┘ │
│              │                               │
└──────────────┴───────────────────────────────┘
```

- Sidebar: flat list with icons, active item highlighted, vertically scrollable
- Detail pane: scrollable, each setting is a row (label + description on left, control on right)
- Category heading + subtitle at top of detail pane

## Settings Categories

### Initial categories

Settings marked *(existing)* are migrated from the current panel. Settings marked *(new)* are added as part of this work.

| Category | Settings |
|----------|----------|
| **Appearance** | Theme *(existing)*, accent color *(existing)*, font size *(existing)*, editor font *(new)*, line height *(new)* |
| **Editor** | Mode *(existing)*, navigation mode *(existing)* |
| **Preview** | Math rendering *(existing)*, diagrams *(existing)*, frontmatter display *(existing)*, line numbers *(existing)*, wikilinks *(existing)* |
| **Advanced** | Reset onboarding tips *(existing, moved from its own section)* |

Editor font and line height require adding new fields to `AppConfig` (both Rust struct and TypeScript interface):

- **`editor_font`**: `String`, default `""` (empty string = system monospace). Freeform text input — user types a font family name. Displayed in a text field, not a constrained dropdown.
- **`line_height`**: `f64`, unitless multiplier, default `1.6`. Dropdown with presets: 1.2 (Compact), 1.4 (Normal), 1.6 (Comfortable), 1.8 (Relaxed).

The Advanced category starts with just the onboarding reset button — it's a natural home for it and avoids silently dropping an existing feature.

### Future (architecture supports, not built yet)

| Category | Potential settings |
|----------|-------------------|
| **Files** | Default folder, auto-save, file sorting, backup |
| **Keyboard** | Shortcut customization, vim mode |
| **Advanced** (expand) | Developer options, debug info, export/import config |

## Architecture

### Multi-Window Communication

Both windows share the same Rust backend. State syncs via Tauri events.

**Signal reconciliation and immediate persistence:** The current codebase keeps `editorMode` and `navMode` as independent Solid.js signals in `state.ts`, separate from `appConfig`. In the current panel, toggling these updates the local signal without persisting. The new architecture changes this behavior: **all settings changes are persisted immediately via `save_config_cmd`**. The settings window reads current values from `get_config()` on mount and writes via `save_config_cmd` on every change. The main window's `config-changed` handler is the sole mechanism for syncing — it must call `setEditorMode(config.editor_mode)`, `setNavMode(config.nav_mode)`, and `applyTheme(config)` to ensure all state is consistent. This is a deliberate behavioral change: mode changes now persist immediately rather than only when other settings are saved.

```
┌─────────────────┐         ┌─────────────────┐
│   Main Window    │         │ Settings Window  │
│   (editor)       │         │ (sidebar+detail) │
│                  │         │                  │
│  listens:        │         │  calls:          │
│  "config-changed"│◄────────│  save_config_cmd()│
│                  │  event  │                   │
│  reloads config  │         │  calls:           │
│  reapplies theme │         │  get_config()     │
└────────┬─────────┘         └────────┬──────────┘
         │                            │
         └──────────┬─────────────────┘
                    │
            ┌───────▼───────┐
            │ Rust Backend  │
            │               │
            │ get_config()  │
            │ save_config_cmd()│
            │ open_settings()│
            │               │
            │ AppConfig     │
            │ (disk persist)│
            └───────────────┘
```

### Window Lifecycle

1. User presses Cmd+, (or clicks toolbar button, or uses menu item)
2. Calls Rust command `open_settings`
3. Rust creates settings window via `WebviewWindowBuilder` with label `"settings"` — if it already exists, focuses it
4. Settings window loads `index.html#/settings` (hash routing within the same SPA)
5. Settings window calls `get_config()` on mount, renders current values
6. On change: calls `save_config_cmd()`, Rust persists and emits `config-changed` event
7. Main window receives event, reloads config, updates local signals (`editorMode`, `navMode`, etc.), and applies changes live
8. User closes settings window — window is destroyed

### Routing

The project currently has no router. Add `@solidjs/router` with hash-mode integration.

The Solid.js app uses hash-based routing to distinguish the two entry points:

- `index.html` or `index.html#/` — main editor
- `index.html#/settings` — settings window

In `index.tsx`, wrap the app with `<HashRouter>` from `@solidjs/router`. Define two routes: `/` renders the existing `App` component (editor), `/settings` renders the new `SettingsWindow` component. The settings window only loads settings-related components.

### New Rust Code

- **`open_settings` command**: Creates or focuses the settings window. Uses `WebviewWindowBuilder` with fixed label `"settings"`, title `"Settings"`, size 700x500.
- **`config-changed` event**: Emitted by `save_config_cmd` after persisting. Payload is the full `AppConfig`.

### New Frontend Components

Each category is its own Solid.js component:

```
src/components/settings/
  SettingsWindow.tsx      -- root layout (sidebar + detail pane)
  SettingsSidebar.tsx     -- category list
  categories/
    AppearanceSettings.tsx
    EditorSettings.tsx
    PreviewSettings.tsx
    AdvancedSettings.tsx
  shared/
    SettingRow.tsx         -- reusable row (label + description + control)
    SettingToggle.tsx      -- toggle switch
    SettingSelect.tsx      -- dropdown
    SettingSlider.tsx      -- range slider
    SettingColorPicker.tsx -- color picker
```

Category registry: an array of `{ id, label, icon, component }` objects. Adding a category = adding one entry + one component file.

## Migration

### Removed

- `src/components/settings/SettingsPanel.tsx` — deleted entirely
- `appState.showSettings` signal from `state.ts`
- Settings overlay CSS from `base.css` (`.settings-overlay`, `.settings-panel`, `.settings-header`, `.settings-body`, `.settings-section`, `.setting-row`, `.toggle-switch`)

### Changed

- Toolbar settings button: calls `open_settings` IPC command instead of toggling `showSettings`
- `save_config_cmd` Rust command: now also emits `config-changed` event after persisting

### Unchanged

- Config persistence logic (`config.rs`)
- Theme application (`theme.ts`, `applyTheme`)
- CSS custom property system
- All other Tauri commands

### Extended

- `AppConfig` struct (Rust) and TypeScript interface — add `editor_font: String` and `line_height: f64` fields with `#[serde(default)]` for backward compatibility with existing config files
- `save_config_cmd` Rust command — function signature gains `app: AppHandle` parameter to emit the `config-changed` event

## Design Decisions

**Why destroy on close instead of hide?** Simpler lifecycle — no stale state, no memory for an invisible window. Settings opens fast enough that re-creation is imperceptible.

**Why hash routing instead of a separate HTML entry point?** Shares the same Vite build output, CSS, and Tauri command bindings. No duplicate bundling. The router adds negligible overhead.

**Why rebuild controls instead of extracting from SettingsPanel?** The current controls are interleaved with panel-specific layout logic. Clean components in the new structure are easier to maintain and extend than adapted extractions.

**Why flat sidebar instead of nested/grouped?** Flat scales to ~10 categories before needing groups. By the time nesting is needed, the category list will have stabilized enough to design good groupings. YAGNI for now.
