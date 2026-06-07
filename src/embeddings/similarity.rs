/// Calcula similitud coseno entre dos vectores f32.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "vectors must have same dimension");

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Resultado de coincidencia semantica.
#[derive(Debug, Clone)]
pub struct SemanticMatch {
    /// ID de la memoria.
    pub memory_id: uuid::Uuid,
    /// Score de similitud coseno.
    pub cosine_score: f32,
    /// Score combinado (coseno * boost * decaimiento).
    pub combined_score: f64,
}

/// Ordena coincidencias semanticas por score combinado descendente.
pub fn rank_by_combined_score(matches: &mut [SemanticMatch]) {
    matches.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
}
