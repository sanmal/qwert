use pulldown_cmark::{html, Event, Options, Parser};

pub fn render_markdown(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);
    opts.insert(Options::ENABLE_GFM);

    let parser = Parser::new_ext(markdown, opts).filter(|event| {
        !matches!(event, Event::Html(_) | Event::InlineHtml(_))
    });

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!html.contains("<script>"), "script tag must not appear in: {html}");
        assert!(!html.contains("</script>"), "closing script tag must not appear in: {html}");
    }

    #[test]
    fn raw_iframe_tag_is_stripped() {
        let md = "text\n<iframe src=\"evil\"></iframe>\nmore";
        let html = render_markdown(md);
        assert!(!html.contains("<iframe"), "iframe must not appear in: {html}");
    }

    #[test]
    fn bold_renders_as_strong() {
        let html = render_markdown("**bold**");
        assert!(html.contains("<strong>bold</strong>"), "expected <strong>bold</strong> in: {html}");
    }
}
