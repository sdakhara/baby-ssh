use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::tui::theme;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(3),
        ])
        .split(area);

    render_search(f, app, chunks[0]);
    render_list(f, app, chunks[1]);
    render_help(f, chunks[2]);
}

fn render_search(f: &mut Frame, app: &App, area: Rect) {
    let (line, border_color) = match &app.search {
        Some(q) => (
            Line::from(vec![
                Span::styled("Search: ", Style::default().fg(theme::LABEL)),
                Span::styled(q.clone(), Style::default().fg(theme::TEXT)),
            ]),
            theme::ACCENT,
        ),
        None => (
            Line::from(Span::styled(
                "Search: (press / to search)",
                Style::default().fg(theme::MUTED),
            )),
            theme::BORDER,
        ),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            " baby-ssh ",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(Paragraph::new(line).block(block), area);
}

fn render_list(f: &mut Frame, app: &App, area: Rect) {
    let items = app.visible_connections();
    let list_items: Vec<ListItem> = if items.is_empty() {
        vec![ListItem::new(Span::styled(
            "No saved connections yet. Press N to create one.",
            Style::default().fg(theme::MUTED),
        ))]
    } else {
        items
            .iter()
            .map(|c| {
                let star = if c.favorite { "\u{2605} " } else { "  " };
                let star_style = if c.favorite {
                    Style::default().fg(theme::FAVORITE)
                } else {
                    Style::default()
                };
                let line = Line::from(vec![
                    Span::styled(star, star_style),
                    Span::styled(
                        format!("{:<24}", c.name),
                        Style::default().fg(theme::TEXT).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        format!("{}:{}", c.host, c.port),
                        Style::default().fg(theme::ACCENT),
                    ),
                    Span::styled(
                        format!("  {}", c.username),
                        Style::default().fg(theme::LABEL),
                    ),
                ]);
                ListItem::new(line)
            })
            .collect()
    };

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme::BORDER))
                .title(Span::styled(
                    " SSH Connections ",
                    Style::default().fg(theme::LABEL),
                )),
        )
        .highlight_style(
            Style::default()
                .bg(theme::SELECTED_BG)
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let mut state = app.list_state.clone();
    f.render_stateful_widget(list, area, &mut state);
}

fn render_help(f: &mut Frame, area: Rect) {
    let pairs = [
        ("Enter", "Connect"),
        ("N", "New"),
        ("E", "Edit"),
        ("D", "Delete"),
        ("F", "Favorite"),
        ("/", "Search"),
        ("R", "Refresh"),
        ("Q", "Quit"),
    ];
    let mut spans = Vec::with_capacity(pairs.len() * 3);
    for (key, label) in pairs {
        spans.push(Span::styled(
            key,
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}  "),
            Style::default().fg(theme::MUTED),
        ));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::BORDER));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}
