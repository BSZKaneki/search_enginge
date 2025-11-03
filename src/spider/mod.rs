// src/spider/mod.rs

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};
use serde::{Deserialize, Serialize};

pub mod datascraper;
use datascraper::{Scraper, ScrapeResult};

/// This struct holds the raw data gathered during the crawl, before final processing.
#[derive(Default, Serialize, Deserialize)]
struct CrawlData {
    /// Maps a URL to a map of words and their counts on that page (for TF).
    page_term_counts: HashMap<String, HashMap<String, u32>>,
    /// Maps a word to the number of documents it appears in (for IDF).
    doc_frequencies: HashMap<String, u32>,
    /// Stores the web graph: URL -> a set of URLs it links to (for PageRank).
    link_graph: HashMap<String, HashSet<String>>,
}

/// This is the final, scored index that will be saved to a file for the searcher.
/// It is `pub` so it can be accessed by `main.rs`.
#[derive(Default, Serialize, Deserialize)]
pub struct ScoredIndex {
    /// Maps a word to a map of URLs and their final combined scores for that word.
    pub scores: HashMap<String, HashMap<String, f64>>,
}

/// The Spider manages the overall crawling process.
#[derive(Clone)]
pub struct Spider {
    scraper: Scraper,
    visited: Arc<Mutex<HashSet<String>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
    crawl_data: Arc<Mutex<CrawlData>>,
}

impl Spider {
    /// Creates a new Spider, ready to start crawling from a given URL.
    pub fn new(start_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            scraper: Scraper::new(),
            visited: Arc::new(Mutex::new(HashSet::new())),
            queue: Arc::new(Mutex::new(VecDeque::from(vec![start_url.to_string()]))),
            crawl_data: Arc::new(Mutex::new(CrawlData::default())),
        })
    }
    
    /// Concurrently crawls web pages, gathering term counts, document frequencies, and the link graph.
    pub async fn crawl(&mut self, limit: usize, concurrency: usize) -> Result<(), Box<dyn std::error::Error>> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();

        while self.visited.lock().await.len() < limit {
            if self.queue.lock().await.is_empty() && semaphore.available_permits() == concurrency {
                break;
            }

            let permit = semaphore.clone().acquire_owned().await?;
            let mut queue_guard = self.queue.lock().await;

            let url = match queue_guard.pop_front() {
                Some(url) => url,
                None => { drop(permit); drop(queue_guard); tokio::time::sleep(Duration::from_millis(50)).await; continue; }
            };
            drop(queue_guard);

            let mut visited_guard = self.visited.lock().await;
            if visited_guard.contains(&url) { continue; }
            println!("Crawling: {}", url);
            visited_guard.insert(url.clone());
            drop(visited_guard);

            let spider_clone = self.clone();

            let handle = tokio::spawn(async move {
                let _permit = permit;
                match spider_clone.scraper.scrape(&url).await {
                    Ok(result) => {
                        println!("  > Counted {} unique words and found {} links on {}.", result.word_counts.len(), result.links.len(), &url);
                        
                        let mut queue_guard = spider_clone.queue.lock().await;
                        for link in &result.links {
                            queue_guard.push_back(link.clone());
                        }
                        drop(queue_guard);

                        // Collect all raw data from the scrape result
                        let mut crawl_data_guard = spider_clone.crawl_data.lock().await;

                        // 1. Update document frequencies for IDF
                        for word in result.word_counts.keys() {
                            *crawl_data_guard.doc_frequencies.entry(word.clone()).or_insert(0) += 1;
                        }
                        
                        // 2. Store the link graph for PageRank
                        let links_set: HashSet<String> = result.links.into_iter().collect();
                        crawl_data_guard.link_graph.insert(url.clone(), links_set);
                        
                        // 3. Store the term counts for TF
                        crawl_data_guard.page_term_counts.insert(url.clone(), result.word_counts);
                    }
                    Err(e) => eprintln!("  > Failed to scrape {}: {}", url, e),
                }
            });
            handles.push(handle);
        }

        for handle in handles { handle.await?; }
        Ok(())
    }

    /// Processes all raw crawl data to build and save the final, ranked search index.
    pub async fn build_and_save_index(&self, index_file: &str) -> Result<(), Box<dyn std::error::Error>> {
        println!("\nCrawl complete. Now processing data...");
        let crawl_data_guard = self.crawl_data.lock().await;
        
        // --- Step 1: Calculate PageRank for Authority Scoring ---
        println!("Calculating PageRank for all discovered pages...");
        let page_ranks = self.calculate_pagerank(&crawl_data_guard.link_graph);
        println!("PageRank calculation complete.");

        // --- Step 2: Calculate TF-IDF and Combine with PageRank ---
        println!("Building final index by combining TF-IDF and PageRank...");
        let total_docs = crawl_data_guard.page_term_counts.len() as f64;
        let mut final_index = ScoredIndex::default();

        for (url, term_counts) in &crawl_data_guard.page_term_counts {
            let total_words_on_page = term_counts.values().sum::<u32>() as f64;
            if total_words_on_page == 0.0 { continue; }

            // Get the pre-calculated authority score for this page.
            let authority_score = page_ranks.get(url).cloned().unwrap_or(0.1);

            for (word, count) in term_counts {
                // Calculate TF (Term Frequency) - How relevant is this word to this page?
                let tf = *count as f64 / total_words_on_page;
                
                // Calculate IDF (Inverse Document Frequency) - How important is this word overall?
                let docs_with_word = *crawl_data_guard.doc_frequencies.get(word).unwrap_or(&1) as f64;
                let idf = (total_docs / docs_with_word).log10();
                
                let relevance_score = tf * idf;

                // Combine relevance and authority for the final score.
                let final_score = relevance_score * authority_score;

                final_index.scores.entry(word.clone()).or_default().insert(url.clone(), final_score);
            }
        }
        
        // --- Step 3: Save the Completed Index to a File ---
        println!("Saving final index to {}...", index_file);
        let json_data = serde_json::to_string(&final_index)?;
        let mut file = File::create(index_file)?;
        file.write_all(json_data.as_bytes())?;
        println!("Index saved successfully.");
        Ok(())
    }

    /// Calculates PageRank for all URLs in the link graph using an iterative algorithm.
    fn calculate_pagerank(&self, link_graph: &HashMap<String, HashSet<String>>) -> HashMap<String, f64> {
        if link_graph.is_empty() {
            return HashMap::new();
        }
        
        let mut ranks: HashMap<String, f64> = link_graph.keys().map(|url| (url.clone(), 1.0)).collect();
        let damping_factor = 0.85; // Standard PageRank damping factor
        let iterations = 20; // Number of iterations to run for convergence

        for _ in 0..iterations {
            let mut new_ranks = HashMap::new();
            for (url, _links) in link_graph {
                let mut new_rank = 1.0 - damping_factor;
                
                // Find all pages that link TO the current page (`url`).
                for (source_url, source_links) in link_graph {
                    if source_links.contains(url) {
                        let source_rank = ranks.get(source_url).cloned().unwrap_or(1.0);
                        let source_link_count = source_links.len() as f64;
                        if source_link_count > 0.0 {
                            // Add a portion of the source's rank to the current page's new rank.
                            new_rank += damping_factor * (source_rank / source_link_count);
                        }
                    }
                }
                new_ranks.insert(url.clone(), new_rank);
            }
            ranks = new_ranks;
        }
        ranks
    }
}