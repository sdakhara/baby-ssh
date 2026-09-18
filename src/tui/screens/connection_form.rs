use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::app::state::{FormField, FormMode, FormState};
use crate::models::AuthType;
use crate::tui::theme;

pub fn render(f: &mut Frame, form: &FormState) {
    let area = f.area();
    let title = match form.mode {
        FormMode::New => " Create Connection ",
        FormMode::Edit => " Edit Connection ",
    };

    let mut rows: Vec<(FormField, &str, &str, bool)> = vec![
        (FormField::Name, "Name", form.name.as_str(), false),
        (FormField::Host, "Host", form.host.as_str(), false),
        (FormField::Port, "Port (default 22)", form.port.as_str(), false),
        (FormField::Username, "Username", form.username.as_str(), false),
    ];
    let auth_row_idx = rows.len();
    rows.push((FormField::AuthType, "", "", false)); // placeholder row, rendered specially below
    match form.auth_type {
        AuthType::Password => {
            rows.push((FormField::Password, "Password", form.password.as_str(), true));
        }
        AuthType::PrivateKey => {
            rows.push((
                FormField::PrivateKeyPath,
                "Private Key Path",
                form.private_key_path.as_str(),
                false,
            ));
            rows.push((FormField::Passphrase, "Passphrase", form.passphrase.as_str(), true));
        }
    }
    rows.push((FormField::Description, "Description", form.description.as_str(), false));
    rows.push((FormField::Tags, "Tags (comma separated)", form.tags.as_str(), false));

    let mut constraints: Vec<Constraint> = rows.iter().map(|_| Constraint::Length(3)).collect();
    constraints.push(Constraint::Length(2));
    constraints.push(Constraint::Min(1));

    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .title(Span::styled(
            title,
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        ));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let mut cursor: Option<(u16, u16)> = None;

    for (i, (field, label, value, mask)) in rows.iter().enumerate() {
        if i == auth_row_idx {
            render_auth_type_row(f, chunks[i], form);
            continue;
        }
        let focused = form.focus == *field;
        if let Some(pos) = render_text_row(f, chunks[i], label, value, *mask, focused) {
            cursor = Some(pos);
        }
    }

    render_hint(f, chunks[rows.len()], form);
    render_help(f, chunks[rows.len() + 1]);

    if let Some(pos) = cursor {
        f.set_cursor_position(pos);
    }
}

/// Renders one `Label: value` row and returns the terminal cursor position (end of the value)
/// when this row is focused, so the caller can show a real blinking cursor there.
fn render_text_row(
    f: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    mask: bool,
    focused: bool,
) -> Option<(u16, u16)> {
    let border_style = if focused {
        Style::default().fg(theme::ACCENT)
    } else {
        Style::default().fg(theme::BORDER)
    };
    let block = Block::default().borders(Borders::ALL).border_style(border_style);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let display_value: String = if mask {
        "*".repeat(value.chars().count())
    } else {
        value.to_string()
    };
    let label_style = if focused {
        Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::LABEL)
    };

    let line = Line::from(vec![
        Span::styled(format!("{label}: "), label_style),
        Span::styled(display_value.clone(), Style::default().fg(theme::TEXT)),
    ]);
    f.render_widget(Paragraph::new(line), inner);

    if focused && inner.width > 0 {
        let offset = label.chars().count() as u16 + 2 + display_value.chars().count() as u16;
        let max_offset = inner.width.saturating_sub(1);
        Some((inner.x + offset.min(max_offset), inner.y))
    } else {
        None
    }
}

fn render_auth_type_row(f: &mut Frame, area: Rect, form: &FormState) {
    let focused = form.focus == FormField::AuthType;
    let outer_style = if focused {
        Style::default().fg(theme::ACCENT)
    } else {
        Style::default().fg(theme::BORDER)
    };
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(outer_style)
        .title(Span::styled(" Authentication ", Style::default().fg(theme::LABEL)));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    render_auth_button(
        f,
        cols[0],
        AuthType::Password.label(),
        form.auth_type == AuthType::Password,
    );
    render_auth_button(
        f,
        cols[1],
        AuthType::PrivateKey.label(),
        form.auth_type == AuthType::PrivateKey,
    );
}

fn render_auth_button(f: &mut Frame, area: Rect, label: &str, selected: bool) {
    let (style, border_style) = if selected {
        (
            Style::default()
                .fg(theme::BUTTON_ACTIVE_FG)
                .bg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
            Style::default().fg(theme::ACCENT),
        )
    } else {
        (
            Style::default().fg(theme::MUTED),
            Style::default().fg(theme::BORDER),
        )
    };
    let marker = if selected { "\u{25cf} " } else { "\u{25cb} " };
    let block = Block::default().borders(Borders::ALL).border_style(border_style);
    let p = Paragraph::new(format!("{marker}{label}"))
        .style(style)
        .alignment(Alignment::Center)
        .block(block);
    f.render_widget(p, area);
}

fn render_hint(f: &mut Frame, area: Rect, form: &FormState) {
    let line = if let Some(err) = &form.error {
        Line::from(Span::styled(err.clone(), Style::default().fg(theme::DANGER)))
    } else if form.focus == FormField::PrivateKeyPath {
        match form.passphrase_hint {
            Some(true) => Line::from(Span::styled(
                "This key appears to require a passphrase.",
                Style::default().fg(theme::MUTED),
            )),
            Some(false) => Line::from(Span::styled(
                "This key does not require a passphrase.",
                Style::default().fg(theme::MUTED),
            )),
            None => Line::from(""),
        }
    } else if form.focus == FormField::AuthType {
        Line::from(Span::styled(
            "\u{2190}/\u{2192} or Enter to switch",
            Style::default().fg(theme::MUTED),
        ))
    } else if form.mode == FormMode::Edit {
        Line::from(Span::styled(
            "Leave Password/Passphrase blank to keep the existing saved value.",
            Style::default().fg(theme::MUTED),
        ))
    } else {
        Line::from("")
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_help(f: &mut Frame, area: Rect) {
    let pairs = [("Tab/Shift+Tab", "Move"), ("Enter", "Next/Save"), ("Esc", "Cancel")];
    let mut spans = Vec::with_capacity(pairs.len() * 2);
    for (key, label) in pairs {
        spans.push(Span::styled(
            key,
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {label}   "),
            Style::default().fg(theme::MUTED),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
