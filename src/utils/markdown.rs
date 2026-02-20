use ammonia::Builder;
use comrak::{markdown_to_html, Options};
use std::collections::HashSet;

/// Render raw Markdown to sanitized HTML.
///
/// Uses comrak for GFM-compatible parsing (tables, task lists, strikethrough,
/// autolink, etc.) and ammonia for XSS-safe HTML sanitization.
pub fn render_markdown(raw: &str) -> String {
    let mut options = Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.autolink = true;
    options.extension.tasklist = true;
    options.extension.superscript = true;
    options.extension.description_lists = true;
    options.render.unsafe_ = true; // let comrak emit raw HTML; ammonia will sanitize

    let html = markdown_to_html(raw, &options);
    sanitize_html(&html)
}

fn sanitize_html(html: &str) -> String {
    let extra_tags: HashSet<&str> = [
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "pre",
        "code",
        "blockquote",
        "hr",
        "table",
        "thead",
        "tbody",
        "tfoot",
        "tr",
        "th",
        "td",
        "img",
        "input",
        "del",
        "s",
        "details",
        "summary",
        "sup",
        "sub",
        "dd",
        "dl",
        "dt",
    ]
    .iter()
    .copied()
    .collect();

    let url_schemes: HashSet<&str> = ["http", "https", "mailto"].iter().copied().collect();

    let mut builder = Builder::default();
    builder.add_tags(&extra_tags);

    builder.add_tag_attributes("a", &["href", "title"]);
    builder.add_tag_attributes("img", &["src", "alt", "title"]);
    builder.add_tag_attributes("code", &["class"]);
    builder.add_tag_attributes("input", &["type", "checked", "disabled"]);
    builder.add_tag_attributes("td", &["align"]);
    builder.add_tag_attributes("th", &["align"]);

    builder.url_schemes(url_schemes);
    builder.link_rel(Some("noopener noreferrer"));

    builder.clean(html).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_markdown_rendering() {
        let html = render_markdown("# Hello\n\nThis is **bold** and *italic*.");
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn gfm_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let html = render_markdown(md);
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>1</td>"));
    }

    #[test]
    fn gfm_tasklist() {
        let md = "- [x] done\n- [ ] todo";
        let html = render_markdown(md);
        assert!(html.contains("<input"));
        assert!(html.contains("checked"));
    }

    #[test]
    fn gfm_strikethrough() {
        let html = render_markdown("~~deleted~~");
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn xss_script_tag_removed() {
        let html = render_markdown("<script>alert('xss')</script>");
        assert!(!html.contains("<script>"));
        assert!(!html.contains("alert"));
    }

    #[test]
    fn xss_javascript_url_removed() {
        let html = render_markdown("[click](javascript:alert(1))");
        assert!(!html.contains("javascript:"));
    }

    #[test]
    fn xss_event_handler_removed() {
        let html = render_markdown("<img src=x onerror=alert(1)>");
        assert!(!html.contains("onerror"));
    }

    #[test]
    fn empty_input() {
        let html = render_markdown("");
        assert!(html.trim().is_empty());
    }

    #[test]
    fn code_block_with_language() {
        let md = "```rust\nfn main() {}\n```";
        let html = render_markdown(md);
        assert!(html.contains("<code"));
        assert!(html.contains("fn main()"));
    }

    #[test]
    fn autolink() {
        let html = render_markdown("Visit https://example.com today");
        assert!(html.contains("<a href=\"https://example.com\""));
    }
}
