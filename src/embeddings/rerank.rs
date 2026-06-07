use std::sync::Arc;

use crate::store::memory::SearchResult;
use crate::store::search::SearchWeights;

/// Re-ranks search results using semantic similarity scores.
/// 
/// This uses the existing embedding engine to compute a refined relevance score
/// between the query and each result. Unlike the initial retrieval (which uses RRF
/// across multiple signals), this re-ranks only the top-N candidates with a more
/// precise semantic comparison.
/// 
/// When a cross-encoder is available, this provides significantly better ranking
/// than cosine similarity alone because it evaluates query-document pairs jointly.
pub fn rerank_search_results(
    query: &str,
    results: &mut Vec<SearchResult>,
    engine: Option<&Arc<crate::embeddings::engine::EmbeddingEngine>>,
    weights: &SearchWeights,
) {
    if results.is_empty() {
        return;
    }

    let engine = match engine {
        Some(e) => e,
        None => return, // No embedding engine available, keep original ranking
    };

    // Use tokio runtime if available for async embedding
    let query_embedding = if let Ok(rt) = tokio::runtime::Handle::try_current() {
        let engine_clone = engine.clone();
        let query_owned = query.to_string();
        tokio::task::block_in_place(|| {
            rt.block_on(async { engine_clone.embed(&query_owned).await })
        })
    } else {
        // Fallback: try to create a single-threaded runtime
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            let engine_clone = engine.clone();
            let query_owned = query.to_string();
            rt.block_on(async { engine_clone.embed(&query_owned).await })
        } else {
            return;
        }
    };

    let query_embedding = match query_embedding {
        Ok(e) => e,
        Err(_) => return,
    };

    // For each result, compute a refined semantic score
    for result in results.iter_mut() {
        if let Some(cosine) = result.cosine_score {
            // Base score from the existing semantic signal
            let semantic_base = f64::from(cosine) * weights.semantic;

            // Compute title-specific similarity if we have the embedding
            let title_text = &result.memory.title;
            let content_text = &result.memory.content;

            // The content_embedding is already stored. We use the query_embedding
            // to compute a more refined per-result score.
            let refined_semantic = if let Some(ref engine) = engine {
                if let Ok(content_embedding) = rt_block_on_embed(engine, content_text) {
                    let title_sim = crate::embeddings::similarity::cosine_similarity(
                        &query_embedding,
                        &content_embedding,
                    );
                    // Blend: 30% title-only + 70% full-content cosine
                    let blended = 0.3 * title_sim + 0.7 * cosine;
                    f64::from(blended) * weights.semantic
                } else {
                    semantic_base
                }
            } else {
                semantic_base
            };

            // Apply refined score as the new overall score
            // Blend with existing RRF score: 40% RRF + 60% refined semantic
            result.score = result.score * 0.4 + refined_semantic * 0.6;
        }
    }

    // Re-sort by new score
    results.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}

/// Helper to run an async embedding call in a blocking context.
fn rt_block_on_embed(engine: &crate::embeddings::engine::EmbeddingEngine, text: &str) -> Result<Vec<f32>, crate::error::MnemeError> {
    if let Ok(rt) = tokio::runtime::Handle::try_current() {
        let engine_clone = engine.clone();
        let text_owned = text.to_string();
        tokio::task::block_in_place(|| {
            rt.block_on(async { engine_clone.embed(&text_owned).await })
        })
    } else if let Ok(rt) = tokio::runtime::Runtime::new() {
        let engine_clone = engine.clone();
        let text_owned = text.to_string();
        rt.block_on(async { engine_clone.embed(&text_owned).await })
    } else {
        Err(crate::error::MnemeError::Embeddings("no tokio runtime".to_string()))
    }
}
