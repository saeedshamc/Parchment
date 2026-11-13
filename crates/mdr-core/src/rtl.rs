//! Script statistics used to pick the document direction and language.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub rtl: usize,
    pub ltr: usize,
    pub persian: usize,
    pub arabic: usize,
    pub hebrew: usize,
}

pub fn is_rtl_char(c: char) -> bool {
    matches!(c as u32, 0x0590..=0x08FF | 0xFB1D..=0xFDFF | 0xFE70..=0xFEFF)
}

impl Counts {
    pub fn add(&mut self, text: &str) {
        for c in text.chars() {
            if is_rtl_char(c) {
                if !(c.is_alphabetic()) {
                    continue; // ignore Arabic punctuation / marks
                }
                self.rtl += 1;
                // Letters that only exist in the Persian alphabet (incl. its ک / ی forms).
                if matches!(
                    c as u32,
                    0x067E | 0x0686 | 0x0698 | 0x06AF | 0x06A9 | 0x06CC
                ) {
                    self.persian += 1;
                }
                // Arabic-only letters: ك ي ة ى
                if matches!(c as u32, 0x0643 | 0x064A | 0x0629 | 0x0649) {
                    self.arabic += 1;
                }
                if matches!(c as u32, 0x0590..=0x05FF) {
                    self.hebrew += 1;
                }
            } else if c.is_alphabetic() {
                self.ltr += 1;
            }
        }
    }

    /// True when right-to-left letters dominate.
    pub fn is_rtl(&self) -> bool {
        self.rtl > self.ltr
    }

    /// Best-effort BCP-47 language tag for the dominant script.
    pub fn guess_lang(&self) -> &'static str {
        if !self.is_rtl() {
            "en"
        } else if self.hebrew * 2 > self.rtl {
            "he"
        } else if self.persian == 0 && self.arabic > 0 {
            "ar"
        } else {
            // Arabic-script text without distinguishing letters: assume Persian.
            "fa"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persian_document() {
        let mut c = Counts::default();
        c.add("این یک متن فارسی است و فقط یک کلمه English دارد");
        assert!(c.is_rtl());
        assert_eq!(c.guess_lang(), "fa");
    }

    #[test]
    fn english_document() {
        let mut c = Counts::default();
        c.add("Plain English with one کلمه");
        assert!(!c.is_rtl());
        assert_eq!(c.guess_lang(), "en");
    }

    #[test]
    fn ambiguous_arabic_script_defaults_to_persian() {
        let mut c = Counts::default();
        c.add("سلام");
        assert_eq!(c.guess_lang(), "fa");
    }

    #[test]
    fn arabic_and_hebrew() {
        let mut c = Counts::default();
        c.add("اللغة العربية");
        assert_eq!(c.guess_lang(), "ar");
        let mut h = Counts::default();
        h.add("שלום עולם");
        assert_eq!(h.guess_lang(), "he");
    }
}
