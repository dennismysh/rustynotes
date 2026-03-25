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
