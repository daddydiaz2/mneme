use std::f64::consts::PI;

/// Computes (x, y) positions for each node in [0.0, 100.0] canvas space.
///
/// Uses a circular layout centered at (50, 50) with radius 38.
/// For a single node, returns the center. For 2 nodes, returns left/right.
pub fn layout_nodes(count: usize) -> Vec<(f64, f64)> {
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![(50.0, 50.0)];
    }

    let cx = 50.0_f64;
    let cy = 50.0_f64;
    let radius = 38.0_f64;

    (0..count)
        .map(|i| {
            let angle = 2.0 * PI * (i as f64) / (count as f64) - PI / 2.0;
            let x = cx + radius * angle.cos();
            let y = cy + radius * angle.sin();
            (x, y)
        })
        .collect()
}

/// Truncates a title to fit within `max_chars`, appending "…" if cut.
pub fn truncate_title(title: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    let chars: Vec<char> = title.chars().collect();
    if chars.len() <= max_chars {
        title.to_string()
    } else {
        let cut = max_chars.saturating_sub(1);
        chars[..cut].iter().collect::<String>() + "…"
    }
}
