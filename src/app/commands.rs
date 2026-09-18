use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::credentials::CredentialStore;
use crate::models::{AuthType, Connection};
use crate::ssh::{self, SessionEvent};
use crate::storage::ConnectionStore;

use super::state::{
    App, ConnectingState, ErrorState, FormField, FormMode, FormState, PendingSession, Screen,
};

enum ScreenTag {
    Connections,
    Form,
    DeleteConfirm,
    Connecting,
    Error,
}

/// Top-level dispatcher for terminal input. Any error surfaced here is shown to the user via
/// the Error screen rather than crashing the app.
pub fn handle_terminal_event(app: &mut App, event: Event) {
    let Event::Key(key) = event else { return };
    if key.kind == KeyEventKind::Release {
        return;
    }

    let tag = match &app.screen {
        Screen::Connections => ScreenTag::Connections,
        Screen::Form(_) => ScreenTag::Form,
        Screen::DeleteConfirm { .. } => ScreenTag::DeleteConfirm,
        Screen::Connecting(_) => ScreenTag::Connecting,
        Screen::Error(_) => ScreenTag::Error,
    };

    let result = match tag {
        ScreenTag::Connections => handle_key_connections(app, key),
        ScreenTag::Form => handle_key_form(app, key),
        ScreenTag::DeleteConfirm => {
            handle_key_delete_confirm(app, key);
            Ok(())
        }
        ScreenTag::Connecting => {
            handle_key_connecting(app, key);
            Ok(())
        }
        ScreenTag::Error => {
            app.screen = Screen::Connections;
            Ok(())
        }
    };

    if let Err(e) = result {
        app.screen = Screen::Error(ErrorState {
            summary: "An unexpected error occurred.".to_string(),
            reasons: Vec::new(),
            detail: format!("{e:#}"),
        });
    }
}

pub fn handle_session_event(app: &mut App, event: SessionEvent) {
    let result = (|| -> Result<()> {
        match event {
            SessionEvent::Status(s) => {
                if let Screen::Connecting(state) = &mut app.screen {
                    state.status = s;
                }
            }
            SessionEvent::HostPrompt(req) => {
                if let Screen::Connecting(state) = &mut app.screen {
                    state.host_prompt = Some(req);
                }
            }
            SessionEvent::Connected {
                channel,
                handle,
                connection,
            } => {
                app.connections.touch_last_connected(connection.id)?;
                app.pending_session = Some(PendingSession {
                    channel,
                    handle,
                    connection,
                });
            }
            SessionEvent::Failed {
                summary,
                reasons,
                detail,
            } => {
                app.screen = Screen::Error(ErrorState {
                    summary,
                    reasons,
                    detail,
                });
            }
        }
        Ok(())
    })();

    if let Err(e) = result {
        app.screen = Screen::Error(ErrorState {
            summary: "An unexpected error occurred.".to_string(),
            reasons: Vec::new(),
            detail: format!("{e:#}"),
        });
    }
}

/// Called by `main` once the interactive SSH passthrough loop returns control.
pub fn finish_session(app: &mut App) {
    app.screen = Screen::Connections;
    clamp_selection(app);
}

fn move_selection(app: &mut App, delta: i32) {
    let len = app.visible_connections().len();
    if len == 0 {
        app.list_state.select(None);
        return;
    }
    let current = app.list_state.selected().unwrap_or(0) as i32;
    let next = (current + delta).rem_euclid(len as i32);
    app.list_state.select(Some(next as usize));
}

fn clamp_selection(app: &mut App) {
    let len = app.visible_connections().len();
    if len == 0 {
        app.list_state.select(None);
    } else {
        let current = app.list_state.selected().unwrap_or(0);
        if current >= len {
            app.list_state.select(Some(len - 1));
        } else if app.list_state.selected().is_none() {
            app.list_state.select(Some(0));
        }
    }
}

fn handle_key_connections(app: &mut App, key: KeyEvent) -> Result<()> {
    if let Some(search) = &mut app.search {
        match key.code {
            KeyCode::Esc => app.search = None,
            KeyCode::Enter => {}
            KeyCode::Backspace => {
                search.pop();
            }
            KeyCode::Char(c) => search.push(c),
            _ => {}
        }
        clamp_selection(app);
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => app.should_quit = true,
        KeyCode::Up => move_selection(app, -1),
        KeyCode::Down => move_selection(app, 1),
        KeyCode::Enter => start_connect_selected(app)?,
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.screen = Screen::Form(FormState::new_create());
        }
        KeyCode::Char('e') | KeyCode::Char('E') => open_edit_form(app),
        KeyCode::Char('d') | KeyCode::Char('D') => open_delete_confirm(app),
        KeyCode::Char('f') | KeyCode::Char('F') => toggle_favorite(app)?,
        KeyCode::Char('/') => app.search = Some(String::new()),
        KeyCode::Char('r') | KeyCode::Char('R') => refresh(app)?,
        _ => {}
    }
    Ok(())
}

fn open_edit_form(app: &mut App) {
    if let Some(id) = app.selected_connection_id() {
        if let Some(c) = app.connections.get(id) {
            app.screen = Screen::Form(FormState::new_edit(c));
        }
    }
}

fn open_delete_confirm(app: &mut App) {
    if let Some(id) = app.selected_connection_id() {
        if let Some(c) = app.connections.get(id) {
            app.screen = Screen::DeleteConfirm {
                id,
                name: c.name.clone(),
                host: c.host.clone(),
            };
        }
    }
}

fn toggle_favorite(app: &mut App) -> Result<()> {
    if let Some(id) = app.selected_connection_id() {
        app.connections.toggle_favorite(id)?;
    }
    Ok(())
}

fn refresh(app: &mut App) -> Result<()> {
    app.connections = ConnectionStore::load()?;
    clamp_selection(app);
    Ok(())
}

fn handle_key_delete_confirm(app: &mut App, key: KeyEvent) {
    let Screen::DeleteConfirm { id, .. } = &app.screen else {
        return;
    };
    let id = *id;
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Err(e) = app.connections.delete(id) {
                tracing::warn!("failed to delete connection: {e:#}");
            }
            if let Err(e) = CredentialStore::delete_all(id) {
                tracing::warn!("failed to delete stored credentials: {e:#}");
            }
            app.screen = Screen::Connections;
            clamp_selection(app);
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.screen = Screen::Connections;
        }
        _ => {}
    }
}

fn handle_key_connecting(app: &mut App, key: KeyEvent) {
    let Screen::Connecting(state) = &mut app.screen else {
        return;
    };

    if state.host_prompt.is_some() {
        let answer = match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => Some(true),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Some(false),
            _ => None,
        };
        if let Some(trust) = answer {
            if let Some(prompt) = state.host_prompt.take() {
                let _ = prompt.reply.send(trust);
            }
        }
        return;
    }

    if key.code == KeyCode::Esc {
        state.join.abort();
        app.screen = Screen::Connections;
    }
}

enum FormAction {
    None,
    Cancel,
    Submit,
}

fn handle_key_form(app: &mut App, key: KeyEvent) -> Result<()> {
    let mut action = FormAction::None;

    if let Screen::Form(form) = &mut app.screen {
        match key.code {
            KeyCode::Esc => action = FormAction::Cancel,
            KeyCode::Tab | KeyCode::Down => advance_focus(form, 1),
            KeyCode::BackTab | KeyCode::Up => advance_focus(form, -1),
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                if form.focus == FormField::AuthType =>
            {
                form.auth_type = form.auth_type.toggled();
            }
            KeyCode::Enter if form.focus == FormField::AuthType => {
                form.auth_type = form.auth_type.toggled();
            }
            KeyCode::Enter => {
                let order = FormField::order(form.auth_type);
                if order.last() == Some(&form.focus) {
                    action = FormAction::Submit;
                } else {
                    advance_focus(form, 1);
                }
            }
            KeyCode::Backspace if form.focus != FormField::AuthType => {
                let focus = form.focus;
                if let Some(text) = form.field_text_mut(focus) {
                    text.pop();
                }
            }
            KeyCode::Char(c)
                if form.focus != FormField::AuthType
                    && !key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let focus = form.focus;
                if let Some(text) = form.field_text_mut(focus) {
                    text.push(c);
                }
            }
            _ => {}
        }
    }

    match action {
        FormAction::Cancel => app.screen = Screen::Connections,
        FormAction::Submit => submit_form(app)?,
        FormAction::None => {}
    }
    Ok(())
}

fn advance_focus(form: &mut FormState, delta: i32) {
    let order = FormField::order(form.auth_type);
    let current_idx = order.iter().position(|f| *f == form.focus).unwrap_or(0) as i32;
    let len = order.len() as i32;
    let next = (current_idx + delta).rem_euclid(len);

    if form.focus == FormField::PrivateKeyPath && !form.private_key_path.trim().is_empty() {
        let path = std::path::Path::new(form.private_key_path.trim());
        form.passphrase_hint = if path.exists() {
            Some(ssh::authentication::key_requires_passphrase(path))
        } else {
            None
        };
    }

    form.focus = order[next as usize];
}

fn set_form_error(app: &mut App, message: impl Into<String>) {
    if let Screen::Form(form) = &mut app.screen {
        form.error = Some(message.into());
    }
}

fn submit_form(app: &mut App) -> Result<()> {
    let Screen::Form(form) = &app.screen else {
        return Ok(());
    };

    let name = form.name.trim().to_string();
    let host = form.host.trim().to_string();
    let username = form.username.trim().to_string();
    let port_raw = form.port.trim().to_string();
    let auth_type = form.auth_type;
    let password = form.password.clone();
    let private_key_path_raw = form.private_key_path.trim().to_string();
    let passphrase = form.passphrase.clone();
    let description_raw = form.description.trim().to_string();
    let tags_raw = form.tags.clone();
    let id = form.id;
    let created_at = form.created_at;
    let favorite = form.favorite;
    let last_connected_at = form.last_connected_at;
    let mode = form.mode;

    if name.is_empty() || host.is_empty() || username.is_empty() {
        set_form_error(app, "Name, host, and username are required.");
        return Ok(());
    }

    let port: u16 = if port_raw.is_empty() {
        22
    } else {
        match port_raw.parse() {
            Ok(p) => p,
            Err(_) => {
                set_form_error(app, "Port must be a number between 1 and 65535.");
                return Ok(());
            }
        }
    };

    let mut private_key_path: Option<PathBuf> = None;
    if auth_type == AuthType::PrivateKey {
        if private_key_path_raw.is_empty() {
            set_form_error(
                app,
                "A private key path is required for private-key authentication.",
            );
            return Ok(());
        }
        let p = PathBuf::from(&private_key_path_raw);
        if !p.exists() {
            set_form_error(app, "The selected private key file does not exist.");
            return Ok(());
        }
        private_key_path = Some(p);
    } else if mode == FormMode::New && password.is_empty() {
        set_form_error(app, "A password is required for password authentication.");
        return Ok(());
    }

    let tags: Vec<String> = tags_raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let description = if description_raw.is_empty() {
        None
    } else {
        Some(description_raw)
    };

    // Credentials are persisted before the profile, so we never end up with a saved
    // connection that points at a secret which failed to store.
    if auth_type == AuthType::Password {
        if !password.is_empty() {
            if let Err(e) = CredentialStore::set_password(id, &password) {
                set_form_error(app, format!("Failed to store password: {e:#}"));
                return Ok(());
            }
        }
        let _ = CredentialStore::delete_passphrase(id);
    } else {
        if !passphrase.is_empty() {
            if let Err(e) = CredentialStore::set_passphrase(id, &passphrase) {
                set_form_error(app, format!("Failed to store passphrase: {e:#}"));
                return Ok(());
            }
        }
        let _ = CredentialStore::delete_password(id);
    }

    let connection = Connection {
        id,
        name,
        host,
        port,
        username,
        auth_type,
        private_key_path,
        description,
        tags,
        favorite,
        created_at,
        updated_at: Utc::now(),
        last_connected_at,
    };

    app.connections.upsert(connection)?;
    app.screen = Screen::Connections;
    clamp_selection(app);
    Ok(())
}

fn start_connect_selected(app: &mut App) -> Result<()> {
    let Some(id) = app.selected_connection_id() else {
        return Ok(());
    };
    let Some(connection) = app.connections.get(id).cloned() else {
        return Ok(());
    };

    let secret = match connection.auth_type {
        AuthType::Password => match CredentialStore::get_password(id)? {
            Some(p) => ssh::AuthMaterial::Password(p),
            None => {
                app.screen = Screen::Error(ErrorState {
                    summary: "No saved password found for this connection.".to_string(),
                    reasons: vec!["Edit the connection and re-enter the password.".to_string()],
                    detail: String::new(),
                });
                return Ok(());
            }
        },
        AuthType::PrivateKey => {
            let Some(path) = connection.private_key_path.clone() else {
                app.screen = Screen::Error(ErrorState {
                    summary: "This connection has no private key configured.".to_string(),
                    reasons: vec!["Edit the connection and select a private key file.".to_string()],
                    detail: String::new(),
                });
                return Ok(());
            };
            if !path.exists() {
                app.screen = Screen::Error(ErrorState {
                    summary: "The private key file could not be found.".to_string(),
                    reasons: vec![format!("Expected it at: {}", path.display())],
                    detail: String::new(),
                });
                return Ok(());
            }
            let passphrase = CredentialStore::get_passphrase(id)?;
            ssh::AuthMaterial::PrivateKey { path, passphrase }
        }
    };

    let timeout = Duration::from_secs(app.config.connect_timeout_secs);
    let known_hosts = app.known_hosts.clone();
    let events_tx = app.event_tx.clone();
    let task_connection = connection.clone();
    let join = tokio::spawn(async move {
        ssh::client::run_connect_task(task_connection, secret, known_hosts, timeout, events_tx)
            .await;
    });

    app.screen = Screen::Connecting(ConnectingState {
        connection,
        status: "Connecting...".to_string(),
        host_prompt: None,
        join,
    });
    Ok(())
}
