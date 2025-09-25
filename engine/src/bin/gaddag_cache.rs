use std::path::PathBuf;
use engine as eng;

fn print_usage() {
    eprintln!(
        "Usage: gaddag_cache <input.txt> <output.cbor[.gz|.zst]> [--case-fold] [--norm nfc|nfkc] [--tokenizer grapheme|char] [--gzip|--zstd]"
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let input = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            print_usage();
            std::process::exit(2);
        }
    };
    let output = match args.next() {
        Some(p) => PathBuf::from(p),
        None => {
            print_usage();
            std::process::exit(2);
        }
    };

    let mut case_fold = false;
    let mut norm = eng::NormalizationMode::NFC;
    let mut tokenizer = eng::TokenizerRef::default();

    let mut do_gzip = false;
    let mut do_zstd = false;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--case-fold" => case_fold = true,
            "--norm" => {
                let v = args.next().unwrap_or_else(|| "nfc".to_string());
                norm = match v.to_ascii_lowercase().as_str() {
                    "nfc" => eng::NormalizationMode::NFC,
                    "nfkc" => eng::NormalizationMode::NFKC,
                    other => {
                        eprintln!("Unknown normalization mode: {}", other);
                        std::process::exit(2);
                    }
                };
            }
            "--tokenizer" => {
                let v = args.next().unwrap_or_else(|| "grapheme".to_string());
                match v.to_ascii_lowercase().as_str() {
                    "grapheme" => tokenizer = eng::TokenizerRef::default(),
                    "char" | "chars" | "character" => tokenizer = eng::TokenizerRef::characters(),
                    other => {
                        eprintln!("Unknown tokenizer: {}", other);
                        std::process::exit(2);
                    }
                }
            }
            "--gzip" => do_gzip = true,
            "--zstd" => do_zstd = true,
            other => {
                eprintln!("Unknown flag: {}", other);
                print_usage();
                std::process::exit(2);
            }
        }
    }

    let opts = eng::DictionaryOptions {
        case_fold,
        min_len: None,
        max_len: None,
        norm,
        tokenizer,
    };
    eprintln!(
        "Building GADDAG from {} (case_fold={}, norm={:?})",
        input.display(), case_fold, norm
    );
    let dict = eng::GaddagDictionary::from_file(&input, opts)?;
    // Always get raw CBOR bytes first
    let bytes = dict.to_gaddag_bytes()?;
    let use_gzip = do_gzip || output.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("gz")).unwrap_or(false);
    let use_zstd = do_zstd || output.extension().and_then(|s| s.to_str()).map(|e| e.eq_ignore_ascii_case("zst")).unwrap_or(false);
    if use_gzip {
        use std::io::Write;
        let f = std::fs::File::create(&output)?;
        let mut enc = flate2::write::GzEncoder::new(f, flate2::Compression::default());
        enc.write_all(&bytes)?;
        enc.finish()?;
    } else if use_zstd {
        #[cfg(feature = "zstd")]
        {
            std::fs::write(&output, zstd::stream::encode_all(std::io::Cursor::new(&bytes), 10)?)?;
        }
        #[cfg(not(feature = "zstd"))]
        {
            eprintln!("--zstd requested but 'zstd' feature not enabled; re-run with: cargo run -p tiletangle-engine --features zstd --bin gaddag_cache ...");
            std::process::exit(2);
        }
    } else {
        std::fs::write(&output, &bytes)?;
    }
    eprintln!("Wrote {}", output.display());
    Ok(())
}
