//! Dictionary subsystem (extracted from lib.rs)

use std::any::Any;

use crate::{NormalizationMode, TokenizerRef};

pub trait Dictionary {
    fn contains(&self, word: &str) -> bool;
    fn has_prefix(&self, _prefix: &str) -> bool {
        false
    }
    fn as_any(&self) -> &dyn Any;
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync>;
}

#[derive(Debug, Clone)]
pub struct DictionaryOptions {
    pub case_fold: bool,
    pub min_len: Option<usize>,
    pub max_len: Option<usize>,
    pub norm: NormalizationMode,
    pub tokenizer: TokenizerRef,
}

impl Default for DictionaryOptions {
    fn default() -> Self {
        Self {
            case_fold: false,
            min_len: None,
            max_len: None,
            norm: NormalizationMode::NFC,
            tokenizer: TokenizerRef::default(),
        }
    }
}

mod set;
mod fst;
mod dawg;
mod gaddag;

pub use set::SetDictionary;
pub use fst::FstDictionary;
pub use dawg::DawgDictionary;
pub use gaddag::{GaddagDictionary, GaddagCursor, GaddagRight};
