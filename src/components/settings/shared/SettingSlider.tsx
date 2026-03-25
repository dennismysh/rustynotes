interface SettingSliderProps {
  value: number;
  min: number;
  max: number;
  step?: number;
  unit?: string;
  onChange: (value: number) => void;
}

export function SettingSlider(props: SettingSliderProps) {
  return (
    <div class="setting-slider">
      <input
        type="range"
        min={props.min}
        max={props.max}
        step={props.step ?? 1}
        value={props.value}
        onInput={(e) => props.onChange(Number(e.currentTarget.value))}
      />
      <span class="setting-slider-value">
        {props.value}{props.unit ?? ""}
      </span>
    </div>
  );
}
