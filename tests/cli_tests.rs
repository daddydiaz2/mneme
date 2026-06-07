use mneme::cli::output;
use mneme::store::memory::{
    GraphData, GraphEdge, GraphNode, HealthReport, MemoryStats, ProjectSummary,
};

#[test]
fn test_color_constants_are_non_empty() {
    assert!(!output::GREEN.is_empty());
    assert!(!output::YELLOW.is_empty());
    assert!(!output::CYAN.is_empty());
    assert!(!output::RED.is_empty());
    assert!(!output::BOLD.is_empty());
    assert!(!output::DIM.is_empty());
    assert!(!output::RESET.is_empty());
}

#[test]
fn test_print_success_does_not_panic() {
    output::print_success("operación completada");
}

#[test]
fn test_print_error_does_not_panic() {
    output::print_error("algo salió mal");
}

#[test]
fn test_print_warning_does_not_panic() {
    output::print_warning("precaución");
}

#[test]
fn test_print_memory_list_empty_does_not_panic() {
    output::print_memory_list(&[]);
}

#[test]
fn test_print_search_results_empty_does_not_panic() {
    output::print_search_results(&[]);
}

#[test]
fn test_print_projects_empty_does_not_panic() {
    output::print_projects(&[]);
}

#[test]
fn test_print_project_list_with_data_does_not_panic() {
    let projects = vec![ProjectSummary {
        name: "test-proj".into(),
        memory_count: 5,
        session_count: 2,
        last_activity: Some(chrono::Utc::now()),
    }];
    output::print_projects(&projects);
}

#[test]
fn test_print_stats_does_not_panic() {
    let stats = MemoryStats {
        project: "test".into(),
        total_memories: 10,
        total_relations: 5,
        total_sessions: 2,
        total_prompts: 1,
        by_type: [("note".into(), 5u32), ("decision".into(), 5u32)]
            .into_iter()
            .collect(),
        by_importance: [("high".into(), 8u32), ("medium".into(), 2u32)]
            .into_iter()
            .collect(),
        by_scope: [("project".into(), 10u32)].into_iter().collect(),
        oldest_memory: Some(chrono::Utc::now()),
        newest_memory: Some(chrono::Utc::now()),
        most_accessed: Some("Test Memory".into()),
    };
    output::print_stats(&stats);
}

#[test]
fn test_print_graph_does_not_panic() {
    let graph = GraphData {
        nodes: vec![GraphNode {
            id: "a".into(),
            title: "Node A".into(),
            memory_type: "decision".into(),
            importance: "high".into(),
        }],
        edges: vec![GraphEdge {
            source: "a".into(),
            target: "b".into(),
            relation_type: "related".into(),
            confidence: 0.95,
        }],
    };
    output::print_graph(&graph);
}

#[test]
fn test_print_health_does_not_panic() {
    let report = HealthReport {
        version: "0.1.0".into(),
        db_size_mb: 1.5,
        total_memories: 42,
        orphaned_memories: 0,
        unindexed_embeddings: 3,
        last_sync: None,
        embedding_model: "BAAI/bge-small-en-v1.5".into(),
    };
    output::print_health(&report);
}
