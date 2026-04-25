use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::logging::Logger;
use anyhow::{bail, Context, Result};
use reqwest::blocking::{Client, Response};
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct GenerateReq {
    pub prompt: String,
    pub model: String,
    pub n: u32,
    pub size: String,
    pub quality: String,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse {
    // kept for doc-completeness and external consumers; not currently logged.
    #[allow(dead_code)]
    pub created: i64,
    pub data: Vec<ImageData>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct ImageData {
    pub b64_json: Option<String>,
    pub url: Option<String>,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub struct ApiError {
    pub error: ApiErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct ApiErrorBody {
    // upstream ships `code` alongside `type`; we display `type` in errors and
    // keep `code` parsed for Debug output.
    #[allow(dead_code)]
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub err_type: Option<String>,
    pub message: String,
    pub param: Option<String>,
}

/// 调用接口前汇总得到的"最终参数"，由 CLI + Config 合并产出。
pub struct EffectiveCfg {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub size: String,
    pub quality: String,
    pub prompt: String,
    pub image: Option<PathBuf>,
    pub mask: Option<PathBuf>,
}

pub fn generate(client: &Client, cfg: &EffectiveCfg, log: &Logger) -> Result<ApiResponse> {
    let url = format!(
        "{}/v1/images/generations",
        cfg.base_url.trim_end_matches('/')
    );
    let req = GenerateReq {
        prompt: cfg.prompt.clone(),
        model: cfg.model.clone(),
        n: 1,
        size: cfg.size.clone(),
        quality: cfg.quality.clone(),
    };
    log.endpoint("POST", &url);
    log.body(&format!(
        "prompt_len={} size={} quality={} model={}",
        cfg.prompt.len(),
        cfg.size,
        cfg.quality,
        cfg.model,
    ));
    let t0 = Instant::now();
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .json(&req)
        .send()
        .map_err(|e| anyhow::anyhow!("network error: {}", e))?;
    decode_response(resp, &url, t0, log)
}

pub fn edit(client: &Client, cfg: &EffectiveCfg, log: &Logger) -> Result<ApiResponse> {
    use reqwest::blocking::multipart;

    let url = format!("{}/v1/images/edits", cfg.base_url.trim_end_matches('/'));
    let image = cfg
        .image
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("input error: --image required for edit"))?;

    let mut form = multipart::Form::new()
        .text("prompt", cfg.prompt.clone())
        .text("model", cfg.model.clone())
        .text("n", "1")
        .text("quality", cfg.quality.clone())
        .text("size", cfg.size.clone())
        .file("image", image)
        .with_context(|| format!("io error: cannot read image {}", image.display()))?;
    if let Some(mask) = &cfg.mask {
        form = form
            .file("mask", mask)
            .with_context(|| format!("io error: cannot read mask {}", mask.display()))?;
    }

    log.endpoint("POST", &url);
    log.body(&format!(
        "prompt_len={} size={} quality={} model={} image={} mask={}",
        cfg.prompt.len(),
        cfg.size,
        cfg.quality,
        cfg.model,
        image.display(),
        cfg.mask
            .as_ref()
            .map(|m| m.display().to_string())
            .unwrap_or_else(|| "<none>".into()),
    ));

    let t0 = Instant::now();
    let resp = client
        .post(&url)
        .bearer_auth(&cfg.api_key)
        .multipart(form)
        .send()
        .map_err(|e| anyhow::anyhow!("network error: {}", e))?;
    decode_response(resp, &url, t0, log)
}

fn decode_response(resp: Response, url: &str, t0: Instant, log: &Logger) -> Result<ApiResponse> {
    let status = resp.status();
    let bytes = resp
        .bytes()
        .map_err(|e| anyhow::anyhow!("network error reading body: {}", e))?;
    log.received(status.as_u16(), t0.elapsed(), bytes.len());

    if status.is_success() {
        let parsed: ApiResponse = serde_json::from_slice(&bytes)
            .with_context(|| format!("response parse error from {}", url))?;
        log.response_summary(&parsed);
        Ok(parsed)
    } else {
        let body_text = String::from_utf8_lossy(&bytes);
        let msg = match serde_json::from_slice::<ApiError>(&bytes) {
            Ok(api_err) => format!(
                "{} {}: {}\n  param: {}\n  endpoint: POST {}",
                status.as_u16(),
                api_err.error.err_type.as_deref().unwrap_or("unknown"),
                api_err.error.message,
                api_err.error.param.as_deref().unwrap_or("null"),
                url,
            ),
            Err(_) => format!(
                "{}: {}\n  endpoint: POST {}",
                status.as_u16(),
                truncate(&body_text, 2048),
                url,
            ),
        };
        bail!("api error: {}", msg)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // 在字符边界处截断，避免落到多字节 UTF-8 字符中间触发 panic
    let cut = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max)
        .last()
        .unwrap_or(0);
    format!("{}...(truncated)", &s[..cut])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_req_serializes_expected_fields() {
        let req = GenerateReq {
            prompt: "hello".into(),
            model: "gpt-image-2".into(),
            n: 1,
            size: "auto".into(),
            quality: "high".into(),
        };
        let s = serde_json::to_string(&req).unwrap();
        assert!(s.contains(r#""prompt":"hello""#));
        assert!(s.contains(r#""model":"gpt-image-2""#));
        assert!(s.contains(r#""n":1"#));
        assert!(s.contains(r#""size":"auto""#));
        assert!(s.contains(r#""quality":"high""#));
    }

    #[test]
    fn parses_doc_sample_success_response() {
        // 来自 spec §8.3 / 飞书文档示例
        let json = r#"{
          "created": 1700000000,
          "data": [{ "b64_json": "iVBORw0KGgo..." }],
          "usage": { "total_tokens": 1580, "input_tokens": 25, "output_tokens": 1555 }
        }"#;
        let parsed: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.created, 1_700_000_000);
        assert_eq!(parsed.data.len(), 1);
        assert_eq!(parsed.data[0].b64_json.as_deref(), Some("iVBORw0KGgo..."));
        assert_eq!(parsed.usage.unwrap().total_tokens, 1580);
    }

    #[test]
    fn parses_response_with_url_and_revised_prompt() {
        let json = r#"{
          "created": 1,
          "data": [{ "url": "https://example.com/a.png", "revised_prompt": "updated" }]
        }"#;
        let parsed: ApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed.data[0].url.as_deref(),
            Some("https://example.com/a.png")
        );
        assert_eq!(parsed.data[0].revised_prompt.as_deref(), Some("updated"));
        assert!(parsed.usage.is_none());
    }

    #[test]
    fn parses_api_error_body() {
        let json = r#"{
          "error": {
            "code": "invalid_request_error",
            "type": "invalid_request_error",
            "message": "mask dimensions must match image dimensions.",
            "param": "mask"
          }
        }"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert_eq!(
            err.error.message,
            "mask dimensions must match image dimensions."
        );
        assert_eq!(err.error.param.as_deref(), Some("mask"));
        assert_eq!(err.error.err_type.as_deref(), Some("invalid_request_error"));
    }

    #[test]
    fn parses_api_error_with_null_param() {
        let json = r#"{"error":{"code":"authentication_error","type":"authentication_error","message":"Invalid API key","param":null}}"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert!(err.error.param.is_none());
    }

    #[test]
    fn truncate_short_unchanged() {
        assert_eq!(truncate("hi", 10), "hi");
    }

    #[test]
    fn truncate_long_is_cut() {
        let s = "a".repeat(100);
        let r = truncate(&s, 10);
        assert_eq!(r.len(), 10 + "...(truncated)".len());
        assert!(r.starts_with("aaaaaaaaaa"));
        assert!(r.ends_with("(truncated)"));
    }

    #[test]
    fn truncate_does_not_panic_on_multibyte_boundary() {
        // "你好" = 6 bytes (3 bytes per char). max=4 lands inside "好";
        // 朴素 &s[..4] 会 panic，必须回退到上一个字符边界。
        let s = "你好世界";
        let r = truncate(s, 4);
        assert!(r.ends_with("(truncated)"));
        // 前 3 字节是"你"
        assert!(r.starts_with("你"));
    }
}
