# baby-ssh

A lightweight, keyboard-driven terminal SSH client for Windows. Save connection profiles once,
then connect with a single `Enter` — no more re-typing `ssh -i C:\Users\...\id_ed25519 -p 2222 user@host`.

## Features

- Create / edit / delete / search / favorite connection profiles
- Password and private-key authentication (with or without a passphrase), detected automatically
- Secrets stored in the **Windows Credential Manager** — never written to disk in plaintext
- Real interactive SSH shell: PTY, ANSI/colors, resize, Ctrl+C/D/Z, tab completion, `vim`/`nano`/`top`/`sudo`, etc.
- SSH host-key verification with a persistent known-hosts store, and a blocking warning if a
  host key ever changes
- Friendly error screens (with technical detail available) instead of raw error codes
- File-based logging at `%APPDATA%\baby-ssh\logs\baby-ssh.log` that never logs secrets

## Installation

### Option 1: Download the release (recommended)

1. Download the latest `baby-ssh-*-windows-x86_64.zip` from the
   [Releases page](https://github.com/sdakhara/baby-ssh/releases).
2. Unzip it anywhere.
3. Run `baby-ssh.exe` directly — it's fully self-contained, no installer or extra dependencies.

To be able to just type `baby-ssh` from any terminal, copy it somewhere permanent and add that
folder to your PATH:

```powershell
$installDir = "$env:LOCALAPPDATA\Programs\baby-ssh"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item ".\baby-ssh.exe" -Destination "$installDir\baby-ssh.exe" -Force

$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
$parts = $userPath -split ';' | Where-Object { $_ -ne "" }
if ($parts -notcontains $installDir) {
    [Environment]::SetEnvironmentVariable("PATH", (($parts + $installDir) -join ';'), "User")
}
```

Open a **new** terminal window afterward — PATH changes don't apply to already-open ones — then
just run:

```powershell
baby-ssh
```

### Option 2: Build from source

Requires the Rust toolchain ([rustup.rs](https://rustup.rs)).

```powershell
git clone https://github.com/sdakhara/baby-ssh.git
cd baby-ssh
cargo run                    # run it directly, for development
cargo build --release        # or produce .\target\release\baby-ssh.exe
```

## Keyboard controls

| Key | Action |
| --- | --- |
| `↑` / `↓` | Move selection |
| `Enter` | Connect to the selected profile |
| `N` | New connection |
| `E` | Edit the selected connection |
| `D` | Delete (with confirmation) |
| `F` | Toggle favorite |
| `/` | Search (matches name, host, username, description, tags) |
| `R` | Refresh the list from disk |
| `Q` | Quit |
| `Esc` | Back / cancel |

Inside a connection form: `Tab`/`Shift+Tab` (or `↑`/`↓`) move between fields, `Enter` moves to
the next field (or saves from the last one), and the Authentication field toggles between
Password and Private Key with `Enter`/`Space`/arrow keys.

Once connected, every keystroke goes straight to the remote shell — the same as any normal SSH
client. `Esc` only cancels while a connection attempt is still in progress.

## Where things are stored

```
%APPDATA%\baby-ssh\
├── config\config.json        Non-sensitive app settings
├── connections\connections.json   Connection profiles (no secrets)
├── known_hosts                Trusted SSH host keys
└── logs\baby-ssh.log          Rotationless log file (metadata only, never secrets)
```

Passwords and key passphrases live in the Windows Credential Manager under the `baby-ssh:password`
and `baby-ssh:passphrase` services, keyed by the connection's id — never in the JSON files above.

## Architecture

```
src/
├── app/          Application state machine, commands, validation
├── tui/          ratatui screens and rendering
├── ssh/          russh-based client, PTY passthrough, host-key verification
├── credentials/  OS credential store (Windows Credential Manager today, via the `keyring` crate)
├── storage/      JSON connection/config/known-hosts persistence
└── models/       Connection and settings data types
```

One deliberate deviation from the original spec's suggested file layout: there's no
`credentials/windows.rs` / `linux.rs` / `macos.rs`, because the `keyring` crate already does that
OS dispatch internally — those files would have been empty pass-throughs.

## Tech stack

Rust, `ratatui` + `crossterm` for the TUI, `russh` for a pure-Rust SSH implementation (no OpenSSL
dependency to manage on Windows), `keyring` for OS-backed credential storage, `serde`/`serde_json`
for the on-disk profile format, and `tracing` for logging.
