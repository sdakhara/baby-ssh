use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use futures_util::StreamExt;
use russh::client::Msg;
use russh::{Channel, ChannelMsg};
use tokio::io::{stdout, AsyncWriteExt};

/// Hands the terminal over to a live SSH shell channel: keystrokes and pastes are forwarded to
/// the remote PTY as raw bytes, remote output is written straight to stdout, and local resize
/// events are relayed with `window_change`. Returns once the remote channel closes.
///
/// This borrows the same `EventStream` the outer TUI loop uses, so there is only ever one
/// reader of stdin for the whole app — no separate blocking thread, no race between this and
/// ratatui's own input handling once control returns to the connection list.
pub async fn run_interactive_session(events: &mut EventStream, mut channel: Channel<Msg>) {
    let mut out = stdout();
    loop {
        tokio::select! {
            ev = events.next() => {
                match ev {
                    Some(Ok(Event::Key(key))) => {
                        if key.kind == KeyEventKind::Release {
                            continue;
                        }
                        if let Some(bytes) = key_event_to_bytes(key) {
                            if channel.data_bytes(bytes).await.is_err() {
                                break;
                            }
                        }
                    }
                    Some(Ok(Event::Paste(text))) => {
                        if channel.data_bytes(text.into_bytes()).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Event::Resize(cols, rows))) => {
                        let _ = channel.window_change(cols as u32, rows as u32, 0, 0).await;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        if out.write_all(&data).await.is_err() {
                            break;
                        }
                        let _ = out.flush().await;
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        if out.write_all(&data).await.is_err() {
                            break;
                        }
                        let _ = out.flush().await;
                    }
                    Some(ChannelMsg::Eof) => {}
                    Some(ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }
    let _ = channel.eof().await;
}

/// Converts a decoded key event back into the raw bytes a real terminal would have sent, so the
/// remote shell (vim, nano, htop, tab completion, Ctrl+C/D/Z, ...) sees exactly what it expects.
fn key_event_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    let mut bytes: Vec<u8> = match key.code {
        KeyCode::Char(c) => {
            if ctrl {
                control_char_bytes(c)
            } else {
                let mut buf = [0u8; 4];
                c.encode_utf8(&mut buf).as_bytes().to_vec()
            }
        }
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => vec![0x7f],
        KeyCode::Tab => vec![b'\t'],
        KeyCode::BackTab => b"\x1b[Z".to_vec(),
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => b"\x1b[A".to_vec(),
        KeyCode::Down => b"\x1b[B".to_vec(),
        KeyCode::Right => b"\x1b[C".to_vec(),
        KeyCode::Left => b"\x1b[D".to_vec(),
        KeyCode::Home => b"\x1b[H".to_vec(),
        KeyCode::End => b"\x1b[F".to_vec(),
        KeyCode::PageUp => b"\x1b[5~".to_vec(),
        KeyCode::PageDown => b"\x1b[6~".to_vec(),
        KeyCode::Delete => b"\x1b[3~".to_vec(),
        KeyCode::Insert => b"\x1b[2~".to_vec(),
        KeyCode::F(n) => function_key_bytes(n),
        _ => return None,
    };

    if bytes.is_empty() {
        return None;
    }

    if alt {
        let mut prefixed = vec![0x1b];
        prefixed.append(&mut bytes);
        bytes = prefixed;
    }

    Some(bytes)
}

fn control_char_bytes(c: char) -> Vec<u8> {
    let upper = c.to_ascii_uppercase();
    if upper.is_ascii_alphabetic() {
        return vec![(upper as u8) & 0x1f];
    }
    match c {
        '@' => vec![0x00],
        '[' => vec![0x1b],
        '\\' => vec![0x1c],
        ']' => vec![0x1d],
        '^' => vec![0x1e],
        '_' => vec![0x1f],
        '?' => vec![0x7f],
        _ => c.to_string().into_bytes(),
    }
}

fn function_key_bytes(n: u8) -> Vec<u8> {
    match n {
        1 => b"\x1bOP".to_vec(),
        2 => b"\x1bOQ".to_vec(),
        3 => b"\x1bOR".to_vec(),
        4 => b"\x1bOS".to_vec(),
        5 => b"\x1b[15~".to_vec(),
        6 => b"\x1b[17~".to_vec(),
        7 => b"\x1b[18~".to_vec(),
        8 => b"\x1b[19~".to_vec(),
        9 => b"\x1b[20~".to_vec(),
        10 => b"\x1b[21~".to_vec(),
        11 => b"\x1b[23~".to_vec(),
        12 => b"\x1b[24~".to_vec(),
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEventState;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn plain_char_is_utf8_passthrough() {
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(b"a".to_vec())
        );
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('\u{e9}'), KeyModifiers::NONE)),
            Some("\u{e9}".as_bytes().to_vec())
        );
    }

    #[test]
    fn ctrl_letters_map_to_control_codes() {
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![0x03])
        );
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('d'), KeyModifiers::CONTROL)),
            Some(vec![0x04])
        );
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('z'), KeyModifiers::CONTROL)),
            Some(vec![0x1a])
        );
    }

    #[test]
    fn enter_backspace_tab_and_esc() {
        assert_eq!(key_event_to_bytes(key(KeyCode::Enter, KeyModifiers::NONE)), Some(vec![b'\r']));
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(vec![0x7f])
        );
        assert_eq!(key_event_to_bytes(key(KeyCode::Tab, KeyModifiers::NONE)), Some(vec![b'\t']));
        assert_eq!(key_event_to_bytes(key(KeyCode::Esc, KeyModifiers::NONE)), Some(vec![0x1b]));
    }

    #[test]
    fn arrow_keys_are_ansi_escape_sequences() {
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Up, KeyModifiers::NONE)),
            Some(b"\x1b[A".to_vec())
        );
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Down, KeyModifiers::NONE)),
            Some(b"\x1b[B".to_vec())
        );
    }

    #[test]
    fn alt_prefixes_escape_byte() {
        assert_eq!(
            key_event_to_bytes(key(KeyCode::Char('a'), KeyModifiers::ALT)),
            Some(vec![0x1b, b'a'])
        );
    }

    #[test]
    fn unsupported_keys_produce_nothing() {
        assert_eq!(key_event_to_bytes(key(KeyCode::CapsLock, KeyModifiers::NONE)), None);
    }
}
