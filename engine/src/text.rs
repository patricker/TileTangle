use std::sync::Arc;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

// -------- Symbols & Text --------

pub type Symbol = String; // NFC-normalized grapheme string

pub fn nfc<S: AsRef<str>>(s: S) -> Symbol {
    s.as_ref().nfc().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NormalizationMode {
    #[default]
    NFC,
    NFKC,
}

pub fn normalize_with_mode<S: AsRef<str>>(s: S, mode: NormalizationMode) -> String {
    match mode {
        NormalizationMode::NFC => s.as_ref().nfc().collect(),
        NormalizationMode::NFKC => s.as_ref().nfkc().collect(),
    }
}

pub trait Tokenizer: Send + Sync {
    fn segment(&self, text: &str) -> Vec<String>;
}

#[derive(Clone)]
pub struct TokenizerRef {
    inner: Arc<dyn Tokenizer>,
}

impl std::fmt::Debug for TokenizerRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TokenizerRef").finish_non_exhaustive()
    }
}

impl TokenizerRef {
    pub fn new(inner: Arc<dyn Tokenizer>) -> Self {
        Self { inner }
    }

    pub fn grapheme() -> Self {
        Self::new(Arc::new(GraphemeTokenizer))
    }

    pub fn characters() -> Self {
        Self::new(Arc::new(CharacterTokenizer))
    }

    pub fn segment(&self, text: &str) -> Vec<String> {
        self.inner.segment(text)
    }
}

impl Default for TokenizerRef {
    fn default() -> Self {
        Self::grapheme()
    }
}

struct GraphemeTokenizer;

impl Tokenizer for GraphemeTokenizer {
    fn segment(&self, text: &str) -> Vec<String> {
        text.graphemes(true).map(|g| g.to_string()).collect()
    }
}

struct CharacterTokenizer;

impl Tokenizer for CharacterTokenizer {
    fn segment(&self, text: &str) -> Vec<String> {
        text.chars().map(|c| c.to_string()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_normalizes_decomposed() {
        let decomposed = "Cafe\u{301}"; // e + combining acute
        let result = nfc(decomposed);
        assert_eq!(result, "Caf\u{e9}"); // precomposed é
    }

    #[test]
    fn nfc_passes_through_ascii() {
        assert_eq!(nfc("HELLO"), "HELLO");
    }

    #[test]
    fn normalize_with_mode_nfkc() {
        let fi = "\u{FB01}"; // ﬁ ligature
        let result = normalize_with_mode(fi, NormalizationMode::NFKC);
        assert_eq!(result, "fi");
    }

    #[test]
    fn grapheme_tokenizer_ascii() {
        let tok = TokenizerRef::grapheme();
        assert_eq!(tok.segment("ABC"), vec!["A", "B", "C"]);
    }

    #[test]
    fn grapheme_tokenizer_combining_marks() {
        let tok = TokenizerRef::grapheme();
        // e + combining acute = one grapheme
        let tokens = tok.segment("e\u{301}x");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[1], "x");
    }

    #[test]
    fn character_tokenizer_splits_combining() {
        let tok = TokenizerRef::characters();
        let tokens = tok.segment("e\u{301}");
        assert_eq!(tokens.len(), 2);
    }

    #[test]
    fn default_tokenizer_is_grapheme() {
        let tok = TokenizerRef::default();
        assert_eq!(tok.segment("AB"), vec!["A", "B"]);
    }
}
