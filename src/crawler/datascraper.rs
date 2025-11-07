// src/crawler/datascraper.rs

use reqwest::Client;
use scraper::{Html, Selector};
use url::Url;

// ---
// CHANGE #1: The ScrapeResult struct is now document-oriented.
// It provides the raw components the indexer needs, instead of pre-processed data.
// ---
#[derive(Debug)] // Added Debug for easier printing.
pub struct ScrapeResult {
    /// The final URL of the page after any redirects.
    pub url: String,
    /// The title of the page, if found.
    pub title: Option<String>,
    /// The extracted text content intended for full-text indexing.
    pub body_text: String,
    /// All of the valid, absolute URLs found on the page.
    pub links: Vec<String>,
    /// Indicates if the scrape was only partial (e.g., metadata only due to a paywall).
    pub is_partial: bool,
}

/// The Scraper is responsible for the network and parsing logic for a single URL.
#[derive(Clone)]
pub struct Scraper {
    client: Client,
}

impl Scraper {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap();
        
        Self { client }
    }

    /// Fetches a URL and extracts its title, body text, and links.
    pub async fn scrape(&self, url_str: &str) -> Result<ScrapeResult, Box<dyn std::error::Error + Send + Sync>> {
        let base_url = Url::parse(url_str)?;
        
        let response = self.client.get(url_str).send().await?;
        if !response.status().is_success() {
            return Err(format!("Request failed with status: {}", response.status()).into());
        }

        // ---
        // CHANGE #2: Capture the final URL after potential redirects.
        // ---
        let final_url = response.url().to_string();
        let body_html = response.text().await?;
        let document = Html::parse_document(&body_html);
        
        // ---
        // CHANGE #3: The logic is simplified. We always extract title and links.
        // Then we decide which text to use for the `body_text`.
        // ---
        let links = self.extract_links(&document, &base_url);
        let title = self.extract_title(&document);

        let (body_text, is_partial) = if self.is_paywalled(&document) {
            // If paywalled, the body text is just the metadata.
            (self.extract_metadata_text(&document), true)
        } else {
            // Otherwise, we use the full text from the <body> tag.
            (self.extract_body_text(&document), false)
        };

        Ok(ScrapeResult {
            url: final_url,
            title,
            body_text,
            links,
            is_partial,
        })
    }
    
    // This function is unchanged.
    fn is_paywalled(&self, document: &Html) -> bool {
        let paywall_selectors = [
            ".paywall", "#paywall", ".subscription-prompt", "#subscription-prompt",
            "div[class*='paywall']", "div[id*='paywall']",
            "div[class*='subscribe']", "div[id*='subscribe']"
        ].join(", ");
        
        let selector = Selector::parse(&paywall_selectors).unwrap();
        document.select(&selector).next().is_some()
    }

    // This function is unchanged.
    fn extract_links(&self, document: &Html, base_url: &Url) -> Vec<String> {
        let link_selector = Selector::parse("a[href]").unwrap();
        document.select(&link_selector)
            .filter_map(|element| element.value().attr("href"))
            .filter_map(|href| base_url.join(href).ok())
            .map(|mut url| {
                url.set_fragment(None); // Remove fragments like #section
                url.to_string()
            })
            .collect()
    }
    
    // ---
    // CHANGE #4: A new helper specifically for extracting the title.
    // ---
    /// Extracts the text from the `<title>` tag.
    fn extract_title(&self, document: &Html) -> Option<String> {
        let selector = Selector::parse("title").unwrap();
        document.select(&selector).next().map(|element| element.text().collect::<String>().trim().to_owned())
    }

    // ---
    // CHANGE #5: This function now returns a raw String, not a HashMap.
    // ---
    /// Extracts text from metadata tags (like meta description).
    fn extract_metadata_text(&self, document: &Html) -> String {
        let selector = Selector::parse("meta[name='description']").unwrap();
        if let Some(element) = document.select(&selector).next() {
            if let Some(content) = element.value().attr("content") {
                return content.to_string();
            }
        }
        String::new() // Return empty string if no description is found
    }

    // ---
    // CHANGE #6: This function now returns a raw String, not a HashMap.
    // It's also simplified.
    // ---
    /// Extracts all text from the `<body>` tag, simplifying it for indexing.
    fn extract_body_text(&self, document: &Html) -> String {
        let selector = Selector::parse("body").unwrap();
        if let Some(body_node) = document.select(&selector).next() {
            // Collect all text nodes into a single string, separated by spaces.
            return body_node.text().collect::<Vec<_>>().join(" ");
        }
        String::new()
    }

    // ---
    // DELETED: The `count_words_from_text` and `count_words_from_body` methods.
    // This logic is no longer needed because Tantivy will do it for us.
    // ---
}