use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::OnceLock;
use url::Url;
use whatlang::{detect, Lang}; // Language detection

static PAYWALL_SELECTOR: OnceLock<Selector> = OnceLock::new();
static LINK_SELECTOR: OnceLock<Selector> = OnceLock::new();
static TITLE_SELECTOR: OnceLock<Selector> = OnceLock::new();
static META_DESC_SELECTOR: OnceLock<Selector> = OnceLock::new();
static BODY_SELECTOR: OnceLock<Selector> = OnceLock::new();

#[derive(Debug)]
pub struct ScrapeResult {
    pub url: String,
    pub title: Option<String>,
    pub body_text: String,
    pub links: Vec<String>,
    pub is_partial: bool,
    pub language: String, // Added language field
}

#[derive(Clone)]
pub struct Scraper {
    client: Client,
}

impl Scraper {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .timeout(std::time::Duration::from_secs(10)) // 10s connection timeout
            .build()
            .expect("Failed to build HTTP client");
        
        Self { client }
    }

    pub async fn scrape(&self, url_str: &str) -> Result<ScrapeResult, Box<dyn std::error::Error + Send + Sync>> {
        let base_url = Url::parse(url_str)?;
        let response = self.client.get(url_str).send().await?;
        
        if !response.status().is_success() {
            return Err(format!("Request failed: {}", response.status()).into());
        }

        let final_url = response.url().to_string();
        let body_html = response.text().await?;
        let document = Html::parse_document(&body_html);
        
        self.init_selectors();

        let links = self.extract_links(&document, &base_url);
        let title = self.extract_title(&document);

        let (body_text, is_partial) = if self.is_paywalled(&document) {
            (self.extract_metadata_text(&document), true)
        } else {
            (self.extract_body_text(&document), false)
        };

        // Detect Language
        let language = match detect(&body_text) {
            Some(info) => info.lang().code().to_string(), // "en", "fr", "pl"
            None => "unknown".to_string(),
        };

        Ok(ScrapeResult {
            url: final_url,
            title,
            body_text,
            links,
            is_partial,
            language,
        })
    }

    fn init_selectors(&self) {
        LINK_SELECTOR.get_or_init(|| Selector::parse("a[href]").unwrap());
        TITLE_SELECTOR.get_or_init(|| Selector::parse("title").unwrap());
        META_DESC_SELECTOR.get_or_init(|| Selector::parse("meta[name='description']").unwrap());
        BODY_SELECTOR.get_or_init(|| Selector::parse("body").unwrap());
        PAYWALL_SELECTOR.get_or_init(|| {
            Selector::parse(".paywall, #paywall, .subscription-prompt, #subscription-prompt").unwrap()
        });
    }
    
    fn is_paywalled(&self, document: &Html) -> bool {
        document.select(PAYWALL_SELECTOR.get().unwrap()).next().is_some()
    }

    fn extract_links(&self, document: &Html, base_url: &Url) -> Vec<String> {
        let selector = LINK_SELECTOR.get().unwrap();
        let mut links = Vec::with_capacity(32);
        for element in document.select(selector) {
            if let Some(href) = element.value().attr("href") {
                if let Ok(mut url) = base_url.join(href) {
                    url.set_fragment(None);
                    links.push(url.to_string());
                }
            }
        }
        links
    }
    
    fn extract_title(&self, document: &Html) -> Option<String> {
        document.select(TITLE_SELECTOR.get().unwrap())
            .next()
            .map(|e| self.clean_text(e.text()))
    }

    fn extract_metadata_text(&self, document: &Html) -> String {
        let selector = META_DESC_SELECTOR.get().unwrap();
        if let Some(element) = document.select(selector).next() {
            if let Some(content) = element.value().attr("content") {
                return content.trim().to_string();
            }
        }
        String::new()
    }

    fn extract_body_text(&self, document: &Html) -> String {
        if let Some(body_node) = document.select(BODY_SELECTOR.get().unwrap()).next() {
            return self.clean_text(body_node.text());
        }
        String::new()
    }

    fn clean_text<'a>(&self, text_iter: impl Iterator<Item = &'a str>) -> String {
        let mut buffer = String::with_capacity(1024);
        let mut first = true;
        for part in text_iter {
            let trimmed = part.trim();
            if !trimmed.is_empty() {
                if !first { buffer.push(' '); }
                buffer.push_str(trimmed);
                first = false;
            }
        }
        buffer
    }
}