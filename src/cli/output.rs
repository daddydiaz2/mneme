use crate::store::memory::{Memory, MemoryStats, ProjectSummary, SearchResult};
use unicode_width::UnicodeWidthStr;

/// ANSI green color code.
pub const GREEN: &str = "\x1b[32m";
/// ANSI yellow color code.
pub const YELLOW: &str = "\x1b[33m";
/// ANSI cyan color code.
pub const CYAN: &str = "\x1b[36m";
/// ANSI red color code.
pub const RED: &str = "\x1b[31m";
/// ANSI bold code.
pub const BOLD: &str = "\x1b[1m";
/// ANSI dim code.
pub const DIM: &str = "\x1b[2m";
/// ANSI reset code.
pub const RESET: &str = "\x1b[0m";

/// Prints a single memory with full details.
pub fn print_memory(memory: &Memory) {
    println!(
        "{BOLD}┌{}
",
        "─".repeat(78)
    );
    println!("{BOLD}│ {CYAN}{}{RESET}", memory.title);
    println!(
        "{BOLD}├{}
",
        "─".repeat(78)
    );
    println!("{DIM}│ ID:{RESET}        {}", memory.id);
    println!("{DIM}│ Project:{RESET}   {}", memory.project);
    println!("{DIM}│ Type:{RESET}      {}", memory.memory_type);
    println!("{DIM}│ Importance:{RESET} {}", memory.importance);
    println!("{DIM}│ Scope:{RESET}     {}", memory.scope);
    println!(
        "{DIM}│ Created:{RESET}   {}",
        memory.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "{DIM}│ Updated:{RESET}   {}",
        memory.updated_at.format("%Y-%m-%d %H:%M:%S")
    );
    if !memory.tags.is_empty() {
        println!("{DIM}│ Tags:{RESET}      {}", memory.tags.join(", "));
    }
    if let Some(topic_key) = &memory.topic_key {
        println!("{DIM}│ Topic:{RESET}     {}", topic_key);
    }
    println!("{BOLD}├{}", "─".repeat(78));
    println!("{RESET}{}", memory.content);
    if let Some(what) = &memory.what {
        println!("\n{BOLD}What:{RESET}\n{}", what);
    }
    if let Some(why) = &memory.why {
        println!("\n{BOLD}Why:{RESET}\n{}", why);
    }
    if let Some(ctx) = &memory.context {
        println!("\n{BOLD}Context:{RESET}\n{}", ctx);
    }
    if let Some(learned) = &memory.learned {
        println!("\n{BOLD}Learned:{RESET}\n{}", learned);
    }
    println!(
        "{BOLD}└{}
",
        "─".repeat(78)
    );
}

/// Prints a table of memories.
pub fn print_memory_list(memories: &[Memory]) {
    if memories.is_empty() {
        println!("{YELLOW}No memories found.{RESET}");
        return;
    }

    let headers = ["ID", "TYPE", "IMP", "TITLE", "PROJECT", "DATE"];
    let col_widths = [10, 14, 6, 28, 14, 10];

    print_table_header(&headers, &col_widths);

    for mem in memories {
        let id_short = &mem.id.to_string()[..8];
        let type_str = mem.memory_type.to_string();
        let imp_str = mem.importance.to_string();
        let title = truncate(&mem.title, col_widths[3]);
        let project = truncate(&mem.project, col_widths[4]);
        let date = mem.updated_at.format("%Y-%m-%d").to_string();

        println!(
            "{} {:<10} {:<14} {:<6} {:<28} {:<14} {:<10} {}",
            DIM, id_short, type_str, imp_str, title, project, date, RESET
        );
    }

    println!();
    println!("{DIM}Total: {}{} memories{}", memories.len(), RESET, RESET);
}

/// Prints search results.
pub fn print_search_results(results: &[SearchResult]) {
    if results.is_empty() {
        println!("{YELLOW}No results found.{RESET}");
        return;
    }

    println!("{BOLD}{} result(s):{RESET}\n", results.len());

    for (i, result) in results.iter().enumerate() {
        let mem = &result.memory;
        let score_color = if result.score > 1.5 {
            GREEN
        } else if result.score > 0.8 {
            CYAN
        } else {
            YELLOW
        };

        println!(
            "{BOLD}[{}]{RESET} {CYAN}{}{RESET} {}{:.2}{}",
            i + 1,
            mem.title,
            score_color,
            result.score,
            RESET
        );
        println!(
            "    {DIM}ID:{} {} | {} | {} | {}{}",
            RESET,
            &mem.id.to_string()[..8],
            mem.memory_type,
            mem.importance,
            mem.project,
            RESET
        );
        if let Some(snippet) = &result.snippet {
            println!("    {DIM}{}{}", snippet, RESET);
        }
        println!();
    }
}

/// Prints memory stats.
pub fn print_stats(stats: &MemoryStats) {
    println!(
        "{BOLD}Stats for project '{}{}'{RESET}\n",
        CYAN, stats.project
    );
    println!("  {DIM}Total memories:{RESET}  {}", stats.total_memories);
    println!("  {DIM}Total relations:{RESET} {}", stats.total_relations);
    println!("  {DIM}Total sessions:{RESET}  {}", stats.total_sessions);
    println!("  {DIM}Total prompts:{RESET}   {}", stats.total_prompts);

    if !stats.by_type.is_empty() {
        println!("\n  {BOLD}By type:{RESET}");
        for (t, count) in &stats.by_type {
            println!("    {:<16} {}", t, count);
        }
    }

    if !stats.by_importance.is_empty() {
        println!("\n  {BOLD}By importance:{RESET}");
        for (imp, count) in &stats.by_importance {
            println!("    {:<16} {}", imp, count);
        }
    }

    if !stats.by_scope.is_empty() {
        println!("\n  {BOLD}By scope:{RESET}");
        for (scope, count) in &stats.by_scope {
            println!("    {:<16} {}", scope, count);
        }
    }

    if let Some(oldest) = stats.oldest_memory {
        println!(
            "\n  {DIM}Oldest memory:{RESET}  {}",
            oldest.format("%Y-%m-%d")
        );
    }
    if let Some(newest) = stats.newest_memory {
        println!(
            "  {DIM}Newest memory:{RESET}  {}",
            newest.format("%Y-%m-%d")
        );
    }
    if let Some(most_accessed) = &stats.most_accessed {
        println!("  {DIM}Most accessed:{RESET}  {}", most_accessed);
    }
}

/// Prints a list of projects.
pub fn print_projects(projects: &[ProjectSummary]) {
    if projects.is_empty() {
        println!("{YELLOW}No projects found.{RESET}");
        return;
    }

    let headers = ["PROJECT", "MEMORIES", "SESSIONS", "LAST ACTIVITY"];
    let col_widths = [24, 12, 12, 20];

    print_table_header(&headers, &col_widths);

    for proj in projects {
        let last = proj
            .last_activity
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{DIM} {:<24} {:<12} {:<12} {:<20} {}",
            proj.name, proj.memory_count, proj.session_count, last, RESET
        );
    }

    println!();
    println!("{DIM}Total: {}{} projects{}", projects.len(), RESET, RESET);
}

/// Prints a success message.
pub fn print_success(msg: &str) {
    println!("{GREEN}✓ {}{}", msg, RESET);
}

/// Prints an error message.
pub fn print_error(msg: &str) {
    eprintln!("{RED}✗ {}{}", msg, RESET);
}

/// Prints a warning message.
pub fn print_warning(msg: &str) {
    println!("{YELLOW}⚠ {}{}", msg, RESET);
}

/// Prints an audit report.
pub fn print_audit(report: &crate::store::memory::AuditReport) {
    println!("{BOLD}Audit Report{RESET}\n");
    println!(
        "  {DIM}Average revisions:{RESET}  {:.2}",
        report.average_revisions
    );
    println!(
        "  {DIM}Duplicate groups:{RESET}   {}",
        report.duplicate_groups
    );
    println!();

    println!("{BOLD}Type Distribution:{RESET}");
    for (t, count) in &report.type_distribution {
        println!("  {:<16} {}", t, count);
    }
    println!();

    println!(
        "{BOLD}Stale memories ({}){RESET}",
        report.stale_memories.len()
    );
    for mem in &report.stale_memories {
        println!("  - {} ({})", mem.title, mem.id);
    }
    println!();

    println!(
        "{BOLD}Untagged memories ({}){RESET}",
        report.untagged_memories.len()
    );
    for mem in &report.untagged_memories {
        println!("  - {} ({})", mem.title, mem.id);
    }
    println!();

    println!(
        "{BOLD}Short memories ({}){RESET}",
        report.short_memories.len()
    );
    for mem in &report.short_memories {
        println!("  - {} ({})", mem.title, mem.id);
    }
}

/// Prints duplicate groups.
pub fn print_duplicate_groups(groups: &[crate::store::memory::DuplicateGroup]) {
    if groups.is_empty() {
        println!("{YELLOW}No duplicate groups found.{RESET}");
        return;
    }
    println!("{BOLD}{} duplicate group(s):{RESET}\n", groups.len());
    for (i, group) in groups.iter().enumerate() {
        println!("{BOLD}[{}]{RESET} Score: {:.3}", i + 1, group.cosine_score);
        for (id, title) in group.memory_ids.iter().zip(group.titles.iter()) {
            println!("  - {} ({})", title, id);
        }
        println!();
    }
}

/// Prints a graph.
pub fn print_graph(graph: &crate::store::memory::GraphData) {
    println!("{BOLD}Knowledge Graph{RESET}\n");
    println!("{BOLD}Nodes ({}){RESET}", graph.nodes.len());
    for node in &graph.nodes {
        println!("  {} [{}] {}", node.id, node.memory_type, node.title);
    }
    println!();
    println!("{BOLD}Edges ({}){RESET}", graph.edges.len());
    for edge in &graph.edges {
        println!(
            "  {} --[{} ({:.2})]--> {}",
            edge.source, edge.relation_type, edge.confidence, edge.target
        );
    }
}

/// Prints a health report.
pub fn print_health(report: &crate::store::memory::HealthReport) {
    println!("{BOLD}System Health{RESET}\n");
    println!("  {DIM}Version:{RESET}            {}", report.version);
    println!(
        "  {DIM}DB size:{RESET}            {:.2} MB",
        report.db_size_mb
    );
    println!(
        "  {DIM}Total memories:{RESET}     {}",
        report.total_memories
    );
    println!(
        "  {DIM}Orphaned:{RESET}           {}",
        report.orphaned_memories
    );
    println!(
        "  {DIM}Unindexed embeddings:{RESET} {}",
        report.unindexed_embeddings
    );
    println!(
        "  {DIM}Embedding model:{RESET}    {}",
        report.embedding_model
    );
}

/// Prints reminders.
pub fn print_remind(memories: &[crate::store::memory::Memory]) {
    if memories.is_empty() {
        println!("{YELLOW}No reminders found.{RESET}");
        return;
    }
    println!("{BOLD}{} reminder(s):{RESET}\n", memories.len());
    for (i, mem) in memories.iter().enumerate() {
        println!(
            "{BOLD}[{}]{RESET} {} ({})",
            i + 1,
            mem.title,
            mem.importance
        );
        println!(
            "    {DIM}{}{}",
            &mem.content[..mem.content.len().min(100)],
            RESET
        );
        println!();
    }
}

/// Prints knowledge gaps.
pub fn print_knowledge_gaps(report: &crate::store::memory::KnowledgeGapsReport) {
    println!("{BOLD}Knowledge Gaps{RESET}\n");
    println!(
        "  {DIM}Coverage score:{RESET} {:.1}%",
        report.coverage_score * 100.0
    );
    println!();
    if report.gaps.is_empty() {
        println!("{GREEN}No significant gaps detected.{RESET}");
        return;
    }
    for gap in &report.gaps {
        println!("{YELLOW}⚠ {}{RESET}", gap.area);
        println!("  Count: {}", gap.count);
        println!("  Suggestion: {}", gap.suggestion);
        println!();
    }
}

fn print_table_header(headers: &[&str], widths: &[usize]) {
    print!("{BOLD}");
    for (i, header) in headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(12);
        print!(" {:<1$}", header, w);
    }
    println!("{RESET}");
    print!("{DIM}");
    for (i, _header) in headers.iter().enumerate() {
        let w = widths.get(i).copied().unwrap_or(12);
        print!(" {}", "─".repeat(w));
    }
    println!("{RESET}");
}

fn truncate(s: &str, max_width: usize) -> String {
    let width = s.width();
    if width <= max_width {
        return s.to_string();
    }

    let mut result = String::new();
    let mut current_width = 0;
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(1);
        if current_width + ch_width + 1 > max_width {
            result.push('…');
            break;
        }
        result.push(ch);
        current_width += ch_width;
    }
    result
}

/// Prints a list of sync peers.
pub fn print_peer_list(peers: &[crate::sync::peer::Peer]) {
    if peers.is_empty() {
        println!("{YELLOW}No peers found.{RESET}");
        return;
    }

    let headers = ["ID", "NAME", "TRANSPORT", "ADDRESS", "PROJECT", "AUTO"];
    let col_widths = [10, 20, 12, 24, 16, 6];

    print_table_header(&headers, &col_widths);

    for peer in peers {
        let id_short = &peer.id.to_string()[..8];
        let auto = if peer.auto_sync { "yes" } else { "no" };
        println!(
            "{DIM} {:<10} {:<20} {:<12} {:<24} {:<16} {:<6} {RESET}",
            id_short, peer.name, peer.transport, peer.address, peer.project, auto
        );
    }

    println!();
    println!("{DIM}Total: {}{} peers{}", peers.len(), RESET, RESET);
}

/// Prints sync status.
pub fn print_sync_status(stats: &crate::sync::protocol::ExportStats) {
    println!("{BOLD}Sync Export{RESET}\n");
    println!(
        "  {DIM}Memories exported:{RESET} {}",
        stats.memories_exported
    );
    println!("  {DIM}Bytes written:{RESET} {}", stats.bytes_written);
}

/// Prints sync results.
pub fn print_sync_result(results: &[crate::sync::protocol::SyncResult]) {
    if results.is_empty() {
        println!("{YELLOW}No sync results.{RESET}");
        return;
    }

    println!("{BOLD}Sync Results{RESET}\n");
    for result in results {
        let status_color = match result.status {
            crate::sync::protocol::SyncStatus::Ok => GREEN,
            crate::sync::protocol::SyncStatus::Partial => YELLOW,
            crate::sync::protocol::SyncStatus::Error => RED,
        };
        println!(
            "  {DIM}Peer:{RESET} {} ({:?})",
            result.peer_name, result.direction
        );
        println!(
            "  {DIM}Sent:{RESET} {} {DIM}Received:{RESET} {} {DIM}Conflicts:{RESET} {}",
            result.memories_sent, result.memories_received, result.conflicts_resolved
        );
        println!(
            "  {DIM}Duration:{RESET} {}ms {DIM}Status:{RESET} {}{:?}{}",
            result.duration_ms, status_color, result.status, RESET
        );
        if let Some(ref error) = result.error {
            println!("  {RED}Error: {}{}", error, RESET);
        }
        println!();
    }
}

/// Prints sync log.
pub fn print_sync_log(entries: &[crate::sync::protocol::SyncResult]) {
    if entries.is_empty() {
        println!("{YELLOW}No sync log entries.{RESET}");
        return;
    }

    println!("{BOLD}Sync Log{RESET}\n");
    for entry in entries {
        println!(
            "  {DIM}{} | {} | sent:{} recv:{} conflicts:{}{}",
            entry.peer_name,
            entry.duration_ms,
            entry.memories_sent,
            entry.memories_received,
            entry.conflicts_resolved,
            RESET
        );
    }
}
