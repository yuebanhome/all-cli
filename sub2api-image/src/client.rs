use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::time::Duration;

pub fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent(concat!("sub2api-image/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("network error: cannot build HTTP client")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_client() {
        build_client().unwrap();
    }
}
