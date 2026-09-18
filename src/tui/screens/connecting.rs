use ratatui::layout::Alignment;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;

use crate::app::state::ConnectingState;
use crate::ssh::{HostPromptKind, HostPromptRequest};
use crate::tui::theme;

pub fn render(f: &mut Frame, state: &ConnectingState) {
    if let Some(prompt) = &state.host_prompt {
        render_host_prompt(f, prompt);
        return;
    }

    let area = f.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            " Connecting ",
            Style::default().fg(theme::ACCENT),
        ));
    let status_color = if state.status == "Connected." {
        theme::SUCCESS
    } else {
        theme::ACCENT
    };
    let lines = vec![
        Line::from(Span::styled(
            format!("Connecting to {}...", state.connection.name),
            Style::default().fg(theme::TEXT),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("Host: ", Style::default().fg(theme::LABEL)),
            Span::styled(state.connection.host.clone(), Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("Port: ", Style::default().fg(theme::LABEL)),
            Span::styled(state.connection.port.to_string(), Style::default().fg(theme::TEXT)),
        ]),
        Line::from(vec![
            Span::styled("User: ", Style::default().fg(theme::LABEL)),
            Span::styled(state.connection.username.clone(), Style::default().fg(theme::TEXT)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            state.status.clone(),
            Style::default().fg(status_color),
        )),
        Line::from(""),
        Line::from(Span::styled("Esc to cancel", Style::default().fg(theme::MUTED))),
    ];
    let p = Paragraph::new(lines)
        .block(block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn render_host_prompt(f: &mut Frame, prompt: &HostPromptRequest) {
    let area = f.area();
    let (title, accent) = match &prompt.kind {
        HostPromptKind::New => ("Unknown SSH Host", theme::ACCENT),
        HostPromptKind::Changed { .. } => ("WARNING: Host Key Changed", theme::DANGER),
    };

    let mut lines = vec![
        Line::from(format!("Host: {}", prompt.host)),
        Line::from(format!("Port: {}", prompt.port)),
        Line::from(format!("Algorithm: {}", prompt.algorithm)),
        Line::from(format!("Fingerprint: {}", prompt.fingerprint)),
        Line::from(""),
    ];

    match &prompt.kind {
        HostPromptKind::New => {
            lines.push(Line::from("This host has not been seen before."));
            lines.push(Line::from(""));
            lines.push(Line::from("[Y] Trust    [N] Cancel"));
        }
        HostPromptKind::Changed { previous_fingerprint } => {
            lines.push(Line::from(format!("Previously known: {previous_fingerprint}")));
            lines.push(Line::from(format!("Currently received: {}", prompt.fingerprint)));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "This can indicate a server reinstallation, configuration change, or a potential security issue.",
                Style::default().fg(theme::DANGER),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from("[Y] Accept New Key    [N] Cancel"));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(accent));
    let p = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
    f.render_widget(p, area);
}
