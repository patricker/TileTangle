use std::any::Any;

use crate::{NormalizationMode, nfc, normalize_with_mode};

use super::{Dictionary, DictionaryOptions};

#[derive(Debug, Clone, Default)]
pub struct SetDictionary {
    words: std::collections::HashSet<String>,
    pub(crate) case_fold: bool,
    pub(crate) norm: NormalizationMode,
}

impl SetDictionary {
    pub fn from_file<P: AsRef<std::path::Path>>(
        path: P,
        opts: DictionaryOptions,
    ) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let mut set = std::collections::HashSet::new();
        let reader = BufReader::new(f);
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
            set.insert(w);
        }
        Ok(Self {
            words: set,
            case_fold: opts.case_fold,
            norm: opts.norm,
        })
    }

    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set = std::collections::HashSet::new();
        for w in iter {
            let mut s = nfc(w.into());
            if case_fold {
                s = s.to_lowercase();
            }
            set.insert(s);
        }
        Self {
            words: set,
            case_fold,
            norm: NormalizationMode::NFC,
        }
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut set = std::collections::HashSet::new();
        for w in iter {
            let mut s = normalize_with_mode(w.into(), opts.norm);
            if opts.case_fold {
                s = s.to_lowercase();
            }
            set.insert(s);
        }
        Self {
            words: set,
            case_fold: opts.case_fold,
            norm: opts.norm,
        }
    }
}

impl Dictionary for SetDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = normalize_with_mode(word, self.norm);
        if self.case_fold {
            s = s.to_lowercase();
        }
        self.words.contains(&s)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        let mut p = normalize_with_mode(prefix, self.norm);
        if self.case_fold {
            p = p.to_lowercase();
        }
        self.words.iter().any(|w| w.starts_with(&p))
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
    fn dictionary_normalization_and_casefold() {
        use unicode_normalization::UnicodeNormalization;
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let dict = SetDictionary::from_words(vec![decomposed.clone()], false);
        assert!(dict.contains(&composed));
        let dict_cf = SetDictionary::from_words(vec!["café".to_string()], true);
        assert!(dict_cf.contains("CAFÉ"));
        let dict_no = SetDictionary::from_words(vec!["café".to_string()], false);
        assert!(!dict_no.contains("CAFÉ"));
    }
    #[test]
    fn dictionary_loader_from_file() {
        let dir = std::env::temp_dir();
        let path = dir.join("tiletangle_dict_test.txt");
        let content = "# sample\nHELLO\nworld\n \nCafé\n";
        std::fs::write(&path, content).unwrap();
        let dict = SetDictionary::from_file(
            &path,
            super::DictionaryOptions {
                case_fold: true,
                min_len: Some(2),
                max_len: None,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(dict.contains("hello"));
        assert!(dict.contains("WORLD"));
        assert!(dict.contains("cafe\u{301}"));
        assert!(!dict.contains("x"));
        let _ = std::fs::remove_file(&path);
    }
}
