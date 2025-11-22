use tantivy::schema::*;
use tantivy::tokenizer::{TextAnalyzer, SimpleTokenizer, LowerCaser, Stemmer, Language};

pub struct WebpageSchema {
    pub url: Field,
    pub title: Field,
    pub body: Field,
    pub pagerank: Field,
}

impl WebpageSchema {
    pub fn build() -> (Schema, Self) {
        let mut schema_builder = Schema::builder();

        // 1. Define Custom Text Options
        // Instead of using the default TEXT, we define options that use an "en_stem" tokenizer.
        // We will register what "en_stem" actually does in the helper function below.
        let text_options = TextOptions::default()
            .set_indexing_options(TextFieldIndexing::default()
                .set_tokenizer("en_stem") // <--- TELLS TANTIVY TO USE STEMMING
                .set_index_option(IndexRecordOption::WithFreqsAndPositions));

        // URL: Keep as STRING (exact match, no stemming needed for the link itself)
        let url = schema_builder.add_text_field("url", STRING | STORED);

        // Title: Use our custom stemmed options, and STORE it so we can display it.
        let title_options = text_options.clone().set_stored();
        let title = schema_builder.add_text_field("title", title_options);

        // Body: Use our custom stemmed options. Not stored (to save space).
        // This ensures "general-purpose" is indexed as "general" and "purpose".
        let body = schema_builder.add_text_field("body", text_options);
        
        // PageRank: Keep as FAST for scoring.
        let pagerank = schema_builder.add_f64_field("pagerank", FAST | STORED);

        let schema = schema_builder.build();
        
        let fields = Self {
            url,
            title,
            body,
            pagerank,
        };

        (schema, fields)
    }

    /// REGISTER TOKENIZER
    /// You must call this function immediately after creating or opening the Index.
    /// It defines what "en_stem" means.
    pub fn register_tokenizer(index: &tantivy::Index) {
        let analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
            .filter(LowerCaser)       // "Rust" -> "rust"
            .filter(Stemmer::new(Language::English)) // "programming" -> "program"
            .build();
            
        index.tokenizers().register("en_stem", analyzer);
    }
}