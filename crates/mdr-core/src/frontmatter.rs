//! Minimal YAML-ish front matter (`key: value` lines between `---` fences).

use std::collections::BTreeMap;

/// Splits optional front matter from the document. Returns the key/value pairs
/// and the remaining Markdown. Without a closing fence the input is untouched.
pub fn split(src: &str) -> (BTreeMap<String, String>, &str) {
    let src_nobom = src.strip_prefix('\u{feff}').unwrap_or(src);
    let mut lines = src_nobom.split_inclusive('\n');
    let first = match lines.next() {
        Some(l) => l,
        None => return (BTreeMap::new(), src),
    };
    if first.trim_end() != "---" {
        return (BTreeMap::new(), src);
    }
    let mut meta = BTreeMap::new();
    let mut consumed = first.len();
    for line in lines {
        consumed += line.len();
        let t = line.trim_end();
        if t == "---" || t == "..." {
            return (meta, &src_nobom[consumed..]);
        }
        if let Some((k, v)) = t.split_once(':') {
            let key = k.trim().to_lowercase();
            let val = v.trim().trim_matches(|c| c == '"' || c == '\'').trim();
            if !key.is_empty() && !key.contains(' ') {
                meta.insert(key, val.to_string());
            }
        }
    }
    (BTreeMap::new(), src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meta() {
        let (m, rest) = split("---\ntitle: \"Hi there\"\nlang: fa\n---\n# Body\n");
        assert_eq!(m.get("title").unwrap(), "Hi there");
        assert_eq!(m.get("lang").unwrap(), "fa");
        assert_eq!(rest, "# Body\n");
    }

    #[test]
    fn unclosed_is_ignored() {
        let src = "---\nnot closed\n# Title";
        let (m, rest) = split(src);
        assert!(m.is_empty());
        assert_eq!(rest, src);
    }

    #[test]
    fn thematic_break_without_meta_is_kept() {
        let src = "Hello\n---\nWorld";
        let (m, rest) = split(src);
        assert!(m.is_empty());
        assert_eq!(rest, src);
    }
}
