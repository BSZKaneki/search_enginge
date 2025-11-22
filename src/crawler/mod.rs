use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::time::{Duration, Instant};

// --- Module Declarations and Imports ---
pub mod datascraper;
use datascraper::{Scraper, ScrapeResult};

// --- Struct Definition ---

#[derive(Clone)]
pub struct Crawler {
    scraper: Scraper,
    visited: Arc<Mutex<HashSet<String>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
}

impl Crawler {
    pub fn new(start_url: &str) -> Self {
        Self {
            scraper: Scraper::new(),
            visited: Arc::new(Mutex::new(HashSet::new())),
            queue: Arc::new(Mutex::new(VecDeque::from(vec![start_url.to_string()]))),
        }
    }
    
    pub async fn crawl(&mut self, limit: usize, concurrency: usize) -> Result<Vec<ScrapeResult>, Box<dyn std::error::Error>> {
        let semaphore = Arc::new(Semaphore::new(concurrency));
        let mut handles = Vec::new();
        let results = Arc::new(Mutex::new(Vec::new()));

        while self.visited.lock().await.len() < limit {
            // Break if queue is empty and no active tasks remain
            if self.queue.lock().await.is_empty() && semaphore.available_permits() == concurrency {
                break; 
            }

            // Acquire a permit. This waits if we have reached max concurrency.
            let permit = semaphore.clone().acquire_owned().await?;
            
            // Get next URL from queue
            let url = {
                let mut queue_guard = self.queue.lock().await;
                match queue_guard.pop_front() {
                    Some(url) => url,
                    None => { 
                        // If empty, release permit, wait a bit, and retry loop
                        drop(permit); 
                        tokio::time::sleep(Duration::from_millis(50)).await; 
                        continue; 
                    }
                }
            };

            // Mark as visited
            let mut visited_guard = self.visited.lock().await;
            if visited_guard.contains(&url) { 
                continue; 
            }
            println!("Crawling: {}", url);
            visited_guard.insert(url.clone());
            drop(visited_guard);

            // Clone data for the thread
            let scraper_clone = self.scraper.clone();
            let queue_clone = self.queue.clone();
            let results_clone = results.clone();

            // Spawn the worker
            let handle = tokio::spawn(async move {
                // Permit is held for the duration of this block. 
                // When this block ends (success or skip), permit is dropped and next task can start.
                let _permit = permit; 
                
                let scrape_future = scraper_clone.scrape(&url);
                
                // We enforce a 15-second timeout on the scrape attempt
                match tokio::time::timeout(Duration::from_secs(15), scrape_future).await {
                    
                    // 1. Scraper finished within time limit
                    Ok(scrape_attempt) => {
                        match scrape_attempt {
                            // A. SUCCESS
                            Ok(scrape_result) => {
                                if scrape_result.is_partial {
                                    println!("  > Partial scrape: {} (paywall/error).", &url);
                                } else {
                                    let unique_words = scrape_result.body_text.split_whitespace().count();
                                    println!("  > Success: {} words, {} links found on {}.", unique_words, scrape_result.links.len(), &url);
                                }
                                
                                // Add found links to queue
                                let mut queue_guard = queue_clone.lock().await;
                                for link in &scrape_result.links {
                                    queue_guard.push_back(link.clone());
                                }
                                drop(queue_guard);

                                // Save result
                                results_clone.lock().await.push(scrape_result);
                            },
                            
                            // B. SCRAPER ERROR (404, DNS, etc.)
                            Err(e) => {
                                // Just print skip and do nothing else. The task ends here.
                                eprintln!("  > [SKIP] Error accessing {}: {}", url, e);
                            }
                        }
                    },

                    // 2. TIMEOUT (Took longer than 15s)
                    Err(_) => {
                        // Just print skip and do nothing else. The task ends here.
                        eprintln!("  > [SKIP] Timeout (15s) on {}", url);
                    }
                }
                // End of Async Block: _permit is dropped here automatically.
            });
            handles.push(handle);
        }

        // Wait for remaining tasks to finish
        for handle in handles { 
            if let Err(e) = handle.await {
                eprintln!("Task panic: {}", e);
            }
        }

        // Return results
        let final_results = match Arc::try_unwrap(results) {
            Ok(mutex) => mutex.into_inner(),
            Err(arc) => {
                let mut guard = arc.lock().await;
                std::mem::take(&mut *guard)
            }
        };

        Ok(final_results)
    }
}