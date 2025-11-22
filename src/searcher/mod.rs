// src/searcher.rs

use std::io::{self, Write};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, TantivyDocument};

// Import our schema definition, which is the correct way to access fields.
use crate::indexer::schema::WebpageSchema;

/// Runs the interactive search prompt using the Tantivy index.
pub fn run_searcher(index_path: &str) {
    println!("Loading search index from '{}'...", index_path);
    let index = match Index::open_in_dir(index_path) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("Error: Failed to open index directory '{}'. {}", index_path, e);
            eprintln!("Please run the indexer first with: `cargo run -- index`");
            return;
        }
    };

    // Use our schema struct for safe field access.
    let (_schema, fields) = WebpageSchema::build();

    let reader = index.reader().expect("Failed to create index reader.");
    let searcher = reader.searcher();

    // The query parser now correctly searches both the title and body fields.
    let query_parser = QueryParser::for_index(&index, vec![fields.title, fields.body]);

    println!("Index loaded. Ready to search.");

    loop {
        print!("\nEnter search query (or 'exit'): ");
        io::stdout().flush().unwrap();

        let mut query_text = String::new();
        if io::stdin().read_line(&mut query_text).is_err() {
            continue;
        }

        let trimmed_query = query_text.trim();
        if trimmed_query.is_empty() { continue; }
        if trimmed_query.eq_ignore_ascii_case("exit") { break; }

        // --- CHANGE ---
        // We now use the standard text query directly. The custom FunctionScoreQuery
        // has been removed because it is not part of the stable Tantivy API.
        let query = match query_parser.parse_query(trimmed_query) {
            Ok(q) => q,
            Err(e) => {
                eprintln!(" > Error parsing query: {}", e);
                continue;
            }
        };

        // Execute the search using the standard query.
        // The score will be the relevance score (BM25) calculated by Tantivy.
        let top_docs = match searcher.search(&query, &TopDocs::with_limit(10)) {
            Ok(docs) => docs,
            Err(e) => {
                eprintln!(" > Error executing search: {}", e);
                continue;
            }
        };

        if top_docs.is_empty() {
            println!("\nNo results found for '{}'.", trimmed_query);
            continue;
        }
        
        println!("\nFound {} relevant pages for '{}':", top_docs.len(), trimmed_query);

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
            
            // Retrieve title and URL from the stored document fields.
            let title = retrieved_doc
                .get_first(fields.title)
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");

            let url = retrieved_doc
                .get_first(fields.url)
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown URL");

            println!("\n  - Score: {:.4}", score);
            println!("    Title: {}", title);
            println!("    URL:   {}", url);
        }
    }
}