// src/main.rs

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufReader, Write};

mod spider;
use spider::{ScoredIndex, Spider};

const INDEX_FILE: &str = "scored_index.json";

/// The main entry point, which dispatches to the correct mode (index or search).
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    // Use the first argument after the program name as the command.
    // Default to "search" if no command is given.
    let command = args.get(1).map_or("search", |s| s.as_str());

    match command {
        "index" => run_indexer().await,
        "search" => run_searcher(),
        _ => print_usage(),
    }
}

/// Runs the web crawler to build and save the search index.
async fn run_indexer() {
    let mut spider = Spider::new("https://en.wikipedia.org/wiki/Main_Page")
        .expect("Failed to create spider");
    
    let page_limit = 200;
    let concurrency = 10;

    println!("Starting crawler to build search index from the Wikipedia Main Page...");
    if let Err(e) = spider.crawl(page_limit, concurrency).await {
        eprintln!("Crawler finished with an error: {}", e);
    }

    if let Err(e) = spider.build_and_save_index(INDEX_FILE).await {
        eprintln!("Failed to save index: {}", e);
    }
}

/// Runs the interactive search prompt, using a pre-built index file.
fn run_searcher() {
    println!("Loading search index...");
    let file = match File::open(INDEX_FILE) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Error: Index file '{}' not found.", INDEX_FILE);
            eprintln!("Please run the indexer first with: `cargo run -- index`");
            return;
        }
    };

    let reader = BufReader::new(file);
    let index: ScoredIndex = serde_json::from_reader(reader).expect("Failed to parse index file.");
    println!("Index loaded. Ready to search.");

    loop {
        print!("\nEnter search query (e.g., 'berserk anime') or 'exit': ");
        io::stdout().flush().unwrap();

        let mut query = String::new();
        if io::stdin().read_line(&mut query).is_err() {
            continue;
        }
        
        let query_terms: Vec<String> = query.trim().to_lowercase().split_whitespace().map(String::from).collect();

        if query_terms.is_empty() { continue; }
        if query_terms.len() == 1 && query_terms[0] == "exit" { break; }

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

        let mut sorted_results: Vec<_> = results.into_iter().collect();
        sorted_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        println!("\nFound {} relevant pages for '{}':", sorted_results.len(), query.trim());
        
        for (url, score) in sorted_results.iter().take(10) {
            println!("  - [{:.4}] {}", score, url);
        }
    }
}

/// Prints the help message for the user.
fn print_usage() {
    println!("--- Mini Search Engine ---");
    println!("Usage: cargo run -- [COMMAND]");
    println!("\nCommands:");
    println!("  index     Crawl the web and build the search index (takes several minutes).");
    println!("  search    Start the interactive search prompt (default).");
}