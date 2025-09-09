use clap::Parser;
use fst::SetBuilder;
use std::fs::File;
use std::io::{BufRead, BufReader};
use unicode_normalization::UnicodeNormalization;

#[derive(Parser, Debug)]
#[command(author, version, about = "Build .fst set file from wordlist")] 
struct Args {
    /// Input text word list (newline separated)
    input: String,
    /// Output .fst path
    output: String,
    /// Apply case-fold (lowercase)
    #[arg(long, default_value_t = true)]
    case_fold: bool,
    /// Minimum length (characters)
    #[arg(long)]
    min_len: Option<usize>,
    /// Maximum length (characters)
    #[arg(long)]
    max_len: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let f = File::open(&args.input)?;
    let reader = BufReader::new(f);
    let mut words: Vec<String> = Vec::new();
    for line in reader.lines() {
        let s = line?;
        let s = s.trim();
        if s.is_empty() || s.starts_with('#') { continue; }
        let mut w: String = s.nfc().collect();
        if args.case_fold { w = w.to_lowercase(); }
        let len = w.chars().count();
        if let Some(min) = args.min_len { if len < min { continue; } }
        if let Some(max) = args.max_len { if len > max { continue; } }
        words.push(w);
    }
    words.sort();
    words.dedup();
    let out = File::create(&args.output)?;
    let mut builder = SetBuilder::new(out)?;
    for w in words { builder.insert(w)?; }
    builder.finish()?;
    Ok(())
}
