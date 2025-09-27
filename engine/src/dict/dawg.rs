use std::any::Any;

use crate::{NormalizationMode, TokenizerRef, normalize_with_mode};

use super::{Dictionary, DictionaryOptions};

#[derive(Debug, Clone, Default)]
pub struct DawgDictionary {
    pub(crate) nodes: Vec<DawgNode>,
    pub(crate) case_fold: bool,
    pub(crate) norm: NormalizationMode,
    pub(crate) tokenizer: TokenizerRef,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DawgNode {
    edges: std::collections::HashMap<String, usize>,
    terminal: bool,
}

impl DawgDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let opts = DictionaryOptions {
            case_fold,
            ..Default::default()
        };
        Self::from_words_opts(iter, opts)
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut words: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = normalize_with_mode(w.into(), opts.norm);
                if opts.case_fold {
                    s = s.to_lowercase();
                }
                s
            })
            .collect();
        words.sort();
        words.dedup();
        Self::build_from_words(words, opts)
    }

    fn build_from_words(words: Vec<String>, opts: DictionaryOptions) -> Self {
        let DictionaryOptions {
            case_fold,
            min_len: _,
            max_len: _,
            norm,
            tokenizer,
        } = opts;
        let mut nodes = vec![DawgNode::default()];
        for w in &words {
            let tokens = tokenizer.segment(w);
            if tokens.is_empty() {
                continue;
            }
            let mut node = 0usize;
            for token in tokens {
                let next = if let Some(&id) = nodes[node].edges.get(&token) {
                    id
                } else {
                    let id = nodes.len();
                    nodes.push(DawgNode::default());
                    nodes[node].edges.insert(token.clone(), id);
                    id
                };
                node = next;
            }
            nodes[node].terminal = true;
        }
        Self {
            nodes,
            case_fold,
            norm,
            tokenizer,
        }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut v: Vec<String> = Vec::new();
        for line in reader.lines() {
            let s = line?;
            let s = s.trim();
            if s.is_empty() || s.starts_with('#') {
                continue;
            }
            let mut w = normalize_with_mode(s, opts.norm);
            if opts.case_fold {
                w = w.to_lowercase();
            }
            let len = opts.tokenizer.segment(&w).len();
            if let Some(min) = opts.min_len
                && len < min
            {
                continue;
            }
            if let Some(max) = opts.max_len
                && len > max
            {
                continue;
            }
            v.push(w);
        }
        Ok(Self::from_words_opts(v, opts))
    }

    pub fn tokenizer(&self) -> &TokenizerRef {
        &self.tokenizer
    }
}

impl Dictionary for DawgDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = normalize_with_mode(word, self.norm);
        if self.case_fold {
            s = s.to_lowercase();
        }
        let mut node = 0usize;
        for token in self.tokenizer.segment(&s) {
            if let Some(&nxt) = self.nodes[node].edges.get(&token) {
                node = nxt;
            } else {
                return false;
            }
        }
        self.nodes.get(node).map(|n| n.terminal).unwrap_or(false)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        let mut p = normalize_with_mode(prefix, self.norm);
        if self.case_fold {
            p = p.to_lowercase();
        }
        let mut node = 0usize;
        for token in self.tokenizer.segment(&p) {
            if let Some(&nxt) = self.nodes[node].edges.get(&token) {
                node = nxt;
            } else {
                return false;
            }
        }
        true
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dawg_dictionary_contains_and_prefix() {
        let dict = DawgDictionary::from_words(
            vec!["AB".to_string(), "ABC".to_string(), "BEE".to_string()],
            true,
        );
        assert!(dict.contains("ab"));
        assert!(dict.has_prefix("ab"));
        assert!(dict.contains("abc"));
        assert!(!dict.contains("abd"));
        assert!(!dict.has_prefix("zz"));
    }
    #[test]
    fn dawg_normalization_and_casefold() {
        use unicode_normalization::UnicodeNormalization;
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let dict_cf = DawgDictionary::from_words(vec![decomposed.clone()], true);
        assert!(dict_cf.contains(&composed));
        assert!(dict_cf.has_prefix("caf"));
        let dict_no = DawgDictionary::from_words(vec!["café".to_string()], false);
        assert!(!dict_no.contains("CAFÉ"));
    }
}
