use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
        assert_eq!(parsed.data[0].url.as_deref(), Some("https://example.com/a.png"));
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
        assert_eq!(err.error.message, "mask dimensions must match image dimensions.");
        assert_eq!(err.error.param.as_deref(), Some("mask"));
        assert_eq!(err.error.err_type.as_deref(), Some("invalid_request_error"));
    }

    #[test]
    fn parses_api_error_with_null_param() {
        let json = r#"{"error":{"code":"authentication_error","type":"authentication_error","message":"Invalid API key","param":null}}"#;
        let err: ApiError = serde_json::from_str(json).unwrap();
        assert!(err.error.param.is_none());
    }
}
