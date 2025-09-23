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
