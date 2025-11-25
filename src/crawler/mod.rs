use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use std::time::Duration;

// Expose the datascraper module so others can use ScrapeResult if needed
pub mod datascraper;
use datascraper::{Scraper, ScrapeResult};

#[derive(Clone)]
pub struct Crawler {
    scraper: Scraper,
    visited: Arc<Mutex<HashSet<String>>>,
    queue: Arc<Mutex<VecDeque<String>>>,
}

impl Crawler {
    pub fn new(seed_urls: &[&str]) -> Self {
        let queue: VecDeque<String> = seed_urls.iter().map(|s| s.to_string()).collect();
        Self {
            scraper: Scraper::new(),
            visited: Arc::new(Mutex::new(HashSet::new())),
            queue: Arc::new(Mutex::new(queue)),
        }
    }
    
    pub async fn crawl(&mut self, limit: usize, concurrency: usize) -> Result<Vec<ScrapeResult>, Box<dyn std::error::Error>> {
        let mut final_results = Vec::with_capacity(limit);
        let mut join_set = JoinSet::new();

        println!("Starting crawl with concurrency: {}", concurrency);

        loop {
            while join_set.len() < concurrency {
                if self.visited.lock().await.len() >= limit { break; }

                let mut queue_guard = self.queue.lock().await;
                let url_str = match queue_guard.pop_front() {
                    Some(u) => u,
                    None => break,
                };
                drop(queue_guard);

                let mut visited_guard = self.visited.lock().await;
                if visited_guard.contains(&url_str) { continue; }
                
                println!("Crawling: {}", url_str);
                visited_guard.insert(url_str.clone());
                drop(visited_guard);

                let scraper = self.scraper.clone();
                let u = url_str.clone();

                join_set.spawn(async move {
                    let fut = scraper.scrape(&u);
                    match tokio::time::timeout(Duration::from_secs(15), fut).await {
                        Ok(res) => (u, res),
                        Err(_) => (u, Err("Timeout".into())),
                    }
                });
            }

            if join_set.is_empty() { break; }

            if let Some(res) = join_set.join_next().await {
                if let Ok((url, result_enum)) = res {
                    match result_enum {
                        Ok(scrape_result) => {
                            if !scrape_result.is_partial {
                                let visited_cnt = self.visited.lock().await.len();
                                if visited_cnt < limit {
                                    let mut q = self.queue.lock().await;
                                    for link in &scrape_result.links {
                                        q.push_back(link.clone());
                                    }
                                }
                                println!("  > Success: {} words, {} links found. [Lang: {}]", 
                                    scrape_result.body_text.split_whitespace().count(), 
                                    scrape_result.links.len(),
                                    scrape_result.language
                                );
                                final_results.push(scrape_result);
                            }
                        }
                        Err(e) => eprintln!("  > [SKIP] {}: {}", url, e),
                    }
                }
            }
        }

        Ok(final_results)
    }
}