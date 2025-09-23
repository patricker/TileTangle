use std::any::Any;

use fst::Automaton;
use serde::{Deserialize, Serialize};

use crate::{normalize_with_mode, NormalizationMode, TokenizerRef};

use super::{Dictionary, DictionaryOptions};
use super::FstDictionary;

#[derive(Debug, Clone)]
pub struct GaddagDictionary {
    pub(crate) forward: FstDictionary,
    // packed graph
    pub(crate) g_nodes: Vec<PackedNode>,
    pub(crate) g_arcs: Vec<PackedArc>,
    // symbol mapping
    pub(crate) sym2id: std::collections::HashMap<String, u16>,
    pub(crate) id2sym: Vec<String>,
    sep: String,
    sep_id: u16,
    tokenizer: TokenizerRef,
}

#[derive(Debug, Clone, Copy)]
pub struct PackedNode {
    pub(crate) offset: u32,
    pub(crate) degree: u16,
    pub(crate) flags: u16, // bit 0 => terminal
}
impl PackedNode {
    #[inline]
    pub fn terminal(&self) -> bool { (self.flags & 1) != 0 }
}

#[derive(Debug, Clone, Copy)]
pub struct PackedArc {
    pub(crate) label: u16,
    pub(crate) target: u32,
}

// Transient builder node (u16-labeled sorted map)
#[derive(Default)]
struct BuildNode {
    edges: std::collections::BTreeMap<u16, usize>,
    terminal: bool,
}

// On-disk representation for packed GADDAG
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackedNodeDisk {
    offset: u32,
    degree: u16,
    flags: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackedArcDisk {
    label: u16,
    target: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GaddagDiskImage {
    sep: String,
    sep_id: u16,
    id2sym: Vec<String>,
    nodes: Vec<PackedNodeDisk>,
    arcs: Vec<PackedArcDisk>,
    words: Vec<String>,
    case_fold: bool,
    // 0 = NFC, 1 = NFKC
    norm_mode: u8,
}

impl GaddagDictionary {
    pub fn from_words<I, S>(iter: I, case_fold: bool) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let opts = DictionaryOptions { case_fold, ..Default::default() };
        Self::from_words_opts(iter, opts)
    }

    pub fn from_words_opts<I, S>(iter: I, opts: DictionaryOptions) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let DictionaryOptions { case_fold, min_len, max_len, norm, tokenizer } = opts;
        let sep = "+".to_string();

        // Normalize/filter words first
        let mut words: Vec<String> = Vec::new();
        for w in iter.into_iter() {
            let mut s = normalize_with_mode(w.into(), norm);
            if case_fold { s = s.to_lowercase(); }
            if s.is_empty() { continue; }
            let tks = tokenizer.segment(&s);
            if tks.is_empty() { continue; }
            let len = tks.len();
            if let Some(min) = min_len { if len < min { continue; } }
            if let Some(max) = max_len { if len > max { continue; } }
            words.push(s);
        }

        // Build small symbol table
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        let mut id2sym: Vec<String> = Vec::new();
        let mut intern = |sym: &str,
                          map: &mut std::collections::HashMap<String, u16>,
                          vec: &mut Vec<String>| -> u16 {
            if let Some(&id) = map.get(sym) { return id; }
            let id = vec.len() as u16;
            map.insert(sym.to_string(), id);
            vec.push(sym.to_string());
            id
        };
        for s in &words {
            for tk in tokenizer.segment(s) {
                let _ = intern(&tk, &mut sym2id, &mut id2sym);
            }
        }
        let sep_id = intern(&sep, &mut sym2id, &mut id2sym);

        // Build trie over u16 labels
        let mut build_nodes: Vec<BuildNode> = vec![BuildNode::default()]; // root = 0
        for s in &words {
            let tokens = tokenizer.segment(s);
            let n = tokens.len();
            for split in 0..=n {
                let mut seq: Vec<u16> = Vec::with_capacity(n + 1);
                for tk in tokens[..split].iter().rev() { seq.push(*sym2id.get(tk).expect("interned")); }
                seq.push(sep_id);
                for tk in &tokens[split..] { seq.push(*sym2id.get(tk).expect("interned")); }
                let mut node = 0usize;
                for &lab in &seq {
                    let next = if let Some(&id) = build_nodes[node].edges.get(&lab) {
                        id
                    } else {
                        let id = build_nodes.len();
                        build_nodes.push(BuildNode::default());
                        build_nodes[node].edges.insert(lab, id);
                        id
                    };
                    node = next;
                }
                build_nodes[node].terminal = true;
            }
        }

        // Freeze packed arrays
        let mut g_nodes: Vec<PackedNode> = Vec::with_capacity(build_nodes.len());
        let mut g_arcs: Vec<PackedArc> = Vec::new();
        for bn in &build_nodes {
            let offset = g_arcs.len() as u32;
            let degree = bn.edges.len() as u16;
            for (&label, &target) in bn.edges.iter() {
                g_arcs.push(PackedArc { label, target: target as u32 });
            }
            g_nodes.push(PackedNode { offset, degree, flags: if bn.terminal { 1 } else { 0 } });
        }

        let forward_opts = DictionaryOptions { case_fold, min_len, max_len, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(words.clone(), forward_opts);
        Self { forward, g_nodes, g_arcs, sym2id, id2sym, sep, sep_id, tokenizer }
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P, opts: DictionaryOptions) -> std::io::Result<Self> {
        use std::io::{BufRead, BufReader};
        let f = std::fs::File::open(path)?;
        let reader = BufReader::new(f);
        let mut words: Vec<String> = Vec::new();
        for line in reader.lines() {
            let raw = line?;
            let mut s = normalize_with_mode(raw.trim(), opts.norm);
            if opts.case_fold { s = s.to_lowercase(); }
            if s.is_empty() { continue; }
            let len = opts.tokenizer.segment(&s).len();
            if let Some(min) = opts.min_len && len < min { continue; }
            if let Some(max) = opts.max_len && len > max { continue; }
            words.push(s);
        }
        Ok(Self::from_words_opts(words, opts))
    }

    pub fn to_gaddag_file<P: AsRef<std::path::Path>>(&self, path: P) -> std::io::Result<()> {
        use std::io::BufWriter;
        let norm_mode: u8 = match self.forward.norm { NormalizationMode::NFC => 0, NormalizationMode::NFKC => 1 };
        // Collect all words from FST using an empty prefix automaton.
        let mut words: Vec<String> = Vec::new();
        {
            use fst::{IntoStreamer, Streamer, automaton::Str};
            let aut = Str::new("").starts_with();
            let mut stream = self.forward.set.search(aut).into_stream();
            while let Some(bytes) = stream.next() {
                let s = std::str::from_utf8(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("invalid utf8 in FST: {}", e)))?
                    .to_string();
                words.push(s);
            }
        }
        let nodes: Vec<PackedNodeDisk> = self
            .g_nodes
            .iter()
            .map(|n| PackedNodeDisk { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let arcs: Vec<PackedArcDisk> = self
            .g_arcs
            .iter()
            .map(|a| PackedArcDisk { label: a.label, target: a.target })
            .collect();
        let image = GaddagDiskImage {
            sep: self.sep.clone(),
            sep_id: self.sep_id,
            id2sym: self.id2sym.clone(),
            nodes,
            arcs,
            words,
            case_fold: self.forward.case_fold,
            norm_mode,
        };
        let f = std::fs::File::create(path)?;
        let mut w = BufWriter::new(f);
        ciborium::ser::into_writer(&image, &mut w)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    }

    pub fn to_gaddag_bytes(&self) -> std::io::Result<Vec<u8>> {
        let norm_mode: u8 = match self.forward.norm { NormalizationMode::NFC => 0, NormalizationMode::NFKC => 1 };
        let mut words: Vec<String> = Vec::new();
        {
            use fst::{IntoStreamer, Streamer, automaton::Str};
            let aut = Str::new("").starts_with();
            let mut stream = self.forward.set.search(aut).into_stream();
            while let Some(bytes) = stream.next() {
                let s = std::str::from_utf8(bytes)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("invalid utf8 in FST: {}", e)))?
                    .to_string();
                words.push(s);
            }
        }
        let nodes: Vec<PackedNodeDisk> = self
            .g_nodes
            .iter()
            .map(|n| PackedNodeDisk { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let arcs: Vec<PackedArcDisk> = self
            .g_arcs
            .iter()
            .map(|a| PackedArcDisk { label: a.label, target: a.target })
            .collect();
        let image = GaddagDiskImage { sep: self.sep.clone(), sep_id: self.sep_id, id2sym: self.id2sym.clone(), nodes, arcs, words, case_fold: self.forward.case_fold, norm_mode };
        let mut buf = Vec::new();
        ciborium::ser::into_writer(&image, &mut buf)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        Ok(buf)
    }

    pub fn from_gaddag_file<P: AsRef<std::path::Path>>(path: P, tokenizer: TokenizerRef) -> std::io::Result<Self> {
        use std::io::BufReader;
        let f = std::fs::File::open(path)?;
        let r = BufReader::new(f);
        let image: GaddagDiskImage =
            ciborium::de::from_reader(r).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let g_nodes: Vec<PackedNode> = image
            .nodes
            .iter()
            .map(|n| PackedNode { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let g_arcs: Vec<PackedArc> = image
            .arcs
            .iter()
            .map(|a| PackedArc { label: a.label, target: a.target })
            .collect();
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        for (i, s) in image.id2sym.iter().enumerate() { sym2id.insert(s.clone(), i as u16); }
        let norm = match image.norm_mode { 0 => NormalizationMode::NFC, 1 => NormalizationMode::NFKC, _ => NormalizationMode::NFC };
        let forward_opts = DictionaryOptions { case_fold: image.case_fold, min_len: None, max_len: None, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(image.words, forward_opts.clone());
        Ok(Self { forward, g_nodes, g_arcs, sym2id, id2sym: image.id2sym, sep: image.sep, sep_id: image.sep_id, tokenizer })
    }

    pub fn from_gaddag_bytes<D: AsRef<[u8]>>(bytes: D, tokenizer: TokenizerRef) -> std::io::Result<Self> {
        let cursor = std::io::Cursor::new(bytes.as_ref());
        let image: GaddagDiskImage =
            ciborium::de::from_reader(cursor).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        let g_nodes: Vec<PackedNode> = image
            .nodes
            .iter()
            .map(|n| PackedNode { offset: n.offset, degree: n.degree, flags: n.flags })
            .collect();
        let g_arcs: Vec<PackedArc> = image
            .arcs
            .iter()
            .map(|a| PackedArc { label: a.label, target: a.target })
            .collect();
        let mut sym2id: std::collections::HashMap<String, u16> = std::collections::HashMap::new();
        for (i, s) in image.id2sym.iter().enumerate() { sym2id.insert(s.clone(), i as u16); }
        let norm = match image.norm_mode { 0 => NormalizationMode::NFC, 1 => NormalizationMode::NFKC, _ => NormalizationMode::NFC };
        let forward_opts = DictionaryOptions { case_fold: image.case_fold, min_len: None, max_len: None, norm, tokenizer: tokenizer.clone() };
        let forward = FstDictionary::from_words_opts(image.words, forward_opts.clone());
        Ok(Self { forward, g_nodes, g_arcs, sym2id, id2sym: image.id2sym, sep: image.sep, sep_id: image.sep_id, tokenizer })
    }

    pub fn root(&self) -> usize { 0 }
    pub fn sep_token(&self) -> &str { &self.sep }

    pub fn step_token(&self, node: usize, token: &str) -> Option<usize> {
        let &id = self.sym2id.get(token)?;
        self.step_by_id(node, id)
    }

    #[inline]
    fn step_by_id(&self, node: usize, sym_id: u16) -> Option<usize> {
        let n = *self.g_nodes.get(node)?;
        let slice = &self.g_arcs[n.offset as usize .. n.offset as usize + n.degree as usize];
        let mut lo = 0usize;
        let mut hi = slice.len();
        while lo < hi {
            let mid = (lo + hi) >> 1;
            let m = &slice[mid];
            if m.label < sym_id { lo = mid + 1; } else { hi = mid; }
        }
        if lo < slice.len() && slice[lo].label == sym_id { Some(slice[lo].target as usize) } else { None }
    }

    pub fn is_terminal(&self, node: usize) -> bool {
        self.g_nodes.get(node).map(|n| n.terminal()).unwrap_or(false)
    }

    /// Enumerate simple rightward suffixes from an anchor with given left context using rack letters.
    pub fn enumerate_suffixes_simple(
        &self,
        left_context: &str,
        rack: &mut std::collections::HashMap<String, usize>,
        max_len: usize,
    ) -> Vec<String> {
        // compute pre node from left_context quickly
        let mut node = self.root();
        for token in self.tokenizer.segment(left_context).into_iter().rev() {
            let &id = if let Some(id) = self.sym2id.get(&token) { id } else { return vec![]; };
            if let Some(n2) = self.step_by_id(node, id) { node = n2; } else { return vec![]; }
        }
        if let Some(n2) = self.step_by_id(node, self.sep_id) { node = n2; } else { return vec![]; }

        // DFS on packed arcs
        let mut out = Vec::new();
        fn dfs(
            g: &GaddagDictionary,
            node: usize,
            built: &mut Vec<String>,
            rack: &mut std::collections::HashMap<String, usize>,
            out: &mut Vec<String>,
            left_context: &str,
            max_len: usize,
        ) {
            if built.len() >= max_len { return; }
            if !built.is_empty() && g.is_terminal(node) {
                let suffix = built.join("");
                out.push(format!("{}{}", left_context, suffix));
            }
            let ninfo = &g.g_nodes[node];
            let arcs = &g.g_arcs[ninfo.offset as usize .. ninfo.offset as usize + ninfo.degree as usize];
            for arc in arcs {
                if arc.label == g.sep_id { continue; }
                let sym = &g.id2sym[arc.label as usize];
                if rack.get(sym).copied().unwrap_or(0) > 0 {
                    { let c = rack.get_mut(sym).unwrap(); *c -= 1; }
                    built.push(sym.clone());
                    dfs(g, arc.target as usize, built, rack, out, left_context, max_len);
                    built.pop();
                    { let c = rack.get_mut(sym).unwrap(); *c += 1; }
                }
            }
        }
        dfs(self, node, &mut Vec::new(), rack, &mut out, left_context, max_len);
        out
    }

    pub fn step_symbol(&self, node: usize, sym: &str) -> Option<usize> { self.step_token(node, sym) }
    pub fn tokenizer(&self) -> &TokenizerRef { &self.tokenizer }

    // helpers
    pub fn symbol_id(&self, sym: &str) -> Option<u16> { self.sym2id.get(sym).copied() }
    pub fn id_to_symbol(&self, id: u16) -> &str { &self.id2sym[id as usize] }
    pub fn alphabet_len(&self) -> usize { self.id2sym.len() }
}

pub struct GaddagCursor<'a> {
    dict: &'a GaddagDictionary,
    pre: usize,
}

pub struct GaddagRight<'a> {
    dict: &'a GaddagDictionary,
    node: usize,
}

impl<'a> GaddagCursor<'a> {
    pub fn new(dict: &'a GaddagDictionary, left_context: &str) -> Option<Self> {
        let mut node = dict.root();
        for token in dict.tokenizer.segment(left_context).into_iter().rev() {
            node = dict.step_token(node, &token)?;
        }
        Some(Self { dict, pre: node })
    }
    // Fast path without tokenization
    pub fn new_from_tokens(dict: &'a GaddagDictionary, left_tokens: &[u16]) -> Option<Self> {
        let mut node = dict.root();
        for &id in left_tokens.iter().rev() { node = dict.step_by_id(node, id)?; }
        Some(Self { dict, pre: node })
    }
    pub fn step_left(&self, sym: &str) -> Option<Self> {
        let n = self.dict.step_token(self.pre, sym)?;
        Some(Self { dict: self.dict, pre: n })
    }
    pub fn branch_right(&self) -> Option<GaddagRight<'a>> {
        let n = self.dict.step_token(self.pre, self.dict.sep_token())?;
        Some(GaddagRight { dict: self.dict, node: n })
    }
    pub fn pre_node(&self) -> usize { self.pre }
}

impl<'a> GaddagRight<'a> {
    pub fn step(&self, sym: &str) -> Option<Self> {
        let n = self.dict.step_token(self.node, sym)?;
        Some(Self { dict: self.dict, node: n })
    }
    // Step by symbol id (fast path)
    pub fn step_id(&self, sym_id: u16) -> Option<Self> {
        let n = self.dict.step_by_id(self.node, sym_id)?;
        Some(Self { dict: self.dict, node: n })
    }
    pub fn is_terminal(&self) -> bool { self.dict.is_terminal(self.node) }
    pub fn node(&self) -> usize { self.node }
}

impl Dictionary for GaddagDictionary {
    fn contains(&self, word: &str) -> bool { self.forward.contains(word) }
    fn has_prefix(&self, prefix: &str) -> bool { self.forward.has_prefix(prefix) }
    fn as_any(&self) -> &dyn Any { self }
    fn boxed_clone(&self) -> Box<dyn Dictionary + Send + Sync> { Box::new(self.clone()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TokenizerRef, Tokenizer, NormalizationMode};

    #[test]
    fn gaddag_dictionary_basic() {
        let dict = GaddagDictionary::from_words(vec!["CARE".to_string(), "CARES".to_string()], true);
        assert!(dict.contains("care"));
        assert!(dict.has_prefix("ca"));
    }

    #[test]
    fn gaddag_forms_for_cares() {
        let gd = GaddagDictionary::from_words(vec!["CARES".to_string()], true);
        fn has_seq(gd: &GaddagDictionary, s: &str) -> bool {
            let mut node = gd.root();
            for token in gd.tokenizer().segment(s) {
                if let Some(nxt) = gd.step_token(node, &token) { node = nxt; } else { return false; }
            }
            gd.is_terminal(node)
        }
        assert!(has_seq(&gd, "+cares"));
        assert!(has_seq(&gd, "c+ares"));
        assert!(has_seq(&gd, "ac+res"));
        assert!(has_seq(&gd, "rac+es"));
        assert!(has_seq(&gd, "erac+s"));
        assert!(has_seq(&gd, "serac+"));
    }

    #[test]
    fn gaddag_id_cursor_and_step_id() {
        let gd = GaddagDictionary::from_words(vec!["AB".to_string()], true);
        let a = gd.symbol_id("a").expect("a id");
        let b = gd.symbol_id("b").expect("b id");
        let cur = GaddagCursor::new_from_tokens(&gd, &[a]).expect("cursor");
        let right = cur.branch_right().expect("branch");
        let r2 = right.step_id(b).expect("step b");
        assert!(r2.is_terminal());
    }

    #[test]
    fn gaddag_enumerate_suffixes_simple_ab() {
        let gd = GaddagDictionary::from_words(vec!["ab".to_string()], true);
        let mut rack: std::collections::HashMap<String, usize> =
            std::collections::HashMap::from([("a".to_string(), 1usize), ("b".to_string(), 1usize)]);
        let words = gd.enumerate_suffixes_simple("", &mut rack, 8);
        assert!(words.contains(&"ab".to_string()));
    }

    #[test]
    fn gaddag_normalization_and_casefold() {
        use unicode_normalization::UnicodeNormalization;
        let composed = "Café".to_string();
        let decomposed = "Cafe\u{301}".nfc().collect::<String>();
        let gd = GaddagDictionary::from_words(vec![decomposed.clone()], true);
        assert!(gd.contains(&composed));
        assert!(gd.has_prefix("caf"));
    }

    #[test]
    fn gaddag_serialize_roundtrip_default() {
        use std::path::PathBuf;
        let gd = GaddagDictionary::from_words(vec!["CARE".to_string(), "CARES".to_string()], true);
        let path = PathBuf::from(std::env::temp_dir()).join("gaddag_test_default.cbor");
        gd.to_gaddag_file(&path).unwrap();
        let gd2 = GaddagDictionary::from_gaddag_file(&path, TokenizerRef::default()).unwrap();
        assert!(gd2.contains("cares"));
        assert!(gd2.contains("CARE"));
    }

    #[test]
    fn gaddag_serialize_roundtrip_custom_tokenizer() {
        use std::sync::Arc;
        struct QuTokenizer;
        impl Tokenizer for QuTokenizer {
            fn segment(&self, text: &str) -> Vec<String> {
                let mut out = Vec::new();
                let mut chars = text.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == 'q' && chars.peek() == Some(&'u') {
                        chars.next();
                        out.push("qu".to_string());
                    } else {
                        out.push(ch.to_string());
                    }
                }
                out
            }
        }
        let tokenizer = TokenizerRef::new(Arc::new(QuTokenizer));
        let opts = super::DictionaryOptions { tokenizer: tokenizer.clone(), ..Default::default() };
        let gd = GaddagDictionary::from_words_opts(vec!["qu".to_string()], opts);
        let p = std::env::temp_dir().join("gaddag_test_qu.cbor");
        gd.to_gaddag_file(&p).unwrap();
        let gd2 = GaddagDictionary::from_gaddag_file(&p, tokenizer).unwrap();
        assert!(gd2.step_symbol(gd2.root(), "qu").is_some());
    }

    #[test]
    fn gaddag_bytes_roundtrip() {
        let gd = GaddagDictionary::from_words(vec!["AB".to_string(), "ABC".to_string()], true);
        let bytes = gd.to_gaddag_bytes().unwrap();
        let gd2 = GaddagDictionary::from_gaddag_bytes(bytes, TokenizerRef::default()).unwrap();
        assert!(gd2.contains("ab"));
        assert!(gd2.contains("abc"));
    }
}
