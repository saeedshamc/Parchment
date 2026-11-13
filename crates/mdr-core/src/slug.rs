//! Unicode-aware heading slugs (keeps Persian / Arabic / CJK letters).

/// Turns heading text into an anchor id. Letters and digits of any script are
/// kept, whitespace / `-` / `_` / ZWNJ become single dashes, other punctuation
/// is dropped.
pub fn slugify(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending_dash = false;
    for c in text.chars() {
        if c.is_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.extend(c.to_lowercase());
        } else if c.is_whitespace() || matches!(c, '-' | '_' | '\u{200c}') {
            pending_dash = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latin() {
        assert_eq!(slugify("Hello, World!  Again"), "hello-world-again");
    }

    #[test]
    fn persian_keeps_letters() {
        assert_eq!(slugify("فهرست مطالب ۱"), "فهرست-مطالب-۱");
    }

    #[test]
    fn empty_when_only_punctuation() {
        assert_eq!(slugify("?!"), "");
    }
}
