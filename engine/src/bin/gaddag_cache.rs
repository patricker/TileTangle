use std::path::PathBuf;
use engine as eng;

fn print_usage() {
    eprintln!(
        "Usage: gaddag_cache <input.txt> <output.cbor> [--case-fold] [--norm nfc|nfkc] [--tokenizer grapheme|char]"
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
    dict.to_gaddag_file(&output)?;
    eprintln!(
        "Wrote {}",
        output.display()
    );
    Ok(())
}
