use std::collections::HashMap;
use std::io::{self, Read};
use clap::Parser;

#[derive(Parser, Debug)]

#[clap(author, version, about = "Compte la fréquence des mots dans un texte donné.")]
struct Cli {
    #[arg(long, value_name = "N")]
    top: Option<usize>,

    #[arg(long)]
    ignore_case: bool,

    input_text: Option<String>,
}

fn main() -> io::Result<()> {
    let args = Cli::parse();
    
    let mut input = String::new();

    if let Some(text) = args.input_text {
              input = text;
    } else {
                io::stdin().read_to_string(&mut input)?;
    }
    
    if input.is_empty() {
        return Ok(());
    }

    let mut word_counts: HashMap<String, usize> = HashMap::new();

    for word in input.split_whitespace() {
        let mut clean_word = word.trim_matches(|c: char| !c.is_alphanumeric()).to_string();

        if args.ignore_case {
            clean_word = clean_word.to_lowercase();
        }
        
        if clean_word.is_empty() {
            continue;
        }

        *word_counts.entry(clean_word).or_insert(0) += 1;
    }

    let mut sorted_counts: Vec<(String, usize)> = word_counts.into_iter().collect();

    sorted_counts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let top_n = args.top.unwrap_or(sorted_counts.len());
    let results = sorted_counts.iter().take(top_n);
    
    println!("word frequency:");
    if args.top.is_some() {
        println!("Top {} words:", args.top.unwrap());
    } else {
        println!("word frequency:");
    }

    for (word, count) in results {
        println!("{}: {}", word, count);
    }

    Ok(())
}
