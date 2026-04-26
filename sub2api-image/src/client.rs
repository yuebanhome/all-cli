use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::time::Duration;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(user_agent())
        .build()
        .context("network error: cannot build HTTP client")
}

/// UA 风格参考 codex-tui:
/// `sub2api-image/0.1.0 (Debian 13.0.0; x86_64) WindowsTerminal (sub2api-image; 0.1.0)`
fn user_agent() -> String {
    const NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let info = os_info::get();
    let os_label = {
        let os_type = info.os_type().to_string();
        let version = info.version().to_string();
        if version.is_empty() || version == "Unknown" {
            os_type
        } else {
            format!("{} {}", os_type, version)
        }
    };
    let arch = std::env::consts::ARCH;
    let term = std::env::var("TERM_PROGRAM")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Terminal".to_string());

    format!("{NAME}/{VERSION} ({os_label}; {arch}) {term} ({NAME}; {VERSION})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_client() {
        build_client().unwrap();
    }

    #[test]
    fn request_timeout_allows_slow_image_generation() {
        assert_eq!(REQUEST_TIMEOUT, Duration::from_secs(300));
    }

    #[test]
    fn user_agent_has_expected_shape() {
        let ua = user_agent();
        assert!(ua.starts_with("sub2api-image/"), "ua={ua}");
        assert!(ua.contains("; "));
        assert!(ua.contains("(sub2api-image;"));
    }
}
