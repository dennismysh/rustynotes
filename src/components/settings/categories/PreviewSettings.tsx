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
