//! The POESESSID cookie for private profiles, kept in the Windows credential store: never in a file, a log or the UI.

use keyring::Entry;

fn entry() -> Result<Entry, String> {
    Entry::new("PoeMate", "POESESSID").map_err(|e| e.to_string())
}

/// Old sessions are 32 hex characters, current ones a JWT (base64url parts joined by dots, about 900 characters).
/// Only cookie-safe characters, and short enough for the credential store (2560 bytes as UTF-16).
fn valid(v: &str) -> bool {
    (32..=1200).contains(&v.len()) && v.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_.=+/".contains(&b))
}

pub fn get() -> Option<String> {
    entry().ok()?.get_password().ok()
}

/// An empty value removes the stored cookie.
pub fn set(v: &str) -> Result<(), String> {
    let v = v.trim();
    if v.is_empty() {
        return match entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.to_string()),
        };
    }
    if !valid(v) {
        return Err("That doesn't look like a POESESSID (letters, digits and . - _ only, no spaces)".into());
    }
    entry()?.set_password(v).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::valid;

    #[test]
    fn hex_and_jwt_sessions_pass_anything_unsafe_does_not() {
        assert!(valid("47a3dcae931fde792809edd0c9ec6d69"));
        assert!(valid(&format!("eyJ0eXAi.{}.{}", "a1B2-_".repeat(100), "z9")));
        assert!(!valid("short"));
        assert!(!valid("47a3dcae931fde792809edd0c9ec6d6
"));
        assert!(!valid("47a3dcae931fde792809edd0c9ec6d69; other=1"));
        assert!(!valid(&"a".repeat(1201)));
    }
}
