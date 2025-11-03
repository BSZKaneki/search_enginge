// src/bin/searcher.rs

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Write};
use serde::Deserialize;

#[derive(Default, Deserialize)]
struct ScoredIndex {
    scores: HashMap<String, HashMap<String, f64>>,
}

fn main() -> io::Result<()> {
    println!("Loading search index...");
    let file = File::open("scored_index.json")?;
    let reader = BufReader::new(file);
    let index: ScoredIndex = serde_json::from_reader(reader)
        .expect("Failed to parse scored_index.json. Run the crawler first via `cargo run`.");
    println!("Index loaded. Ready to search.");

    loop {
        print!("\nEnter search query (e.g., 'berserk anime') or 'exit': ");
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query_terms: Vec<String> = query.trim().to_lowercase().split_whitespace().map(String::from).collect();

        if query_terms.is_empty() {
            continue;
        }
        if query_terms.len() == 1 && query_terms[0] == "exit" {
            break;
        }

        // Aggregate scores for each URL based on the query terms.
        let mut results: HashMap<String, f64> = HashMap::new();
        for term in &query_terms {
            if let Some(url_scores) = index.scores.get(term) {
                for (url, score) in url_scores {
                    *results.entry(url.clone()).or_insert(0.0) += score;
                }
            }
        }
        
        if results.is_empty() {
            println!("No results found for '{}'.", query.trim());
            continue;
        }

        // Sort the results by score in descending order for ranking.
        let mut sorted_results: Vec<_> = results.into_iter().collect();
        sorted_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        println!("\nFound {} relevant pages for '{}':", sorted_results.len(), query.trim());
        
        // Print the top 10 most relevant results.
        for (url, score) in sorted_results.iter().take(10) {
            println!("  - [{:.4}] {}", score, url);
        }
    }

    Ok(())
}