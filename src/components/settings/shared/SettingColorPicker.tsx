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
