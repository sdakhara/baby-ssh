use std::path::Path;

use russh::keys;

/// Best-effort detection of whether a key file is passphrase-protected: an unencrypted key
/// loads successfully with no passphrase, an encrypted one does not.
pub fn key_requires_passphrase(path: &Path) -> bool {
    keys::load_secret_key(path, None).is_err()
}
