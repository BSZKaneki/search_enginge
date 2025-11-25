// src/searcher.rs

use std::io::{self, Write};
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, TantivyDocument};

// Import schema from the indexer module
use crate::indexer::schema::WebpageSchema;

/// Runs the interactive search prompt.
pub fn run_searcher(index_path: &str) {
    println!("Loading search index from '{}'...", index_path);
    
    let index = match Index::open_in_dir(index_path) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("Error: Failed to open index directory '{}'. {}", index_path, e);
            eprintln!("Please run the indexer first with: `cargo run`");
            return;
        }
    };

    // CRITICAL: We must register the "en_stem" tokenizer logic in the searcher too,
    // otherwise it won't know how to parse the query words.
    WebpageSchema::register_tokenizer(&index);

    // Build fields helper to access field constants safely
    let (_schema, fields) = WebpageSchema::build();

    let reader = index.reader().expect("Failed to create index reader.");
    let searcher = reader.searcher();

    // We search in Title and Body
    let query_parser = QueryParser::for_index(&index, vec![fields.title, fields.body]);

    println!("Index loaded. Ready to search.");
    println!("Type 'exit' to quit.");

    loop {
        print!("\nSearch Query > ");
        io::stdout().flush().unwrap();

        let mut query_text = String::new();
        if io::stdin().read_line(&mut query_text).is_err() {
            continue;
        }

        let trimmed = query_text.trim();
        if trimmed.is_empty() { continue; }
        if trimmed.eq_ignore_ascii_case("exit") { break; }

        // Parse the query
        let query = match query_parser.parse_query(trimmed) {
            Ok(q) => q,
            Err(e) => {
                eprintln!("Error parsing query: {}", e);
                continue;
            }
        };

        // Execute search. 
        // We get the top 10 documents sorted by BM25 relevance score.
        let top_docs = match searcher.search(&query, &TopDocs::with_limit(10)) {
            Ok(docs) => docs,
            Err(e) => {
                eprintln!("Error executing search: {}", e);
                continue;
            }
        };

        if top_docs.is_empty() {
            println!("No results found.");
            continue;
        }
        
        println!("\nFound {} results:", top_docs.len());

        for (score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address).unwrap();
            
            // Helper to extract string fields
            let get_text = |field| {
                retrieved_doc.get_first(field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("[Missing]")
            };

            // Helper to extract f64 fields
            let get_f64 = |field| {
                retrieved_doc.get_first(field)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
            };

            let title = get_text(fields.title);
            let url = get_text(fields.url);
            let lang = get_text(fields.language);
            let pr = get_f64(fields.pagerank);

            println!("------------------------------------------------");
            println!("Title:    {}", title);
            println!("URL:      {}", url);
            println!("Relevance: {:.4} | PageRank: {:.6} | Lang: {}", score, pr, lang);
        }
    }
}