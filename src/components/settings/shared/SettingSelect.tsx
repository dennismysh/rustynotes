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
