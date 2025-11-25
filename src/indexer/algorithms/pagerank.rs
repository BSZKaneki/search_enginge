use std::collections::{HashMap, HashSet};
use rayon::prelude::*; 

pub type LinkGraph = HashMap<String, HashSet<String>>;
pub type PageRanks = HashMap<String, f64>;

const DAMPING_FACTOR: f64 = 0.85; 
const MAX_ITERATIONS: usize = 100; 
const CONVERGENCE_THRESHOLD: f64 = 0.0001;

pub fn calculate_pagerank(link_graph: &LinkGraph) -> PageRanks {
    if link_graph.is_empty() {
        return HashMap::new();
    }

    // 1. Collect all unique URLs
    let all_urls: HashSet<String> = link_graph
        .keys()
        .cloned()
        .chain(link_graph.values().flatten().cloned())
        .collect();

    let num_pages = all_urls.len() as f64;
    // Initial rank is evenly distributed
    let initial_rank = 1.0 / num_pages;

    let mut ranks: PageRanks = all_urls.iter().map(|url| (url.clone(), initial_rank)).collect();
    let all_urls_vec: Vec<String> = all_urls.into_iter().collect();

    // 2. Build Reverse Graph & Identify Dangling Nodes (pages with no outgoing links)
    let mut incoming_links: HashMap<String, Vec<String>> = HashMap::new();
    let mut dangling_nodes: Vec<String> = Vec::new();

    for url in &all_urls_vec {
        if let Some(outgoing) = link_graph.get(url) {
            if outgoing.is_empty() {
                dangling_nodes.push(url.clone());
            } else {
                for target in outgoing {
                    incoming_links.entry(target.clone()).or_default().push(url.clone());
                }
            }
        } else {
            dangling_nodes.push(url.clone());
        }
    }

    // 3. Iterative Calculation
    for i in 0..MAX_ITERATIONS {
        // Calculate mass from dangling nodes to redistribute
        let dangling_sum: f64 = dangling_nodes.iter()
            .map(|u| *ranks.get(u).unwrap_or(&0.0))
            .sum();
            
        let dangling_weight = (DAMPING_FACTOR * dangling_sum) / num_pages;
        let random_jump_rank = (1.0 - DAMPING_FACTOR) / num_pages;
        let base_rank = random_jump_rank + dangling_weight;

        // Parallel update using Rayon
        let new_ranks: PageRanks = all_urls_vec.par_iter()
            .map(|url| {
                let rank_from_links: f64 = if let Some(sources) = incoming_links.get(url) {
                    sources.iter().map(|source_url| {
                        let source_rank = *ranks.get(source_url).unwrap_or(&0.0);
                        let source_out_degree = link_graph.get(source_url).unwrap().len() as f64;
                        source_rank / source_out_degree
                    }).sum()
                } else {
                    0.0
                };

                let new_rank = base_rank + (DAMPING_FACTOR * rank_from_links);
                (url.clone(), new_rank)
            })
            .collect();

        // Check convergence
        let total_change: f64 = all_urls_vec.par_iter()
            .map(|url| {
                let old = *ranks.get(url).unwrap_or(&0.0);
                let new = *new_ranks.get(url).unwrap_or(&0.0);
                (new - old).abs()
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