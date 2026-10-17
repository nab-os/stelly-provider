//! Where the provider is, and the token it wants.

use std::collections::BTreeMap;
use std::path::Path;

/// Where a provider on this machine listens unless told otherwise.
pub const DEFAULT_URL: &str = "http://127.0.0.1:7710";

#[derive(Debug, Clone)]
pub struct Config {
    /// The provider's base URL, without a trailing slash.
    pub url: String,
    /// Sent as a bearer token, when the provider wants one.
    pub token: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            url: DEFAULT_URL.to_string(),
            token: None,
        }
    }
}

impl Config {
    /// `STELLY_PROVIDER_URL` and `STELLY_PROVIDER_TOKEN`, from the
    /// environment over `env_file` if one is given, the default URL and no
    /// token otherwise. `STELLY_QOBUZ_URL` and `STELLY_QOBUZ_TOKEN`, what
    /// they were called when the only provider was Qobuz's gateway, still
    /// count where the new names are not set.
    pub fn from_env(env_file: Option<&Path>) -> Self {
        let file = env_file.map(parse_env).unwrap_or_default();
        let get = |key: &str| {
            std::env::var(key)
                .ok()
                .or_else(|| file.get(key).cloned())
                .filter(|v| !v.is_empty())
        };
        Self {
            url: get("STELLY_PROVIDER_URL")
                .or_else(|| get("STELLY_QOBUZ_URL"))
                .unwrap_or_else(|| DEFAULT_URL.to_string())
                .trim_end_matches('/')
                .to_string(),
            token: get("STELLY_PROVIDER_TOKEN").or_else(|| get("STELLY_QOBUZ_TOKEN")),
        }
    }
}

/// A .env file's keys and values; nothing if it is not there.
pub fn parse_env(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.strip_prefix("export ").unwrap_or(l))
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), env_value(v.trim())))
        .collect()
}

/// A value as a shell reads it: quoted, it is what is between the quotes;
/// bare, a ` #` starts a comment.
fn env_value(raw: &str) -> String {
    for quote in ['"', '\''] {
        if let Some(rest) = raw.strip_prefix(quote) {
            return rest.split(quote).next().unwrap_or_default().to_string();
        }
    }
    raw.split(" #")
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn env_values_as_a_shell_reads_them() {
        assert_eq!(env_value("abc"), "abc");
        assert_eq!(env_value("abc # the app id"), "abc");
        assert_eq!(env_value("\"a b\" # quoted"), "a b");
        assert_eq!(env_value("'x#y'"), "x#y");
        assert_eq!(env_value("a#b"), "a#b");
    }
}
