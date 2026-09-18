use std::sync::{Arc, Mutex as StdMutex};

use chrono::{DateTime, Utc};
use ratatui::widgets::ListState;
use russh::client::{self, Msg};
use russh::Channel;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use uuid::Uuid;

use crate::models::{AppConfig, AuthType, Connection};
use crate::ssh::{ClientHandler, HostPromptRequest, SessionEvent};
use crate::storage::{ConnectionStore, KnownHostsStore};

pub enum Screen {
    Connections,
    Form(FormState),
    DeleteConfirm { id: Uuid, name: String, host: String },
    Connecting(ConnectingState),
    Error(ErrorState),
}

pub struct ConnectingState {
    pub connection: Connection,
    pub status: String,
    pub host_prompt: Option<HostPromptRequest>,
    pub join: JoinHandle<()>,
}

pub struct ErrorState {
    pub summary: String,
    pub reasons: Vec<String>,
    pub detail: String,
}

/// A fully authenticated channel with a shell already requested, waiting to be handed off to
/// the interactive passthrough loop. Held on `App` rather than run inline in `handle_session_event`
/// because running it needs mutable access to the crossterm `EventStream` that only `main`'s
/// loop owns.
pub struct PendingSession {
    pub channel: Channel<Msg>,
    pub handle: client::Handle<ClientHandler>,
    pub connection: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    New,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormField {
    Name,
    Host,
    Port,
    Username,
    AuthType,
    Password,
    PrivateKeyPath,
    Passphrase,
    Description,
    Tags,
}

impl FormField {
    /// The tab order for the currently selected authentication type.
    pub fn order(auth_type: AuthType) -> Vec<FormField> {
        let mut fields = vec![
            FormField::Name,
            FormField::Host,
            FormField::Port,
            FormField::Username,
            FormField::AuthType,
        ];
        match auth_type {
            AuthType::Password => fields.push(FormField::Password),
            AuthType::PrivateKey => {
                fields.push(FormField::PrivateKeyPath);
                fields.push(FormField::Passphrase);
            }
        }
        fields.push(FormField::Description);
        fields.push(FormField::Tags);
        fields
    }
}

pub struct FormState {
    pub mode: FormMode,
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub favorite: bool,
    pub last_connected_at: Option<DateTime<Utc>>,
    pub auth_type: AuthType,
    pub focus: FormField,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub private_key_path: String,
    pub passphrase: String,
    pub description: String,
    pub tags: String,
    pub passphrase_hint: Option<bool>,
    pub error: Option<String>,
}

impl FormState {
    pub fn new_create() -> Self {
        Self {
            mode: FormMode::New,
            id: Uuid::new_v4(),
            created_at: Utc::now(),
            favorite: false,
            last_connected_at: None,
            auth_type: AuthType::Password,
            focus: FormField::Name,
            name: String::new(),
            host: String::new(),
            port: String::new(),
            username: String::new(),
            password: String::new(),
            private_key_path: String::new(),
            passphrase: String::new(),
            description: String::new(),
            tags: String::new(),
            passphrase_hint: None,
            error: None,
        }
    }

    pub fn new_edit(c: &Connection) -> Self {
        Self {
            mode: FormMode::Edit,
            id: c.id,
            created_at: c.created_at,
            favorite: c.favorite,
            last_connected_at: c.last_connected_at,
            auth_type: c.auth_type,
            focus: FormField::Name,
            name: c.name.clone(),
            host: c.host.clone(),
            port: c.port.to_string(),
            username: c.username.clone(),
            password: String::new(),
            private_key_path: c
                .private_key_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_default(),
            passphrase: String::new(),
            description: c.description.clone().unwrap_or_default(),
            tags: c.tags.join(", "),
            passphrase_hint: None,
            error: None,
        }
    }

    pub fn field_text_mut(&mut self, field: FormField) -> Option<&mut String> {
        match field {
            FormField::Name => Some(&mut self.name),
            FormField::Host => Some(&mut self.host),
            FormField::Port => Some(&mut self.port),
            FormField::Username => Some(&mut self.username),
            FormField::Password => Some(&mut self.password),
            FormField::PrivateKeyPath => Some(&mut self.private_key_path),
            FormField::Passphrase => Some(&mut self.passphrase),
            FormField::Description => Some(&mut self.description),
            FormField::Tags => Some(&mut self.tags),
            FormField::AuthType => None,
        }
    }
}

pub struct App {
    pub connections: ConnectionStore,
    pub known_hosts: Arc<StdMutex<KnownHostsStore>>,
    pub config: AppConfig,
    pub screen: Screen,
    pub list_state: ListState,
    pub search: Option<String>,
    pub should_quit: bool,
    pub pending_session: Option<PendingSession>,
    pub event_tx: UnboundedSender<SessionEvent>,
    pub event_rx: UnboundedReceiver<SessionEvent>,
}

impl App {
    pub fn new(
        connections: ConnectionStore,
        known_hosts: KnownHostsStore,
        config: AppConfig,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let mut list_state = ListState::default();
        if !connections.all().is_empty() {
            list_state.select(Some(0));
        }
        Self {
            connections,
            known_hosts: Arc::new(StdMutex::new(known_hosts)),
            config,
            screen: Screen::Connections,
            list_state,
            search: None,
            should_quit: false,
            pending_session: None,
            event_tx,
            event_rx,
        }
    }

    pub fn visible_connections(&self) -> Vec<&Connection> {
        let query = self.search.as_deref().unwrap_or("");
        self.connections.search(query)
    }

    pub fn selected_connection_id(&self) -> Option<Uuid> {
        let items = self.visible_connections();
        self.list_state
            .selected()
            .and_then(|i| items.get(i))
            .map(|c| c.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_form_order_excludes_key_fields() {
        let order = FormField::order(AuthType::Password);
        assert!(order.contains(&FormField::Password));
        assert!(!order.contains(&FormField::PrivateKeyPath));
        assert!(!order.contains(&FormField::Passphrase));
    }

    #[test]
    fn private_key_form_order_excludes_password() {
        let order = FormField::order(AuthType::PrivateKey);
        assert!(order.contains(&FormField::PrivateKeyPath));
        assert!(order.contains(&FormField::Passphrase));
        assert!(!order.contains(&FormField::Password));
    }

    #[test]
    fn form_order_has_no_duplicates() {
        for auth_type in [AuthType::Password, AuthType::PrivateKey] {
            let order = FormField::order(auth_type);
            let unique: std::collections::HashSet<_> = order.iter().collect();
            assert_eq!(order.len(), unique.len());
        }
    }

    #[test]
    fn edit_form_prefills_from_connection_but_not_secrets() {
        let c = Connection {
            id: Uuid::new_v4(),
            name: "home".to_string(),
            host: "10.0.0.5".to_string(),
            port: 2222,
            username: "sujal".to_string(),
            auth_type: AuthType::PrivateKey,
            private_key_path: Some(std::path::PathBuf::from("C:\\keys\\id_ed25519")),
            description: None,
            tags: vec![],
            favorite: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            last_connected_at: None,
        };
        let form = FormState::new_edit(&c);
        assert_eq!(form.name, "home");
        assert_eq!(form.port, "2222");
        assert_eq!(form.private_key_path, "C:\\keys\\id_ed25519");
        assert!(form.password.is_empty());
        assert!(form.passphrase.is_empty());
    }
}
