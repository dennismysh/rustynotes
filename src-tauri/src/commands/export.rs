use crate::export::{self, ExportOptions};

#[tauri::command]
pub fn export_file(
    markdown: String,
    output_path: String,
    format: String,
    include_theme: bool,
) -> Result<(), String> {
    let exporter = export::get_exporter(&format)
        .ok_or_else(|| format!("Unsupported export format: {}", format))?;
    let options = ExportOptions {
        format,
        include_theme,
    };
    let output = exporter.export(&markdown, &options)?;
    std::fs::write(&output_path, &output)
        .map_err(|e| format!("Failed to write export file: {}", e))?;
    Ok(())
}
