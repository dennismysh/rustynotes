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
