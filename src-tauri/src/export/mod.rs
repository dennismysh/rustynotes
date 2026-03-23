pub mod html;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExportOptions {
    pub format: String,
    pub include_theme: bool,
}

pub trait Exporter {
    fn export(&self, markdown: &str, options: &ExportOptions) -> Result<Vec<u8>, String>;
    fn file_extension(&self) -> &str;
    fn mime_type(&self) -> &str;
}

pub fn get_exporter(format: &str) -> Option<Box<dyn Exporter>> {
    match format {
        "html" => Some(Box::new(html::HtmlExporter)),
        _ => None,
    }
}
