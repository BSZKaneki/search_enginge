use tantivy::schema::*;

/// Defines the Tantivy schema for our web page index.
///
/// A schema is the blueprint for a Tantivy index. It declares the fields,
/// how they should be indexed, and whether they should be stored.
pub struct WebpageSchema {
    pub url: Field,
    pub title: Field,
    pub body: Field,
    pub pagerank: Field,
}

impl WebpageSchema {
    /// Creates a new schema with all the fields for a webpage.
    pub fn build() -> (Schema, Self) {
        let mut schema_builder = Schema::builder();

        // URL: The unique identifier for the page.
        // - STRING: Treated as a single, un-tokenized string.
        // - STORED: The full URL will be stored in the index.
        // - INDEXED: We can search for documents by this field.
        let url = schema_builder.add_text_field("url", STRING | STORED);

        // Title: The title of the webpage.
        // - TEXT: The text will be tokenized (split into words), stemmed, etc.
        // - STORED: The full title will be stored for display in search results.
        let title = schema_builder.add_text_field("title", TEXT | STORED);

        // Body: The main content of the page.
        // - TEXT: Tokenized for full-text search.
        // We don't store the body to save space, but it's fully searchable.
        let body = schema_builder.add_text_field("body", TEXT);
        
        // PageRank: The authority score of the page.
        // - FAST: This is crucial. It makes the field's value quickly accessible
        //   for use in scoring functions during search time.
        // - STORED: We store it for debugging and potential display.
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
}