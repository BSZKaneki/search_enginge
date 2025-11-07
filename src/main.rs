use std::env;
// Use the public modules from our library crate.
// Replace `mini_search_engine` with the actual name of your project from Cargo.toml.
use search_enginge::{indexer, searcher};

// The Crawler module is a dependency for the indexer, but main.rs doesn't call it directly,
// so we don't need to `use` it here.

// A single constant for the application's configuration.
const INDEX_PATH: &str = "./search_index";

/// The main entry point, which dispatches to the correct command module.
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    // Use the first argument as the command, defaulting to "search".
    let command = args.get(1).map_or("search", |s| s.as_str());

    match command {
        "index" => indexer::run_indexer(INDEX_PATH).await,
        "search" => searcher::run_searcher(INDEX_PATH),
        _ => print_usage(),
    }
}

/// Prints the help message for the user.
fn print_usage() {
    println!("--- Mini Search Engine ---");
    println!("Usage: cargo run -- [COMMAND]");
    println!("\nCommands:");
    println!("  index     Crawl the web and build the search index.");
    println!("  search    Start the interactive search prompt (default).");
}