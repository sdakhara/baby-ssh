use crate::storage::KnownHostsStore;

/// The outcome of comparing a server's presented host key against baby-ssh's known-hosts store.
pub enum Verdict {
    /// Matches the key we've seen before for this host.
    Trusted,
    /// We have never connected to this host before.
    New,
    /// We have connected before, but the key is different from what we trusted last time.
    Changed { previous_fingerprint: String },
}

pub fn evaluate(store: &KnownHostsStore, host: &str, port: u16, key_openssh: &str) -> Verdict {
    match store.lookup(host, port) {
        None => Verdict::New,
        Some(entry) if entry.key_openssh == key_openssh => Verdict::Trusted,
        Some(entry) => Verdict::Changed {
            previous_fingerprint: entry.fingerprint.clone(),
        },
    }
}
