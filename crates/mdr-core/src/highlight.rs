//! Fenced code block rendering with optional syntax highlighting.
//!
//! Highlighting emits CSS classes prefixed with `hl-`; the colours live in
//! [`HIGHLIGHT_CSS`], which is prepended to every template's stylesheet.

/// Escapes `& < > " '` for HTML text and attribute values.
pub fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + s.len() / 8);
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// Colour variables and token rules shared by all templates.
pub const HIGHLIGHT_CSS: &str = r#":root{--hl-comment:#6a737d;--hl-string:#0a7f5a;--hl-keyword:#d73a49;--hl-number:#005cc5;--hl-func:#6f42c1;--hl-var:#e36209}
@media (prefers-color-scheme:dark){:root{--hl-comment:#8b949e;--hl-string:#7ee787;--hl-keyword:#ff7b72;--hl-number:#79c0ff;--hl-func:#d2a8ff;--hl-var:#ffa657}}
.hl-comment{color:var(--hl-comment);font-style:italic}
.hl-string{color:var(--hl-string)}
.hl-keyword,.hl-storage{color:var(--hl-keyword)}
.hl-constant{color:var(--hl-number)}
.hl-entity.hl-name,.hl-support{color:var(--hl-func)}
.hl-variable{color:var(--hl-var)}
"#;

/// Renders a fenced/indented code block to `<pre><code>` HTML.
pub fn code_block(code: &str, lang: &str, enable: bool) -> String {
    let lang = lang.trim();
    let inner = if enable && !lang.is_empty() {
        highlight_inner(code, lang)
    } else {
        None
    }
    .unwrap_or_else(|| escape_html(code));
    if lang.is_empty() {
        format!("<pre class=\"code\"><code>{inner}</code></pre>\n")
    } else {
        let l = escape_html(lang);
        format!("<pre class=\"code\" data-lang=\"{l}\"><code class=\"language-{l}\">{inner}</code></pre>\n")
    }
}

/// Maps common fence names to something the bundled syntax set knows.
#[cfg_attr(not(feature = "highlight"), allow(dead_code))]
fn normalize_token(lang: &str) -> String {
    let l = lang.to_lowercase();
    match l.as_str() {
        "ts" | "tsx" | "typescript" | "jsx" | "mjs" | "cjs" | "javascript" => "js".into(),
        "kotlin" | "kt" | "kts" | "dart" | "swift" => "java".into(),
        "zsh" | "shell" | "console" | "dockerfile" | "termux" | "ps1" | "powershell" => "sh".into(),
        "batch" | "cmd" => "bat".into(),
        "jsonc" | "json5" => "json".into(),
        "vue" | "svelte" | "htm" => "html".into(),
        "c++" | "cc" | "hpp" => "cpp".into(),
        "c#" | "cs" => "cs".into(),
        "yml" => "yaml".into(),
        _ => l,
    }
}

#[cfg(feature = "highlight")]
fn highlight_inner(code: &str, lang: &str) -> Option<String> {
    use std::sync::OnceLock;
    use syntect::html::{ClassStyle, ClassedHTMLGenerator};
    use syntect::parsing::SyntaxSet;
    use syntect::util::LinesWithEndings;

    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    let ss = SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines);
    let token = normalize_token(lang);
    let syntax = ss.find_syntax_by_token(&token)?;
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        ss,
        ClassStyle::SpacedPrefixed { prefix: "hl-" },
    );
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(generator.finalize())
}

#[cfg(not(feature = "highlight"))]
fn highlight_inner(_code: &str, _lang: &str) -> Option<String> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes() {
        assert_eq!(
            escape_html("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn plain_block_is_escaped() {
        let h = code_block("a < b", "", true);
        assert!(h.contains("a &lt; b"));
        assert!(!h.contains("data-lang"));
    }

    #[test]
    fn unknown_language_falls_back_to_plain() {
        let h = code_block("x <y>", "nosuchlang", true);
        assert!(h.contains("x &lt;y&gt;"));
        assert!(h.contains("language-nosuchlang"));
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn rust_is_highlighted() {
        let h = code_block("fn main() { let x = 1; }", "rust", true);
        assert!(h.contains("hl-"), "{h}");
        assert!(h.contains("data-lang=\"rust\""));
    }

    #[cfg(feature = "highlight")]
    #[test]
    fn disabled_highlighting_has_no_spans() {
        let h = code_block("fn main() {}", "rust", false);
        assert!(!h.contains("<span"));
    }
}
