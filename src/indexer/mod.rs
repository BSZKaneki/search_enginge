use std::collections::{HashMap, HashSet};
use std::path::Path;
use tantivy::{doc, Index};
use std::time::Duration;

use crate::crawler::Crawler;

// --- Module Declarations ---
pub mod algorithms;
pub mod schema;

use algorithms::pagerank;
use schema::WebpageSchema;

/// Runs the full indexing pipeline.
/// 1. Crawls the web to fetch pages.
/// 2. Processes the crawled data to calculate PageRank.
/// 3. Builds a Tantivy search index from the results.
pub async fn run_indexer(index_path: &str) {
    // --- 1. CRAWLING ---
    // The indexer is responsible for kicking off the crawl.
    println!("--- Starting Crawler ---");
    let start_url = "https://en.wikipedia.org/wiki/Rust_(programming_language)";
    let page_limit = 1500; 
    let concurrency = 10;
    
    let mut crawler = Crawler::new(start_url);

    let scraped_data = match crawler.crawl(page_limit, concurrency).await {
        Ok(data) => {
            println!("Crawler finished successfully. Scraped {} pages.", data.len());
            data
        },
        Err(e) => {
            eprintln!("Crawler finished with an error: {}", e);
            return;
        }
    };

    // --- 2. DATA PROCESSING ---
    // The indexer now processes the raw data from the crawler.
    println!("\n--- Calculating PageRank ---");

    // First, build the link graph from the scrape results.
    let link_graph: pagerank::LinkGraph = scraped_data
        .iter()
        .map(|data| (data.url.clone(), data.links.iter().cloned().collect::<HashSet<String>>()))
        .collect();

    // Now, calculate the PageRank scores.
    let page_ranks = pagerank::calculate_pagerank(&link_graph);
    println!("PageRank calculation complete.");

    // --- 3. TANTIVY INDEXING ---
    // Finally, build the search index.
    println!("\n--- Building Tantivy Index at '{}' ---", index_path);

    let (schema, fields) = WebpageSchema::build();
    let index_dir = Path::new(index_path);
    
    // Create the directory if it doesn't exist.
    if !index_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(index_dir) {
            eprintln!("Failed to create index directory: {}", e);
            return;
        }
    }

    let index = match Index::open_or_create(tantivy::directory::MmapDirectory::open(index_dir).unwrap(), schema.clone()) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("Failed to create or open index: {}", e);
            return;
        }
    };

    // An IndexWriter is used to add, delete, and update documents in the index.
    // We give it a large memory arena (heap) for faster indexing.
    let mut index_writer = index.writer(200_000_000).expect("Failed to create index writer");

    // Clear out any old documents before adding new ones.
    index_writer.delete_all_documents().expect("Failed to clear old index");

    println!("Adding {} documents to the index...", scraped_data.len());
    for result in scraped_data {
        // Look up the calculated PageRank for this URL.
        let pagerank_score = page_ranks.get(&result.url).cloned().unwrap_or(0.0);

        // Tantivy documents are created with the `doc!` macro.
        index_writer.add_document(doc!(
            fields.url => result.url,
            // Assuming ScrapeResult has a title field.
            fields.title => result.title.unwrap_or_default(), 
            // Use the body_text field as the searchable body.
            fields.body => result.body_text.clone(),
            fields.pagerank => pagerank_score
        )).expect("Failed to add document");
    }

    // Commit writes all pending changes to the index. This is an expensive operation.
    if let Err(e) = index_writer.commit() {
        eprintln!("Failed to commit index: {}", e);
    } else {
        println!("Indexing complete and committed successfully.");
    }
}