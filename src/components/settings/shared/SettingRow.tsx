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
