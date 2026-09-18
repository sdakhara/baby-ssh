use anyhow::{Context, Result};
use keyring::Entry;
use uuid::Uuid;

/// Credential-store service names. Kept separate per secret kind so deleting a connection's
/// password never touches an unrelated passphrase entry for the same id.
const PASSWORD_SERVICE: &str = "baby-ssh:password";
const PASSPHRASE_SERVICE: &str = "baby-ssh:passphrase";

/// Thin wrapper around the OS-backed secure credential store (Windows Credential Manager,
/// macOS Keychain, or the Linux Secret Service, chosen automatically by the `keyring` crate).
/// baby-ssh's connection database only ever holds metadata; every secret lives here, keyed by
/// the connection's id.
pub struct CredentialStore;

impl CredentialStore {
    pub fn set_password(id: Uuid, password: &str) -> Result<()> {
        set(PASSWORD_SERVICE, id, password)
    }

    pub fn get_password(id: Uuid) -> Result<Option<String>> {
        get(PASSWORD_SERVICE, id)
    }

    pub fn delete_password(id: Uuid) -> Result<()> {
        delete(PASSWORD_SERVICE, id)
    }

    pub fn set_passphrase(id: Uuid, passphrase: &str) -> Result<()> {
        set(PASSPHRASE_SERVICE, id, passphrase)
    }

    pub fn get_passphrase(id: Uuid) -> Result<Option<String>> {
        get(PASSPHRASE_SERVICE, id)
    }

    pub fn delete_passphrase(id: Uuid) -> Result<()> {
        delete(PASSPHRASE_SERVICE, id)
    }

    /// Removes every secret associated with a connection. Used when a connection is deleted.
    /// Best-effort: a missing entry is not an error.
    pub fn delete_all(id: Uuid) -> Result<()> {
        Self::delete_password(id)?;
        Self::delete_passphrase(id)?;
        Ok(())
    }
}

fn entry(service: &str, id: Uuid) -> Result<Entry> {
    Entry::new(service, &id.to_string()).context("could not reach the system credential store")
}

fn set(service: &str, id: Uuid, secret: &str) -> Result<()> {
    entry(service, id)?
        .set_password(secret)
        .context("failed to store the secret in the system credential store")
}

fn get(service: &str, id: Uuid) -> Result<Option<String>> {
    match entry(service, id)?.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e).context("failed to read the secret from the system credential store"),
    }
}

fn delete(service: &str, id: Uuid) -> Result<()> {
    match entry(service, id)?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e).context("failed to delete the secret from the system credential store"),
    }
}
