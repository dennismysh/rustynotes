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
