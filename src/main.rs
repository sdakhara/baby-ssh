mod app;
mod credentials;
mod logging;
mod models;
mod ssh;
mod storage;
mod tui;

use anyhow::Result;
use crossterm::event::EventStream;
use futures_util::StreamExt;

use app::App;

#[tokio::main]
async fn main() -> Result<()> {
    let _log_guard = logging::init()?;
    storage::paths::ensure_dirs()?;

    let connections = storage::ConnectionStore::load()?;
    let known_hosts = storage::KnownHostsStore::load()?;
    let config = storage::config::load()?;
    let mut app = App::new(connections, known_hosts, config);

    let mut terminal = tui::init()?;
    let mut events = EventStream::new();

    let result = run(&mut terminal, &mut app, &mut events).await;

    tui::restore();

    if let Err(e) = result {
        eprintln!("baby-ssh exited with an error: {e:#}");
        std::process::exit(1);
    }
    Ok(())
}

async fn run(terminal: &mut tui::Tui, app: &mut App, events: &mut EventStream) -> Result<()> {
    loop {
        terminal.draw(|f| tui::draw(f, app))?;

        tokio::select! {
            maybe_event = events.next() => {
                match maybe_event {
                    Some(Ok(event)) => app::commands::handle_terminal_event(app, event),
                    Some(Err(e)) => {
                        tracing::warn!("terminal event stream error: {e}");
                    }
                    None => app.should_quit = true,
                }
            }
            Some(session_event) = app.event_rx.recv() => {
                app::commands::handle_session_event(app, session_event);
            }
        }

        if let Some(session) = app.pending_session.take() {
            // Held until the interactive session ends; russh's background connection driver
            // keeps running independently of this, but dropping the handle early is needless.
            let _handle_keepalive = session.handle;
            terminal.clear()?;
            tracing::info!(host = %session.connection.host, port = session.connection.port, "SSH session started");
            ssh::pty::run_interactive_session(events, session.channel).await;
            tracing::info!(host = %session.connection.host, port = session.connection.port, "SSH session ended");
            app::commands::finish_session(app);
            terminal.clear()?;
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
