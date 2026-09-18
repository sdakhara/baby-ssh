use std::path::PathBuf;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Duration;

use russh::client::{self, Msg};
use russh::keys::{self, HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use russh::Channel;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::oneshot;

use crate::models::{AuthType, Connection};
use crate::storage::KnownHostsStore;

use super::host_keys::{self, Verdict};

/// Why the user is being asked to confirm a host key.
pub enum HostPromptKind {
    New,
    Changed { previous_fingerprint: String },
}

/// Sent from the SSH handshake to the UI when a host key needs a human decision. The UI answers
/// by sending a `bool` (trust or not) through `reply`; the handshake blocks on that answer.
pub struct HostPromptRequest {
    pub host: String,
    pub port: u16,
    pub algorithm: String,
    pub fingerprint: String,
    pub kind: HostPromptKind,
    pub reply: oneshot::Sender<bool>,
}

/// Credential material resolved from the credential store, ready to hand to the SSH layer.
pub enum AuthMaterial {
    Password(String),
    PrivateKey {
        path: PathBuf,
        passphrase: Option<String>,
    },
}

/// Progress and outcome events emitted by a background connect task.
pub enum SessionEvent {
    Status(String),
    HostPrompt(HostPromptRequest),
    Connected {
        channel: Channel<Msg>,
        handle: client::Handle<ClientHandler>,
        connection: Connection,
    },
    Failed {
        summary: String,
        reasons: Vec<String>,
        detail: String,
    },
}

pub struct ClientHandler {
    host: String,
    port: u16,
    known_hosts: Arc<StdMutex<KnownHostsStore>>,
    events: UnboundedSender<SessionEvent>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = server_public_key else {
            tracing::warn!("rejecting certificate-based host key (unsupported in Phase 1)");
            return Ok(false);
        };

        let algorithm = key.algorithm().to_string();
        let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
        let key_openssh = key.to_openssh().unwrap_or_default();

        let verdict = {
            let store = self.known_hosts.lock().unwrap_or_else(|e| e.into_inner());
            host_keys::evaluate(&store, &self.host, self.port, &key_openssh)
        };

        let trust = match verdict {
            Verdict::Trusted => true,
            Verdict::New => {
                self.prompt_and_maybe_trust(&algorithm, &fingerprint, &key_openssh, HostPromptKind::New)
                    .await
            }
            Verdict::Changed { previous_fingerprint } => {
                self.prompt_and_maybe_trust(
                    &algorithm,
                    &fingerprint,
                    &key_openssh,
                    HostPromptKind::Changed { previous_fingerprint },
                )
                .await
            }
        };

        Ok(trust)
    }
}

impl ClientHandler {
    async fn prompt_and_maybe_trust(
        &self,
        algorithm: &str,
        fingerprint: &str,
        key_openssh: &str,
        kind: HostPromptKind,
    ) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        let _ = self.events.send(SessionEvent::HostPrompt(HostPromptRequest {
            host: self.host.clone(),
            port: self.port,
            algorithm: algorithm.to_string(),
            fingerprint: fingerprint.to_string(),
            kind,
            reply: reply_tx,
        }));

        let trust = reply_rx.await.unwrap_or(false);
        if trust {
            let mut store = self.known_hosts.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = store.trust(&self.host, self.port, algorithm, fingerprint, key_openssh) {
                tracing::warn!("failed to persist known host entry: {e:#}");
            }
        }
        trust
    }
}

pub struct ConnectError {
    pub summary: String,
    pub reasons: Vec<String>,
    pub detail: String,
}

impl ConnectError {
    fn simple(summary: &str, reasons: Vec<&str>) -> Self {
        Self {
            summary: summary.to_string(),
            reasons: reasons.into_iter().map(String::from).collect(),
            detail: String::new(),
        }
    }

    fn with_detail(mut self, detail: impl std::fmt::Display) -> Self {
        self.detail = detail.to_string();
        self
    }
}

fn network_reasons() -> Vec<&'static str> {
    vec![
        "Server is offline",
        "SSH service is not running",
        "Port is incorrect",
        "Firewall is blocking the connection",
    ]
}

/// Runs a full connect -> authenticate -> open-shell sequence in the background, reporting
/// progress and the final outcome through `events`. Intended to be spawned with `tokio::spawn`.
pub async fn run_connect_task(
    connection: Connection,
    secret: AuthMaterial,
    known_hosts: Arc<StdMutex<KnownHostsStore>>,
    timeout: Duration,
    events: UnboundedSender<SessionEvent>,
) {
    match do_connect(&connection, secret, known_hosts, timeout, &events).await {
        Ok((channel, handle)) => {
            let _ = events.send(SessionEvent::Connected {
                channel,
                handle,
                connection,
            });
        }
        Err(e) => {
            let _ = events.send(SessionEvent::Failed {
                summary: e.summary,
                reasons: e.reasons,
                detail: e.detail,
            });
        }
    }
}

async fn do_connect(
    connection: &Connection,
    secret: AuthMaterial,
    known_hosts: Arc<StdMutex<KnownHostsStore>>,
    timeout_dur: Duration,
    events: &UnboundedSender<SessionEvent>,
) -> Result<(Channel<Msg>, client::Handle<ClientHandler>), ConnectError> {
    let _ = events.send(SessionEvent::Status(format!(
        "Connecting to {}:{}...",
        connection.host, connection.port
    )));

    let handler = ClientHandler {
        host: connection.host.clone(),
        port: connection.port,
        known_hosts,
        events: events.clone(),
    };
    // nodelay disables Nagle's algorithm. Without it, small interactive packets (a single
    // keystroke is a few bytes) sit in the kernel's send buffer waiting to be batched with more
    // data before going out, which is what makes typing over SSH feel laggy. Real ssh clients
    // always set this for interactive sessions.
    let config = Arc::new(client::Config {
        nodelay: true,
        ..Default::default()
    });
    let addr = (connection.host.as_str(), connection.port);

    let mut handle = tokio::time::timeout(timeout_dur, client::connect(config, addr, handler))
        .await
        .map_err(|_| {
            ConnectError::simple(
                "Connection timed out.",
                vec![
                    "Server is offline or unreachable",
                    "Firewall is blocking the connection",
                    "Host or port is incorrect",
                ],
            )
        })?
        .map_err(|e| {
            ConnectError::simple("Connection failed.", network_reasons())
                .with_detail(format!("{}:{} - {e}", connection.host, connection.port))
        })?;

    let _ = events.send(SessionEvent::Status("Authenticating...".to_string()));

    let auth_result = match secret {
        AuthMaterial::Password(password) => handle
            .authenticate_password(&connection.username, password)
            .await
            .map_err(|e| {
                ConnectError::simple(
                    "Authentication failed.",
                    vec!["Username or password may be incorrect"],
                )
                .with_detail(e)
            })?,
        AuthMaterial::PrivateKey { path, passphrase } => {
            let key = keys::load_secret_key(&path, passphrase.as_deref()).map_err(|e| {
                ConnectError::simple(
                    "Unable to load the private key.",
                    vec![
                        "The passphrase may be incorrect",
                        "The key file may be invalid or corrupted",
                    ],
                )
                .with_detail(e)
            })?;
            let hash_alg = handle
                .best_supported_rsa_hash()
                .await
                .map_err(|e| {
                    ConnectError::simple("Authentication failed.", network_reasons()).with_detail(e)
                })?
                .flatten();
            handle
                .authenticate_publickey(
                    &connection.username,
                    PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
                )
                .await
                .map_err(|e| {
                    ConnectError::simple(
                        "Authentication failed.",
                        vec!["The server may have rejected this key"],
                    )
                    .with_detail(e)
                })?
        }
    };

    if !auth_result.success() {
        return Err(ConnectError::simple(
            "Authentication failed.",
            match connection.auth_type {
                AuthType::Password => vec!["The username or password is incorrect"],
                AuthType::PrivateKey => vec![
                    "The server does not recognize this key",
                    "The username may be incorrect",
                ],
            },
        ));
    }

    let channel = handle.channel_open_session().await.map_err(|e| {
        ConnectError::simple("Failed to open a session channel.", network_reasons()).with_detail(e)
    })?;

    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    let term = std::env::var("TERM").unwrap_or_else(|_| "xterm-256color".to_string());
    channel
        .request_pty(true, &term, cols as u32, rows as u32, 0, 0, &[])
        .await
        .map_err(|e| {
            ConnectError::simple("Failed to allocate a remote terminal.", network_reasons())
                .with_detail(e)
        })?;
    channel.request_shell(true).await.map_err(|e| {
        ConnectError::simple("Failed to start the remote shell.", network_reasons()).with_detail(e)
    })?;

    let _ = events.send(SessionEvent::Status("Connected.".to_string()));

    Ok((channel, handle))
}
