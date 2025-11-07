use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};

// --- Module Declarations and Imports ---
// We only need the datascraper now.
pub mod datascraper;
use datascraper::{Scraper, ScrapeResult};

// --- Struct Definition ---

#[derive(Clone)]
pub struct Crawler {
    scraper: Scraper,
    visited: Arc<Mutex<HashSet<String>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
}

impl std::fmt::Debug for Crawler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Crawler")
            .field("scraper", &"<Scraper>")
            .field("visited", &"<visited>")
            .field("queue", &"<queue>")
            .finish()
    }
}

impl Crawler {
    /// Creates a new Crawler, ready to start from a given URL.
    pub fn new(start_url: &str) -> Self {
        Self {
            scraper: Scraper::new(),
            visited: Arc::new(Mutex::new(HashSet::new())),
            queue: Arc::new(Mutex::new(VecDeque::from(vec![start_url.to_string()]))),
        }
    }
    
    /// Crawls the web up to a given limit and returns all the scraped data.
    /// The crawler's single responsibility ends here.
    pub async fn crawl(&mut self, limit: usize, concurrency: usize) -> Result<Vec<ScrapeResult>, Box<dyn std::error::Error>> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();
        // This is the new part: A place to store the results of the crawl.
        let results = Arc::new(Mutex::new(Vec::new()));

        while self.visited.lock().await.len() < limit {
            if self.queue.lock().await.is_empty() && semaphore.available_permits() == concurrency {
                break; // Stop if the queue is empty and all tasks are finished.
            }

            let permit = semaphore.clone().acquire_owned().await?;
            
            // Wait for a URL to become available in the queue.
            let url = {
                let mut queue_guard = self.queue.lock().await;
                match queue_guard.pop_front() {
                    Some(url) => url,
                    // If queue is empty, release permit and wait briefly before retrying.
                    None => { 
                        drop(permit); 
                        tokio::time::sleep(Duration::from_millis(50)).await; 
                        continue; 
                    }
                }
            };

            // Check if we've already visited this URL.
            let mut visited_guard = self.visited.lock().await;
            if visited_guard.contains(&url) { 
                continue; // Skip if already visited.
            }
            println!("Crawling: {}", url);
            visited_guard.insert(url.clone());
            drop(visited_guard);

            // Clone the necessary parts for the async task.
            let scraper_clone = self.scraper.clone();
            let queue_clone = self.queue.clone();
            let results_clone = results.clone();

            let handle = tokio::spawn(async move {
                // The permit is moved into the task and dropped when the task finishes.
                let _permit = permit; 
                match scraper_clone.scrape(&url).await {
                    Ok(scrape_result) => {
                        // --- REFACTORED LOGIC ---
                        // Log what we found.
                        if scrape_result.is_partial {
                            println!("  > Partially scraped metadata from {} (paywall detected). Found {} links.", &url, scrape_result.links.len());
                        } else {
                            // Count unique words from the scraped body_text since ScrapeResult no longer has `word_counts`.
                            let unique_words = scrape_result
                                .body_text
                                .split_whitespace()
                                .collect::<HashSet<_>>()
                                .len();
                            println!("  > Counted {} unique words and found {} links on {}.", unique_words, scrape_result.links.len(), &url);
                        }
                        
                        // Add new links to the shared queue.
                        let mut queue_guard = queue_clone.lock().await;
                        for link in &scrape_result.links {
                            queue_guard.push_back(link.clone());
                        }
                        drop(queue_guard);

                        // The ONLY thing we do with the data is save the raw result.
                        // No processing, no calculating scores.
                        results_clone.lock().await.push(scrape_result);
                    }
                    Err(e) => eprintln!("  > Failed to scrape {}: {}", url, e),
                }
            });
            handles.push(handle);
        }

        // Wait for all crawling tasks to complete.
        for handle in handles { 
            handle.await?; 
        }

        // Return the collected raw data. The crawler's job is done.
        let final_results = match Arc::try_unwrap(results) {
            Ok(mutex) => mutex.into_inner(),
            Err(arc) => {
                // If we couldn't unwrap the Arc because there are other references,
                // lock the mutex and take the vector out without cloning individual items.
                let mut guard = arc.lock().await;
                std::mem::take(&mut *guard)
            }
        };

        Ok(final_results)
    }
}