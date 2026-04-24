use crate::api::ApiResponse;
use std::time::Duration;

pub struct Logger {
    quiet: bool,
}

impl Logger {
    pub fn new(quiet: bool) -> Self {
        Self { quiet }
    }

    pub fn endpoint(&self, method: &str, url: &str) {
        if self.quiet {
            return;
        }
        eprintln!("[sub2api-image] endpoint: {} {}", method, url);
    }

    pub fn body(&self, s: &str) {
        if self.quiet {
            return;
        }
        eprintln!("[sub2api-image] body: {}", s);
    }

    pub fn received(&self, status: u16, elapsed: Duration, bytes: usize) {
        if self.quiet {
            return;
        }
        eprintln!(
            "[sub2api-image] ← {} in {:.1}s  bytes={}",
            status,
            elapsed.as_secs_f64(),
            human_bytes(bytes),
        );
    }

    pub fn response_summary(&self, r: &ApiResponse) {
        if self.quiet {
            return;
        }
        if let Some(u) = &r.usage {
            eprintln!(
                "[sub2api-image] usage: total={} in={} out={}",
                u.total_tokens, u.input_tokens, u.output_tokens
            );
        }
        let first = r.data.first();
        let b64_len = first
            .and_then(|d| d.b64_json.as_ref().map(|s| s.len()))
            .unwrap_or(0);
        let url_some = first.and_then(|d| d.url.as_ref()).is_some();
        let rev_some = first.and_then(|d| d.revised_prompt.as_ref()).is_some();
        eprintln!(
            "[sub2api-image] response: data.len={} b64_json.len={} url={} revised_prompt={}",
            r.data.len(),
            b64_len,
            if url_some { "Some" } else { "None" },
            if rev_some { "Some" } else { "None" },
        );
    }
}

fn human_bytes(n: usize) -> String {
    const KB: usize = 1024;
    const MB: usize = 1024 * 1024;
    if n >= MB {
        format!("{:.1}MB", n as f64 / MB as f64)
    } else if n >= KB {
        format!("{:.1}KB", n as f64 / KB as f64)
    } else {
        format!("{}B", n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_formats() {
        assert_eq!(human_bytes(500), "500B");
        assert_eq!(human_bytes(2048), "2.0KB");
        assert_eq!(human_bytes(2 * 1024 * 1024), "2.0MB");
    }

    #[test]
    fn quiet_logger_produces_no_panic() {
        let log = Logger::new(true);
        log.endpoint("POST", "http://x");
        log.body("prompt_len=3");
        log.received(200, Duration::from_millis(100), 1024);
    }
}
