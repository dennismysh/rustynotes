use super::{ExportOptions, Exporter};
use crate::markdown_parser::MarkdownParser;

pub struct HtmlExporter;

impl HtmlExporter {
    fn default_css() -> &'static str {
        r#"
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
            font-size: 16px; line-height: 1.6; color: #1d1d1f;
            max-width: 800px; margin: 0 auto; padding: 40px 20px; background: #fff;
        }
        h1 { font-size: 2em; margin: 0.67em 0; }
        h2 { font-size: 1.5em; margin: 0.75em 0; }
        h3 { font-size: 1.17em; margin: 0.83em 0; }
        code { background: #f5f5f7; padding: 2px 6px; border-radius: 4px; font-family: 'SF Mono', monospace; font-size: 0.9em; }
        pre { background: #f5f5f7; border: 1px solid #d2d2d7; border-radius: 8px; padding: 16px; overflow-x: auto; }
        pre code { background: none; padding: 0; }
        blockquote { border-left: 3px solid #007aff; padding-left: 16px; color: #6e6e73; margin: 1em 0; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #d2d2d7; padding: 8px 12px; text-align: left; }
        th { background: #f5f5f7; font-weight: 600; }
        a { color: #007aff; text-decoration: none; }
        img { max-width: 100%; }
        "#
    }
}

impl Exporter for HtmlExporter {
    fn export(&self, markdown: &str, options: &ExportOptions) -> Result<Vec<u8>, String> {
        let body_html = MarkdownParser::parse(markdown);
        let html = if options.include_theme {
            format!(
                "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<title>Exported Document</title>\n<style>{}</style>\n</head>\n<body>\n{}\n</body>\n</html>",
                Self::default_css(),
                body_html
            )
        } else {
            format!(
                "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"UTF-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">\n<title>Exported Document</title>\n</head>\n<body>\n{}\n</body>\n</html>",
                body_html
            )
        };
        Ok(html.into_bytes())
    }

    fn file_extension(&self) -> &str {
        "html"
    }

    fn mime_type(&self) -> &str {
        "text/html"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_export_basic() {
        let exporter = HtmlExporter;
        let options = ExportOptions {
            format: "html".into(),
            include_theme: true,
        };
        let result = exporter.export("# Hello\n\nWorld", &options).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("<style>"));
    }

    #[test]
    fn test_html_export_no_theme() {
        let exporter = HtmlExporter;
        let options = ExportOptions {
            format: "html".into(),
            include_theme: false,
        };
        let result = exporter.export("# Hello", &options).unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(!html.contains("<style>"));
    }

    #[test]
    fn test_html_export_gfm() {
        let exporter = HtmlExporter;
        let options = ExportOptions {
            format: "html".into(),
            include_theme: true,
        };
        let result = exporter
            .export("| A | B |\n|---|---|\n| 1 | 2 |", &options)
            .unwrap();
        let html = String::from_utf8(result).unwrap();
        assert!(html.contains("<table>"));
    }

    #[test]
    fn test_get_exporter_html() {
        assert!(crate::export::get_exporter("html").is_some());
    }

    #[test]
    fn test_get_exporter_unknown() {
        assert!(crate::export::get_exporter("pdf").is_none());
    }
}
