use names::{NameStore, parse_scope};
use pointer::PointerStore;
use types::{Result, SisiError};

/// Parse and resolve an address string into a manifest hash.
///
/// Supported formats (in resolution order):
///
/// 1. **Human name** — `chiko` or `chiko@forum`
///    Resolved via the local [`NameStore`] → pubkey → [`PointerStore`] → hash.
///
/// 2. **Pubkey hex** — 64 lower-case hex chars that do *not* start with `bafy`
///    Looked up in the local [`PointerStore`]; the most recent site for that
///    key is returned.
///
/// 3. **Raw hash** — everything else (typically a BLAKE3 base32 hash starting
///    with `bafy`).
///    Passed through as-is to the gateway.
pub async fn parse_and_resolve(addr: &str) -> Result<String> {
    let stripped = addr
        .trim()
        .trim_start_matches("sisi://")
        .trim_end_matches('/');

    if is_human_name(stripped) {
        return resolve_human_name(stripped).await;
    }

    if is_pubkey_hex(stripped) {
        match resolve_pubkey(stripped, None).await {
            Ok(hash) => return Ok(hash),
            // Not found in pointer store — treat as a direct manifest hash.
            Err(_) => return Ok(stripped.to_string()),
        }
    }

    Ok(stripped.to_string())
}

async fn resolve_human_name(name: &str) -> Result<String> {
    let store = NameStore::open().map_err(|e| SisiError::InvalidAddress(e.to_string()))?;

    let pubkey = store
        .resolve(name)
        .map_err(|e| SisiError::InvalidAddress(e.to_string()))?
        .ok_or_else(|| {
            SisiError::InvalidAddress(format!(
                "name '{name}' is not in your local index — \
                 subscribe to the relevant topic to discover it"
            ))
        })?;

    let scope = parse_scope(name);

    resolve_pubkey(&pubkey, scope).await
}

async fn resolve_pubkey(pubkey: &str, scope: Option<&str>) -> Result<String> {
    let store = PointerStore::load()
        .await
        .map_err(|e| SisiError::Iroh(anyhow::anyhow!(e)))?;

    let short = if pubkey.len() >= 8 {
        &pubkey[..8]
    } else {
        pubkey
    };

    if let Some(s) = scope
        && let Some(p) = store.get_scoped(pubkey, Some(s))
    {
        return Ok(p.hash.clone());
    }

    // Try unscoped first (the common case).
    if let Some(p) = store.get(pubkey) {
        return Ok(p.hash.clone());
    }

    if let Some(p) = store.get_scoped(pubkey, Some("default")) {
        return Ok(p.hash.clone());
    }

    // Fall back to any pointer for this pubkey regardless of scope.
    // This handles the case where someone published with a scope set
    // but is navigating via the bare pubkey.
    store
        .all_for_pubkey(pubkey)
        .into_iter()
        // Pick highest version if multiple scopes exist.
        .max_by_key(|p| p.version)
        .map(|p| p.hash.clone())
        .ok_or_else(|| SisiError::ManifestNotFound(format!("no pointer found for pubkey {short}")))
}

/// Returns `true` when the address looks like a human-readable name rather
/// than a hash or pubkey.
///
/// Positive match requires one of:
/// - Contains `@` (scoped name, e.g. `chiko@forum`), OR
/// - Short (≤ 32 chars), not all hex, not starting with `bafy`, and contains
///   only name-legal characters `[a-z0-9_-]` (i.e. already validated as a
///   name segment).
///
/// Everything else (long strings, mixed-case, base32 hashes, pubkeys) falls
/// through to hash/pubkey resolution.
fn is_human_name(s: &str) -> bool {
    if s.contains('@') {
        // Must still look like a valid scoped name, not a URL or email.
        // Quick sanity: both sides non-empty and name-legal chars only.
        return looks_like_name_chars(s);
    }
    // Unscoped: must be short, name-legal, and not a hash prefix.
    s.len() <= 32 && !s.starts_with("bafy") && !is_pubkey_hex(s) && looks_like_name_chars(s)
}

/// Returns `true` when every character in `s` is legal in a name segment
/// (`[a-z0-9_@-]`).
fn looks_like_name_chars(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-' || c == '@'
        })
}

/// Returns `true` when `s` looks like a raw ed25519 pubkey (64 lowercase hex
/// chars, not starting with `bafy`).
fn is_pubkey_hex(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) && !s.starts_with("bafy")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_human_name_should_detect_scoped_name() {
        assert!(is_human_name("chiko@forum"));
    }

    #[test]
    fn is_human_name_should_detect_unscoped_short_name() {
        assert!(is_human_name("chiko"));
    }

    #[test]
    fn is_human_name_should_reject_pubkey_hex() {
        let pubkey = "a".repeat(64);
        assert!(!is_human_name(&pubkey));
    }

    #[test]
    fn is_human_name_should_reject_bafy_hash() {
        assert!(!is_human_name("bafyreic3n7fa4qx"));
    }

    #[test]
    fn is_human_name_should_reject_long_base32_hash() {
        // Typical iroh-blobs CID: 59 base32 chars starting with bafy
        assert!(!is_human_name(
            "bafkreib4zn5okmvk5lrjvulcgdhu7sfc7yqzxgq6x4xqoqlv4lhyms7m4"
        ));
    }

    #[test]
    fn is_human_name_should_reject_uppercase_string() {
        assert!(!is_human_name("MyBlog"));
    }

    #[test]
    fn is_human_name_should_reject_long_name() {
        // Over 32 chars — shouldn't be treated as a name
        assert!(!is_human_name(
            "this-is-a-very-long-name-that-exceeds-the-limit"
        ));
    }

    #[test]
    fn is_pubkey_hex_should_accept_64_char_hex() {
        let pubkey = "ab".repeat(32); // 64 chars
        assert!(is_pubkey_hex(&pubkey));
    }

    #[test]
    fn is_pubkey_hex_should_reject_short_string() {
        assert!(!is_pubkey_hex("abc123"));
    }
}
