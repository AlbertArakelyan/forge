use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::state::app_state::AppState;
use crate::state::collection::CollectionItem;
use crate::state::focus::Focus;
use super::layout::{ACCENT_BLUE, BORDER_INACTIVE};

const TEXT_MUTED: Color = Color::Rgb(86, 95, 137);
const TEXT_PRIMARY: Color = Color::Rgb(192, 202, 245);
const SURFACE: Color = Color::Rgb(36, 40, 59);

// ─── Flat tree model ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum NodeKind {
    Collection { collapsed: bool },
    Folder { collapsed: bool },
    Request { method: String },
}

#[derive(Debug, Clone)]
pub struct SidebarNode {
    pub depth: u16,
    pub kind: NodeKind,
    pub id: String,
    pub label: String,
}

/// Walk the workspace collections and produce a flat ordered list of visible nodes.
/// Collapsed collections/folders hide their children.
/// If `search_query` is non-empty, only nodes whose label contains the query are shown
/// (search ignores collapse state — all matching items are visible).
pub fn flatten_tree(state: &AppState) -> Vec<SidebarNode> {
    let mut out = Vec::new();
    let query = state.sidebar.search_query.to_lowercase();
    let searching = state.sidebar.search_mode && !query.is_empty();

    for col in &state.workspace.collections {
        let collapsed = state.sidebar.collapsed_ids.contains(&col.id);

        if !searching {
            out.push(SidebarNode {
                depth: 0,
                kind: NodeKind::Collection { collapsed },
                id: col.id.clone(),
                label: col.name.clone(),
            });
        }

        let col_match = searching && col.name.to_lowercase().contains(&query);
        if col_match {
            out.push(SidebarNode {
                depth: 0,
                kind: NodeKind::Collection { collapsed: false },
                id: col.id.clone(),
                label: col.name.clone(),
            });
        }

        // Show children if: not searching + not collapsed, OR searching
        if !collapsed || searching {
            push_items(&col.items, 1, &mut out, state, &query, searching);
        }
    }

    out
}

fn push_items(
    items: &[CollectionItem],
    depth: u16,
    out: &mut Vec<SidebarNode>,
    state: &AppState,
    query: &str,
    searching: bool,
) {
    for item in items {
        match item {
            CollectionItem::Folder(f) => {
                let collapsed = state.sidebar.collapsed_ids.contains(&f.id);
                let folder_match = searching && f.name.to_lowercase().contains(query);

                if !searching || folder_match {
                    out.push(SidebarNode {
                        depth,
                        kind: NodeKind::Folder {
                            collapsed: if searching { false } else { collapsed },
                        },
                        id: f.id.clone(),
                        label: f.name.clone(),
                    });
                }

                if !collapsed || searching {
                    push_items(&f.items, depth + 1, out, state, query, searching);
                }
            }
            CollectionItem::Request(r) => {
                if searching && !r.name.to_lowercase().contains(query) {
                    continue;
                }
                out.push(SidebarNode {
                    depth,
                    kind: NodeKind::Request {
                        method: r.method.clone(),
                    },
                    id: r.id.clone(),
                    label: r.name.clone(),
                });
            }
        }
    }
}

fn method_badge_color(method: &str) -> Color {
    match method {
        "GET" => Color::Rgb(115, 218, 202),
        "POST" => Color::Rgb(158, 206, 106),
        "PUT" => Color::Rgb(224, 175, 104),
        "PATCH" => Color::Rgb(187, 154, 247),
        "DELETE" => Color::Rgb(247, 118, 142),
        "HEAD" | "OPTIONS" => Color::Rgb(86, 95, 137),
        _ => Color::White,
    }
}

// ─── Render ──────────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, state: &AppState) {
    let focused = matches!(state.focus, Focus::Sidebar);
    let border_color = if focused { ACCENT_BLUE } else { BORDER_INACTIVE };

    let block = Block::default()
        .title(" forge ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 3 || inner.height < 2 {
        return;
    }

    let nodes = flatten_tree(state);

    // Always reserve the last 1 row for the footer (hints or search bar)
    let (list_area, footer_area) = if inner.height < 3 {
        (inner, None)
    } else {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(inner);
        (chunks[0], Some(chunks[1]))
    };

    // Empty state
    if nodes.is_empty() && !state.sidebar.search_mode {
        let hint = Paragraph::new(Line::from(Span::styled(
            "Ctrl+n: new collection",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::DIM),
        )));
        frame.render_widget(hint, list_area);
    } else if nodes.is_empty() {
        let hint = Paragraph::new(Line::from(Span::styled(
            "No results",
            Style::default().fg(TEXT_MUTED).add_modifier(Modifier::DIM),
        )));
        frame.render_widget(hint, list_area);
    } else {
        let scroll = state.sidebar.scroll_offset;
        let visible_nodes = nodes.iter().skip(scroll);

        for (i, node) in visible_nodes.enumerate() {
            let y = list_area.y + i as u16;
            if y >= list_area.y + list_area.height {
                break;
            }
            let abs_idx = i + scroll;
            let is_cursor = abs_idx == state.sidebar.cursor;
            let row_bg = if is_cursor { SURFACE } else { Color::Reset };
            let row_area = Rect { y, height: 1, ..list_area };

            let indent = "  ".repeat(node.depth as usize);
            let line = match &node.kind {
                NodeKind::Collection { collapsed } => {
                    let arrow = if *collapsed { "▶ " } else { "▼ " };
                    let label_style = if is_cursor {
                        Style::default()
                            .fg(Color::White)
                            .bg(row_bg)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(TEXT_PRIMARY).bg(row_bg).add_modifier(Modifier::BOLD)
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("{}{}", indent, arrow),
                            Style::default().fg(ACCENT_BLUE).bg(row_bg),
                        ),
                        Span::styled(node.label.clone(), label_style),
                    ])
                }
                NodeKind::Folder { collapsed } => {
                    let arrow = if *collapsed { "▶ " } else { "▼ " };
                    let label_style = if is_cursor {
                        Style::default().fg(Color::White).bg(row_bg)
                    } else {
                        Style::default().fg(TEXT_PRIMARY).bg(row_bg)
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("{}{}", indent, arrow),
                            Style::default().fg(TEXT_MUTED).bg(row_bg),
                        ),
                        Span::styled(node.label.clone(), label_style),
                    ])
                }
                NodeKind::Request { method } => {
                    let color = method_badge_color(method);
                    let method_display = format!("{:<6} ", method);
                    let label_style = if is_cursor {
                        Style::default().fg(Color::White).bg(row_bg)
                    } else {
                        Style::default().fg(TEXT_PRIMARY).bg(row_bg)
                    };
                    Line::from(vec![
                        Span::styled(
                            format!("{}  ", indent),
                            Style::default().bg(row_bg),
                        ),
                        Span::styled(
                            method_display,
                            Style::default().fg(color).bg(row_bg).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(node.label.clone(), label_style),
                    ])
                }
            };

            frame.render_widget(Paragraph::new(line), row_area);
        }
    }

    // Footer: search bar when searching, otherwise key hints
    if let Some(fa) = footer_area {
        if state.sidebar.search_mode {
            let search_line = Line::from(vec![
                Span::styled("/ ", Style::default().fg(ACCENT_BLUE)),
                Span::styled(
                    state.sidebar.search_query.clone(),
                    Style::default().fg(TEXT_PRIMARY),
                ),
            ]);
            frame.render_widget(Paragraph::new(search_line), fa);
        } else {
            let hints = Line::from(vec![
                Span::styled("^n", Style::default().fg(ACCENT_BLUE)),
                Span::styled(" col  ", Style::default().fg(TEXT_MUTED)),
                Span::styled("n", Style::default().fg(ACCENT_BLUE)),
                Span::styled(" req  ", Style::default().fg(TEXT_MUTED)),
                Span::styled("d", Style::default().fg(ACCENT_BLUE)),
                Span::styled(" del  ", Style::default().fg(TEXT_MUTED)),
                Span::styled("/", Style::default().fg(ACCENT_BLUE)),
                Span::styled(" search", Style::default().fg(TEXT_MUTED)),
            ]);
            frame.render_widget(
                Paragraph::new(hints).style(Style::default().add_modifier(Modifier::DIM)),
                fa,
            );
        }
    }
}
