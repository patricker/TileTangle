use criterion::{Criterion, black_box, criterion_group, criterion_main};
use eng::Dictionary;
use engine as eng;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

fn load_words(limit: usize) -> Vec<String> {
    // Resolve repo-rooted path: engine benches run with CWD at the crate dir
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // repo root
    path.push("assets/dictionaries/TWL06.txt");
    let f = File::open(&path).expect("open TWL06.txt");
    let reader = BufReader::new(f);
    let mut out = Vec::with_capacity(limit);
    for line in reader.lines() {
        if out.len() >= limit {
            break;
        }
        let s = line.expect("read line");
        let w = s.trim();
        if w.is_empty() || w.starts_with('#') {
            continue;
        }
        out.push(w.to_string());
    }
    out
}

fn sample_queries(words: &[String], n_in: usize, n_out: usize) -> (Vec<String>, Vec<String>) {
    let mut in_q = Vec::with_capacity(n_in);
    for (i, w) in words.iter().enumerate() {
        if i >= n_in {
            break;
        }
        in_q.push(w.clone());
    }
    // Create simple non-words by appending a symbol unlikely to exist
    let mut out_q = Vec::with_capacity(n_out);
    for (i, w) in words.iter().enumerate() {
        if i >= n_out {
            break;
        }
        out_q.push(format!("{}{}", w, "#"));
    }
    (in_q, out_q)
}

fn bench_build(c: &mut Criterion) {
    let words = load_words(10_000);
    let opts = eng::DictionaryOptions {
        case_fold: true,
        ..Default::default()
    };

    c.bench_function("dict_build_set_10k", |b| {
        b.iter(|| {
            let dict = eng::SetDictionary::from_words_opts(words.clone(), opts.clone());
            black_box(dict);
        })
    });

    c.bench_function("dict_build_fst_10k", |b| {
        b.iter(|| {
            let dict = eng::FstDictionary::from_words_opts(words.clone(), opts.clone());
            black_box(dict);
        })
    });

    c.bench_function("dict_build_dawg_10k", |b| {
        b.iter(|| {
            let dict = eng::DawgDictionary::from_words_opts(words.clone(), opts.clone());
            black_box(dict);
        })
    });

    c.bench_function("dict_build_gaddag_10k", |b| {
        b.iter(|| {
            let dict = eng::GaddagDictionary::from_words_opts(words.clone(), opts.clone());
            black_box(dict);
        })
    });
}

fn bench_lookup(c: &mut Criterion) {
    let words = load_words(10_000);
    let opts = eng::DictionaryOptions {
        case_fold: true,
        ..Default::default()
    };
    let (in_q, out_q) = sample_queries(&words, 2000, 2000);

    // Set
    c.bench_function("dict_lookup_set_4k", |b| {
        let dict = eng::SetDictionary::from_words_opts(words.clone(), opts.clone());
        b.iter(|| {
            for w in &in_q {
                black_box(dict.contains(w));
            }
            for w in &out_q {
                black_box(dict.contains(w));
            }
        })
    });

    // FST
    c.bench_function("dict_lookup_fst_4k", |b| {
        let dict = eng::FstDictionary::from_words_opts(words.clone(), opts.clone());
        b.iter(|| {
            for w in &in_q {
                black_box(dict.contains(w));
            }
            for w in &out_q {
                black_box(dict.contains(w));
            }
        })
    });

    // DAWG
    c.bench_function("dict_lookup_dawg_4k", |b| {
        let dict = eng::DawgDictionary::from_words_opts(words.clone(), opts.clone());
        b.iter(|| {
            for w in &in_q {
                black_box(dict.contains(w));
            }
            for w in &out_q {
                black_box(dict.contains(w));
            }
        })
    });

    // GADDAG (contains forwards to FST-backed forward dictionary)
    c.bench_function("dict_lookup_gaddag_4k", |b| {
        let dict = eng::GaddagDictionary::from_words_opts(words.clone(), opts.clone());
        b.iter(|| {
            for w in &in_q {
                black_box(dict.contains(w));
            }
            for w in &out_q {
                black_box(dict.contains(w));
            }
        })
    });
}

criterion_group!(benches, bench_build, bench_lookup);
criterion_main!(benches);
