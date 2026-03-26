use rustynotes_common::{ThemeData, ThemeOverrides};
use wasm_bindgen::JsCast;

const LIGHT_THEME_JSON: &str = include_str!("../../../styles/themes/default-light.json");
const DARK_THEME_JSON: &str = include_str!("../../../styles/themes/default-dark.json");

/// Parse a theme JSON string into a `ThemeData`.
pub fn load_theme(json: &str) -> ThemeData {
    serde_json::from_str(json).expect("theme JSON must be valid")
}

/// Detect the system color-scheme preference via `matchMedia`.
pub fn get_system_theme() -> &'static str {
    let window = web_sys::window().expect("no global `window`");
    match window.match_media("(prefers-color-scheme: dark)") {
        Ok(Some(mql)) if mql.matches() => "dark",
        _ => "light",
    }
}

/// Resolve a theme name (including `"auto"`) to a concrete `ThemeData`.
pub fn resolve_theme(active: &str) -> ThemeData {
    let key = if active == "auto" {
        get_system_theme()
    } else {
        active
    };

    match key {
        "dark" => load_theme(DARK_THEME_JSON),
        _ => load_theme(LIGHT_THEME_JSON),
    }
}

/// Map theme typography keys to CSS custom property names.
/// Matches the mapping in the existing TypeScript `theme.ts`.
fn typography_css_prop(key: &str) -> String {
    match key {
        "body-font" => "--font-body".to_string(),
        "body-size" => "--font-size".to_string(),
        "mono-font" => "--font-mono".to_string(),
        "line-height" => "--line-height".to_string(),
        other => format!("--{other}"),
    }
}

/// Apply a resolved theme (and optional overrides) to the document root as
/// CSS custom properties. This mirrors the behaviour of `applyTheme` in
/// `src/lib/theme.ts`.
pub fn apply_theme(theme: &ThemeData, overrides: Option<&ThemeOverrides>) {
    let window = web_sys::window().expect("no global `window`");
    let document = window.document().expect("no document");
    let root = document.document_element().expect("no document element");
    let root: &web_sys::HtmlElement = root.unchecked_ref();
    let style = root.style();

    // --- base colors ---
    for (key, value) in &theme.colors {
        let _ = style.set_property(&format!("--{key}"), value);
    }

    // --- base typography (with key mapping) ---
    for (key, value) in &theme.typography {
        let prop = typography_css_prop(key);
        let _ = style.set_property(&prop, value);
    }

    // --- base spacing ---
    for (key, value) in &theme.spacing {
        let _ = style.set_property(&format!("--{key}"), value);
    }

    // --- overrides ---
    if let Some(ov) = overrides {
        for (k, v) in &ov.colors {
            let _ = style.set_property(&format!("--{k}"), v);
        }
        for (k, v) in &ov.typography {
            let prop = typography_css_prop(k);
            let _ = style.set_property(&prop, v);
        }
        for (k, v) in &ov.spacing {
            let _ = style.set_property(&format!("--{k}"), v);
        }
    }

    // --- sync meta theme-color ---
    let bg_secondary = overrides
        .and_then(|ov| ov.colors.get("bg-secondary"))
        .or_else(|| theme.colors.get("bg-secondary"));
    if let Some(color) = bg_secondary {
        if let Ok(Some(meta)) = document.query_selector(r#"meta[name="theme-color"]"#) {
            let _ = meta.set_attribute("content", color);
        }
    }
}
