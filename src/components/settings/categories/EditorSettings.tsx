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
