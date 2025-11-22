use std::collections::{HashMap, HashSet};
use rayon::prelude::*; 
pub type LinkGraph = HashMap<String, HashSet<String>>;
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

    let all_urls: HashSet<String> = link_graph
        .keys()
        .cloned()
        .chain(link_graph.values().flatten().cloned())
        .collect();

    let num_pages = all_urls.len() as f64;
    let initial_rank = 1.0 / num_pages;

    // Initialize ranks
    let mut ranks: PageRanks = all_urls.iter().map(|url| (url.clone(), initial_rank)).collect();

    // Pre-calculate incoming links (Reverse Graph)
    // This is read-only during the loop, so it's safe to share across threads.
    let mut incoming_links: HashMap<String, Vec<String>> = HashMap::new();
    for (source_url, outgoing_links) in link_graph {
        for target_url in outgoing_links {
            incoming_links
                .entry(target_url.clone())
                .or_default()
                .push(source_url.clone());
        }
    }

    // Convert all_urls to a Vec for better Rayon performance (par_iter on HashSet is slower)
    let all_urls_vec: Vec<String> = all_urls.into_iter().collect();

    for i in 0..MAX_ITERATIONS {
        // --- PARALLELISM STARTS HERE ---
        // We calculate the new rank for every URL simultaneously using all CPU cores.
        let new_ranks: PageRanks = all_urls_vec.par_iter()
            .map(|url| {
                let random_jump_rank = (1.0 - DAMPING_FACTOR) / num_pages;

                let rank_from_links: f64 = if let Some(sources) = incoming_links.get(url) {
                    sources.iter().map(|source_url| {
                        // Read from the OLD ranks map (safe shared access)
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
                (url.clone(), new_rank) // Return tuple for collection
            })
            .collect(); // Rayon automatically builds the HashMap from the parallel results

        // Calculate convergence (Difference between old ranks and new ranks)
        // We can also parallelize this check
        let total_change: f64 = all_urls_vec.par_iter()
            .map(|url| {
                let old_rank = *ranks.get(url).unwrap_or(&0.0);
                let new_rank = *new_ranks.get(url).unwrap_or(&0.0);
                (new_rank - old_rank).abs()
            })
            .sum();

        ranks = new_ranks;

        if total_change < CONVERGENCE_THRESHOLD {
            println!("PageRank converged after {} iterations.", i + 1);
            break;
        }
    }

    ranks
}