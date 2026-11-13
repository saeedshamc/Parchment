//! Markdown -> HTML fragment rendering with RTL awareness.

use std::collections::{BTreeMap, HashSet};

use pulldown_cmark::{
    html, CodeBlockKind, CowStr, Event, Options as MdOptions, Parser, Tag, TagEnd,
};

use crate::digits;
use crate::frontmatter;
use crate::highlight::{self, escape_html};
use crate::rtl::Counts;
use crate::slug::slugify;
use crate::toc::{self, Heading};

/// Text direction policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirMode {
    /// Document direction from the dominant script; every block gets `dir="auto"`.
    Auto,
    Ltr,
    Rtl,
}

impl DirMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "ltr" => Some(Self::Ltr),
            "rtl" => Some(Self::Rtl),
            _ => None,
        }
    }
}

/// Table-of-contents policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TocMode {
    /// Shown when the document has at least 4 headings in range.
    Auto,
    On,
    Off,
}

impl TocMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "on" | "true" | "yes" | "1" => Some(Self::On),
            "off" | "false" | "no" | "0" => Some(Self::Off),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Options {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub dir: DirMode,
    pub toc: TocMode,
    pub toc_min_level: u8,
    pub toc_max_level: u8,
    /// Convert Latin digits in prose to Persian digits (code is untouched).
    pub persian_digits: bool,
    /// Pass raw HTML in the Markdown through. When false it is shown as text.
    pub raw_html: bool,
    /// Rewrite relative `*.md` links to `*.html` (for multi-page sites).
    pub rewrite_md_links: bool,
    pub smart_punctuation: bool,
    pub highlight: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            title: None,
            lang: None,
            dir: DirMode::Auto,
            toc: TocMode::Auto,
            toc_min_level: 2,
            toc_max_level: 3,
            persian_digits: false,
            raw_html: true,
            rewrite_md_links: false,
            smart_punctuation: false,
            highlight: true,
        }
    }
}

impl Options {
    /// Applies front-matter overrides (`title`, `lang`, `dir`, `toc`).
    fn with_meta(&self, meta: &BTreeMap<String, String>) -> Options {
        let mut o = self.clone();
        if let Some(v) = meta.get("title").filter(|v| !v.is_empty()) {
            o.title = Some(v.clone());
        }
        if let Some(v) = meta.get("lang").filter(|v| !v.is_empty()) {
            o.lang = Some(v.clone());
        }
        if let Some(d) = meta.get("dir").and_then(|v| DirMode::parse(v)) {
            o.dir = d;
        }
        if let Some(t) = meta.get("toc").and_then(|v| TocMode::parse(v)) {
            o.toc = t;
        }
        o
    }
}

/// A rendered document fragment plus the metadata templates need.
#[derive(Clone, Debug)]
pub struct Document {
    pub title: String,
    pub lang: String,
    /// `"ltr"` or `"rtl"`.
    pub dir: &'static str,
    pub body: String,
    /// Complete `<nav class="toc">` markup, or empty.
    pub toc: String,
    pub headings: Vec<Heading>,
}

/// Rewrites `foo.md#x` to `foo.html#x` for relative links; `None` otherwise.
pub fn rewrite_md_link(dest: &str) -> Option<String> {
    let cut = dest.find(['#', '?']).unwrap_or(dest.len());
    let (path, rest) = dest.split_at(cut);
    let lower = path.to_ascii_lowercase();
    let stem_len = if lower.ends_with(".markdown") {
        path.len() - 9
    } else if lower.ends_with(".md") {
        path.len() - 3
    } else {
        return None;
    };
    if path.starts_with("//") {
        return None;
    }
    if let Some(colon) = path.find(':') {
        if !path[..colon].contains('/') {
            return None; // has a URL scheme
        }
    }
    Some(format!("{}.html{}", &path[..stem_len], rest))
}

struct Ctx<'o> {
    o: &'o Options,
    counts: Counts,
    auto_dir: bool,
}

impl<'o> Ctx<'o> {
    fn map<'a>(&mut self, ev: Event<'a>) -> Event<'a> {
        match ev {
            Event::Text(t) => {
                self.counts.add(&t);
                if self.o.persian_digits {
                    Event::Text(CowStr::from(digits::to_persian(&t)))
                } else {
                    Event::Text(t)
                }
            }
            Event::Html(h) | Event::InlineHtml(h) if !self.o.raw_html => Event::Text(h),
            Event::Start(Tag::Paragraph) if self.auto_dir => Event::Html("<p dir=\"auto\">".into()),
            Event::End(TagEnd::Paragraph) if self.auto_dir => Event::Html("</p>\n".into()),
            Event::Start(Tag::Item) if self.auto_dir => Event::Html("<li dir=\"auto\">".into()),
            Event::End(TagEnd::Item) if self.auto_dir => Event::Html("</li>\n".into()),
            Event::Start(Tag::BlockQuote(..)) if self.auto_dir => {
                Event::Html("<blockquote dir=\"auto\">\n".into())
            }
            Event::End(TagEnd::BlockQuote(..)) if self.auto_dir => {
                Event::Html("</blockquote>\n".into())
            }
            Event::Start(Tag::Link {
                link_type,
                dest_url,
                title,
                id,
            }) if self.o.rewrite_md_links => {
                let dest_url = match rewrite_md_link(&dest_url) {
                    Some(d) => CowStr::from(d),
                    None => dest_url,
                };
                Event::Start(Tag::Link {
                    link_type,
                    dest_url,
                    title,
                    id,
                })
            }
            other => other,
        }
    }
}

fn unique_id(used: &mut HashSet<String>, base: String) -> String {
    let mut cand = base.clone();
    let mut n = 0;
    while used.contains(&cand) {
        n += 1;
        cand = format!("{base}-{n}");
    }
    used.insert(cand.clone());
    cand
}

/// Renders Markdown (with optional front matter) into a [`Document`].
pub fn render(source: &str, base: &Options) -> Document {
    let (meta, md) = frontmatter::split(source);
    let o = base.with_meta(&meta);

    let mut md_opts = MdOptions::ENABLE_TABLES
        | MdOptions::ENABLE_FOOTNOTES
        | MdOptions::ENABLE_STRIKETHROUGH
        | MdOptions::ENABLE_TASKLISTS
        | MdOptions::ENABLE_HEADING_ATTRIBUTES;
    if o.smart_punctuation {
        md_opts |= MdOptions::ENABLE_SMART_PUNCTUATION;
    }

    let events: Vec<Event> = Parser::new_ext(md, md_opts).collect();
    let mut ctx = Ctx {
        o: &o,
        counts: Counts::default(),
        auto_dir: o.dir == DirMode::Auto,
    };
    let mut out: Vec<Event> = Vec::with_capacity(events.len() + 16);
    let mut headings: Vec<Heading> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    let mut first_h1: Option<String> = None;

    let mut i = 0;
    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading {
                level, id, classes, ..
            }) => {
                let mut j = i + 1;
                while j < events.len() && !matches!(events[j], Event::End(TagEnd::Heading(_))) {
                    j += 1;
                }
                let mut text = String::new();
                for e in &events[i + 1..j.min(events.len())] {
                    if let Event::Text(t) | Event::Code(t) = e {
                        text.push_str(t);
                    }
                }
                let lvl = *level as u8;
                let base_id = match id {
                    Some(x) => x.to_string(),
                    None => {
                        let s = slugify(&text);
                        if s.is_empty() {
                            "section".to_string()
                        } else {
                            s
                        }
                    }
                };
                let id = unique_id(&mut used, base_id);
                let class_attr = if classes.is_empty() {
                    String::new()
                } else {
                    let joined: Vec<&str> = classes.iter().map(|c| &**c).collect();
                    format!(" class=\"{}\"", escape_html(&joined.join(" ")))
                };
                let dir_attr = if ctx.auto_dir { " dir=\"auto\"" } else { "" };
                out.push(Event::Html(
                    format!("<h{lvl} id=\"{}\"{class_attr}{dir_attr}>", escape_html(&id)).into(),
                ));
                for e in &events[i + 1..j.min(events.len())] {
                    let mapped = ctx.map(e.clone());
                    out.push(mapped);
                }
                out.push(Event::Html(format!("</h{lvl}>\n").into()));
                if lvl == 1 && first_h1.is_none() {
                    first_h1 = Some(text.clone());
                }
                headings.push(Heading {
                    level: lvl,
                    id,
                    text,
                });
                i = j + 1;
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(l) => l
                        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
                        .next()
                        .unwrap_or("")
                        .to_string(),
                    CodeBlockKind::Indented => String::new(),
                };
                let mut code = String::new();
                let mut j = i + 1;
                while j < events.len() {
                    match &events[j] {
                        Event::End(TagEnd::CodeBlock) => break,
                        Event::Text(t) => code.push_str(t),
                        _ => {}
                    }
                    j += 1;
                }
                out.push(Event::Html(
                    highlight::code_block(&code, &lang, o.highlight).into(),
                ));
                i = j + 1;
            }
            other => {
                let mapped = ctx.map(other.clone());
                out.push(mapped);
                i += 1;
            }
        }
    }

    let mut body = String::with_capacity(md.len() * 2);
    html::push_html(&mut body, out.into_iter());

    let counts = ctx.counts;
    let dir = match o.dir {
        DirMode::Ltr => "ltr",
        DirMode::Rtl => "rtl",
        DirMode::Auto => {
            if counts.is_rtl() {
                "rtl"
            } else {
                "ltr"
            }
        }
    };
    let lang = o
        .lang
        .clone()
        .unwrap_or_else(|| counts.guess_lang().to_string());
    let title = o
        .title
        .clone()
        .or(first_h1)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "Untitled".to_string());

    let in_range = toc::count_in_range(&headings, o.toc_min_level, o.toc_max_level);
    let show_toc = match o.toc {
        TocMode::On => in_range > 0,
        TocMode::Auto => in_range >= 4,
        TocMode::Off => false,
    };
    let toc_html = if show_toc {
        toc::build(&headings, o.toc_min_level, o.toc_max_level, &lang)
    } else {
        String::new()
    };

    Document {
        title,
        lang,
        dir,
        body,
        toc: toc_html,
        headings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> Options {
        Options::default()
    }

    #[test]
    fn heading_ids_are_unique_and_unicode() {
        let d = render("## فهرست\n\n## فهرست\n\n## Same\n\n## Same\n", &opts());
        let ids: Vec<&str> = d.headings.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["فهرست", "فهرست-1", "same", "same-1"]);
        assert!(d.body.contains("id=\"فهرست-1\""));
    }

    #[test]
    fn title_from_first_h1_and_frontmatter_override() {
        assert_eq!(render("# Hello\n", &opts()).title, "Hello");
        let d = render("---\ntitle: From Meta\n---\n# Hello\n", &opts());
        assert_eq!(d.title, "From Meta");
        assert_eq!(render("text", &opts()).title, "Untitled");
    }

    #[test]
    fn persian_doc_is_rtl_with_auto_blocks() {
        let d = render("# سلام دنیا\n\nاین یک متن است.\n\n- مورد اول\n", &opts());
        assert_eq!(d.dir, "rtl");
        assert_eq!(d.lang, "fa");
        assert!(d.body.contains("<p dir=\"auto\">"));
        assert!(d.body.contains("<li dir=\"auto\">"));
        assert!(d.body.contains("<h1 id=\"سلام-دنیا\" dir=\"auto\">"));
    }

    #[test]
    fn forced_direction_skips_block_dir() {
        let o = Options {
            dir: DirMode::Ltr,
            ..opts()
        };
        let d = render("سلام\n", &o);
        assert_eq!(d.dir, "ltr");
        assert!(!d.body.contains("dir=\"auto\""));
        assert!(d.body.contains("<p>"));
    }

    #[test]
    fn persian_digits_skip_code() {
        let o = Options {
            persian_digits: true,
            ..opts()
        };
        let d = render("عدد 123 و `456`\n\n```\n789\n```\n", &o);
        assert!(d.body.contains("۱۲۳"));
        assert!(d.body.contains("<code>456</code>"));
        assert!(d.body.contains("789"));
    }

    #[test]
    fn raw_html_can_be_disabled() {
        let src = "<script>alert(1)</script>\n\ntext <b>x</b>\n";
        let on = render(src, &opts());
        assert!(on.body.contains("<script>"));
        let off = render(
            src,
            &Options {
                raw_html: false,
                ..opts()
            },
        );
        assert!(!off.body.contains("<script>"));
        assert!(off.body.contains("&lt;script&gt;"));
    }

    #[test]
    fn md_links_rewritten_only_when_relative() {
        assert_eq!(rewrite_md_link("a/b.md#x").as_deref(), Some("a/b.html#x"));
        assert_eq!(rewrite_md_link("B.MD").as_deref(), Some("B.html"));
        assert_eq!(rewrite_md_link("https://x.com/a.md"), None);
        assert_eq!(rewrite_md_link("mailto:a@b.md"), None);
        assert_eq!(rewrite_md_link("img.png"), None);
        let o = Options {
            rewrite_md_links: true,
            ..opts()
        };
        assert!(render("[x](other.md#top)", &o)
            .body
            .contains("href=\"other.html#top\""));
        assert!(render("[x](other.md)", &opts())
            .body
            .contains("href=\"other.md\""));
    }

    #[test]
    fn toc_auto_needs_four_headings() {
        let few = render("## a\n\n## b\n", &opts());
        assert!(few.toc.is_empty());
        let many = render("## a\n\n## b\n\n### c\n\n## d\n", &opts());
        assert!(many.toc.contains("class=\"toc\""));
        let forced = render("---\ntoc: on\n---\n## a\n", &opts());
        assert!(!forced.toc.is_empty());
        let off = render(
            "---\ntoc: off\n---\n## a\n\n## b\n\n## c\n\n## d\n",
            &opts(),
        );
        assert!(off.toc.is_empty());
    }

    #[test]
    fn tables_tasks_footnotes_and_code() {
        let src = "| a | b |\n|---|---|\n| 1 | 2 |\n\n- [x] done\n- [ ] todo\n\nText[^1]\n\n[^1]: note\n\n```rust\nfn main() {}\n```\n";
        let d = render(src, &opts());
        assert!(d.body.contains("<table>"));
        assert!(d.body.contains("type=\"checkbox\""));
        assert!(d.body.contains("footnote"));
        assert!(d.body.contains("language-rust"));
    }

    #[test]
    fn heading_attributes_supported() {
        let d = render("## Title {#custom .fancy}\n", &opts());
        assert!(d.body.contains("id=\"custom\""));
        assert!(d.body.contains("class=\"fancy\""));
    }
}
