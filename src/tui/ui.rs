use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{App, DetailTab};

const BG: Color = Color::Rgb(13, 17, 23);
const BG2: Color = Color::Rgb(22, 27, 34);
const BG3: Color = Color::Rgb(30, 36, 44);
const BDR: Color = Color::Rgb(48, 54, 61);
const BDRF: Color = Color::Rgb(88, 166, 255);
const TXT: Color = Color::Rgb(201, 209, 217);
const DIM: Color = Color::Rgb(110, 118, 129);
const HDR: Color = Color::Rgb(240, 246, 252);
const BLU: Color = Color::Rgb(88, 166, 255);
const GRN: Color = Color::Rgb(126, 231, 135);
const YLW: Color = Color::Rgb(210, 153, 34);
const RED: Color = Color::Rgb(218, 54, 51);
const MAG: Color = Color::Rgb(195, 117, 238);
const CYN: Color = Color::Rgb(86, 207, 225);

pub fn render(frame: &mut Frame, app: &App) {
    let r = frame.size();
    if r.width < 60 || r.height < 10 {
        return;
    }

    // ── HEADER ──
    let hdr = format!(
        " mneme v0.2.0  {}  {} mems  {} sessions  [Tab]Graph [e]Ent [t]Temp [/]Search [q]Quit",
        app.project,
        app.total_mems,
        app.sessions.len()
    );
    frame.render_widget(
        Paragraph::new(Span::styled(&hdr, Style::default().fg(HDR).bg(BG2)))
            .style(Style::default().bg(BG2)),
        Rect::new(0, 0, r.width, 1),
    );

    let body = Rect::new(0, 1, r.width, r.height.saturating_sub(2));

    match app.active_panel {
        3 | 4 | 5 => render_view_panel(frame, app, body),
        _ => render_main(frame, app, body),
    }

    // ── STATUS BAR ──
    let status = status_line(app);
    frame.render_widget(
        Paragraph::new(Span::styled(&status, Style::default().fg(DIM).bg(BG2)))
            .style(Style::default().bg(BG2)),
        Rect::new(0, r.height.saturating_sub(1), r.width, 1),
    );

    // ── HELP OVERLAY ──
    if app.show_help {
        let a = centered(65, 22, r);
        frame.render_widget(Clear, a);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(CYN))
            .style(Style::default().bg(BG))
            .title(" mneme TUI Help ");
        let inner = block.inner(a);
        frame.render_widget(block, a);

        let help_lines = vec![
            Line::from(Span::styled(
                "Navigation",
                Style::default().fg(CYN).add_modifier(Modifier::BOLD),
            )),
            Line::from("  j/k or ↑↓    Navigate list"),
            Line::from("  g           First item    G   Last item"),
            Line::from("  J/K         Page down/up"),
            Line::from("  →           Open detail   ←  Back to list"),
            Line::from(""),
            Line::from(Span::styled(
                "Search & Actions",
                Style::default().fg(CYN).add_modifier(Modifier::BOLD),
            )),
            Line::from("  /           Start search  Enter  Execute search"),
            Line::from("  r           Refresh list  d   Delete selected"),
            Line::from("  Tab         Knowledge graph view"),
            Line::from("  e           Entity graph  t   Temporal view"),
            Line::from("  [] or ←→    Switch detail tabs"),
            Line::from("  z/Z         Scroll detail content"),
            Line::from(""),
            Line::from(Span::styled(
                "Global",
                Style::default().fg(CYN).add_modifier(Modifier::BOLD),
            )),
            Line::from("  ?           Toggle this help"),
            Line::from("  q or Ctrl+C Quit"),
            Line::from(""),
            Line::from(Span::styled("Press ? to close", Style::default().fg(DIM))),
        ];
        frame.render_widget(
            Paragraph::new(help_lines)
                .style(Style::default().bg(BG))
                .block(Block::default()),
            inner,
        );
    }

    // ── SEARCH OVERLAY ──
    if app.active_panel == 2 {
        let a = centered(60, 3, r);
        frame.render_widget(Clear, a);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BDRF))
            .style(Style::default().bg(BG));
        let inner_a = block.inner(a);
        frame.render_widget(block, a);
        let txt = if app.search.is_empty() {
            " Search: _".into()
        } else {
            format!(" Search: {}█", app.search)
        };
        frame.render_widget(Paragraph::new(txt).style(Style::default().fg(TXT)), inner_a);
    }
}

fn render_main(frame: &mut Frame, app: &App, area: Rect) {
    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // ── LEFT: MEMORY LIST ──
    let focus = app.active_panel == 0;
    let b = panel("memories", focus);
    let inr = b.inner(panels[0]);
    frame.render_widget(b, panels[0]);

    if app.memories.is_empty() {
        let msg = if app.search.is_empty() {
            " nothing yet\n / search  r reload"
        } else {
            " no results"
        };
        frame.render_widget(Paragraph::new(msg).style(Style::default().fg(DIM)), inr);
    } else {
        let mut y = inr.y;
        let max = inr.y + inr.height;
        for i in app.scroll.. {
            if y >= max || i >= app.memories.len() {
                break;
            }
            let m = &app.memories[i];
            let sel = i == app.selected;
            let imp = match m.importance {
                crate::store::memory::Importance::Critical => RED,
                crate::store::memory::Importance::High => YLW,
                _ => DIM,
            };
            let ta = type_abbr(&m.memory_type);
            let bg = if sel { BG3 } else { BG };

            // Title line
            let title = if m.title.len() as u16 > panels[0].width.saturating_sub(8) {
                format!(
                    "{}…",
                    &m.title[..(panels[0].width as usize).saturating_sub(9)]
                )
            } else {
                m.title.clone()
            };

            let line = Line::from(vec![
                Span::styled(format!(" {} ", ta), Style::default().fg(BG).bg(imp)),
                Span::styled(
                    format!(" {}", title),
                    Style::default().fg(if sel { HDR } else { TXT }).bg(bg),
                ),
            ]);
            frame.render_widget(
                Paragraph::new(line).style(Style::default().bg(bg)),
                Rect::new(inr.x + 1, y, inr.width.saturating_sub(2), 1),
            );
            y += 1;

            // Preview line (if selected)
            if sel && y < max {
                let pre = m.content.chars().take(40).collect::<String>();
                let tags: Vec<&str> = m.tags.iter().map(|t| t.as_str()).collect();
                let sub = format!("  {}  {}", tags.join(","), pre);
                frame.render_widget(
                    Paragraph::new(Span::styled(&sub, Style::default().fg(DIM).bg(bg)))
                        .style(Style::default().bg(bg)),
                    Rect::new(inr.x + 1, y, inr.width.saturating_sub(2), 1),
                );
                y += 1;
            }
        }
    }

    // ── RIGHT: DETAIL ──
    let dfocus = app.active_panel == 1;
    let db = panel("detail", dfocus);
    let dinr = db.inner(panels[1]);
    frame.render_widget(db, panels[1]);

    if let Some(m) = app.sel() {
        render_detail(frame, app, dinr, m);
    } else {
        frame.render_widget(
            Paragraph::new(" ← select a memory").style(Style::default().fg(DIM)),
            dinr,
        );
    }
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect, m: &crate::store::memory::Memory) {
    if area.width < 10 || area.height < 4 {
        return;
    }

    // Tab bar
    let tabs = [
        DetailTab::Content,
        DetailTab::Structured,
        DetailTab::Entities,
        DetailTab::Temporal,
        DetailTab::Relations,
    ];
    let mut x = area.x + 1;
    let ty = area.y;
    for tab in &tabs {
        let active = *tab == app.detail_tab;
        let label = tab.label();
        let s = if active {
            Style::default()
                .fg(BG)
                .bg(BDRF)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(DIM).bg(BG)
        };
        let w = label.len() as u16 + 2;
        if x + w < area.x + area.width {
            frame.render_widget(
                Paragraph::new(Span::styled(format!(" {} ", label), s)),
                Rect::new(x, ty, w, 1),
            );
            x += w;
        }
    }

    let ca = Rect::new(
        area.x + 1,
        area.y + 2,
        area.width.saturating_sub(2),
        area.height.saturating_sub(3),
    );
    if ca.width < 4 || ca.height < 1 {
        return;
    }

    match app.detail_tab {
        DetailTab::Content => {
            let txt = format!(
                "{}\n\n{}\n\n{}",
                m.title,
                m.content,
                m.learned.as_deref().unwrap_or("")
            );
            let lc = txt.lines().count();
            let p = Paragraph::new(txt.as_str())
                .style(Style::default().fg(TXT))
                .scroll((
                    app.detail_scroll.min(lc.saturating_sub(ca.height as usize)) as u16,
                    0,
                ))
                .wrap(Wrap { trim: false });
            frame.render_widget(p, ca);
        }
        DetailTab::Structured => {
            let mut ls = vec![];
            if let Some(ref w) = m.what {
                ls.push(format!("  what  {}", w));
            }
            if let Some(ref w) = m.why {
                ls.push(format!("  why   {}", w));
            }
            if let Some(ref c) = m.context {
                ls.push(format!("  ctx   {}", c));
            }
            if let Some(ref l) = m.learned {
                ls.push(format!("  lrn   {}", l));
            }
            ls.push(String::new());
            ls.push(format!(
                "  type: {}  imp: {}  scope: {}",
                m.memory_type, m.importance, m.scope
            ));
            ls.push(format!(
                "  access: {}  rev: {}  dup: {}",
                m.access_count, m.revision_count, m.duplicate_count
            ));
            ls.push(format!("  tags: {}", m.tags.join(", ")));
            ls.push(format!(
                "  created: {}",
                m.created_at.format("%Y-%m-%d %H:%M")
            ));
            ls.push(format!(
                "  updated: {}",
                m.updated_at.format("%Y-%m-%d %H:%M")
            ));
            frame.render_widget(
                Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(TXT)),
                ca,
            );
        }
        DetailTab::Entities => {
            if let Ok(es) = &app.db.entities().get_memory_entities(m.id) {
                if es.is_empty() {
                    frame.render_widget(
                        Paragraph::new("  none").style(Style::default().fg(DIM)),
                        ca,
                    );
                    return;
                }
                let ls: Vec<String> = es
                    .iter()
                    .map(|e| {
                        format!(
                            "  {}  ({})  {:.2}",
                            e.entity_name, e.entity_type, e.confidence
                        )
                    })
                    .collect();
                frame.render_widget(
                    Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(CYN)),
                    ca,
                );
            }
        }
        DetailTab::Temporal => {
            let vf = m
                .valid_from
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "—".into());
            let vu = m
                .valid_until
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "—".into());
            let pv = m.provenance.as_deref().unwrap_or("—");
            frame.render_widget(
                Paragraph::new(Text::from(format!(
                    "  from: {}\n  until: {}\n  prov: {}",
                    vf, vu, pv
                )))
                .style(Style::default().fg(TXT)),
                ca,
            );
        }
        DetailTab::Relations => {
            if let Ok(data) = app.db.memories().get_graph(&app.project) {
                let es: Vec<String> = data
                    .edges
                    .iter()
                    .filter(|e| e.source == m.id.to_string() || e.target == m.id.to_string())
                    .map(|e| {
                        let other = if e.source == m.id.to_string() {
                            &e.target
                        } else {
                            &e.source
                        };
                        let dir = if e.source == m.id.to_string() {
                            "→"
                        } else {
                            "←"
                        };
                        format!(
                            "  {} {} {} ({:.2})",
                            other, dir, e.relation_type, e.confidence
                        )
                    })
                    .collect();
                if es.is_empty() {
                    frame.render_widget(
                        Paragraph::new("  none").style(Style::default().fg(DIM)),
                        ca,
                    );
                    return;
                }
                frame.render_widget(
                    Paragraph::new(Text::from(es.join("\n"))).style(Style::default().fg(MAG)),
                    ca,
                );
            }
        }
    }
}

fn render_view_panel(frame: &mut Frame, app: &App, area: Rect) {
    match app.active_panel {
        3 => {
            // GRAPH
            let b = panel("knowledge graph", true);
            let inr = b.inner(area);
            frame.render_widget(b, area);
            if let Some(ref d) = app.graph {
                let mut ls = vec![format!(
                    "  nodes: {}  edges: {}  selected: {}",
                    d.nodes.len(),
                    d.edges.len(),
                    app.graph_sel + 1
                )];
                for (i, n) in d.nodes.iter().enumerate() {
                    let m = if i == app.graph_sel { "→" } else { " " };
                    ls.push(format!(
                        "  {} [{}] {} ({})",
                        m, n.importance, n.title, n.memory_type
                    ));
                }
                frame.render_widget(
                    Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(TXT)),
                    inr,
                );
            }
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " [j/k] nav  [Tab] back",
                    Style::default().fg(DIM).bg(BG2),
                ))
                .style(Style::default().bg(BG2)),
                Rect::new(0, area.y + area.height.saturating_sub(1), area.width, 1),
            );
        }
        4 => {
            // ENTITY
            let b = panel("entity graph", true);
            let inr = b.inner(area);
            frame.render_widget(b, area);
            if let Some(ref d) = app.entity_data {
                let ls: Vec<String> = d
                    .iter()
                    .map(|(n, _, c)| format!("  {}  ({})", n, c))
                    .collect();
                frame.render_widget(
                    Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(CYN)),
                    inr,
                );
            }
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] back  [r] refresh",
                    Style::default().fg(DIM).bg(BG2),
                ))
                .style(Style::default().bg(BG2)),
                Rect::new(0, area.y + area.height.saturating_sub(1), area.width, 1),
            );
        }
        5 => {
            // TEMPORAL
            let b = panel("temporal view", true);
            let inr = b.inner(area);
            frame.render_widget(b, area);
            if let Some(ref td) = app.temporal_data {
                let mode = match td.1 {
                    0 => "all",
                    1 => "valid now",
                    2 => "expired",
                    _ => "",
                };
                let total = td.0.len();
                let valid =
                    td.0.iter()
                        .filter(|m| {
                            let n = chrono::Utc::now();
                            match (m.valid_from, m.valid_until) {
                                (Some(vf), Some(vu)) => vf <= n && n < vu,
                                (Some(vf), None) => vf <= n,
                                (None, Some(vu)) => n < vu,
                                (None, None) => true,
                            }
                        })
                        .count();
                let exp =
                    td.0.iter()
                        .filter(|m| {
                            m.valid_until
                                .map(|vu| vu <= chrono::Utc::now())
                                .unwrap_or(false)
                        })
                        .count();
                let mut ls = vec![format!(
                    "  {}  total: {}  valid: {}  expired: {}",
                    mode, total, valid, exp
                )];
                for m in &td.0 {
                    let vf = m
                        .valid_from
                        .map(|d| d.format("%m-%d").to_string())
                        .unwrap_or_else(|| "--".into());
                    let vu = m
                        .valid_until
                        .map(|d| d.format("%m-%d").to_string())
                        .unwrap_or_else(|| "--".into());
                    ls.push(format!("  {} [{} → {}]", m.title, vf, vu));
                }
                frame.render_widget(
                    Paragraph::new(Text::from(ls.join("\n"))).style(Style::default().fg(YLW)),
                    inr,
                );
            }
            frame.render_widget(
                Paragraph::new(Span::styled(
                    " [Tab] back  [m] cycle  [r] refresh",
                    Style::default().fg(DIM).bg(BG2),
                ))
                .style(Style::default().bg(BG2)),
                Rect::new(0, area.y + area.height.saturating_sub(1), area.width, 1),
            );
        }
        _ => {}
    }
}

fn status_line(app: &App) -> String {
    if let Some(ref msg) = app.status_msg {
        return msg.clone();
    }
    match app.active_panel {
        0 => {
            if let Some(m) = app.sel() {
                format!(" {}·{} · {} acc  [↑↓] nav  [→] detail  []] tab  [][] tabs  [z/Z] scroll  [d] del  [/] search  [r] reload",
                m.memory_type, m.importance, m.access_count)
            } else {
                " [↑↓] nav  [/] search  [r] reload".into()
            }
        }
        1 => " [←] back to list  []] tab  [z/Z] scroll  [Esc] close".into(),
        2 => " type to search · enter to confirm · esc to cancel".into(),
        3 => " [j/k] navigate nodes  [Tab] back".into(),
        4 => " [Tab] back  [r] refresh".into(),
        5 => " [Tab] back  [m] cycle display mode  [r] refresh".into(),
        _ => String::new(),
    }
}

fn panel(title: &str, focus: bool) -> Block<'static> {
    Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if focus { BDRF } else { BDR }))
        .style(Style::default().bg(BG))
}

fn type_abbr(mt: &crate::store::memory::MemoryType) -> &'static str {
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

fn centered(w: u16, h: u16, r: Rect) -> Rect {
    Rect::new(
        r.x + r.width.saturating_sub(w) / 2,
        r.y + r.height.saturating_sub(h) / 2,
        w.min(r.width),
        h.min(r.height),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn center_works() {
        let c = centered(60, 20, Rect::new(0, 0, 100, 50));
        assert_eq!(c.x, 20);
        assert_eq!(c.y, 15);
    }
    #[test]
    fn abbrs() {
        assert_eq!(
            type_abbr(&crate::store::memory::MemoryType::Architecture),
            "ARCH"
        );
        assert_eq!(
            type_abbr(&crate::store::memory::MemoryType::Decision),
            "DEC"
        );
    }
    #[test]
    fn tabs() {
        let mut t = DetailTab::Content;
        t = t.next();
        assert_eq!(t, DetailTab::Structured);
        t = t.next();
        assert_eq!(t, DetailTab::Entities);
        t = t.next();
        assert_eq!(t, DetailTab::Temporal);
        t = t.next();
        assert_eq!(t, DetailTab::Relations);
        t = t.next();
        assert_eq!(t, DetailTab::Content);
    }
}
