use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd, html};

pub fn render_markdown(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_GFM);
    opts.insert(Options::ENABLE_MATH);

    let events: Vec<Event<'_>> = Parser::new_ext(markdown, opts).collect();
    let transformed = transform_events(events);

    let mut html_output = String::new();
    html::push_html(&mut html_output, transformed.into_iter());
    html_output
}

/// Single-pass transform pipeline applied before HTML serialization:
///
/// - Drop `Event::Html` / `Event::InlineHtml` (§13 spec35: no raw HTML in preview)
/// - Convert `$...$` → `<span class="math-inline" data-math="...">` (t14 KaTeX)
/// - Convert `$$...$$` → `<div class="math-block" data-math="...">` (t14 KaTeX)
/// - Convert ` ```mermaid ` blocks → `<div class="mermaid-block" data-diagram="...">` (t15)
fn transform_events<'a>(events: Vec<Event<'a>>) -> Vec<Event<'a>> {
    let mut out: Vec<Event<'a>> = Vec::with_capacity(events.len());
    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            // Drop raw HTML — §13 spec35
            Event::Html(_) | Event::InlineHtml(_) => {}

            // Inline math: $...$  (inline context → InlineHtml to stay in paragraph)
            Event::InlineMath(content) => {
                out.push(Event::InlineHtml(make_math_inline(content.as_ref()).into()));
            }

            // Display math: $$...$$  (block context)
            Event::DisplayMath(content) => {
                out.push(Event::Html(make_math_block(content.as_ref()).into()));
            }

            // Mermaid fenced code block: consume Start + Text* + End, emit one marker
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(lang)))
                if lang.as_ref() == "mermaid" =>
            {
                let mut diagram = String::new();
                i += 1;
                while i < events.len() {
                    match &events[i] {
                        Event::Text(t) => {
                            diagram.push_str(t.as_ref());
                            i += 1;
                        }
                        Event::End(TagEnd::CodeBlock) => {
                            i += 1;
                            break;
                        }
                        _ => {
                            i += 1;
                        }
                    }
                }
                out.push(Event::Html(make_mermaid_block(&diagram).into()));
                continue; // i already advanced past End; skip outer i += 1
            }

            e => out.push(e.clone()),
        }
        i += 1;
    }
    out
}

/// HTML-encode a string for use in a double-quoted attribute value.
/// Encodes &, ", <, >, ', \n, \r.
pub(crate) fn html_attr_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\'' => out.push_str("&#39;"),
            '\n' => out.push_str("&#10;"),
            '\r' => out.push_str("&#13;"),
            c => out.push(c),
        }
    }
    out
}

fn make_math_inline(content: &str) -> String {
    format!(
        r#"<span class="math-inline" data-math="{}"></span>"#,
        html_attr_encode(content)
    )
}

fn make_math_block(content: &str) -> String {
    format!(
        r#"<div class="math-block" data-math="{}"></div>"#,
        html_attr_encode(content)
    )
}

fn make_mermaid_block(content: &str) -> String {
    format!(
        r#"<div class="mermaid-block" data-diagram="{}"></div>"#,
        html_attr_encode(content)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Existing tests (Phase 1) ──────────────────────────────────────────────

    #[test]
    fn gfm_table_renders_as_table_tag() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let html = render_markdown(md);
        assert!(html.contains("<table>"), "expected <table> in: {html}");
    }

    #[test]
    fn task_list_renders_as_checkbox() {
        let md = "- [ ] todo\n- [x] done\n";
        let html = render_markdown(md);
        assert!(
            html.contains("<input") && html.contains(r#"type="checkbox""#),
            "expected checkbox input in: {html}"
        );
    }

    #[test]
    fn raw_script_tag_is_stripped() {
        let md = "hello\n<script>alert('xss')</script>\nworld";
        let html = render_markdown(md);
        assert!(
            !html.contains("<script>"),
            "script tag must not appear in: {html}"
        );
        assert!(
            !html.contains("</script>"),
            "closing script tag must not appear in: {html}"
        );
    }

    #[test]
    fn raw_iframe_tag_is_stripped() {
        let md = "text\n<iframe src=\"evil\"></iframe>\nmore";
        let html = render_markdown(md);
        assert!(
            !html.contains("<iframe"),
            "iframe must not appear in: {html}"
        );
    }

    #[test]
    fn bold_renders_as_strong() {
        let html = render_markdown("**bold**");
        assert!(
            html.contains("<strong>bold</strong>"),
            "expected <strong>bold</strong> in: {html}"
        );
    }

    // ── Math markers (t13) ────────────────────────────────────────────────────

    #[test]
    fn inline_math_produces_marker() {
        let html = render_markdown("Result: $x^2 + 1$.");
        assert!(
            html.contains(r#"class="math-inline""#),
            "expected math-inline span in: {html}"
        );
        assert!(
            html.contains("data-math="),
            "expected data-math attribute in: {html}"
        );
        // Content should not appear as raw dollar-sign math
        assert!(
            !html.contains("$x^2"),
            "dollar-sign math must not appear raw in: {html}"
        );
    }

    #[test]
    fn inline_math_content_is_in_data_attr() {
        let html = render_markdown("$x$");
        assert!(
            html.contains(r#"data-math="x""#),
            "expected data-math=\"x\" in: {html}"
        );
    }

    #[test]
    fn display_math_produces_marker() {
        let html = render_markdown("$$\\int_0^1 x\\,dx$$");
        assert!(
            html.contains(r#"class="math-block""#),
            "expected math-block div in: {html}"
        );
        assert!(
            html.contains("data-math="),
            "expected data-math attribute in: {html}"
        );
    }

    #[test]
    fn display_math_content_is_in_data_attr() {
        let html = render_markdown("$$E = mc^2$$");
        assert!(
            html.contains(r#"data-math="E = mc^2""#),
            "expected encoded content in: {html}"
        );
    }

    // ── Mermaid markers (t13) ─────────────────────────────────────────────────

    #[test]
    fn mermaid_block_produces_marker() {
        let md = "```mermaid\ngraph TD\n  A --> B\n```\n";
        let html = render_markdown(md);
        assert!(
            html.contains(r#"class="mermaid-block""#),
            "expected mermaid-block div in: {html}"
        );
        assert!(
            html.contains("data-diagram="),
            "expected data-diagram attribute in: {html}"
        );
        assert!(
            !html.contains("<pre>"),
            "pre tag must not appear in: {html}"
        );
        assert!(
            !html.contains("<code>"),
            "code tag must not appear in: {html}"
        );
    }

    #[test]
    fn non_mermaid_code_block_renders_normally() {
        let md = "```rust\nfn main() {}\n```\n";
        let html = render_markdown(md);
        assert!(
            html.contains("<code"),
            "expected <code> for non-mermaid fenced block: {html}"
        );
        assert!(
            !html.contains("mermaid-block"),
            "rust block must not be treated as mermaid: {html}"
        );
    }

    // ── HTML attribute encoding ───────────────────────────────────────────────

    #[test]
    fn html_attr_encode_escapes_all_special_chars() {
        assert_eq!(
            html_attr_encode(r#"a&b"c<d>e'f"#),
            "a&amp;b&quot;c&lt;d&gt;e&#39;f"
        );
    }

    #[test]
    fn html_attr_encode_escapes_newlines() {
        assert_eq!(html_attr_encode("a\nb"), "a&#10;b");
        assert_eq!(html_attr_encode("a\r\nb"), "a&#13;&#10;b");
    }

    #[test]
    fn math_content_with_angle_brackets_is_encoded() {
        let html = render_markdown("$a < b > c$");
        assert!(html.contains("&lt;"), "< must be encoded in: {html}");
        assert!(html.contains("&gt;"), "> must be encoded in: {html}");
        // Ensure < > do not create spurious tags
        assert!(
            !html.contains("<b "),
            "raw < must not create tags in: {html}"
        );
    }

    #[test]
    fn raw_html_inside_paragraph_is_stripped() {
        let md = "para <span onclick=\"alert(1)\">text</span> end";
        let html = render_markdown(md);
        assert!(
            !html.contains("onclick"),
            "onclick must be stripped in: {html}"
        );
    }
}
