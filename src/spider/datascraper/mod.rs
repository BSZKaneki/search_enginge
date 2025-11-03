// src/spider/datascraper.rs

use reqwest::Client;
use scraper::{Html, Selector};
use std::collections::HashMap;
use url::Url;

/// A struct to hold the results of scraping a single page.
pub struct ScrapeResult {
    /// All of the valid, absolute URLs found on the page.
    pub links: Vec<String>,
    /// A map of every unique word found on the page and its frequency.
    pub word_counts: HashMap<String, u32>,
}

/// The Scraper is responsible for the network and parsing logic for a single URL.
#[derive(Clone)]
pub struct Scraper {
    client: Client,
}

impl Scraper {
    /// Creates a new Scraper with a pre-configured HTTP client.
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap();
        
        Self { client }
    }

    /// Fetches a URL, extracts links, and counts words, returning a `ScrapeResult`.
    pub async fn scrape(&self, url_str: &str) -> Result<ScrapeResult, Box<dyn std::error::Error + Send + Sync>> {
        let base_url = Url::parse(url_str)?;
        let body = self.client.get(url_str).send().await?.text().await?;
        let document = Html::parse_document(&body);
        
        let links = self.extract_links(&document, &base_url);
        let word_counts = self.count_words(&document);

        Ok(ScrapeResult { links, word_counts })
    }

    /// Parses the HTML document to find all hyperlink `href` attributes.
    fn extract_links(&self, document: &Html, base_url: &Url) -> Vec<String> {
        let link_selector = Selector::parse("a").unwrap();
        let mut links = Vec::new();
        for element in document.select(&link_selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(mut absolute_url) = base_url.join(href) {
                    absolute_url.set_fragment(None);
                    links.push(absolute_url.to_string());
                }
            }
        }
        links
    }

    /// Parses the text content of the HTML `<body>` to count word frequencies.
    fn count_words(&self, document: &Html) -> HashMap<String, u32> {
        let body_selector = Selector::parse("body").unwrap();
        let mut counts = HashMap::new();
        if let Some(body_node) = document.select(&body_selector).next() {
            for text in body_node.text() {
                for word in text.split_whitespace() {
                    // Clean the word: lowercase, alphabetic characters only
                    let clean_word = word
                        .to_lowercase()
                        .chars()
                        .filter(|c| c.is_alphabetic())
                        .collect::<String>();
                    
                    // Ignore very short or empty words
                    if !clean_word.is_empty() && clean_word.len() > 2 {
                        *counts.entry(clean_word).or_insert(0) += 1;
                    }
                }
            }
        }
        counts
    }
}