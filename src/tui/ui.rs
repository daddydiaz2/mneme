use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, Screen, DetailTab, Action};

const BG: Color = Color::Rgb(13, 17, 23);
const BG2: Color = Color::Rgb(22, 27, 34);
const BG3: Color = Color::Rgb(30, 36, 44);
const BORDER: Color = Color::Rgb(48, 54, 61);
const BFOCUS: Color = Color::Rgb(88, 166, 255);
const TXT: Color = Color::Rgb(201, 209, 217);
const DIM: Color = Color::Rgb(110, 118, 129);
const HDR: Color = Color::Rgb(240, 246, 252);
const BLUE: Color = Color::Rgb(88, 166, 255);
const GREEN: Color = Color::Rgb(126, 231, 135);
const YELLOW: Color = Color::Rgb(210, 153, 34);
const RED: Color = Color::Rgb(218, 54, 51);
const MAGENTA: Color = Color::Rgb(195, 117, 238);
const CYAN: Color = Color::Rgb(86, 207, 225);
const ORANGE: Color = Color::Rgb(255, 165, 0);

fn b(title: &str) -> Block<'static> {
    Block::default().title(Span::styled(format!(" {} ", title), Style::default().fg(DIM).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(BORDER)).style(Style::default().bg(BG))
}

pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.size();
    if size.width < 60 || size.height < 10 { return; }
    let lay = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Min(1)]).split(size);
    // Thin top bar
    let bar = format!("🧠 mneme v0.2.0  │  {}  │  {} mems, {} sessions", app.project, app.total_memory_count, app.sessions.len());
    frame.render_widget(Paragraph::new(Span::styled(bar, Style::default().fg(DIM).bg(BG))).style(Style::default().bg(BG)), lay[0]);

    match app.screen {
        Screen::Dashboard => render_dashboard(frame, app, lay[1]),
        Screen::Memories => render_memories(frame, app, lay[1]),
        Screen::Sessions => render_sessions(frame, app, lay[1]),
        Screen::SessionDetail => render_session_detail(frame, app, lay[1]),
        Screen::Prompts => render_prompts(frame, app, lay[1]),
        Screen::Projects => render_projects(frame, app, lay[1]),
        Screen::Search => render_search(frame, app, lay[1]),
        Screen::AgentSetup => render_agent_setup(frame, app, lay[1]),
        Screen::Graph => render_graph(frame, app, lay[1]),
        Screen::EntityGraph => render_entity_graph(frame, app, lay[1]),
        Screen::Temporal => render_temporal(frame, app, lay[1]),
    }
}

// ═══ DASHBOARD ═══
fn render_dashboard(frame: &mut Frame, app: &App, area: Rect) {
    let inner = b("").inner(area);
    frame.render_widget(b(""), area);

    // Welcome
    let welcome = format!("Welcome to mneme · project: {}", app.project);
    frame.render_widget(Paragraph::new(Span::styled(welcome, Style::default().fg(HDR).add_modifier(Modifier::BOLD))).style(Style::default().bg(BG)), Rect::new(inner.x+2, inner.y+1, inner.width.saturating_sub(4), 1));

    // Stats row
    let stats = format!("{} memories · {} sessions · {} projects", app.total_memory_count, app.sessions.len(), app.projects.len());
    frame.render_widget(Paragraph::new(Span::styled(stats, Style::default().fg(DIM))).style(Style::default().bg(BG)), Rect::new(inner.x+2, inner.y+2, inner.width.saturating_sub(4), 1));

    // Actions grid (2x4)
    let actions: Vec<(Action, &str, &str, &str)> = vec![
        (Action::Search, "1/s", "Search memories", "Search across all memories, sessions, and entities"),
        (Action::RecentObservations, "2/o", "Recent observations", "Browse and filter the most recent memories"),
        (Action::BrowseSessions, "3/b", "Browse sessions", "View and navigate work sessions"),
        (Action::ViewPrompts, "4/p", "View prompts", "Review all saved user prompts"),
        (Action::Projects, "5/r", "Projects", "List and switch between projects"),
        (Action::AgentPlugin, "6/a", "Setup agent plugin", "Configure MCP for Claude Code, Codex, etc."),
        (Action::Quit, "7/q", "Quit", "Exit mneme TUI"),
    ];
    let start_y = inner.y + 4;
    for (i, (action, key, title, desc)) in actions.iter().enumerate() {
        let i_u16 = i as u16;
        let x = inner.x + 2 + (i_u16 % 2) * ((inner.width.saturating_sub(6)) / 2);
        let y = start_y + (i_u16 / 2) * 3;
        let card_w = ((inner.width.saturating_sub(6)) / 2).min(40);
        let selected = i == app.selected;

        let bg = if selected { BG3 } else { BG2 };
        let style = Style::default().bg(bg);

        // Key badge
        let key_span = Span::styled(format!(" {} ", key), Style::default().fg(BG).bg(BFOCUS).add_modifier(Modifier::BOLD));
        // Title
        let title_span = Span::styled(format!(" {} ", title), Style::default().fg(if selected { HDR } else { TXT }).bg(bg));

        frame.render_widget(Paragraph::new(Line::from(vec![key_span, title_span])).style(style), Rect::new(x, y, card_w, 1));
        frame.render_widget(Paragraph::new(Span::styled(format!("  {}", desc), Style::default().fg(DIM).bg(bg))).style(style), Rect::new(x, y+1, card_w, 1));
    }

    // Bottom help
    let help = " ↑↓ navigate · Enter select · Esc quit";
    frame.render_widget(Paragraph::new(Span::styled(help, Style::default().fg(DIM))), Rect::new(inner.x+2, inner.y+inner.height.saturating_sub(2), inner.width.saturating_sub(4), 1));
}

// ═══ MEMORIES (previous list+detail) ═══
fn render_memories(frame: &mut Frame, app: &App, area: Rect) {
    let body = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);
    let block = b("Observations");
    let inner = block.inner(body[0]);
    frame.render_widget(block, body[0]);

    if app.memories.is_empty() {
        let empty_msg = if app.search_query.is_empty() {
            String::from(" No memories. Press / to search or 'r' to refresh.")
        } else {
            format!(" No results for \"{}\".", app.search_query)
        };
        frame.render_widget(Paragraph::new(empty_msg.as_str()).style(Style::default().fg(DIM)), inner);
    } else {
        let mut y = inner.y;
        for i in app.scroll_offset..app.memories.len().min(app.scroll_offset + inner.height as usize) {
            if y >= inner.y + inner.height { break; }
            let mem = &app.memories[i];
            let sel = i == app.selected;
            let fg = if sel { HDR } else { TXT };
            let bg = if sel { BG3 } else { BG };
            let imp_color = match mem.importance { crate::store::memory::Importance::Critical => RED, crate::store::memory::Importance::High => YELLOW, crate::store::memory::Importance::Low => DIM, _ => DIM };
            let ta = match mem.memory_type { crate::store::memory::MemoryType::Architecture => "ARCH", crate::store::memory::MemoryType::Decision => "DEC", crate::store::memory::MemoryType::Bugfix => "BUG", crate::store::memory::MemoryType::Pattern => "PAT", crate::store::memory::MemoryType::Convention => "CONV", crate::store::memory::MemoryType::Dependency => "DEP", crate::store::memory::MemoryType::Workflow=>"WRK", crate::store::memory::MemoryType::Note=>"NOTE", crate::store::memory::MemoryType::Config=>"CFG", crate::store::memory::MemoryType::Discovery=>"DIS", crate::store::memory::MemoryType::Learning=>"LRN", crate::store::memory::MemoryType::AgentFact=>"AGT" };
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled(format!(" {} ", ta), Style::default().fg(BG).bg(imp_color)),
                Span::styled(format!(" {}", mem.title), Style::default().fg(fg).bg(bg)),
            ])).style(Style::default().bg(bg)), Rect::new(inner.x+1, y, inner.width.saturating_sub(2), 1));
            y += 1;
        }
    }

    // Detail panel (right)
    let detail_block = b("Detail");
    let d_inner = detail_block.inner(body[1]);
    frame.render_widget(detail_block, body[1]);
    if let Some(mem) = app.selected_memory() {
        render_detail(frame, app, d_inner, mem);
    } else {
        frame.render_widget(Paragraph::new(" Select a memory").style(Style::default().fg(DIM)), d_inner);
    }

    // Status hint
    let hint = match app.selected_memory() {
        Some(m) => format!("{}·{} · {} acc · [↑↓] [][]] Tabs  [K/J] Scroll  [d] Del  [Esc] Back", m.memory_type, m.importance, m.access_count),
        None => " [↑↓] Navigate  [Esc] Back  [/] Search  [r] Refresh".to_string(),
    };
    frame.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect, mem: &crate::store::memory::Memory) {
    if area.width < 8 || area.height < 3 { return; }
    // Tab bar
    let tabs = [" Content ", " Fields ", " Entities ", " Temporal ", " Graph "];
    let tidx = match app.detail_tab { DetailTab::Content=>0, DetailTab::Structured=>1, DetailTab::Entities=>2, DetailTab::Temporal=>3, DetailTab::Relations=>4 };
    let tab_line: Vec<Span> = tabs.iter().enumerate().map(|(i,l)| {
        if i == tidx { Span::styled(*l, Style::default().fg(BFOCUS).add_modifier(Modifier::REVERSED)) } else { Span::styled(*l, Style::default().fg(DIM)) }
    }).collect();
    frame.render_widget(Paragraph::new(Line::from(tab_line)), Rect::new(area.x+1, area.y, area.width.saturating_sub(2), 1));
    let ca = Rect::new(area.x+1, area.y+1, area.width.saturating_sub(2), area.height.saturating_sub(2));
    if ca.width < 4 || ca.height < 1 { return; }
    match app.detail_tab {
        DetailTab::Content => {
            let text = format!("{}\n\n{}\n{}", mem.title, mem.content, mem.learned.as_deref().unwrap_or(""));
            let lc = text.lines().count();
            let p = Paragraph::new(text.as_str()).style(Style::default().fg(TXT)).scroll((app.detail_scroll.min(lc.saturating_sub(ca.height as usize)) as u16,0)).wrap(Wrap{trim:false});
            frame.render_widget(p, ca);
        }
        DetailTab::Structured => {
            let mut ls = vec![];
            if let Some(ref w) = mem.what { ls.push(format!("What: {}", w)); }
            if let Some(ref w) = mem.why { ls.push(format!("Why: {}", w)); }
            if let Some(ref c) = mem.context { ls.push(format!("Context: {}", c)); }
            if let Some(ref l) = mem.learned { ls.push(format!("Learned: {}", l)); }
            ls.push(String::new());
            ls.push(format!("Type: {}  Imp: {}  Scope: {}", mem.memory_type, mem.importance, mem.scope));
            ls.push(format!("Access: {}  Rev: {}  Dup: {}", mem.access_count, mem.revision_count, mem.duplicate_count));
            ls.push(format!("Tags: {}", mem.tags.join(", ")));
            ls.push(format!("Created: {}", mem.created_at.format("%Y-%m-%d %H:%M")));
            ls.push(format!("Updated: {}", mem.updated_at.format("%Y-%m-%d %H:%M")));
            frame.render_widget(Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(TXT)), ca);
        }
        DetailTab::Entities => {
            if let Ok(entities) = &app.db.entities().get_memory_entities(mem.id) {
                if entities.is_empty() { frame.render_widget(Paragraph::new("No entities.").style(Style::default().fg(DIM)), ca); return; }
                let ls: Vec<String> = entities.iter().map(|e| format!("{} ({})  conf: {:.2}", e.entity_name, e.entity_type, e.confidence)).collect();
                frame.render_widget(Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(CYAN)), ca);
            }
        }
        DetailTab::Temporal => {
            let vf = mem.valid_from.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "—".to_string());
            let vu = mem.valid_until.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "—".to_string());
            frame.render_widget(Paragraph::new(Text::from(format!("From: {}\nUntil: {}\nProvenance: {}", vf, vu, mem.provenance.as_deref().unwrap_or("none")))).style(Style::default().fg(TXT)), ca);
        }
        DetailTab::Relations => {
            if let Ok(data) = app.db.memories().get_graph(&app.project) {
                let edges: Vec<String> = data.edges.iter().filter(|e| e.source == mem.id.to_string() || e.target == mem.id.to_string()).map(|e| {
                    let other = if e.source == mem.id.to_string() { &e.target } else { &e.source };
                    let dir = if e.source == mem.id.to_string() { "→" } else { "←" };
                    format!(" {} {} {} ({:.2})", other, dir, e.relation_type, e.confidence)
                }).collect();
                if edges.is_empty() { frame.render_widget(Paragraph::new("No relations.").style(Style::default().fg(DIM)), ca); return; }
                frame.render_widget(Paragraph::new(Text::from(edges.join("\n"))).style(Style::default().fg(MAGENTA)), ca);
            }
        }
    }
}

// ═══ SESSIONS ═══
fn render_sessions(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Sessions");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.sessions.is_empty() {
        frame.render_widget(Paragraph::new(" No sessions found. Start one via MCP: mem_session_start").style(Style::default().fg(DIM)), inner);
        return;
    }
    let mut y = inner.y;
    for (i, s) in app.sessions.iter().enumerate() {
        if y >= inner.y + inner.height { break; }
        let sel = i == app.selected;
        let fg = if sel { HDR } else { TXT };
        let bg = if sel { BG3 } else { BG };
        let start = s.started_at.format("%Y-%m-%d %H:%M").to_string();
        let status = &s.status;
        let mems = s.memory_ids.len();
        frame.render_widget(Paragraph::new(Line::from(vec![
            Span::styled(format!(" [{}] ", status), Style::default().fg(if status=="active"{GREEN}else{DIM}).bg(bg)),
            Span::styled(format!("{}  ", start), Style::default().fg(DIM).bg(bg)),
            Span::styled(format!("{} memories", mems), Style::default().fg(fg).bg(bg)),
        ])).style(Style::default().bg(bg)), Rect::new(inner.x+1, y, inner.width.saturating_sub(2), 1));
        y += 1;
        if let Some(ref summary) = s.summary {
            if y < inner.y + inner.height {
                frame.render_widget(Paragraph::new(Span::styled(format!(" {}", summary), Style::default().fg(DIM).bg(bg))).style(Style::default().bg(bg)), Rect::new(inner.x+2, y, inner.width.saturating_sub(4), 1));
                y += 1;
            }
        }
    }
    let hint = " [↑↓] Navigate  [Enter] View details  [r] Refresh  [Esc] Back";
    frame.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

fn render_session_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Session Detail");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref s) = app.selected_session {
        let mut lines = vec![];
        lines.push(format!("Session: {}", s.id));
        lines.push(format!("Project: {}  Status: {}  Started: {}", s.project, s.status, s.started_at.format("%Y-%m-%d %H:%M")));
        if let Some(ref end) = s.ended_at { lines.push(format!("Ended: {}", end.format("%Y-%m-%d %H:%M"))); }
        if let Some(ref summary) = s.summary { lines.push(format!("Summary: {}", summary)); }
        if let Some(ref dir) = s.directory { lines.push(format!("Directory: {}", dir)); }
        lines.push(format!("Memories in session: {}", s.memory_ids.len()));
        frame.render_widget(Paragraph::new(Text::from(lines.join("\n"))).style(Style::default().fg(TXT)), inner);
    }
    let hint = " [Esc] Back to sessions";
    frame.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

// ═══ PROMPTS ═══
fn render_prompts(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Prompts");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(" Prompts available via MCP: mem_save_prompt · mem_session_summary\n\n Use the HTTP API or MCP tools to record prompts.\n\n Coming in a future release: real prompt viewer.").style(Style::default().fg(DIM)), inner);
    let hint = " [Esc] Back to dashboard";
    frame.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

// ═══ PROJECTS ═══
fn render_projects(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Projects");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.projects.is_empty() {
        frame.render_widget(Paragraph::new(" No projects found.").style(Style::default().fg(DIM)), inner);
    } else {
        let mut y = inner.y;
        for (i, p) in app.projects.iter().enumerate() {
            if y >= inner.y + inner.height { break; }
            let sel = i == app.selected;
            let active = p.name == app.project;
            let fg = if sel { HDR } else if active { GREEN } else { TXT };
            let bg = if sel { BG3 } else { BG };
            let marker = if active { "●" } else { "○" };
            let last = p.last_activity.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "never".to_string());
            frame.render_widget(Paragraph::new(Line::from(vec![
                Span::styled(format!(" {} ", marker), Style::default().fg(if active{GREEN}else{DIM}).bg(bg)),
                Span::styled(format!("{}  ", p.name), Style::default().fg(fg).bg(bg)),
                Span::styled(format!("{} mems  last: {}", p.memory_count, last), Style::default().fg(DIM).bg(bg)),
            ])).style(Style::default().bg(bg)), Rect::new(inner.x+1, y, inner.width.saturating_sub(2), 1));
            y += 1;
        }
    }
    let hint = " [↑↓] Select  [Enter] Switch project  [r] Refresh  [Esc] Back";
    frame.render_widget(Paragraph::new(Span::styled(hint, Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

// ═══ SEARCH ═══
fn render_search(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Search");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let display = if app.search_query.is_empty() { " Type a query and press Enter: _".to_string() } else { format!(" Query: {}█", app.search_query) };
    frame.render_widget(Paragraph::new(display).style(Style::default().fg(TXT)), Rect::new(inner.x+1, inner.y+1, inner.width.saturating_sub(2), 1));
    if app.search_query.len() >= 2 {
        // Show live results count
        let store = app.db.memories();
        let _ = store; // would need async search
        frame.render_widget(Paragraph::new(" Press Enter to search. Esc to cancel.").style(Style::default().fg(DIM)), Rect::new(inner.x+1, inner.y+3, inner.width.saturating_sub(2), 1));
    }
}

// ═══ AGENT SETUP ═══
fn render_agent_setup(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Setup Agent Plugin");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let text = format!(
" mneme works with any agent that supports MCP.\n\n Claude Code:\n   /plugin marketplace add daddydiaz2/mneme\n   /plugin install mneme\n\n OpenCode:\n   Add to ~/.config/opencode/opencode.json:\n   {{ \"mcpServers\": [{{ \"name\": \"mneme\", \"command\": \"mneme\", \"args\": [\"mcp\"] }}] }}\n\n Codex CLI:\n   export OPENCODE_MCP_SERVERS='mneme:mneme mcp'\n\n Manual MCP:\n   mneme mcp --tools all\n\n Current project: {}\n Default MCP project: {}\n Binary: mneme v0.2.0 ({} KB)", app.project, "default", "~13");
    frame.render_widget(Paragraph::new(text.as_str()).style(Style::default().fg(TXT)), inner);
    frame.render_widget(Paragraph::new(Span::styled(" [Esc] Back to dashboard", Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

// ═══ GRAPH ═══
fn render_graph(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Knowledge Graph");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref data) = app.graph_data {
        let mut lines = vec![format!("Nodes: {}  Edges: {}  Selected: {}", data.nodes.len(), data.edges.len(), app.graph_selected+1)];
        for (i, n) in data.nodes.iter().enumerate() {
            let m = if i == app.graph_selected { "→" } else { " " };
            lines.push(format!(" {} [{}] {} ({})", m, n.importance, n.title, n.memory_type));
        }
        frame.render_widget(Paragraph::new(Text::from(lines.join("\n"))).style(Style::default().fg(TXT)), inner);
    }
    frame.render_widget(Paragraph::new(Span::styled(" [j/k] Navigate  [Tab/Esc] Back  [r] Refresh", Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

fn render_entity_graph(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Entity Graph");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref eg) = app.entity_graph_data {
        let lines: Vec<String> = eg.frequent_entities.iter().enumerate().map(|(i,(n,_t,c))| format!(" {}  ({})", n, c)).collect();
        frame.render_widget(Paragraph::new(Text::from(lines.join("\n"))).style(Style::default().fg(CYAN)), inner);
    }
    frame.render_widget(Paragraph::new(Span::styled(" [Tab/Esc] Back  [r] Refresh", Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

fn render_temporal(frame: &mut Frame, app: &App, area: Rect) {
    let block = b("Temporal View");
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref td) = app.temporal_data {
        let mode = match td.display_mode { 0=>"All", 1=>"Valid Now", 2=>"Expired", _=>"" };
        let total = td.memories.len();
        let valid = td.memories.iter().filter(|m| { let n=td.reference_time; match (m.valid_from,m.valid_until) { (Some(vf),Some(vu))=>vf<=n&&n<vu, (Some(vf),None)=>vf<=n, (None,Some(vu))=>n<vu, (None,None)=>true } }).count();
        let expired = td.memories.iter().filter(|m| m.valid_until.map(|vu|vu<=td.reference_time).unwrap_or(false)).count();
        let mut lines = vec![format!("Mode: {}  Total: {}  Valid: {}  Expired: {}", mode, total, valid, expired)];
        for m in &td.memories {
            let vf = m.valid_from.map(|d|d.format("%m-%d").to_string()).unwrap_or_else(||"--".to_string());
            let vu = m.valid_until.map(|d|d.format("%m-%d").to_string()).unwrap_or_else(||"--".to_string());
            lines.push(format!("  {} [{} → {}]", m.title, vf, vu));
        }
        frame.render_widget(Paragraph::new(Text::from(lines.join("\n"))).style(Style::default().fg(YELLOW)), inner);
    }
    frame.render_widget(Paragraph::new(Span::styled(" [Tab/Esc] Back  [m] Cycle mode  [r] Refresh", Style::default().fg(DIM))).style(Style::default().bg(BG2)), Rect::new(area.x, area.y+area.height.saturating_sub(1), area.width, 1));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_centered() {
        let r = centered(60,20,Rect::new(0,0,100,50));
        assert_eq!(r.x,20); assert_eq!(r.y,15);
    }
    fn centered(w:u16,h:u16,r:Rect)->Rect { Rect::new(r.x.saturating_add(r.width.saturating_sub(w)/2),r.y.saturating_add(r.height.saturating_sub(h)/2),w.min(r.width),h.min(r.height)) }
}
