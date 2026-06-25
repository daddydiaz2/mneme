use crate::store::memory::MemoryType;
use crate::tui::app::{App, Screen};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

const BG: Color = Color::Rgb(13, 17, 23);
const BG2: Color = Color::Rgb(22, 27, 34);
const TXT: Color = Color::Rgb(201, 209, 217);
const DIM: Color = Color::Rgb(110, 118, 129);
const HDR: Color = Color::Rgb(240, 246, 252);
const BLU: Color = Color::Rgb(88, 166, 255);
const GRN: Color = Color::Rgb(126, 231, 135);
const YLW: Color = Color::Rgb(210, 153, 34);
const RED: Color = Color::Rgb(218, 54, 51);

pub fn render(frame: &mut Frame, app: &App) {
    let r = frame.size();
    let hdr = format!(
        " mneme v{}  {} projects  {} memories  [q]uit  [/]search  [?]help",
        env!("CARGO_PKG_VERSION"),
        app.projects.len(),
        app.total_mems
    );
    frame.render_widget(
        Paragraph::new(Span::styled(&hdr, Style::default().fg(HDR).bg(BG2)))
            .style(Style::default().bg(BG2)),
        Rect::new(0, 0, r.width, 1),
    );

    let body = Rect::new(0, 1, r.width, r.height.saturating_sub(2));

    match app.screen {
        Screen::Help => render_help(frame, body),
        Screen::Projects => render_projects(frame, app, body),
        Screen::Memories => render_memories(frame, app, body),
        Screen::Detail => render_detail(frame, app, body),
        Screen::Sessions => render_sessions(frame, app, body),
    }
    render_status(frame, app, r);
    render_search(frame, app, r);
}

fn render_status(frame: &mut Frame, app: &App, r: Rect) {
    let status = match app.screen {
        Screen::Projects => " [↑↓] nav  [Enter] open  [/] search  [s] sessions  [q] quit".into(),
        Screen::Memories => {
            if app.searching {
                " type to search · enter to confirm · esc to cancel".into()
            } else {
                format!(
                    " [↑↓] nav  [Enter] detail  [/] search  [d] delete  [Esc] back  [q] quit  {}",
                    app.msg
                )
            }
        }
        Screen::Detail => " [↑↓/j/k] scroll  [Esc] back  [d] delete".into(),
        Screen::Sessions => " [Esc] back".into(),
        Screen::Help => " [q] quit".into(),
    };
    frame.render_widget(
        Paragraph::new(Span::styled(status, Style::default().fg(DIM).bg(BG2)))
            .style(Style::default().bg(BG2)),
        Rect::new(0, r.height.saturating_sub(1), r.width, 1),
    );
}

fn render_search(frame: &mut Frame, app: &App, r: Rect) {
    if app.searching {
        let a = centered(50, 3, r);
        frame.render_widget(Clear, a);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BLU))
            .style(Style::default().bg(BG));
        frame.render_widget(block, a);
        let inner = Rect::new(a.x + 1, a.y + 1, a.width.saturating_sub(2), 1);
        frame.render_widget(
            Paragraph::new(Span::styled(
                format!("> {}", app.search),
                Style::default().fg(TXT),
            )),
            inner,
        );
    }
}

fn render_projects(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .projects
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == app.proj_sel {
                Style::default().bg(BG2).fg(BLU)
            } else {
                Style::default().fg(TXT)
            };
            let last = p
                .last_activity
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default();
            ListItem::new(format!(
                " {:<25} {:>3} mems  {:>3} sessions  {}",
                p.name, p.memory_count, p.session_count, last
            ))
            .style(style)
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("📁 Projects")),
        area,
    );
}

fn render_memories(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .memories
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == app.mem_sel {
                Style::default().bg(BG2).fg(BLU)
            } else {
                Style::default().fg(TXT)
            };
            let t = if m.title.len() > 40 {
                format!("{}...", &m.title[..37])
            } else {
                m.title.clone()
            };
            let mt = format!("{:?}", m.memory_type).to_lowercase();
            let snippet = if let Some(pos) = m.content.find('\n') {
                format!("{}...", &m.content[..pos.min(50)])
            } else if m.content.len() > 50 {
                format!("{}...", &m.content[..47])
            } else {
                m.content.clone()
            };
            ListItem::new(format!(" [{}] {:<40}  {}", mt, t, snippet)).style(style)
        })
        .collect();

    let title = if app.searching {
        format!("🔍 Search")
    } else {
        format!("📝 {} ({})", app.project, app.memories.len())
    };
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title(title.as_str())),
        area,
    );
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(ref m) = app.detail {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(5), Constraint::Min(1)])
            .split(area);

        let lines = vec![
            Line::from(Span::styled(
                &m.title,
                Style::default().fg(HDR).add_modifier(Modifier::BOLD),
            )),
            Line::from(format!(
                "  Type: {:?}  |  Importance: {:?}  |  Project: {}",
                m.memory_type, m.importance, m.project
            )),
            Line::from(format!(
                "  Created: {}  |  ID: {}",
                m.created_at.format("%Y-%m-%d %H:%M"),
                &m.id.to_string()[..8]
            )),
            Line::from(format!("  Tags: {}", m.tags.join(", "))),
            if let Some(ref why) = m.why {
                if !why.is_empty() {
                    Line::from(format!("  Why: {}", why))
                } else {
                    Line::from("")
                }
            } else {
                Line::from("")
            },
        ];
        frame.render_widget(
            Paragraph::new(lines).block(Block::default().borders(Borders::ALL)),
            chunks[0],
        );

        let content: Vec<Line> = m
            .content
            .lines()
            .skip(app.detail_scroll)
            .take(30)
            .map(|l| {
                if l.starts_with("**What**")
                    || l.starts_with("**Why**")
                    || l.starts_with("**Where**")
                    || l.starts_with("**Learned**")
                {
                    Line::from(Span::styled(
                        l,
                        Style::default().fg(YLW).add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(l)
                }
            })
            .collect();
        frame.render_widget(
            Paragraph::new(content)
                .block(Block::default().borders(Borders::ALL).title("Content"))
                .wrap(Wrap { trim: false }),
            chunks[1],
        );
    }
}

fn render_sessions(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .sessions
        .iter()
        .map(|s| {
            ListItem::new(format!(
                " {}  |  {}",
                s.started_at.format("%Y-%m-%d %H:%M"),
                s.summary.as_deref().unwrap_or("(no summary)")
            ))
        })
        .collect();
    frame.render_widget(
        List::new(items).block(Block::default().borders(Borders::ALL).title("📋 Sessions")),
        area,
    );
}

fn render_help(frame: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(Span::styled(
            "mneme TUI Help",
            Style::default().fg(BLU).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Navigation",
            Style::default().fg(GRN).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ↑↓/jk     Navigate lists"),
        Line::from("  Enter     Open project / memory detail"),
        Line::from("  Esc       Go back"),
        Line::from(""),
        Line::from(Span::styled(
            "Search & Actions",
            Style::default().fg(GRN).add_modifier(Modifier::BOLD),
        )),
        Line::from("  /         Search across all projects"),
        Line::from("  d         Delete selected memory"),
        Line::from("  s         View sessions for project"),
        Line::from(""),
        Line::from(Span::styled(
            "Global",
            Style::default().fg(GRN).add_modifier(Modifier::BOLD),
        )),
        Line::from("  q         Quit"),
        Line::from("  ?         This help"),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .style(Style::default().bg(BG));
    frame.render_widget(Paragraph::new(text).block(block), centered(50, 16, area));
}

fn centered(w: u16, h: u16, r: Rect) -> Rect {
    Rect::new(
        r.x + r.width.saturating_sub(w) / 2,
        r.y + r.height.saturating_sub(h) / 2,
        w.min(r.width),
        h.min(r.height),
    )
}
