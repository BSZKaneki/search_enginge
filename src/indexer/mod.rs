use std::collections::HashSet;
use std::path::Path;
use tantivy::{doc, Index};

// Declare modules inside the indexer folder
pub mod schema;
pub mod algorithms;

// Import from siblings and root
use self::algorithms::pagerank;
use self::schema::WebpageSchema;
use crate::crawler::Crawler; // <--- Import Crawler from the separate module

pub async fn run_indexer(index_path: &str) {
    println!("--- 1. Starting Crawler (Demon Mode) ---");
    
    let seed_urls = vec![
        "https://en.wikipedia.org/wiki/Computer_science",
        "https://www.rust-lang.org/",
        "https://news.ycombinator.com/",
        "https://github.com/rust-lang/rust",
        "https://stackoverflow.com/questions/tagged/rust"
    ];

    let page_limit = 500; 
    let concurrency = 25;
    
    // Create Crawler from the crate::crawler module
    let mut crawler = Crawler::new(&seed_urls);

    let scraped_data = match crawler.crawl(page_limit, concurrency).await {
        Ok(data) => {
            println!("Crawler finished. Collected {} pages.", data.len());
            data
        },
        Err(e) => {
            eprintln!("Crawler fatal error: {}", e);
            return;
        }
    };

    // --- 2. Calculate PageRank ---
    println!("\n--- 2. Calculating PageRank ---");
    // We map the scraped data into a format PageRank understands
    let link_graph: pagerank::LinkGraph = scraped_data
        .iter()
        .map(|data| (data.url.clone(), data.links.iter().cloned().collect::<HashSet<String>>()))
        .collect();

    let page_ranks = pagerank::calculate_pagerank(&link_graph);
    println!("PageRank calculation complete.");

    // --- 3. Build Index ---
    println!("\n--- 3. Indexing to '{}' ---", index_path);

    let (schema, fields) = WebpageSchema::build();
    let index_dir = Path::new(index_path);
    
    if !index_dir.exists() {
        std::fs::create_dir_all(index_dir).expect("Failed to create index dir");
    }

    let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(index_dir).unwrap(), schema.clone())
        .expect("Failed to open index");

    WebpageSchema::register_tokenizer(&index);

    let mut index_writer = index.writer(200_000_000).expect("Failed to create writer");
    index_writer.delete_all_documents().expect("Failed to clear old index");

    for result in scraped_data {
        let pr_score = page_ranks.get(&result.url).cloned().unwrap_or(0.0);

        index_writer.add_document(doc!(
            fields.url => result.url,
            fields.title => result.title.unwrap_or_default(),
            fields.body => result.body_text,
            fields.pagerank => pr_score,
            fields.language => result.language
        )).expect("Failed to add doc");
    }

    index_writer.commit().expect("Commit failed");
    println!("Indexing complete.");
}