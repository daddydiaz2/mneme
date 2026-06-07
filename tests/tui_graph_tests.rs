use mneme::tui::graph::{layout_nodes, truncate_title};

#[test]
fn layout_nodes_zero_returns_empty() {
    let positions = layout_nodes(0);
    assert!(positions.is_empty());
}

#[test]
fn layout_nodes_single_returns_center() {
    let positions = layout_nodes(1);
    assert_eq!(positions.len(), 1);
    let (x, y) = positions[0];
    assert!((x - 50.0).abs() < 0.001);
    assert!((y - 50.0).abs() < 0.001);
}

#[test]
fn layout_nodes_circle_positions_are_within_bounds() {
    for n in [2, 3, 5, 10, 20] {
        let positions = layout_nodes(n);
        assert_eq!(positions.len(), n);
        for (x, y) in &positions {
            assert!(*x >= 0.0 && *x <= 100.0, "x={x} out of bounds for n={n}");
            assert!(*y >= 0.0 && *y <= 100.0, "y={y} out of bounds for n={n}");
        }
    }
}

#[test]
fn truncate_title_short_string_unchanged() {
    let result = truncate_title("hello", 12);
    assert_eq!(result, "hello");
}

#[test]
fn truncate_title_exact_length_unchanged() {
    let result = truncate_title("abcdefghijkl", 12);
    assert_eq!(result, "abcdefghijkl");
}

#[test]
fn truncate_title_long_string_is_truncated() {
    let result = truncate_title("this is a very long title", 12);
    assert!(
        result.len() <= 15,
        "truncated result should be short: {result}"
    );
    assert!(result.contains('…'), "should contain ellipsis: {result}");
}

#[test]
fn truncate_title_zero_max_returns_empty() {
    let result = truncate_title("anything", 0);
    assert!(result.is_empty());
}
