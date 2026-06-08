
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, AppMode, DetailTab};

// ── PALETA DE COLORES ──
const BG: Color = Color::Rgb(13, 17, 23);
const BG_PANEL: Color = Color::Rgb(22, 27, 34);
const BG_HOVER: Color = Color::Rgb(30, 36, 44);
const BORDER: Color = Color::Rgb(48, 54, 61);
const BORDER_FOCUS: Color = Color::Rgb(88, 166, 255);
const TEXT: Color = Color::Rgb(201, 209, 217);
const TEXT_DIM: Color = Color::Rgb(110, 118, 129);
const TEXT_HEADER: Color = Color::Rgb(240, 246, 252);
const BLUE: Color = Color::Rgb(88, 166, 255);
const GREEN: Color = Color::Rgb(126, 231, 135);
const YELLOW: Color = Color::Rgb(210, 153, 34);
const RED: Color = Color::Rgb(218, 54, 51);
const MAGENTA: Color = Color::Rgb(195, 117, 238);
const CYAN: Color = Color::Rgb(86, 207, 225);

/// Fn auxiliar para crear un bloque con título.
fn panel(title: &'static str, focused: bool) -> Block<'static> {
    let border_style = if focused {
        Style::default().fg(BORDER_FOCUS)
    } else {
        Style::default().fg(BORDER)
    };
    Block::default()
        .title(Span::styled(format!(" {} ", title), Style::default().fg(TEXT_DIM).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .style(Style::default().bg(BG))
}

// ── RENDER PRINCIPAL ──
pub fn render(frame: &mut Frame, app: &App) {
    let size = frame.size();
    if size.width < 60 || size.height < 10 { return; }

    let main = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // header
            Constraint::Min(1),      // body
            Constraint::Length(2),  // status bar
        ])
        .split(size);

    render_header(frame, app, main[0]);

    match app.mode {
        AppMode::Graph => {
            render_graph_panel(frame, app, main[1]);
            render_statusbar(frame, app, main[2]);
        }
        AppMode::EntityGraph => {
            render_entity_graph_panel(frame, app, main[1]);
            render_statusbar(frame, app, main[2]);
        }
        AppMode::Temporal => {
            render_temporal_panel(frame, app, main[1]);
            render_statusbar(frame, app, main[2]);
        }
        _ => {
            let body = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
                .split(main[1]);
            render_memory_list(frame, app, body[0]);
            render_detail_panel(frame, app, body[1]);
            render_statusbar(frame, app, main[2]);

            // Overlays
            match app.mode {
                AppMode::Searching => {
                    let area = centered(60, 3, frame.size());
                    frame.render_widget(Clear, area);
                    render_search_overlay(frame, app, area);
                }
                AppMode::Confirming { ref action, .. } => {
                    let area = centered(50, 5, frame.size());
                    frame.render_widget(Clear, area);
                    render_confirm_overlay(frame, action, area);
                }
                AppMode::Help => {
                    let area = centered(72, 22, frame.size());
                    frame.render_widget(Clear, area);
                    render_help_overlay(frame, area);
                }
                _ => {}
            }
        }
    }
}

// ── HEADER ──
fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let bg = Block::default().style(Style::default().bg(BG));
    let inner = bg.inner(area);
    frame.render_widget(bg, area);

    let mem_count = format!("{} mems", app.memories.len());
    let total = format!("/ {}", app.total_memory_count);

    let mut spans = vec![
        Span::styled(" mneme ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD)),
        Span::styled("│", Style::default().fg(BORDER)),
        Span::raw(" "),
        Span::styled(&app.project, Style::default().fg(TEXT_HEADER).add_modifier(Modifier::BOLD)),
        Span::styled(" ", Style::default().fg(TEXT_DIM)),
    ];

    // Memory count pill
    let count_text = format!(" {} ", mem_count);
    spans.push(Span::styled(count_text, Style::default().fg(GREEN).bg(BG_PANEL)));
    if app.search_query.is_empty() {
        spans.push(Span::styled(total, Style::default().fg(TEXT_DIM)));
    } else {
        spans.push(Span::styled(format!(" /{}:{}", app.search_query, app.memories.len()), Style::default().fg(YELLOW)));
    }
    spans.push(Span::raw(" "));

    let para = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(BG));
    frame.render_widget(para, inner);
}

// ── LISTA DE MEMORIAS ──
fn render_memory_list(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("Memories", app.selected_memory().is_some());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height > 2 { frame.render_widget(Clear, inner); }

    if app.memories.is_empty() {
        let msg = if app.search_query.is_empty() {
            format!(" No memories for project '{}'.\n Press / to search, r to refresh.", app.project)
        } else {
            format!(" No results for \"{}\".", app.search_query)
        };
        let para = Paragraph::new(msg).style(Style::default().fg(TEXT_DIM)).wrap(Wrap { trim: false });
        frame.render_widget(para, inner);
        return;
    }

    // Render visible items
    let mut y = inner.y;
    let max_y = inner.y + inner.height;
    for i in app.scroll_offset.. {
        if y >= max_y || i >= app.memories.len() { break; }
        let mem = &app.memories[i];
        let selected = i == app.selected;

        // Style
        let bg = if selected { BG_HOVER } else { BG };
        let fg = if selected { TEXT_HEADER } else { TEXT };
        let imp_color = importance_color(&mem.importance);
        let type_abbr = memory_type_abbrev(&mem.memory_type);

        // Title line
        let title = &mem.title;
        let title_s = if title.len() as u16 > area.width.saturating_sub(6) {
            format!("{}…", &title[..(area.width as usize).saturating_sub(7)])
        } else {
            title.to_string()
        };
        let line = Line::from(vec![
            Span::styled(format!(" {} ", type_abbr), Style::default().fg(BG).bg(imp_color)),
            Span::styled(" ", Style::default().bg(bg)),
            Span::styled(&title_s, Style::default().fg(fg).bg(bg)),
        ]);
        let para = Paragraph::new(line);
        frame.render_widget(para, Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1));
        y += 1;

        // Subtitle line (tags or preview)
        if selected || i == app.selected {
            let tags: Vec<&str> = mem.tags.iter().map(|t| t.as_str()).collect();
            let tags_str = if tags.is_empty() { "" } else { &tags.join(", ") };
            let preview = mem.content.chars().take(50).collect::<String>();
            let sub = if !tags_str.is_empty() && y < max_y {
                format!("  {}  {}", tags_str, preview)
            } else if y < max_y {
                format!("  {}", preview)
            } else {
                String::new()
            };
            if !sub.is_empty() && y < max_y {
                let sub_para = Paragraph::new(Span::styled(sub, Style::default().fg(TEXT_DIM)))
                    .style(Style::default().bg(bg));
                frame.render_widget(sub_para, Rect::new(inner.x + 1, y, inner.width.saturating_sub(2), 1));
                y += 1;
            }
        }
    }
}

// ── PANEL DE DETALLE (con tabs) ──
fn render_detail_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("Detail", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height > 2 { frame.render_widget(Clear, inner); }

    let memory = match app.selected_memory() {
        Some(m) => m,
        None => {
            let para = Paragraph::new(" Select a memory to view details.")
                .style(Style::default().fg(TEXT_DIM));
            frame.render_widget(para, inner);
            return;
        }
    };

    // Tab bar
    let tabs = [" Content ", " Fields ", " Entities ", " Temporal ", " Graph "];
    let tab_highlight = match app.detail_tab {
        DetailTab::Content => 0,
        DetailTab::Structured => 1,
        DetailTab::Entities => 2,
        DetailTab::Temporal => 3,
        DetailTab::Relations => 4,
    };
    // Render all tabs at top
    let tabs_area = Rect::new(inner.x + 1, inner.y, inner.width.saturating_sub(2), 1);
    let tab_line: Vec<Span> = tabs.iter().enumerate().map(|(i, label)| {
        if i == tab_highlight {
            Span::styled(label.to_string(), Style::default().fg(BORDER_FOCUS).add_modifier(Modifier::BOLD | Modifier::REVERSED))
        } else {
            Span::styled(label.to_string(), Style::default().fg(TEXT_DIM))
        }
    }).collect();
    frame.render_widget(Paragraph::new(Line::from(tab_line)), tabs_area);

    // Content area
    let content_area = Rect::new(inner.x + 1, inner.y + 1, inner.width.saturating_sub(2), inner.height.saturating_sub(2));
    if content_area.width < 5 || content_area.height < 2 { return; }

    match app.detail_tab {
        DetailTab::Content => render_detail_content(frame, memory, content_area, app.detail_scroll),
        DetailTab::Structured => render_detail_structured(frame, memory, content_area, app.detail_scroll),
        DetailTab::Entities => render_detail_entities(frame, app, content_area),
        DetailTab::Temporal => render_detail_temporal(frame, memory, content_area),
        DetailTab::Relations => render_detail_relations(frame, app, memory, content_area),
    }
}

fn render_detail_content(frame: &mut Frame, memory: &crate::store::memory::Memory, area: Rect, scroll: usize) {
    let text = format!("{}\n\n📝 {}\n\n{}", memory.title, memory.content, memory.learned.as_deref().unwrap_or(""));
    let line_count = text.lines().count();
    let para = Paragraph::new(text.as_str())
        .style(Style::default().fg(TEXT))
        .scroll((scroll.min(line_count.saturating_sub(area.height as usize)) as u16, 0))
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_detail_structured(frame: &mut Frame, memory: &crate::store::memory::Memory, area: Rect, _scroll: usize) {
    let mut lines = Vec::new();
    if let Some(ref w) = memory.what { lines.push(format!(" What: {}", w)); }
    if let Some(ref w) = memory.why { lines.push(format!(" Why: {}", w)); }
    if let Some(ref c) = memory.context { lines.push(format!(" Context: {}", c)); }
    if let Some(ref l) = memory.learned { lines.push(format!(" Learned: {}", l)); }
    if lines.is_empty() {
        lines.push(" No structured fields.".to_string());
    }
    // Meta info
    lines.push(String::new());
    lines.push(format!(" Type: {}  Importance: {}  Scope: {}", memory.memory_type, memory.importance, memory.scope));
    lines.push(format!(" Access: {}  Revisions: {}  Duplicates: {}", memory.access_count, memory.revision_count, memory.duplicate_count));
    lines.push(format!(" Tags: {}", memory.tags.join(", ")));
    lines.push(format!(" Created: {}", memory.created_at.format("%Y-%m-%d %H:%M UTC")));
    lines.push(format!(" Updated: {}", memory.updated_at.format("%Y-%m-%d %H:%M UTC")));

    let text = Text::from(lines.join("\n"));
    let para = Paragraph::new(text).style(Style::default().fg(TEXT));
    frame.render_widget(para, area);
}

fn render_detail_entities(frame: &mut Frame, app: &App, area: Rect) {
    if let Ok(store) = &app.db.entities().get_memory_entities(app.selected_memory().map(|m| m.id).unwrap_or_default()) {
        if store.is_empty() {
            let para = Paragraph::new(" No entities extracted.").style(Style::default().fg(TEXT_DIM));
            frame.render_widget(para, area);
            return;
        }
        let lines: Vec<String> = store.iter().map(|e| {
            format!(" {} ({})  confidence: {:.2}", e.entity_name, e.entity_type, e.confidence)
        }).collect();
        let text = Text::from(lines.join("\n"));
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(CYAN)), area);
    } else {
        let para = Paragraph::new(" No entities data.").style(Style::default().fg(TEXT_DIM));
        frame.render_widget(para, area);
    }
}

fn render_detail_temporal(frame: &mut Frame, memory: &crate::store::memory::Memory, area: Rect) {
    let vf = memory.valid_from.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "—".to_string());
    let vu = memory.valid_until.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_else(|| "—".to_string());
    let prov = memory.provenance.as_deref().unwrap_or("none");
    let text = format!(" Valid From: {}\n Valid Until: {}\n Provenance: {}", vf, vu, prov);
    let para = Paragraph::new(text).style(Style::default().fg(TEXT));
    frame.render_widget(para, area);
}

fn render_detail_relations(frame: &mut Frame, app: &App, memory: &crate::store::memory::Memory, area: Rect) {
    // Get graph data and show relations for this memory
    if let Ok(data) = app.db.memories().get_graph(&app.project) {
        let edges: Vec<String> = data.edges.iter()
            .filter(|e| e.source == memory.id.to_string() || e.target == memory.id.to_string())
            .map(|e| {
                let other = if e.source == memory.id.to_string() { &e.target } else { &e.source };
                let dir = if e.source == memory.id.to_string() { "→" } else { "←" };
                format!(" {} {} {} ({:.2})", other, dir, e.relation_type, e.confidence)
            })
            .collect();
        if edges.is_empty() {
            let para = Paragraph::new(" No relations.").style(Style::default().fg(TEXT_DIM));
            frame.render_widget(para, area);
            return;
        }
        let text = Text::from(edges.join("\n"));
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(MAGENTA)), area);
    }
}

// ── GRAFO ──
fn render_graph_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("Knowledge Graph", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref data) = app.graph_data {
        let _selected_node = data.nodes.get(app.graph_selected);
        let mut lines = vec![format!("Nodes: {}  Edges: {}  Selected: {}", data.nodes.len(), data.edges.len(), app.graph_selected + 1)];
        lines.push(String::new());
        for (i, node) in data.nodes.iter().enumerate() {
            let indent = if i == app.graph_selected { "→" } else { " " };
            lines.push(format!(" {} [{}] {} ({})", indent, node.importance, node.title, node.memory_type));
        }
        let text = Text::from(lines.join("\n"));
        let para = Paragraph::new(text).style(Style::default().fg(TEXT));
        frame.render_widget(para, inner);
    }
}

fn render_entity_graph_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("Entity Graph", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref eg) = app.entity_graph_data {
        let text = Text::from(eg.frequent_entities.iter().enumerate().map(|(i, (name, _etype, count))| {
            format!(" {} {}  ({})", if i == eg.selected { "→" } else { " " }, name, count)
        }).collect::<Vec<_>>().join("\n"));
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(CYAN)), inner);
    }
}

fn render_temporal_panel(frame: &mut Frame, app: &App, area: Rect) {
    let block = panel("Temporal View", true);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if let Some(ref td) = app.temporal_data {
        let mode = match td.display_mode { 0 => "All", 1 => "Valid Now", 2 => "Expired", _ => "" };
        let total = td.memories.len();
        let valid = td.memories.iter().filter(|m| {
            let now = td.reference_time;
            match (m.valid_from, m.valid_until) {
                (Some(vf), Some(vu)) => vf <= now && now < vu,
                (Some(vf), None) => vf <= now,
                (None, Some(vu)) => now < vu,
                (None, None) => true,
            }
        }).count();
        let expired = td.memories.iter().filter(|m| m.valid_until.map(|vu| vu <= td.reference_time).unwrap_or(false)).count();
        let mut lines = vec![format!("Mode: {}  Total: {}  Valid: {}  Expired: {}", mode, total, valid, expired)];
        lines.push(String::new());
        for mem in &td.memories {
            let vf = mem.valid_from.map(|d| d.format("%m-%d").to_string()).unwrap_or_else(|| "--".to_string());
            let vu = mem.valid_until.map(|d| d.format("%m-%d").to_string()).unwrap_or_else(|| "--".to_string());
            lines.push(format!("  {} [{} → {}]", mem.title, vf, vu));
        }
        let text = Text::from(lines.join("\n"));
        frame.render_widget(Paragraph::new(text).style(Style::default().fg(YELLOW)), inner);
    }
}

// ── OVERLAYS ──
fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_FOCUS))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let display = if app.search_query.is_empty() {
        " Search: _".to_string()
    } else {
        format!(" Search: {}█", app.search_query)
    };
    let para = Paragraph::new(display).style(Style::default().fg(TEXT));
    frame.render_widget(para, inner);
}

fn render_confirm_overlay(frame: &mut Frame, action: &str, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(RED))
        .style(Style::default().bg(BG));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let msg = format!(" Confirm {}? (y/N)", action);
    frame.render_widget(Paragraph::new(msg).style(Style::default().fg(TEXT_HEADER)), inner);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(BORDER_FOCUS))
        .style(Style::default().bg(BG))
        .title(" Help ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let help_text = vec![
        Line::from(vec![Span::styled(" NAVIGATION ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(" j/k or ↑/↓  Move selection"),
        Line::from(" g/G         First / last memory"),
        Line::from(" PgUp/PgDn   Page up / down"),
        Line::from(""),
        Line::from(vec![Span::styled(" DETAIL PANEL ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(" Shift+J/K   Scroll detail content"),
        Line::from(" [ / ]       Previous / next detail tab"),
        Line::from(""),
        Line::from(vec![Span::styled(" ACTIONS ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(" /           Search"),
        Line::from(" r           Reload memories"),
        Line::from(" d           Delete selected memory"),
        Line::from(""),
        Line::from(vec![Span::styled(" VIEWS ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(" Tab         Knowledge graph"),
        Line::from(" e           Entity graph (frequent entities)"),
        Line::from(" t           Temporal view (validity windows)"),
        Line::from(""),
        Line::from(vec![Span::styled(" MISC ", Style::default().fg(BLUE).add_modifier(Modifier::BOLD))]),
        Line::from(" ?           Toggle this help"),
        Line::from(" q / Ctrl+C  Quit"),
    ];
    let para = Paragraph::new(Text::from(help_text)).style(Style::default().fg(TEXT));
    frame.render_widget(para, inner);
}

// ── STATUSBAR ──
fn render_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let (text, style): (String, Style) = match app.mode {
        AppMode::Graph => (" [Tab/Esc] Back  [j/k] Select  [q] Quit".to_string(), Style::default().fg(TEXT_DIM).bg(BG_PANEL)),
        AppMode::EntityGraph => (" [Tab/Esc] Back  [r] Refresh  [q] Quit".to_string(), Style::default().fg(TEXT_DIM).bg(BG_PANEL)),
        AppMode::Temporal => (" [Tab/Esc] Back  [m] Cycle mode  [r] Refresh  [q] Quit".to_string(), Style::default().fg(TEXT_DIM).bg(BG_PANEL)),
        AppMode::Searching => (" Type to search. Esc to cancel, Enter to confirm.".to_string(), Style::default().fg(YELLOW).bg(BG_PANEL)),
        AppMode::Confirming { .. } => (" [y] Yes  [n/Esc] No".to_string(), Style::default().fg(RED).bg(BG_PANEL)),
        AppMode::Help => (" Press any key to close help.".to_string(), Style::default().fg(TEXT_DIM).bg(BG_PANEL)),
        AppMode::Normal => {
            let hint = match app.selected_memory() {
                Some(m) => format!(
                    " {}·{} · {} acc | [↑↓] Nav [r] Rld [d] Del [Tab]G [e]Ent [t]Tmp",
                    m.memory_type, m.importance, m.access_count
                ),
                None => " No memories. Press / to search or r to refresh.".to_string(),
            };
            (hint, Style::default().fg(TEXT_DIM).bg(BG_PANEL))
        }
    };
    let bar = Paragraph::new(Span::raw(text.as_str()))
        .style(style);
    frame.render_widget(bar, area);
}

// ── HELPERS ──
fn centered(width: u16, height: u16, r: Rect) -> Rect {
    Rect::new(
        r.x.saturating_add(r.width.saturating_sub(width) / 2),
        r.y.saturating_add(r.height.saturating_sub(height) / 2),
        width.min(r.width),
        height.min(r.height),
    )
}

fn importance_color(imp: &crate::store::memory::Importance) -> Color {
    match imp {
        crate::store::memory::Importance::Critical => RED,
        crate::store::memory::Importance::High => YELLOW,
        crate::store::memory::Importance::Medium => Color::Rgb(110, 118, 129),
        crate::store::memory::Importance::Low => TEXT_DIM,
    }
}

fn memory_type_abbrev(mt: &crate::store::memory::MemoryType) -> &'static str {
    use crate::store::memory::MemoryType;
    match mt {
        MemoryType::Architecture => "ARCH",
        MemoryType::Decision => "DEC",
        MemoryType::Bugfix => "BUG",
        MemoryType::Pattern => "PAT",
        MemoryType::Convention => "CONV",
        MemoryType::Dependency => "DEP",
        MemoryType::Workflow => "WRK",
        MemoryType::Note => "NOTE",
        MemoryType::Config => "CFG",
        MemoryType::Discovery => "DIS",
        MemoryType::Learning => "LRN",
        MemoryType::AgentFact => "AGT",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::App;
    use crate::config::settings::Settings;
    use crate::store::db::Database;
    use std::path::PathBuf;
    use std::sync::Arc;
    use uuid::Uuid;

    #[test]
    fn test_centered_in_bounds() {
        let r = Rect::new(0, 0, 100, 50);
        let c = centered(60, 20, r);
        assert_eq!(c.x, 20);
        assert_eq!(c.y, 15);
    }

    #[test]
    fn test_centered_smaller_than_area() {
        let r = Rect::new(10, 5, 80, 40);
        let c = centered(40, 10, r);
        assert_eq!(c.x, 30);
        assert_eq!(c.y, 20);
    }

    #[test]
    fn test_importance_colors() {
        use crate::store::memory::Importance;
        assert_eq!(importance_color(&Importance::Critical), RED);
        assert_eq!(importance_color(&Importance::High), YELLOW);
        assert_eq!(importance_color(&Importance::Low), TEXT_DIM);
    }

    #[test]
    fn test_memory_type_abbrevs() {
        use crate::store::memory::MemoryType;
        assert_eq!(memory_type_abbrev(&MemoryType::Architecture), "ARCH");
        assert_eq!(memory_type_abbrev(&MemoryType::Decision), "DEC");
        assert_eq!(memory_type_abbrev(&MemoryType::Bugfix), "BUG");
        assert_eq!(memory_type_abbrev(&MemoryType::AgentFact), "AGT");
    }

    #[test]
    fn test_detail_tab_navigation() {
        let mut tab = DetailTab::Content;
        tab = tab.next();
        assert_eq!(tab, DetailTab::Structured);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Entities);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Temporal);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Relations);
        tab = tab.next();
        assert_eq!(tab, DetailTab::Content);
        tab = tab.prev();
        assert_eq!(tab, DetailTab::Relations);
    }
}
