fn is_plain_ascii(b: u8) -> bool {
    (0x21..=0x7E).contains(&b) && b != b'|'
}

/// Normalizes a site per spec 3.1: strip scheme and path, lowercase, plain ASCII only.
pub fn normalize_site(raw: &str) -> Result<String, String> {
    let mut s = raw.trim();
    if let Some((_, rest)) = s.split_once("://") {
        s = rest;
    }
    if let Some(end) = s.find(['/', '?', '#']) {
        s = &s[..end];
    }
    let s = s.to_ascii_lowercase();
    if s.is_empty() {
        return Err("invalid site: empty".into());
    }
    if !s.bytes().all(is_plain_ascii) {
        return Err("invalid site: only ASCII 0x21-0x7E without '|' is allowed".into());
    }
    Ok(s)
}

/// Normalizes an account per spec 3.1: trim, ASCII lowercase, no control characters or '|'.
pub fn normalize_account(raw: &str) -> Result<String, String> {
    let s = raw.trim().to_ascii_lowercase();
    if s.is_empty() {
        return Err("invalid account: empty".into());
    }
    if s.chars().any(|c| c.is_control() || c == '|') {
        return Err("invalid account: control characters and '|' are not allowed".into());
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn site_spellings_collapse() {
        for raw in ["HTTPS://GitHub.com/x?y", "  github.com ", "github.com/login", "git://GITHUB.COM#frag"] {
            assert_eq!(normalize_site(raw).unwrap(), "github.com", "{raw}");
        }
    }

    #[test]
    fn site_keeps_www_and_port() {
        assert_eq!(normalize_site("www.github.com").unwrap(), "www.github.com");
        assert_eq!(normalize_site("http://localhost:8080/a").unwrap(), "localhost:8080");
    }

    #[test]
    fn site_query_scheme_uses_first_separator() {
        assert_eq!(normalize_site("https://a.com/?next=https://b.com").unwrap(), "a.com");
    }

    #[test]
    fn site_rejects_bad_input() {
        for raw in ["", "   ", "https://", "git hub.com", "gíthub.com", "a|b.com", "/path"] {
            assert!(normalize_site(raw).is_err(), "{raw:?}");
        }
    }

    #[test]
    fn account_lowercases_ascii_only() {
        assert_eq!(normalize_account("  Budi@Gmail.COM ").unwrap(), "budi@gmail.com");
        assert_eq!(normalize_account("ÉRIC").unwrap(), "Éric");
    }

    #[test]
    fn account_rejects_bad_input() {
        for raw in ["", "  ", "a|b", "a\nb", "a\tb"] {
            assert!(normalize_account(raw).is_err(), "{raw:?}");
        }
    }
}
