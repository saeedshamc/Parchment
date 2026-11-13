//! Table of contents generation.

use crate::highlight::escape_html;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

/// Localised label for the TOC summary.
pub fn label(lang: &str) -> &'static str {
    match lang.split('-').next().unwrap_or("") {
        "fa" => "فهرست مطالب",
        "ar" => "المحتويات",
        "he" => "תוכן עניינים",
        _ => "Contents",
    }
}

/// Number of headings that fall inside `[min, max]`.
pub fn count_in_range(headings: &[Heading], min: u8, max: u8) -> usize {
    headings
        .iter()
        .filter(|h| h.level >= min && h.level <= max)
        .count()
}

/// Builds a nested `<nav class="toc">`; empty string when nothing matches.
pub fn build(headings: &[Heading], min: u8, max: u8, lang: &str) -> String {
    let items: Vec<&Heading> = headings
        .iter()
        .filter(|h| h.level >= min && h.level <= max)
        .collect();
    if items.is_empty() {
        return String::new();
    }
    let base = items.iter().map(|h| h.level).min().unwrap_or(min);
    let mut list = String::new();
    let mut cur: i32 = -1;
    for h in items {
        // Never jump more than one level deeper than the current one.
        let d = ((h.level - base) as i32).min(cur + 1);
        let link = format!(
            "<a href=\"#{}\">{}</a>",
            escape_html(&h.id),
            escape_html(&h.text)
        );
        match d.cmp(&cur) {
            std::cmp::Ordering::Greater => list.push_str("<ul><li>"),
            std::cmp::Ordering::Equal => list.push_str("</li><li>"),
            std::cmp::Ordering::Less => {
                for _ in 0..(cur - d) {
                    list.push_str("</li></ul>");
                }
                list.push_str("</li><li>");
            }
        }
        list.push_str(&link);
        cur = d;
    }
    for _ in 0..=cur {
        list.push_str("</li></ul>");
    }
    format!(
        "<nav class=\"toc\" aria-label=\"{l}\"><details open><summary>{l}</summary>{list}</details></nav>\n",
        l = label(lang)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(level: u8, id: &str) -> Heading {
        Heading {
            level,
            id: id.into(),
            text: id.into(),
        }
    }

    #[test]
    fn nests_correctly() {
        let hs = vec![h(2, "a"), h(3, "b"), h(3, "c"), h(2, "d")];
        let html = build(&hs, 2, 3, "en");
        assert!(html.contains("<ul><li><a href=\"#a\">a</a><ul><li><a href=\"#b\">b</a></li><li><a href=\"#c\">c</a></li></ul></li><li><a href=\"#d\">d</a></li></ul>"), "{html}");
    }

    #[test]
    fn skipped_levels_do_not_break_markup() {
        let hs = vec![h(1, "a"), h(3, "b")];
        let html = build(&hs, 1, 3, "en");
        assert_eq!(html.matches("<ul>").count(), html.matches("</ul>").count());
        assert_eq!(html.matches("<li>").count(), html.matches("</li>").count());
    }

    #[test]
    fn empty_when_out_of_range() {
        assert_eq!(build(&[h(1, "a")], 2, 3, "en"), "");
    }

    #[test]
    fn persian_label() {
        assert!(build(&[h(2, "x")], 2, 3, "fa").contains("فهرست مطالب"));
    }
}
