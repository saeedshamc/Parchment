//! Templates: an HTML skeleton with `{{placeholders}}` plus a stylesheet.
//!
//! Placeholders: `{{title}}`, `{{lang}}`, `{{dir}}`, `{{styles}}`, `{{toc}}`,
//! `{{content}}`. Substitution is single-pass, so content that happens to
//! contain `{{...}}` is never re-expanded.

use crate::highlight::{escape_html, HIGHLIGHT_CSS};
use crate::render::{render, Document, Options};

#[derive(Clone, Debug)]
pub struct Template {
    pub name: String,
    pub html: String,
    pub css: String,
}

macro_rules! builtin {
    ($name:literal) => {
        (
            $name,
            include_str!(concat!("../templates/", $name, "/template.html")),
            include_str!(concat!("../templates/", $name, "/style.css")),
        )
    };
}

const BUILTIN: [(&str, &str, &str); 4] = [
    builtin!("github"),
    builtin!("minimal"),
    builtin!("dark"),
    builtin!("magazine"),
];

impl Template {
    pub fn new(name: impl Into<String>, html: impl Into<String>, css: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            html: html.into(),
            css: css.into(),
        }
    }

    /// Looks up a bundled template by (case-insensitive) name.
    pub fn builtin(name: &str) -> Option<Template> {
        let n = name.trim().to_lowercase();
        BUILTIN
            .iter()
            .find(|(k, _, _)| *k == n)
            .map(|(k, h, c)| Template::new(*k, *h, *c))
    }

    pub fn builtin_names() -> Vec<&'static str> {
        BUILTIN.iter().map(|(k, _, _)| *k).collect()
    }

    pub fn default_template() -> Template {
        Template::builtin("github").expect("github template is bundled")
    }

    /// Stylesheet as shipped: highlight rules first so templates can override
    /// the `--hl-*` variables.
    pub fn full_css(&self) -> String {
        format!("{HIGHLIGHT_CSS}{}", self.css)
    }
}

/// How the stylesheet reaches the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CssMode {
    /// `<style>` block inside the HTML (one self-contained file).
    Inline,
    /// `<link rel="stylesheet" href="...">`; the caller writes the CSS file.
    Link(String),
}

#[derive(Clone, Debug)]
pub struct Page {
    pub html: String,
    /// Full stylesheet (also embedded when `CssMode::Inline`).
    pub css: String,
    pub title: String,
}

fn fill(template: &str, lookup: &dyn Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(template.len() + 1024);
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                out.push_str(&lookup(after[..end].trim()));
                rest = &after[end + 2..];
            }
            None => {
                out.push_str(&rest[start..]);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Puts a rendered [`Document`] into a template.
pub fn build_page(doc: &Document, tpl: &Template, mode: &CssMode) -> Page {
    let css = tpl.full_css();
    let styles = match mode {
        CssMode::Inline => format!("<style>\n{css}</style>"),
        CssMode::Link(href) => format!("<link rel=\"stylesheet\" href=\"{}\">", escape_html(href)),
    };
    let html = fill(&tpl.html, &|name| match name {
        "title" => escape_html(&doc.title),
        "lang" => escape_html(&doc.lang),
        "dir" => doc.dir.to_string(),
        "styles" => styles.clone(),
        "toc" => doc.toc.clone(),
        "content" => doc.body.clone(),
        _ => String::new(),
    });
    Page {
        html,
        css,
        title: doc.title.clone(),
    }
}

/// Convenience: Markdown text in, finished page out.
pub fn convert(source: &str, tpl: &Template, opts: &Options, mode: &CssMode) -> Page {
    build_page(&render(source, opts), tpl, mode)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtins_load_and_have_every_placeholder() {
        for name in Template::builtin_names() {
            let t = Template::builtin(name).unwrap();
            for ph in [
                "{{title}}",
                "{{lang}}",
                "{{dir}}",
                "{{styles}}",
                "{{toc}}",
                "{{content}}",
            ] {
                assert!(t.html.contains(ph), "{name} lacks {ph}");
            }
            assert!(!t.css.is_empty());
        }
        assert!(Template::builtin("GitHub").is_some());
        assert!(Template::builtin("nope").is_none());
    }

    #[test]
    fn single_pass_substitution() {
        let t = Template::new("t", "<p>{{ content }}|{{unknown}}|{{title}}</p>", "");
        let p = convert("`{{title}}`", &t, &Options::default(), &CssMode::Inline);
        assert!(p.html.contains("<code>{{title}}</code>"), "{}", p.html);
        assert!(p.html.ends_with("||Untitled</p>"), "{}", p.html);
    }

    #[test]
    fn inline_vs_link() {
        let t = Template::default_template();
        let inline = convert("# Hi", &t, &Options::default(), &CssMode::Inline);
        assert!(inline.html.contains("<style>"));
        let linked = convert(
            "# Hi",
            &t,
            &Options::default(),
            &CssMode::Link("../style.css".into()),
        );
        assert!(linked
            .html
            .contains("<link rel=\"stylesheet\" href=\"../style.css\">"));
        assert!(!linked.html.contains("<style>"));
        assert!(linked.css.contains("--hl-comment"));
    }

    #[test]
    fn title_is_escaped() {
        let t = Template::default_template();
        let p = convert("# a < b & c", &t, &Options::default(), &CssMode::Inline);
        assert!(
            p.html.contains("<title>a &lt; b &amp; c</title>"),
            "{}",
            p.html
        );
    }

    #[test]
    fn rtl_page_attributes() {
        let t = Template::default_template();
        let p = convert("# سلام", &t, &Options::default(), &CssMode::Inline);
        assert!(p.html.contains("lang=\"fa\" dir=\"rtl\""));
    }
}
