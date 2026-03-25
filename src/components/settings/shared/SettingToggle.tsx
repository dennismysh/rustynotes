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
