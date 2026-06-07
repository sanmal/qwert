use serde::{Deserialize, Serialize};

/// Category of an invisible / unexpected character (spec §11 第1層).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvisibleCharCategory {
    /// Unicode Tag block: U+E0000–U+E007F
    UnicodeTag,
    /// Null byte: U+0000
    NullByte,
    /// C0 control characters except Tab (U+0009), LF (U+000A), CR (U+000D):
    /// U+0001–U+0008 and U+000E–U+001F
    C0Control,
    /// C1 control characters: U+0080–U+009F
    C1Control,
}

/// A single invisible-character occurrence in a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvisibleCharFinding {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed character column within the line.
    pub column: usize,
    /// The offending character.
    pub char_value: char,
    /// Which category it belongs to.
    pub category: InvisibleCharCategory,
}

impl InvisibleCharFinding {
    /// Unicode code point as a `"U+XXXX"` string (for display / JSON output).
    pub fn char_hex(&self) -> String {
        format!("U+{:04X}", self.char_value as u32)
    }

    /// Category as a static string (matches the Rust variant name).
    pub fn category_str(&self) -> &'static str {
        match self.category {
            InvisibleCharCategory::UnicodeTag => "UnicodeTag",
            InvisibleCharCategory::NullByte => "NullByte",
            InvisibleCharCategory::C0Control => "C0Control",
            InvisibleCharCategory::C1Control => "C1Control",
        }
    }
}

/// Scan `content` (valid UTF-8) for first-layer invisible characters.
///
/// Detects: Unicode Tags, Null bytes, disallowed C0 controls, C1 controls.
/// Does **not** detect Tab / LF / CR (they are legitimate in Markdown).
/// Does **not** modify `content` (detection only).
pub fn detect_invisible_chars(content: &str) -> Vec<InvisibleCharFinding> {
    let mut findings = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_no = line_idx + 1;
        for (col_idx, ch) in line.chars().enumerate() {
            if let Some(category) = classify(ch) {
                findings.push(InvisibleCharFinding {
                    line: line_no,
                    column: col_idx + 1,
                    char_value: ch,
                    category,
                });
            }
        }
    }

    findings
}

fn classify(ch: char) -> Option<InvisibleCharCategory> {
    let cp = ch as u32;

    // Unicode Tag block: U+E0000–U+E007F
    if (0xE0000..=0xE007F).contains(&cp) {
        return Some(InvisibleCharCategory::UnicodeTag);
    }

    // Null byte: U+0000
    if cp == 0x0000 {
        return Some(InvisibleCharCategory::NullByte);
    }

    // C0 control chars: U+0001–U+001F, excluding Tab(09), LF(0A), CR(0D)
    if (0x0001..=0x001F).contains(&cp) && cp != 0x09 && cp != 0x0A && cp != 0x0D {
        return Some(InvisibleCharCategory::C0Control);
    }

    // C1 control chars: U+0080–U+009F
    if (0x0080..=0x009F).contains(&cp) {
        return Some(InvisibleCharCategory::C1Control);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── 第1層: 各カテゴリの検出 ──────────────────────────────────────────────

    #[test]
    fn detects_unicode_tag() {
        // U+E0001 is in the Unicode Tag block
        let content = "normal \u{E0001} text";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, InvisibleCharCategory::UnicodeTag);
        assert_eq!(findings[0].char_value, '\u{E0001}');
    }

    #[test]
    fn detects_unicode_tag_boundary() {
        // U+E0000 (start) and U+E007F (end)
        let findings_start = detect_invisible_chars("\u{E0000}");
        let findings_end = detect_invisible_chars("\u{E007F}");
        assert_eq!(
            findings_start[0].category,
            InvisibleCharCategory::UnicodeTag
        );
        assert_eq!(findings_end[0].category, InvisibleCharCategory::UnicodeTag);
    }

    #[test]
    fn detects_null_byte() {
        let content = "before\x00after";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, InvisibleCharCategory::NullByte);
        assert_eq!(findings[0].char_value, '\0');
    }

    #[test]
    fn detects_c0_control_backspace() {
        // U+0008 = Backspace
        let content = "text\x08more";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, InvisibleCharCategory::C0Control);
    }

    #[test]
    fn detects_c0_control_esc() {
        // U+001B = Escape
        let content = "\x1Bterm";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, InvisibleCharCategory::C0Control);
    }

    #[test]
    fn detects_c1_control() {
        // U+0080 = start of C1 range
        let content = "\u{0080}data";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].category, InvisibleCharCategory::C1Control);
    }

    #[test]
    fn detects_c1_control_boundary() {
        // U+009F = end of C1 range
        let findings = detect_invisible_chars("\u{009F}");
        assert_eq!(findings[0].category, InvisibleCharCategory::C1Control);
        // U+00A0 = non-breaking space, outside C1 range, should NOT be detected
        assert!(detect_invisible_chars("\u{00A0}").is_empty());
    }

    // ── 許可文字は検出しない ──────────────────────────────────────────────────

    #[test]
    fn tab_not_detected() {
        assert!(detect_invisible_chars("\there").is_empty());
    }

    #[test]
    fn lf_not_detected() {
        // LF is the line separator in content.lines(); it won't appear within a line,
        // but test with direct char check as well.
        assert!(detect_invisible_chars("line1\nline2").is_empty());
    }

    #[test]
    fn cr_not_detected() {
        assert!(detect_invisible_chars("text\r\nmore").is_empty());
    }

    #[test]
    fn normal_ascii_not_detected() {
        assert!(detect_invisible_chars("Hello, world! 123").is_empty());
    }

    #[test]
    fn multibyte_unicode_not_detected() {
        // Japanese, emoji, etc. should not trigger findings
        assert!(detect_invisible_chars("こんにちは 🎉").is_empty());
    }

    // ── 位置計算 ─────────────────────────────────────────────────────────────

    #[test]
    fn line_and_column_are_correct() {
        // Line 2, column 5
        let content = "clean\nnote\x08body";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].line, 2);
        assert_eq!(findings[0].column, 5); // "note" = 4 chars, then \x08 at col 5
    }

    #[test]
    fn multiple_findings_same_line() {
        let content = "\x01hello\x02world";
        let findings = detect_invisible_chars(content);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].column, 1);
        assert_eq!(findings[1].column, 7); // "\x01hello" = 6 chars, then \x02
    }

    #[test]
    fn empty_content_returns_empty() {
        assert!(detect_invisible_chars("").is_empty());
    }

    #[test]
    fn multibyte_char_column_counts_chars_not_bytes() {
        // "こん\x08" — "こ"=1char,"ん"=1char,"\x08"=col 3
        let findings = detect_invisible_chars("こん\x08");
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].column, 3);
    }
}
