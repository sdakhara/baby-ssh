use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::ErrorState;
use crate::tui::theme;

pub fn render(f: &mut Frame, state: &ErrorState) {
    let area = f.area();
    let mut lines = vec![Line::from(state.summary.clone()), Line::from("")];
    if !state.reasons.is_empty() {
        lines.push(Line::from("Possible reasons:"));
        for reason in &state.reasons {
            lines.push(Line::from(format!("  - {reason}")));
        }
        lines.push(Line::from(""));
    }
    if !state.detail.is_empty() {
        lines.push(Line::from(format!("Details: {}", state.detail)));
        lines.push(Line::from(""));
    }
    lines.push(Line::from("Press any key to return."));

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Connection Failed")
        .border_style(Style::default().fg(theme::DANGER));
    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
