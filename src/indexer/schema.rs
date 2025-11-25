use tantivy::schema::*;
use tantivy::tokenizer::{TextAnalyzer, SimpleTokenizer, LowerCaser, Stemmer, Language};

pub struct WebpageSchema {
    pub url: Field,
    pub title: Field,
    pub body: Field,
    pub pagerank: Field,
    pub language: Field, // Stores "en", "pl", "de", etc.
}

impl WebpageSchema {
    pub fn build() -> (Schema, Self) {
        let mut schema_builder = Schema::builder();

        // Standard text options with English stemming
        let text_options = TextOptions::default()
            .set_indexing_options(TextFieldIndexing::default()
                .set_tokenizer("en_stem") 
                .set_index_option(IndexRecordOption::WithFreqsAndPositions));

        // URL: Stored, exact match
        let url = schema_builder.add_text_field("url", STRING | STORED);

        // Title: Stored so we can display it
        let title_options = text_options.clone().set_stored();
        let title = schema_builder.add_text_field("title", title_options);

        // Body: Indexed but NOT stored (saves disk space). Searchable.
        let body = schema_builder.add_text_field("body", text_options);
        
        // PageRank: FastField (f64) for mathematical scoring
        let pagerank = schema_builder.add_f64_field("pagerank", FAST | STORED);

        // Language: Stored String for filtering (e.g., "language:en")
        let language = schema_builder.add_text_field("language", STRING | STORED);

        let schema = schema_builder.build();
        
        let fields = Self {
            url,
            title,
            body,
            pagerank,
            language,
        };

        (schema, fields)
    }

    /// Register the "en_stem" tokenizer logic
    pub fn register_tokenizer(index: &tantivy::Index) {
        let analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)
            .filter(Stemmer::new(Language::English))
            .build();
            
        index.tokenizers().register("en_stem", analyzer);
    }
}