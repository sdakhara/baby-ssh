use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::tui::theme;

pub fn render(f: &mut Frame, name: &str, host: &str) {
    let area = centered_rect(60, 50, f.area());
    f.render_widget(Clear, area);
    let lines = vec![
        Line::from("Delete Connection?"),
        Line::from(""),
        Line::from(name.to_string()),
        Line::from(host.to_string()),
        Line::from(""),
        Line::from("This will remove the saved connection and its stored credentials."),
        Line::from("The private key file itself will not be touched."),
        Line::from(""),
        Line::from("[Y] Yes    [N] No"),
    ];
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Confirm Delete")
        .border_style(Style::default().fg(theme::DANGER));
    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
