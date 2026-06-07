use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        canvas::{Canvas, Line as CLine},
        Block, Borders, Clear, List, ListItem, Paragraph, Wrap,
    },
    Frame,
};

use crate::store::memory::{Importance, MemoryType};
use crate::tui::app::{App, AppMode};
use crate::tui::graph::{layout_nodes, truncate_title};

/// Renderiza la interfaz completa en el frame.
pub fn render(frame: &mut Frame, app: &App) {
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(frame.size());

    render_header(frame, app, main_layout[0]);

    match app.mode {
        AppMode::Graph => {
            render_graph(frame, app, main_layout[1]);
            render_statusbar(frame, app, main_layout[2]);
        }
        _ => {
            let body_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
                .split(main_layout[1]);

            render_memory_list(frame, app, body_layout[0]);
            render_detail(frame, app, body_layout[1]);

            match app.mode {
                AppMode::Searching => {
                    let area = centered_rect(60, 3, frame.size());
                    frame.render_widget(Clear, area);
                    render_search_bar(frame, app, area);
                }
                AppMode::Confirming { ref action, .. } => {
                    let area = centered_rect(50, 5, frame.size());
                    frame.render_widget(Clear, area);
                    render_confirm_overlay(frame, action, area);
                }
                AppMode::Help => {
                    let area = centered_rect(70, 22, frame.size());
                    frame.render_widget(Clear, area);
                    render_help_overlay(frame, area);
                }
                AppMode::Normal => {
                    render_statusbar(frame, app, main_layout[2]);
                }
                AppMode::Graph => {} // handled above
            }
        }
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let version = env!("CARGO_PKG_VERSION");
    let header_text = format!(
        "mneme v{} │ Proyecto: {} │ [Q]uit [/]Search [Tab]Grafo [?]Help",
        version, app.project
    );
    let header = Paragraph::new(header_text.as_str()).style(
        Style::default()
            .fg(Color::White)
            .bg(Color::Blue)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(header, area);
}

fn render_memory_list(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" MEMORIAS ");
    let inner = block.inner(area);
    let visible_height = inner.height as usize;

    let items: Vec<ListItem> = app
        .memories
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let is_selected = i == app.selected;
            let encrypted_icon = if m.is_encrypted { "🔒 " } else { "   " };
            let type_abbr = memory_type_abbrev(&m.memory_type);
            let title = if m.title.len() > 22 {
                format!("{}...", &m.title[..22])
            } else {
                m.title.clone()
            };
            let imp_color = importance_color(&m.importance);
            let cursor = if is_selected { ">" } else { " " };

            let line = Line::from(vec![
                Span::raw(format!("{} ", cursor)),
                Span::styled(encrypted_icon, Style::default().fg(Color::Magenta)),
                Span::styled("●", Style::default().fg(imp_color)),
                Span::raw(format!(" [{}] ", type_abbr)),
                Span::styled(
                    title,
                    if is_selected {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let display_items: Vec<ListItem> = items
        .into_iter()
        .skip(app.scroll_offset)
        .take(visible_height)
        .collect();

    let list = List::new(display_items).block(block);
    frame.render_widget(list, area);
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" DETALLE ");
    if let Some(memory) = app.selected_memory() {
        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Título: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&memory.title),
            ]),
            Line::from(vec![
                Span::styled("Tipo: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(memory.memory_type.to_string()),
                Span::raw("  Importancia: "),
                Span::styled(
                    memory.importance.to_string(),
                    Style::default().fg(importance_color(&memory.importance)),
                ),
            ]),
            Line::from(vec![
                Span::styled("Proyecto: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&memory.project),
            ]),
        ];

        if !memory.tags.is_empty() {
            let tags_str = memory
                .tags
                .iter()
                .map(|t| format!("[{}]", t))
                .collect::<Vec<_>>()
                .join(" ");
            lines.push(Line::from(vec![
                Span::styled("Tags: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(tags_str, Style::default().fg(Color::Cyan)),
            ]));
        }

        lines.push(Line::from(""));

        if memory.is_encrypted {
            lines.push(Line::from(vec![Span::styled(
                "🔒 [ENCRIPTADO]",
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from("Esta memoria está protegida con age/SSH."));
        } else {
            lines.push(Line::from(vec![Span::styled(
                "── Contenido ──",
                Style::default().add_modifier(Modifier::DIM),
            )]));
            lines.push(Line::from(memory.content.as_str()));

            if let Some(what) = &memory.what {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("What: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(what.as_str()),
                ]));
            }
            if let Some(why) = &memory.why {
                lines.push(Line::from(vec![
                    Span::styled("Why: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(why.as_str()),
                ]));
            }
            if let Some(ctx) = &memory.context {
                lines.push(Line::from(vec![
                    Span::styled("Context: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(ctx.as_str()),
                ]));
            }
            if let Some(learned) = &memory.learned {
                lines.push(Line::from(vec![
                    Span::styled("Learned: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(learned.as_str()),
                ]));
            }
        }

        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        frame.render_widget(para, area);
    } else {
        let para = Paragraph::new("Seleccioná una memoria con ↑↓").block(block);
        frame.render_widget(para, area);
    }
}

/// Renderiza la vista de grafo interactivo usando Canvas.
fn render_graph(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" GRAFO ");

    let Some(ref data) = app.graph_data else {
        let para = Paragraph::new("No hay relaciones entre memorias")
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(para, area);
        return;
    };

    if data.nodes.is_empty() {
        let para = Paragraph::new("No hay relaciones entre memorias")
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(para, area);
        return;
    }

    let positions = layout_nodes(data.nodes.len());
    let selected_idx = app.graph_selected;

    // Construir índice id→position para edges
    let id_to_pos: std::collections::HashMap<&str, (f64, f64)> = data
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), positions[i]))
        .collect();

    let canvas = Canvas::default()
        .block(block)
        .x_bounds([0.0, 100.0])
        .y_bounds([0.0, 100.0])
        .paint(|ctx| {
            // Dibujar aristas
            for edge in &data.edges {
                let Some(&(x1, y1)) = id_to_pos.get(edge.source.as_str()) else {
                    continue;
                };
                let Some(&(x2, y2)) = id_to_pos.get(edge.target.as_str()) else {
                    continue;
                };
                let color = edge_color(edge.confidence);
                ctx.draw(&CLine {
                    x1,
                    y1,
                    x2,
                    y2,
                    color,
                });

                // Label de relación en el punto medio
                let mx = (x1 + x2) / 2.0;
                let my = (y1 + y2) / 2.0;
                ctx.print(
                    mx,
                    my,
                    Line::styled(
                        truncate_title(&edge.relation_type, 8),
                        Style::default().fg(color).add_modifier(Modifier::DIM),
                    ),
                );
            }

            // Dibujar nodos
            for (i, node) in data.nodes.iter().enumerate() {
                let (x, y) = positions[i];
                let is_selected = i == selected_idx;
                let color = node_color(&node.importance);
                let label = truncate_title(&node.title, 12);

                if is_selected {
                    ctx.print(
                        x - 7.0,
                        y,
                        Line::styled(
                            format!("[{}]", label),
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                    );
                } else {
                    ctx.print(
                        x - 6.0,
                        y,
                        Line::styled(format!(" {} ", label), Style::default().fg(color)),
                    );
                }
            }
        });

    frame.render_widget(canvas, area);
}

fn render_statusbar(frame: &mut Frame, app: &App, area: Rect) {
    let text = match app.mode {
        AppMode::Graph => {
            "[Tab/Esc] Volver  [j/k] Seleccionar nodo  [r] Recargar  [Q] Salir".to_string()
        }
        _ => {
            let base = "[↑↓] Navegar  [Tab] Grafo  [/] Buscar  [d] Eliminar  [Q] Salir";
            if let Some(msg) = &app.status_message {
                format!("{} │ {}", msg, base)
            } else {
                base.to_string()
            }
        }
    };
    let para =
        Paragraph::new(text.as_str()).style(Style::default().fg(Color::White).bg(Color::DarkGray));
    frame.render_widget(para, area);
}

fn render_search_bar(frame: &mut Frame, app: &App, area: Rect) {
    let text = format!("Buscar: {}", app.search_query);
    let block = Block::default().borders(Borders::ALL).title(" Búsqueda ");
    let para = Paragraph::new(text.as_str()).block(block);
    frame.render_widget(para, area);

    let x = area.x + 8 + app.search_query.len() as u16;
    let y = area.y + 1;
    frame.set_cursor(x, y);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("Atajos de teclado"),
        Line::from(""),
        Line::from("↑ / k      Mover selección arriba"),
        Line::from("↓ / j      Mover selección abajo"),
        Line::from("PgUp       Página arriba"),
        Line::from("PgDn       Página abajo"),
        Line::from("g          Ir al primero"),
        Line::from("G          Ir al último"),
        Line::from("/          Activar búsqueda"),
        Line::from("Enter      Confirmar búsqueda"),
        Line::from("Esc        Cancelar búsqueda / cerrar ayuda"),
        Line::from("r          Refrescar memorias"),
        Line::from("d          Eliminar memoria seleccionada"),
        Line::from("Tab        Abrir/cerrar grafo de relaciones"),
        Line::from("?          Mostrar/ocultar ayuda"),
        Line::from("q / Q      Salir"),
        Line::from(""),
        Line::from("En vista de grafo:"),
        Line::from("j/k        Seleccionar nodo"),
        Line::from("r          Recargar grafo"),
        Line::from("Tab/Esc    Volver a lista"),
    ];
    let block = Block::default().borders(Borders::ALL).title(" Ayuda ");
    let para = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });
    frame.render_widget(para, area);
}

fn render_confirm_overlay(frame: &mut Frame, action: &str, area: Rect) {
    let msg = format!("¿Confirmar {}? (y/n)", action);
    let block = Block::default().borders(Borders::ALL).title(" Confirmar ");
    let para = Paragraph::new(msg.as_str())
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(para, area);
}

fn memory_type_abbrev(mt: &MemoryType) -> &'static str {
    match mt {
        MemoryType::Architecture => "ARCH",
        MemoryType::Decision => "DEC",
        MemoryType::Bugfix => "BUG",
        MemoryType::Pattern => "PAT",
        MemoryType::Convention => "CON",
        MemoryType::Dependency => "DEP",
        MemoryType::Workflow => "WRK",
        MemoryType::Note => "NOTE",
        MemoryType::Config => "CFG",
        MemoryType::Discovery => "DIS",
        MemoryType::Learning => "LRN",
    }
}

fn importance_color(imp: &Importance) -> Color {
    match imp {
        Importance::Critical => Color::Red,
        Importance::High => Color::Yellow,
        Importance::Medium => Color::Green,
        Importance::Low => Color::DarkGray,
    }
}

/// Devuelve el color del nodo según el string de importancia del grafo.
fn node_color(importance: &str) -> Color {
    match importance {
        "critical" => Color::Red,
        "high" => Color::Yellow,
        "medium" => Color::Green,
        _ => Color::DarkGray,
    }
}

/// Devuelve el color de la arista según la confianza.
fn edge_color(confidence: f32) -> Color {
    if confidence >= 0.8 {
        Color::Green
    } else if confidence >= 0.5 {
        Color::Yellow
    } else {
        Color::DarkGray
    }
}

/// Crea un rectángulo centrado de porcentajes dados respecto al área base.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
