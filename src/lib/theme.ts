import defaultLight from "../styles/themes/default-light.json";
import defaultDark from "../styles/themes/default-dark.json";

export interface Theme {
  name: string;
  colors: Record<string, string>;
  typography: Record<string, string>;
  spacing: Record<string, string>;
}

export const themes: Record<string, Theme> = {
  light: defaultLight as Theme,
  dark: defaultDark as Theme,
};

export function applyTheme(theme: Theme, overrides?: { colors?: Record<string, string>; typography?: Record<string, string>; spacing?: Record<string, string> }): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.colors)) root.style.setProperty(`--${key}`, value);
  for (const [key, value] of Object.entries(theme.typography)) {
    if (key === "body-font") root.style.setProperty("--font-body", value);
    if (key === "body-size") root.style.setProperty("--font-size", value);
    if (key === "mono-font") root.style.setProperty("--font-mono", value);
    if (key === "line-height") root.style.setProperty("--line-height", value);
  }
  for (const [key, value] of Object.entries(theme.spacing)) root.style.setProperty(`--${key}`, value);
  if (overrides?.colors) for (const [k, v] of Object.entries(overrides.colors)) root.style.setProperty(`--${k}`, v);
  if (overrides?.typography) for (const [k, v] of Object.entries(overrides.typography)) root.style.setProperty(`--${k}`, v);
  if (overrides?.spacing) for (const [k, v] of Object.entries(overrides.spacing)) root.style.setProperty(`--${k}`, v);
}

export function getSystemTheme(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

export function resolveTheme(active: string): Theme {
  if (active === "auto") return themes[getSystemTheme()];
  return themes[active] || themes.light;
}
