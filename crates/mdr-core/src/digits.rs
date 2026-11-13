//! Latin digit -> Persian digit conversion.

/// Converts ASCII digits `0-9` to Persian digits `۰-۹`.
pub fn to_persian(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0'..='9' => char::from_u32('۰' as u32 + (c as u32 - '0' as u32)).unwrap_or(c),
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_only_ascii_digits() {
        assert_eq!(to_persian("Room 204 در ۱۲"), "Room ۲۰۴ در ۱۲");
    }
}
