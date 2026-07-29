use crate::models::note::BlockData;
use std::collections::HashMap;

// ── Style options ──

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PageSize {
    A3, A4, A5, Letter, Legal,
}

impl PageSize {
    fn to_mm(&self) -> (f32, f32) {
        match self {
            PageSize::A3 => (297.0, 420.0),
            PageSize::A4 => (210.0, 297.0),
            PageSize::A5 => (148.0, 210.0),
            PageSize::Letter => (215.9, 279.4),
            PageSize::Legal => (215.9, 355.6),
        }
    }

    fn to_twips(&self) -> (i32, i32) {
        let (w, h) = self.to_mm();
        ((w * 56.7) as i32, (h * 56.7) as i32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FontFamily {
    SansSerif, Serif, Monospace,
}

impl FontFamily {
    fn html_family(&self) -> &'static str {
        match self {
            FontFamily::SansSerif => "'Segoe UI', 'Helvetica Neue', Arial, sans-serif",
            FontFamily::Serif => "'Times New Roman', Georgia, serif",
            FontFamily::Monospace => "'Fira Code', 'Source Code Pro', 'Courier New', monospace",
        }
    }

    fn rtf_font_idx(&self) -> u8 {
        match self {
            FontFamily::SansSerif => 1,
            FontFamily::Serif => 2,
            FontFamily::Monospace => 0,
        }
    }

    fn rtf_font_def(&self) -> &'static str {
        match self {
            FontFamily::SansSerif => r"{\f1 Arial;}",
            FontFamily::Serif => r"{\f2 'Times New Roman';}",
            FontFamily::Monospace => r"{\f0 Courier New;}",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Light, Dark, Sepia,
}

impl Theme {
    fn bg(&self) -> &'static str {
        match self { Theme::Light => "#ffffff", Theme::Dark => "#1e1e2e", Theme::Sepia => "#fbf0d9" }
    }
    fn text(&self) -> &'static str {
        match self { Theme::Light => "#1a1a2e", Theme::Dark => "#cdd6f4", Theme::Sepia => "#5c3e22" }
    }
    fn heading(&self) -> &'static str {
        match self { Theme::Light => "#0f0f1a", Theme::Dark => "#b4befe", Theme::Sepia => "#3d2b14" }
    }
    fn code_bg(&self) -> &'static str {
        match self { Theme::Light => "#f4f4f5", Theme::Dark => "#181825", Theme::Sepia => "#f5e6c8" }
    }
    fn code_text(&self) -> &'static str {
        match self { Theme::Light => "#2d2d3f", Theme::Dark => "#a6e3a1", Theme::Sepia => "#4a3520" }
    }
    fn border(&self) -> &'static str {
        match self { Theme::Light => "#d4d4d8", Theme::Dark => "#45475a", Theme::Sepia => "#d4b88c" }
    }
    fn quote_bg(&self) -> &'static str {
        match self { Theme::Light => "#f1f5f9", Theme::Dark => "#313244", Theme::Sepia => "#f5e6cc" }
    }
    fn quote_text(&self) -> &'static str {
        match self { Theme::Light => "#475569", Theme::Dark => "#a6adc8", Theme::Sepia => "#6b4c2e" }
    }
    fn table_border(&self) -> &'static str {
        match self { Theme::Light => "#cbd5e1", Theme::Dark => "#585b70", Theme::Sepia => "#c9b28a" }
    }
    fn table_header_bg(&self) -> &'static str {
        match self { Theme::Light => "#f1f5f9", Theme::Dark => "#313244", Theme::Sepia => "#e8d5b0" }
    }
    fn rtf_bg(&self) -> &'static str {
        match self { Theme::Light => "16777215", Theme::Dark => "1973790", Theme::Sepia => "16514265" }
    }
    fn rtf_text(&self) -> &'static str {
        match self { Theme::Light => "0", Theme::Dark => "13484916", Theme::Sepia => "6046242" }
    }
    fn rtf_heading(&self) -> &'static str {
        match self { Theme::Light => "0", Theme::Dark => "11845374", Theme::Sepia => "4004628" }
    }
    fn rtf_highlight(&self) -> &'static str {
        match self { Theme::Dark => "3223857", Theme::Sepia => "16110408", _ => "15790320" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorderStyle {
    None, Slim, Medium, Thick,
}

impl BorderStyle {
    fn px(&self) -> u32 {
        match self { BorderStyle::None => 0, BorderStyle::Slim => 1, BorderStyle::Medium => 2, BorderStyle::Thick => 4 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineSpacing {
    Single, OnePointFive, Double,
}

impl LineSpacing {
    fn factor(&self) -> f32 {
        match self { LineSpacing::Single => 1.0, LineSpacing::OnePointFive => 1.5, LineSpacing::Double => 2.0 }
    }
    fn rtf_value(&self) -> &'static str {
        match self { LineSpacing::Single => "240", LineSpacing::OnePointFive => "360", LineSpacing::Double => "480" }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MarginSize {
    Narrow, Normal, Wide,
}

impl MarginSize {
    fn mm(&self) -> f32 {
        match self { MarginSize::Narrow => 12.7, MarginSize::Normal => 25.4, MarginSize::Wide => 38.1 }
    }
    fn px(&self) -> u32 {
        match self { MarginSize::Narrow => 12, MarginSize::Normal => 24, MarginSize::Wide => 36 }
    }
    fn twips(&self) -> i32 {
        (self.mm() * 56.7) as i32
    }
}

#[derive(Debug, Clone)]
pub struct StyleOptions {
    pub page_size: PageSize,
    pub font_family: FontFamily,
    pub font_size: u32,
    pub theme: Theme,
    pub border: BorderStyle,
    pub line_spacing: LineSpacing,
    pub margins: MarginSize,
}

impl Default for StyleOptions {
    fn default() -> Self {
        Self {
            page_size: PageSize::A4,
            font_family: FontFamily::SansSerif,
            font_size: 11,
            theme: Theme::Light,
            border: BorderStyle::None,
            line_spacing: LineSpacing::Single,
            margins: MarginSize::Normal,
        }
    }
}

impl StyleOptions {
    pub fn from_params(params: &HashMap<String, String>) -> Self {
        Self {
            page_size: params.get("page_size").map(|s| match s.as_str() {
                "A3" => PageSize::A3, "A5" => PageSize::A5,
                "Letter" => PageSize::Letter, "Legal" => PageSize::Legal,
                _ => PageSize::A4,
            }).unwrap_or(PageSize::A4),

            font_family: params.get("font").map(|s| match s.as_str() {
                "serif" => FontFamily::Serif, "monospace" => FontFamily::Monospace,
                _ => FontFamily::SansSerif,
            }).unwrap_or(FontFamily::SansSerif),

            font_size: params.get("font_size").and_then(|s| s.parse().ok()).filter(|&n| n >= 8 && n <= 48).unwrap_or(11),

            theme: params.get("theme").map(|s| match s.as_str() {
                "dark" => Theme::Dark, "sepia" => Theme::Sepia,
                _ => Theme::Light,
            }).unwrap_or(Theme::Light),

            border: params.get("border").map(|s| match s.as_str() {
                "slim" => BorderStyle::Slim, "medium" => BorderStyle::Medium,
                "thick" => BorderStyle::Thick,
                _ => BorderStyle::None,
            }).unwrap_or(BorderStyle::None),

            line_spacing: params.get("line_spacing").map(|s| match s.as_str() {
                "1.5" => LineSpacing::OnePointFive, "double" => LineSpacing::Double,
                _ => LineSpacing::Single,
            }).unwrap_or(LineSpacing::Single),

            margins: params.get("margins").map(|s| match s.as_str() {
                "narrow" => MarginSize::Narrow, "wide" => MarginSize::Wide,
                _ => MarginSize::Normal,
            }).unwrap_or(MarginSize::Normal),
        }
    }
}

// ── Export Service ──

pub struct ExportService;

impl ExportService {
    pub fn blocks_to_markdown(blocks: &[BlockData], title: &str, _opts: &StyleOptions) -> String {
        let mut md = format!("# {}\n\n", title);
        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" => {
                    let text = block.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    md.push_str(text);
                    md.push_str("\n\n");
                }
                "heading" => {
                    let text = block.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let level = block.data.get("level").and_then(|v| v.as_u64()).unwrap_or(1);
                    md.push_str(&format!("{} {}\n\n", "#".repeat(level as usize), text));
                }
                "bullet_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            md.push_str(&format!("- {}\n", text));
                        }
                        md.push('\n');
                    }
                }
                "numbered_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for (i, item) in items.iter().enumerate() {
                            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            md.push_str(&format!("{}. {}\n", i + 1, text));
                        }
                        md.push('\n');
                    }
                }
                "todo_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            let chk = if item.get("checked").and_then(|v| v.as_bool()).unwrap_or(false) { "[x]" } else { "[ ]" };
                            md.push_str(&format!("- {} {}\n", chk, text));
                        }
                        md.push('\n');
                    }
                }
                "quote" => {
                    for line in block.data.get("text").and_then(|v| v.as_str()).unwrap_or("").lines() {
                        md.push_str(&format!("> {}\n", line));
                    }
                    md.push('\n');
                }
                "code" => {
                    let lang = block.data.get("language").and_then(|v| v.as_str()).unwrap_or("");
                    let code = block.data.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    md.push_str(&format!("```{}\n{}\n```\n\n", lang, code));
                }
                "divider" => md.push_str("---\n\n"),
                "table" => {
                    if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                        if let Some(first) = rows.first().and_then(|r| r.as_array()) {
                            let cols = first.len();
                            for (r_idx, row) in rows.iter().enumerate() {
                                if let Some(cells) = row.as_array() {
                                    md.push('|');
                                    for cell in cells { md.push_str(&format!(" {} |", cell.as_str().unwrap_or(""))); }
                                    md.push('\n');
                                }
                                if r_idx == 0 && cols > 0 {
                                    md.push('|');
                                    for _ in 0..cols { md.push_str(" --- |"); }
                                    md.push('\n');
                                }
                            }
                        }
                        md.push('\n');
                    }
                }
                "image" => {
                    let src = block.data.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    let alt = block.data.get("alt").and_then(|v| v.as_str()).unwrap_or("");
                    md.push_str(&format!("![{}]({})\n\n", alt, src));
                }
                "chart" => {
                    md.push_str(&chart_to_markdown(block));
                }
                _ => {}
            }
        }
        md
    }

    pub fn blocks_to_html(blocks: &[BlockData], title: &str, opts: &StyleOptions) -> String {
        let mut body = String::new();

        let border_css = if opts.border.px() > 0 {
            format!(
                "border: {}px solid {}; border-radius: 4px; padding: {}px;",
                opts.border.px(),
                opts.theme.border(),
                opts.margins.px() * 2
            )
        } else {
            String::new()
        };

        let line_h = 1.0 + (opts.line_spacing.factor() - 1.0) * 0.5;

        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" => {
                    let text = html_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    body.push_str(&format!("<p>{}</p>\n", text));
                }
                "heading" => {
                    let text = html_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    let level = block.data.get("level").and_then(|v| v.as_u64()).unwrap_or(1);
                    body.push_str(&format!("<h{}>{}</h{}>\n", level, text, level));
                }
                "bullet_list" => {
                    body.push_str("<ul>\n");
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = html_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            body.push_str(&format!("  <li>{}</li>\n", text));
                        }
                    }
                    body.push_str("</ul>\n");
                }
                "numbered_list" => {
                    body.push_str("<ol>\n");
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = html_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            body.push_str(&format!("  <li>{}</li>\n", text));
                        }
                    }
                    body.push_str("</ol>\n");
                }
                "todo_list" => {
                    body.push_str("<ul class=\"todo-list\">\n");
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = html_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            let chk = if item.get("checked").and_then(|v| v.as_bool()).unwrap_or(false) { "checked" } else { "" };
                            body.push_str(&format!("  <li><input type=\"checkbox\" {} disabled /> {}</li>\n", chk, text));
                        }
                    }
                    body.push_str("</ul>\n");
                }
                "quote" => {
                    let text = html_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    body.push_str(&format!("<blockquote><p>{}</p></blockquote>\n", text));
                }
                "code" => {
                    let code = html_escape(block.data.get("code").and_then(|v| v.as_str()).unwrap_or(""));
                    let lang = block.data.get("language").and_then(|v| v.as_str()).unwrap_or("");
                    body.push_str(&format!("<pre><code class=\"language-{}\">{}</code></pre>\n", lang, code));
                }
                "divider" => body.push_str("<hr />\n"),
                "table" => {
                    body.push_str("<table>\n");
                    if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                        for (r_idx, row) in rows.iter().enumerate() {
                            let tag = if r_idx == 0 { "th" } else { "td" };
                            body.push_str("  <tr>\n");
                            if let Some(cells) = row.as_array() {
                                for cell in cells {
                                    body.push_str(&format!("    <{}>{}</{}>\n", tag, html_escape(cell.as_str().unwrap_or("")), tag));
                                }
                            }
                            body.push_str("  </tr>\n");
                        }
                    }
                    body.push_str("</table>\n");
                }
                "image" => {
                    let src = block.data.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    let alt = block.data.get("alt").and_then(|v| v.as_str()).unwrap_or("");
                    body.push_str(&format!("<img src=\"{}\" alt=\"{}\" />\n", src, alt));
                }
                "chart" => {
                    body.push_str(&chart_to_html(block));
                }
                _ => {}
            }
        }

        let css = format!(
            r#"
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
    font-family: {}; font-size: {}pt; line-height: {};
    color: {}; background: {};
    max-width: 100%; padding: {}px; {}
    -webkit-font-smoothing: antialiased;
}}
h1, h2, h3, h4, h5, h6 {{ color: {}; margin-top: 1.2em; margin-bottom: 0.5em; font-weight: 600; line-height: 1.3; }}
h1 {{ font-size: 2.0em; }} h2 {{ font-size: 1.6em; }} h3 {{ font-size: 1.35em; }}
h4 {{ font-size: 1.15em; }} h5 {{ font-size: 1.05em; }} h6 {{ font-size: 1.0em; }}
p {{ margin-bottom: 0.6em; }}
ul, ol {{ padding-left: 1.5em; margin-bottom: 0.6em; }}
li {{ margin-bottom: 0.15em; }}
blockquote {{
    border-left: 4px solid {}; padding: 0.5em 1em; margin: 0.6em 0;
    background: {}; color: {};
    border-radius: 0 4px 4px 0;
}}
pre {{
    background: {}; color: {}; padding: 0.9em 1.1em; border-radius: 6px;
    overflow-x: auto; font-size: 0.85em; line-height: 1.45; margin: 0.6em 0;
}}
code {{ font-family: inherit; }}
pre code {{ background: none; padding: 0; }}
table {{ border-collapse: collapse; width: 100%; margin: 0.6em 0; font-size: 0.92em; }}
th, td {{ border: 1px solid {}; padding: 0.45em 0.7em; text-align: left; }}
th {{ background: {}; font-weight: 600; }}
tr:nth-child(even) td {{ background: rgba(0,0,0,0.02); }}
hr {{ border: none; border-top: 2px solid {}; margin: 1em 0; }}
img {{ max-width: 100%; height: auto; border-radius: 4px; }}
.todo-list {{ list-style: none; padding-left: 0.3em; }}
.todo-list li {{ margin-bottom: 0.25em; }}
.todo-list input[type="checkbox"] {{ margin-right: 0.4em; }}
"#,
            opts.font_family.html_family(),
            opts.font_size,
            line_h,
            opts.theme.text(),
            opts.theme.bg(),
            opts.margins.px(),
            border_css,
            opts.theme.heading(),
            opts.theme.border(),
            opts.theme.quote_bg(),
            opts.theme.quote_text(),
            opts.theme.code_bg(),
            opts.theme.code_text(),
            opts.theme.table_border(),
            opts.theme.table_header_bg(),
            opts.theme.table_border(),
        );

        format!(
            "<!DOCTYPE html>\n<html>\n<head>\n<meta charset=\"utf-8\" />\n<title>{}</title>\n<style>{}</style>\n</head>\n<body>\n{}</body>\n</html>",
            html_escape(title),
            css.trim(),
            body
        )
    }

    pub fn blocks_to_plain_text(blocks: &[BlockData], _title: &str, _opts: &StyleOptions) -> String {
        let mut text = String::new();
        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" | "heading" | "quote" => {
                    if let Some(t) = block.data.get("text").and_then(|v| v.as_str()) {
                        text.push_str(t); text.push_str("\n\n");
                    }
                }
                "code" => {
                    if let Some(t) = block.data.get("code").and_then(|v| v.as_str()) {
                        text.push_str(t); text.push_str("\n\n");
                    }
                }
                "bullet_list" | "numbered_list" | "todo_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            if let Some(t) = item.get("text").and_then(|v| v.as_str()) {
                                text.push_str(t); text.push('\n');
                            }
                        }
                        text.push('\n');
                    }
                }
                "table" => {
                    if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                        for row in rows {
                            if let Some(cells) = row.as_array() {
                                let line: Vec<&str> = cells.iter().filter_map(|c| c.as_str()).collect();
                                text.push_str(&line.join("\t")); text.push('\n');
                            }
                        }
                        text.push('\n');
                    }
                }
                "divider" => text.push_str("---\n\n"),
                "image" => {
                    let alt = block.data.get("alt").and_then(|v| v.as_str()).unwrap_or("image");
                    let src = block.data.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    text.push_str(&format!("[{}]({})\n\n", alt, src));
                }
                "chart" => {
                    text.push_str(&chart_to_plain_text(block));
                }
                _ => {}
            }
        }
        text
    }

    pub fn blocks_to_rtf(blocks: &[BlockData], title: &str, opts: &StyleOptions) -> Vec<u8> {
        use std::io::Write;

        let (page_w, page_h) = opts.page_size.to_twips();
        let margin = opts.margins.twips();
        let line_spacing = opts.line_spacing.rtf_value();

        let mut rtf = Vec::new();
        let header = format!(
            r"{{\rtf1\ansi\deff{}\deflang1033 {{\fonttbl {}}}{{\colortbl;\red{}\green{}\blue{};\red{}\green{}\blue{};\red{}\green{}\blue{};\red{}\green{}\blue{};\red{}\green{}\blue{}}}\paperw{}\paperh{}\margl{}\margr{}\margt{}\margb{}\viewkind4\uc1\pard\sl{}\slmult1\fs{}\cb{}\cf{}\b {}\b0\fs{}\par\par\cf1\cb1\sl{}\slmult1\n",
            opts.font_family.rtf_font_idx(),
            opts.font_family.rtf_font_def(),
            parse_hex(opts.theme.rtf_text()).0, parse_hex(opts.theme.rtf_text()).1, parse_hex(opts.theme.rtf_text()).2,
            parse_hex(opts.theme.rtf_bg()).0, parse_hex(opts.theme.rtf_bg()).1, parse_hex(opts.theme.rtf_bg()).2,
            parse_hex(opts.theme.rtf_heading()).0, parse_hex(opts.theme.rtf_heading()).1, parse_hex(opts.theme.rtf_heading()).2,
            parse_hex(opts.theme.rtf_highlight()).0, parse_hex(opts.theme.rtf_highlight()).1, parse_hex(opts.theme.rtf_highlight()).2,
            parse_hex(opts.theme.rtf_text()).0, parse_hex(opts.theme.rtf_text()).1, parse_hex(opts.theme.rtf_text()).2,
            page_w, page_h,
            margin, margin, margin, margin,
            line_spacing,
            opts.font_size * 2,
            opts.theme.rtf_bg(),
            opts.theme.rtf_text(),
            rtf_escape(title),
            (opts.font_size as f32 * 1.3 * 2.0) as u32,
            line_spacing,
        );
        let _ = write!(rtf, "{}", header);

        for block in blocks {
            match block.block_type.as_str() {
                "paragraph" => {
                    let text = rtf_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    let _ = write!(rtf, "{}\\par\\par\n", text);
                }
                "heading" => {
                    let text = rtf_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    let level = block.data.get("level").and_then(|v| v.as_u64()).unwrap_or(1);
                    let size = (opts.font_size as f32 * (1.6 - (level.min(6) as f32 - 1.0) * 0.12) * 2.0) as u32;
                    let _ = write!(rtf, "\\pard\\cf2\\fs{}\\b {}\\b0\\cf1\\fs{}\\par\\par\n", size, text, opts.font_size * 2);
                }
                "bullet_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = rtf_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            let _ = write!(rtf, "\\pard\\li400\\bullet  {}\\par\n", text);
                        }
                        let _ = write!(rtf, "\\par\n");
                    }
                }
                "numbered_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for (i, item) in items.iter().enumerate() {
                            let text = rtf_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            let _ = write!(rtf, "\\pard\\li400 {}.  {}\\par\n", i + 1, text);
                        }
                        let _ = write!(rtf, "\\par\n");
                    }
                }
                "todo_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for item in items {
                            let text = rtf_escape(item.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                            let chk = if item.get("checked").and_then(|v| v.as_bool()).unwrap_or(false) { "☑" } else { "☐" };
                            let _ = write!(rtf, "\\pard\\li400 {} {}\\par\n", chk, text);
                        }
                        let _ = write!(rtf, "\\par\n");
                    }
                }
                "quote" => {
                    let text = rtf_escape(block.data.get("text").and_then(|v| v.as_str()).unwrap_or(""));
                    let _ = write!(rtf, "\\pard\\li400\\cb3\\cf4\\i {}\\i0\\cf1\\cb1\\par\\par\n", text);
                }
                "code" => {
                    let code = rtf_escape(block.data.get("code").and_then(|v| v.as_str()).unwrap_or(""));
                    let _ = write!(rtf, "\\pard\\f0\\cb3\\cf5\\fs{} ", opts.font_size * 2 - 2);
                    for line in code.lines() { let _ = write!(rtf, "{}\\line ", line); }
                    let _ = write!(rtf, "\\cf1\\cb1\\f{}\\fs{}\\par\\par\n", opts.font_family.rtf_font_idx(), opts.font_size * 2);
                }
                "divider" => {
                    let _ = write!(rtf, "\\pard\\brdrb\\brdrs\\brdrw10\\sa10 \\par\\par\n");
                }
                "table" => {
                    if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                        for row in rows {
                            if let Some(cells) = row.as_array() {
                                let _ = write!(rtf, "\\pard\\intbl ");
                                for cell in cells {
                                    let s = rtf_escape(cell.as_str().unwrap_or(""));
                                    let _ = write!(rtf, "{} \\tab ", s);
                                }
                                let _ = write!(rtf, "\\par\n");
                            }
                        }
                        let _ = write!(rtf, "\\par\n");
                    }
                }
                "image" => {
                    let alt = block.data.get("alt").and_then(|v| v.as_str()).unwrap_or("image");
                    let src = block.data.get("src").and_then(|v| v.as_str()).unwrap_or("");
                    let escaped = rtf_escape(alt);
                    let part = format!("{{\\field{{\\*\\fldinst HYPERLINK \"{}\"}}{{\\fldrslt {}}}}}\\par\\par\n", src, escaped);
                    let _ = write!(rtf, "{}", part);
                }
                "chart" => {
                    let title = block.data.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
                    let chart_type = block.data.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
                    let _ = write!(rtf, "\\pard\\cf2\\fs{} \\b {} [{}]\\b0\\cf1\\fs{}\\par\\par\n", opts.font_size * 2 + 4, rtf_escape(title), rtf_escape(chart_type), opts.font_size * 2);
                    if let Some(labels) = block.data.get("labels").and_then(|v| v.as_array()) {
                        if let Some(datasets) = block.data.get("datasets").and_then(|v| v.as_array()) {
                            let _ = write!(rtf, "\\pard\\intbl ");
                            let _ = write!(rtf, "{} \\tab ", "Label");
                            for ds in datasets {
                                let ds_label = ds.get("label").and_then(|v| v.as_str()).unwrap_or("");
                                let _ = write!(rtf, "{} \\tab ", ds_label);
                            }
                            let _ = write!(rtf, "\\par\n");
                            for (i, label) in labels.iter().enumerate() {
                                let s = rtf_escape(label.as_str().unwrap_or(""));
                                let _ = write!(rtf, "\\pard\\intbl {} \\tab ", s);
                                for ds in datasets {
                                    let val = ds.get("data").and_then(|v| v.as_array())
                                        .and_then(|arr| arr.get(i))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    let _ = write!(rtf, "{} \\tab ", val);
                                }
                                let _ = write!(rtf, "\\par\n");
                            }
                            let _ = write!(rtf, "\\par\n");
                        }
                    }
                }
                _ => {}
            }
        }

        let _ = write!(rtf, "}}");
        rtf
    }

    pub fn blocks_to_pdf(blocks: &[BlockData], title: &str, opts: &StyleOptions) -> Result<Vec<u8>, String> {
        use printpdf::*;

        let (pw, ph) = opts.page_size.to_mm();
        let (doc, p_idx, l_idx) = PdfDocument::new(title, Mm(pw), Mm(ph), "Layer 1");

        let sans = doc.add_builtin_font(BuiltinFont::Helvetica).map_err(|e| e.to_string())?;
        let sans_b = doc.add_builtin_font(BuiltinFont::HelveticaBold).map_err(|e| e.to_string())?;
        let serif = doc.add_builtin_font(BuiltinFont::TimesRoman).map_err(|e| e.to_string())?;
        let serif_b = doc.add_builtin_font(BuiltinFont::TimesBold).map_err(|e| e.to_string())?;
        let mono = doc.add_builtin_font(BuiltinFont::Courier).map_err(|e| e.to_string())?;

        let (body_font, bold_font) = match opts.font_family {
            FontFamily::Serif => (&serif, &serif_b),
            FontFamily::Monospace => (&mono, &mono),
            FontFamily::SansSerif => (&sans, &sans_b),
        };

        let layer = doc.get_page(p_idx).get_layer(l_idx);
        let m = opts.margins.mm();
        let cw = pw - m * 2.0;
        let fs = opts.font_size as f32;
        let h_sizes: [f32; 6] = [fs * 1.8, fs * 1.5, fs * 1.3, fs * 1.15, fs * 1.05, fs * 1.0];
        let lh = opts.line_spacing.factor() * fs * 0.3528;
        let bottom = Mm(m);

        // Color helper — defined once at outer scope so it is also usable for c(0,0,0) resets
        let c = |r: u8, g: u8, b: u8| Color::Rgb(Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, None));

        // Returns fresh-owned palette tuple. Call whenever a new set of colours is needed.
        let pal = || match opts.theme {
            Theme::Light => (c(255,255,255), c(26,26,46), c(15,15,26), c(244,244,245), c(45,45,63), c(241,245,249), c(71,85,105), c(203,213,225), c(241,245,249), c(212,212,216)),
            Theme::Dark  => (c(30,30,46), c(205,214,244), c(180,190,254), c(24,24,37), c(166,227,161), c(49,50,68), c(166,173,200), c(88,91,112), c(49,50,68), c(69,71,90)),
            Theme::Sepia => (c(251,240,217), c(92,62,34), c(61,43,20), c(245,230,200), c(74,53,32), c(245,230,204), c(107,76,46), c(201,178,138), c(232,213,176), c(212,184,140)),
        };
        let (bg_c, tx_c, hd_c, _cb_c, _ct_c, _qb_c, _qt_c, tb_c, _th_c, _dv_c) = pal();
        fn rect(pts: Vec<(Mm, Mm)>) -> Polygon {
            let p: Vec<(Point, bool)> = pts.iter().map(|(x, y)| (Point::new(*x, *y), false)).collect();
            Polygon::from_iter(p)
        }

        fn line(p1: (Mm, Mm), p2: (Mm, Mm)) -> Line {
            Line::from_iter(vec![
                (Point::new(p1.0, p1.1), false),
                (Point::new(p2.0, p2.1), false),
            ])
        }

        // Background fill
        if opts.theme != Theme::Light {
            layer.set_fill_color(bg_c);
            layer.add_polygon(rect(vec![
                (Mm(m), Mm(ph - m)), (Mm(pw - m), Mm(ph - m)),
                (Mm(pw - m), Mm(m)), (Mm(m), Mm(m)),
            ]));
        }

        // Border
        if opts.border.px() > 0 {
            let bw = opts.border.px() as f32 * 0.08;
            layer.set_outline_color(tb_c);
            layer.set_outline_thickness(bw);
            let m2 = m - bw * 0.3;
            layer.add_polygon(rect(vec![
                (Mm(m2), Mm(ph - m2)), (Mm(pw - m2), Mm(ph - m2)),
                (Mm(pw - m2), Mm(m2)), (Mm(m2), Mm(m2)),
            ]));
            layer.set_outline_thickness(0.0);
            layer.set_outline_color(c(0, 0, 0));
        }

        let mut y = Mm(ph - m - fs * 0.3528 * 2.0);
        layer.set_fill_color(hd_c);
        layer.use_text(title, fs * 1.5, Mm(m), y, bold_font);
        layer.set_fill_color(tx_c);
        y = Mm(y.0 - lh * 2.0);

        for block in blocks {
            if y < bottom { break; }
            let (_, tx_c, hd_c, cb_c, ct_c, qb_c, qt_c, tb_c, th_c, dv_c) = pal();

            match block.block_type.as_str() {
                "paragraph" => {
                    let t = block.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if !t.is_empty() { layer.use_text(t, fs, Mm(m), y, body_font); y = Mm(y.0 - lh * 1.4); }
                }
                "heading" => {
                    let t = block.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    let l = block.data.get("level").and_then(|v| v.as_u64()).unwrap_or(1).min(6).max(1) as usize;
                    if !t.is_empty() {
                        y = Mm(y.0 - lh * 0.6);
                        layer.set_fill_color(hd_c);
                        layer.use_text(t, h_sizes[l - 1], Mm(m), y, bold_font);
                        layer.set_fill_color(tx_c);
                        y = Mm(y.0 - lh * 1.6);
                    }
                }
                "bullet_list" | "numbered_list" | "todo_list" => {
                    if let Some(items) = block.data.get("items").and_then(|v| v.as_array()) {
                        for (i, item) in items.iter().enumerate() {
                            let t = item.get("text").and_then(|v| v.as_str()).unwrap_or("");
                            if t.is_empty() || y < bottom { continue; }
                            let prefix = match block.block_type.as_str() {
                                "numbered_list" => format!("{}. ", i + 1),
                                "todo_list" => {
                                    let ch = if item.get("checked").and_then(|v| v.as_bool()).unwrap_or(false) { "☑" } else { "☐" };
                                    format!("{} ", ch)
                                }
                                _ => "• ".to_string(),
                            };
                            layer.use_text(&format!("{}{}", prefix, t), fs, Mm(m + 3.0), y, body_font);
                            y = Mm(y.0 - lh * 1.2);
                        }
                    }
                }
                "quote" => {
                    let t = block.data.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if !t.is_empty() {
                        let qh = lh * (t.lines().count() as f32 * 1.2 + 0.8);
                        if y.0 - qh < bottom.0 { continue; }
                        layer.set_fill_color(qb_c);
                        layer.add_polygon(rect(vec![
                            (Mm(m), y), (Mm(m + cw), y),
                            (Mm(m + cw), Mm(y.0 - qh)), (Mm(m), Mm(y.0 - qh)),
                        ]));
                        layer.set_outline_color(dv_c);
                        layer.set_outline_thickness(0.5);
                        layer.add_line(line((Mm(m), y), (Mm(m), Mm(y.0 - qh))));
                        layer.set_outline_thickness(0.0);
                        layer.set_outline_color(c(0, 0, 0));
                        layer.set_fill_color(qt_c);
                        layer.use_text(&format!("“{}”", t), fs * 0.9, Mm(m + 2.5), y, body_font);
                        layer.set_fill_color(tx_c);
                        y = Mm(y.0 - qh - 1.5);
                    }
                }
                "code" => {
                    let code = block.data.get("code").and_then(|v| v.as_str()).unwrap_or("");
                    if !code.is_empty() {
                        let nl = code.lines().count().max(1) as f32;
                        let ch = nl * fs * 0.3528 * 1.25 + 4.0;
                        if y.0 - ch < bottom.0 { continue; }
                        layer.set_fill_color(cb_c);
                        layer.add_polygon(rect(vec![
                            (Mm(m), y), (Mm(m + cw), y),
                            (Mm(m + cw), Mm(y.0 - ch)), (Mm(m), Mm(y.0 - ch)),
                        ]));
                        layer.set_fill_color(ct_c);
                        layer.use_text(code, fs * 0.85, Mm(m + 1.5), y, &mono);
                        layer.set_fill_color(tx_c);
                        y = Mm(y.0 - ch - 1.5);
                    }
                }
                "divider" => {
                    y = Mm(y.0 - lh * 0.8);
                    if y < bottom { break; }
                    layer.set_outline_color(dv_c);
                    layer.set_outline_thickness(0.3);
                    layer.add_line(line((Mm(m + 5.0), y), (Mm(m + cw - 5.0), y)));
                    layer.set_outline_thickness(0.0);
                    layer.set_outline_color(c(0, 0, 0));
                    y = Mm(y.0 - lh * 0.8);
                }
                "table" => {
                    if let Some(rows) = block.data.get("rows").and_then(|v| v.as_array()) {
                        let cc = rows.first().and_then(|r| r.as_array()).map(|r| r.len()).unwrap_or(0);
                        if cc == 0 { continue; }
                        let cw2 = cw / cc as f32;
                        let rh = fs * 0.3528 * 1.5;
                        let th = rh * rows.len() as f32;
                        if y.0 - th < bottom.0 { continue; }
                        layer.set_outline_color(tb_c);
                        layer.set_outline_thickness(0.2);
                        for (ri, row) in rows.iter().enumerate() {
                            let y0 = Mm(y.0 - ri as f32 * rh);
                            let y1 = Mm(y.0 - (ri as f32 + 1.0) * rh);
                            if ri == 0 {
                                layer.set_fill_color(th_c.clone());
                                layer.add_polygon(rect(vec![
                                    (Mm(m), y0), (Mm(m + cw), y0),
                                    (Mm(m + cw), y1), (Mm(m), y1),
                                ]));
                                layer.set_fill_color(tx_c.clone());
                            }
                            if let Some(cells) = row.as_array() {
                                for (ci, cell) in cells.iter().enumerate() {
                                    let x0 = Mm(m + ci as f32 * cw2);
                                    let x1 = Mm(m + (ci as f32 + 1.0) * cw2);
                                    layer.add_polygon(rect(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]));
                                    let s = cell.as_str().unwrap_or("");
                                    layer.use_text(s, fs * 0.85, Mm(m + ci as f32 * cw2 + 0.4), y0, body_font);
                                }
                            }
                        }
                        layer.set_outline_thickness(0.0);
                        layer.set_outline_color(c(0, 0, 0));
                        y = Mm(y.0 - th - 2.0);
                    }
                }
                "image" => {
                    let alt = block.data.get("alt").and_then(|v| v.as_str()).unwrap_or("");
                    if !alt.is_empty() {
                        layer.use_text(&format!("[Image: {}]", alt), fs * 0.9, Mm(m), y, body_font);
                        y = Mm(y.0 - lh * 1.4);
                    }
                }
                "chart" => {
                    let title = block.data.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let chart_type = block.data.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
                    let empty_arr = vec![];
                    let labels = block.data.get("labels").and_then(|v| v.as_array()).unwrap_or(&empty_arr);
                    let datasets = block.data.get("datasets").and_then(|v| v.as_array()).unwrap_or(&empty_arr);

                    if !title.is_empty() {
                        y = Mm(y.0 - lh * 0.6);
                        layer.set_fill_color(hd_c.clone());
                        layer.use_text(&format!("{} [{}]", title, chart_type), fs * 1.1, Mm(m), y, bold_font);
                        layer.set_fill_color(tx_c.clone());
                        y = Mm(y.0 - lh * 1.6);
                    }

                    // Simple bar chart with rects
                    if let (Some(first_ds), false) = (datasets.first(), labels.is_empty()) {
                        let n = labels.len();
                        let bar_area = cw * 0.7;
                        let bar_w = bar_area / n.max(1) as f32 * 0.7;
                        let gap = bar_area / n.max(1) as f32 * 0.3;
                        let chart_bottom = Mm(y.0 - 45.0);

                        if chart_bottom.0 > bottom.0 {
                            let vals: Vec<f64> = first_ds.get("data").and_then(|v| v.as_array())
                                .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                                .unwrap_or_default();
                            let max_val = vals.iter().cloned().fold(0.0f64, f64::max).max(1.0);

                            for (i, val) in vals.iter().enumerate() {
                                let bar_h = (val / max_val) as f32 * 40.0;
                                let x = Mm(m + gap + i as f32 * (bar_w + gap) + bar_area * 0.15);
                                let y_top = Mm(y.0 - 2.0 - bar_h);
                                if y_top.0 > chart_bottom.0 {
                                    layer.set_fill_color(hd_c.clone());
                                    layer.add_polygon(rect(vec![
                                        (x, Mm(y.0 - 2.0)),
                                        (Mm(x.0 + bar_w), Mm(y.0 - 2.0)),
                                        (Mm(x.0 + bar_w), y_top),
                                        (x, y_top),
                                    ]));
                                    if let Some(l) = labels.get(i).and_then(|v| v.as_str()) {
                                        layer.set_fill_color(tx_c.clone());
                                        layer.use_text(l, fs * 0.6, Mm(x.0 - 1.0), Mm(chart_bottom.0 - 2.0), body_font);
                                    }
                                }
                            }
                            layer.set_fill_color(tx_c.clone());
                            y = Mm(chart_bottom.0 - lh * 0.5);
                        }
                    }

                    // Data table below chart
                    if !labels.is_empty() && !datasets.is_empty() {
                        let cc = datasets.len() + 1;
                        let cw2 = cw / cc as f32;
                        let rh = fs * 0.3528 * 1.5;
                        let th = rh * (labels.len() + 1) as f32;
                        if y.0 - th >= bottom.0 {
                            layer.set_outline_color(tb_c.clone());
                            layer.set_outline_thickness(0.2);
                            let y0 = y;
                            let y1 = Mm(y.0 - rh);
                            layer.set_fill_color(th_c.clone());
                            layer.add_polygon(rect(vec![
                                (Mm(m), y0), (Mm(m + cw), y0),
                                (Mm(m + cw), y1), (Mm(m), y1),
                            ]));
                            layer.set_fill_color(tx_c.clone());
                            layer.add_polygon(rect(vec![(Mm(m), y0), (Mm(m + cw2), y0), (Mm(m + cw2), y1), (Mm(m), y1)]));
                            layer.use_text("Label", fs * 0.8, Mm(m + 0.4), y0, body_font);
                            for (di, ds) in datasets.iter().enumerate() {
                                let dl = ds.get("label").and_then(|v| v.as_str()).unwrap_or("");
                                let x0 = Mm(m + (di as f32 + 1.0) * cw2);
                                let x1 = Mm(m + (di as f32 + 2.0) * cw2);
                                layer.add_polygon(rect(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]));
                                layer.use_text(dl, fs * 0.8, Mm(x0.0 + 0.4), y0, body_font);
                            }
                            for (li, label) in labels.iter().enumerate() {
                                let y0 = Mm(y.0 - (li as f32 + 1.0) * rh);
                                let y1 = Mm(y.0 - (li as f32 + 2.0) * rh);
                                let s = label.as_str().unwrap_or("");
                                layer.use_text(s, fs * 0.8, Mm(m + 0.4), y0, body_font);
                                for (di, ds) in datasets.iter().enumerate() {
                                    let x0 = Mm(m + (di as f32 + 1.0) * cw2);
                                    let x1 = Mm(m + (di as f32 + 2.0) * cw2);
                                    layer.add_polygon(rect(vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)]));
                                    let val = ds.get("data").and_then(|v| v.as_array())
                                        .and_then(|arr| arr.get(li))
                                        .and_then(|v| v.as_f64())
                                        .unwrap_or(0.0);
                                    layer.use_text(&val.to_string(), fs * 0.8, Mm(x0.0 + 0.4), y0, body_font);
                                }
                            }
                            layer.set_outline_thickness(0.0);
                            layer.set_outline_color(c(0, 0, 0));
                            y = Mm(y.0 - th - 2.0);
                        }
                    }
                }
                _ => {}
            }
            y = Mm(y.0 - 1.5);
        }

        let mut buf = std::io::BufWriter::new(Vec::new());
        doc.save(&mut buf).map_err(|e| e.to_string())?;
        buf.into_inner().map_err(|e| e.to_string())
    }

    pub fn blocks_to_epub(blocks: &[BlockData], title: &str, opts: &StyleOptions) -> Result<Vec<u8>, String> {
        use epub_builder::{EpubBuilder, EpubContent, ReferenceType, ZipLibrary};

        let mut builder = EpubBuilder::new(ZipLibrary::new().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        builder.metadata("title", title).map_err(|e| e.to_string())?;
        builder.metadata("author", "NoteFlow").map_err(|e| e.to_string())?;

        let html = Self::blocks_to_html(blocks, title, opts);
        builder.add_content(EpubContent::new("content.xhtml", html.as_bytes())
            .title(title)
            .reftype(ReferenceType::Text))
            .map_err(|e| e.to_string())?;

        let mut bytes = Vec::new();
        builder.generate(&mut bytes).map_err(|e| e.to_string())?;
        Ok(bytes)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn rtf_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('\n', "\\line ")
}

fn parse_hex(s: &str) -> (u8, u8, u8) {
    let n: u32 = s.parse().unwrap_or(0);
    let r = ((n >> 16) & 0xFF) as u8;
    let g = ((n >> 8) & 0xFF) as u8;
    let b = (n & 0xFF) as u8;
    (r, g, b)
}

// ── Chart helpers ──

fn chart_to_markdown(block: &BlockData) -> String {
    let title = block.data.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
    let chart_type = block.data.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
    let empty_arr = vec![];
    let labels = block.data.get("labels").and_then(|v| v.as_array()).unwrap_or(&empty_arr);
    let datasets = block.data.get("datasets").and_then(|v| v.as_array()).unwrap_or(&empty_arr);

    let mut md = format!("**{}** [*{}*]\n\n", title, chart_type);
    if labels.is_empty() || datasets.is_empty() {
        return md;
    }
    md.push('|');
    md.push_str(" Label ");
    for ds in datasets {
        let l = ds.get("label").and_then(|v| v.as_str()).unwrap_or("");
        md.push_str(&format!(" | {} ", l));
    }
    md.push_str("|\n|");
    md.push_str("---|");
    for _ in datasets { md.push_str("---|"); }
    md.push('\n');
    for (i, label) in labels.iter().enumerate() {
        let s = label.as_str().unwrap_or("");
        md.push_str(&format!("| {} ", s));
        for ds in datasets {
            let val = ds.get("data").and_then(|v| v.as_array())
                .and_then(|arr| arr.get(i))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            md.push_str(&format!(" | {} ", val));
        }
        md.push_str("|\n");
    }
    md.push('\n');
    md
}

fn chart_to_html(block: &BlockData) -> String {
    let title = block.data.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
    let chart_type = block.data.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
    let empty_arr = vec![];
    let labels = block.data.get("labels").and_then(|v| v.as_array()).unwrap_or(&empty_arr);
    let datasets = block.data.get("datasets").and_then(|v| v.as_array()).unwrap_or(&empty_arr);

    let mut html = format!("<div class=\"chart-block\"><h4>{} <small>({})</small></h4>\n", html_escape(title), html_escape(chart_type));

    if !labels.is_empty() && !datasets.is_empty() {
        if let Some(first_ds) = datasets.first() {
            let vals: Vec<f64> = first_ds.get("data").and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
                .unwrap_or_default();
            let max_val = vals.iter().cloned().fold(0.0f64, f64::max).max(1.0);
            let bar_color = first_ds.get("backgroundColor").and_then(|v| v.as_str()).unwrap_or("#4f46e5");

            html.push_str("<div class=\"chart-bars\" style=\"display:flex;align-items:flex-end;gap:6px;padding:12px 0;min-height:120px;\">\n");
            for (i, val) in vals.iter().enumerate() {
                let pct = if max_val > 0.0 { (val / max_val) * 100.0 } else { 0.0 };
                let label = labels.get(i).and_then(|v| v.as_str()).unwrap_or("");
                html.push_str(&format!(
                    "<div style=\"display:flex;flex-direction:column;align-items:center;flex:1;\">\
                     <div style=\"width:100%;background:{};height:{:.0}px;border-radius:3px 3px 0 0;min-width:20px;\"></div>\
                     <span style=\"font-size:0.75em;margin-top:4px;\">{}</span></div>\n",
                    bar_color, pct * 1.2, html_escape(label)
                ));
            }
            html.push_str("</div>\n");
        }

        html.push_str("<table class=\"chart-table\"><thead><tr><th>Label</th>");
        for ds in datasets {
            let l = ds.get("label").and_then(|v| v.as_str()).unwrap_or("");
            html.push_str(&format!("<th>{}</th>", html_escape(l)));
        }
        html.push_str("</tr></thead><tbody>\n");
        for (i, label) in labels.iter().enumerate() {
            let s = label.as_str().unwrap_or("");
            html.push_str(&format!("<tr><td>{}</td>", html_escape(s)));
            for ds in datasets {
                let val = ds.get("data").and_then(|v| v.as_array())
                    .and_then(|arr| arr.get(i))
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                html.push_str(&format!("<td>{}</td>", val));
            }
            html.push_str("</tr>\n");
        }
        html.push_str("</tbody></table>\n");
    }
    html.push_str("</div>\n");
    html
}

fn chart_to_plain_text(block: &BlockData) -> String {
    let title = block.data.get("title").and_then(|v| v.as_str()).unwrap_or("Chart");
    let chart_type = block.data.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
    let empty_arr = vec![];
    let labels = block.data.get("labels").and_then(|v| v.as_array()).unwrap_or(&empty_arr);
    let datasets = block.data.get("datasets").and_then(|v| v.as_array()).unwrap_or(&empty_arr);

    let mut text = format!("[Chart: {} - {}]\n", chart_type, title);
    if labels.is_empty() || datasets.is_empty() {
        text.push('\n');
        return text;
    }
    for (i, label) in labels.iter().enumerate() {
        let s = label.as_str().unwrap_or("");
        text.push_str(&format!("  {}: ", s));
        for (di, ds) in datasets.iter().enumerate() {
            let dl = ds.get("label").and_then(|v| v.as_str()).unwrap_or("");
            let val = ds.get("data").and_then(|v| v.as_array())
                .and_then(|arr| arr.get(i))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            if di > 0 { text.push_str(", "); }
            text.push_str(&format!("{} = {}", dl, val));
        }
        text.push('\n');
    }
    text.push('\n');
    text
}
