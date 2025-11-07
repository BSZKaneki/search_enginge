use std::collections::{HashMap, HashSet};

// --- Type Aliases for Clarity ---

/// Represents the web graph, mapping each URL to the set of URLs it links to.
pub type LinkGraph = HashMap<String, HashSet<String>>;
/// Represents the calculated PageRank scores, mapping each URL to its f64 score.
pub type PageRanks = HashMap<String, f64>;

// --- Constants ---

const DAMPING_FACTOR: f64 = 0.85; // Standard PageRank damping factor.
const MAX_ITERATIONS: usize = 100; // A safe limit to prevent infinite loops.
const CONVERGENCE_THRESHOLD: f64 = 0.0001; // Stop when changes between iterations are small.


/// Calculates the PageRank for a set of web pages based on their link structure.
///
/// PageRank is an algorithm that assigns a numerical weighting to each element of a
/// hyperlinked set of documents, such as the World Wide Web, with the purpose of
/// "measuring" its relative importance within the set.
///
/// # Arguments
///
/// * `link_graph` - A map where each key is a URL (source) and the value is a set
///   of URLs (targets) that the source page links to.
///
/// # Returns
///
/// A map where each key is a URL and the value is its calculated PageRank score.
///
pub fn calculate_pagerank(link_graph: &LinkGraph) -> PageRanks {
    if link_graph.is_empty() {
        return HashMap::new();
    }

    // Collect all unique URLs from both linking pages and linked-to pages.
    // This ensures that pages which are only linked to (and don't link out) are included.
    let all_urls: HashSet<String> = link_graph
        .keys()
        .cloned()
        .chain(link_graph.values().flatten().cloned())
        .collect();

    let num_pages = all_urls.len() as f64;
    let initial_rank = 1.0 / num_pages;

    // Initialize ranks for all pages discovered.
    let mut ranks: PageRanks = all_urls.iter().map(|url| (url.clone(), initial_rank)).collect();

    // Build the incoming links graph for efficient calculation.
    let mut incoming_links: HashMap<String, HashSet<String>> = HashMap::new();
    for (source_url, outgoing_links) in link_graph {
        for target_url in outgoing_links {
            incoming_links
                .entry(target_url.clone())
                .or_default()
                .insert(source_url.clone());
        }
    }

    for _ in 0..MAX_ITERATIONS {
        let mut new_ranks = HashMap::new();
        let mut total_change = 0.0;

        for url in &all_urls {
            // Start with the rank from the "random jump" probability.
            let random_jump_rank = (1.0 - DAMPING_FACTOR) / num_pages;

            // Add the rank contribution from all pages that link to the current URL.
            let rank_from_links: f64 = if let Some(sources) = incoming_links.get(url) {
                sources.iter().map(|source_url| {
                    let source_rank = *ranks.get(source_url).unwrap_or(&0.0);
                    let source_out_degree = link_graph.get(source_url).map_or(1, |links| links.len()) as f64;

                    if source_out_degree > 0.0 {
                        source_rank / source_out_degree
                    } else {
                        0.0
                    }
                }).sum()
            } else {
                0.0
            };

            let new_rank = random_jump_rank + DAMPING_FACTOR * rank_from_links;
            new_ranks.insert(url.clone(), new_rank);
            total_change += (new_rank - ranks.get(url).unwrap_or(&0.0)).abs();
        }

        // If the ranks have stabilized, we can exit early.
        if total_change < CONVERGENCE_THRESHOLD {
            break;
        }

        ranks = new_ranks;
    }

    ranks
}