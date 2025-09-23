use std::any::Any;

use fst::{Automaton, Streamer};
use crate::{normalize_with_mode, NormalizationMode};

use super::{Dictionary, DictionaryOptions};

#[derive(Debug, Clone)]
pub struct FstDictionary {
    pub(crate) set: fst::Set<Vec<u8>>,
    pub(crate) case_fold: bool,
    pub(crate) norm: NormalizationMode,
}

impl FstDictionary {
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
            if s.is_empty() || s.starts_with('#') { continue; }
            let mut w = normalize_with_mode(s, opts.norm);
            if opts.case_fold { w = w.to_lowercase(); }
            let len = opts.tokenizer.segment(&w).len();
            if let Some(min) = opts.min_len && len < min { continue; }
            if let Some(max) = opts.max_len && len > max { continue; }
            v.push(w);
        }
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Ok(Self { set, case_fold: opts.case_fold, norm: opts.norm })
    }

    pub fn from_bytes<D: AsRef<[u8]>>(bytes: D, case_fold: bool) -> Result<Self, fst::Error> {
        let set = fst::Set::new(bytes.as_ref().to_vec())?;
        Ok(Self { set, case_fold, norm: NormalizationMode::NFC })
    }
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut v: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = crate::nfc(w.into());
                if case_fold { s = s.to_lowercase(); }
                s
            })
            .collect();
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Self { set, case_fold, norm: NormalizationMode::NFC }
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut v: Vec<String> = iter
            .into_iter()
            .map(|w| {
                let mut s = normalize_with_mode(w.into(), opts.norm);
                if opts.case_fold { s = s.to_lowercase(); }
                s
            })
            .collect();
        v.sort();
        v.dedup();
        let set = fst::Set::from_iter(v.iter()).expect("build fst set");
        Self { set, case_fold: opts.case_fold, norm: opts.norm }
    }
}

impl Dictionary for FstDictionary {
    fn contains(&self, word: &str) -> bool {
        let mut s = normalize_with_mode(word, self.norm);
        if self.case_fold { s = s.to_lowercase(); }
        self.set.contains(&s)
    }
    fn has_prefix(&self, prefix: &str) -> bool {
        use fst::{IntoStreamer, automaton::Str};
        let mut p = normalize_with_mode(prefix, self.norm);
        if self.case_fold { p = p.to_lowercase(); }
        let aut = Str::new(&p).starts_with();
        let mut stream = self.set.search(aut).into_stream();
        stream.next().is_some()
    }
    fn as_any(&self) -> &dyn Any { self }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> { Box::new(self.clone()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fst_dictionary_contains_and_prefix() {
        let dict = FstDictionary::from_words(vec!["AB".to_string(), "ABC".to_string(), "BEE".to_string()], true);
        assert!(dict.contains("ab"));
        assert!(dict.has_prefix("ab"));
        assert!(dict.contains("abc"));
        assert!(!dict.contains("abd"));
        assert!(!dict.has_prefix("zz"));
    }
    #[test]
    fn nfkc_contains_ligature_variant() {
        let opts = super::DictionaryOptions { case_fold: false, min_len: None, max_len: None, norm: crate::NormalizationMode::NFKC, tokenizer: crate::TokenizerRef::default() };
        let dict = FstDictionary::from_words_opts(vec!["coffee".to_string()], opts);
        let ligature = "coﬀee"; // contains U+FB00
        assert!(dict.contains(ligature));
        let dict_nfc = FstDictionary::from_words(vec!["coffee".to_string()], false);
        assert!(!dict_nfc.contains(ligature));
    }
}
