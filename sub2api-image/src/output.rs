use crate::api::ApiResponse;
use crate::errors::AppError;
use anyhow::Result;
use base64::Engine;
use std::path::Path;

pub fn save_first_image(resp: &ApiResponse, out: &Path) -> Result<()> {
    if resp.data.is_empty() {
        return Err(AppError::Parse("data is empty".into()).into());
    }
    let first = &resp.data[0];
    let b64 = match &first.b64_json {
        Some(b) => b,
        None => {
            if first.url.is_some() {
                return Err(AppError::Parse(
                    "server returned URL mode, this version does not support downloading URL responses"
                        .into(),
                )
                .into());
            }
            return Err(AppError::Parse("data[0] has neither b64_json nor url".into()).into());
        }
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| AppError::Parse(format!("base64 decode failed: {}", e)))?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::Io(format!(
                    "cannot create output dir {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }
    }
    std::fs::write(out, &bytes)
        .map_err(|e| AppError::Io(format!("cannot write {}: {}", out.display(), e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ImageData;
    use crate::errors::exit_code_from;
    use base64::Engine;
    use tempfile::TempDir;

    fn resp_with(data: Vec<ImageData>) -> ApiResponse {
        ApiResponse {
            created: 0,
            data,
            usage: None,
        }
    }

    const PNG_1X1: &[u8] = &[
        137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6,
        0, 0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1,
        13, 10, 45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
    ];

    #[test]
    fn saves_decoded_png_to_disk() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1X1);
        let resp = resp_with(vec![ImageData {
            b64_json: Some(b64),
            url: None,
            revised_prompt: None,
        }]);
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("out.png");
        save_first_image(&resp, &path).unwrap();
        let written = std::fs::read(&path).unwrap();
        assert_eq!(&written[..4], &[137, 80, 78, 71], "PNG magic bytes");
    }

    #[test]
    fn creates_missing_parent_directory() {
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1X1);
        let resp = resp_with(vec![ImageData {
            b64_json: Some(b64),
            url: None,
            revised_prompt: None,
        }]);
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("a").join("b").join("c").join("out.png");
        save_first_image(&resp, &path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn empty_data_errors() {
        let resp = resp_with(vec![]);
        let dir = TempDir::new().unwrap();
        let err = save_first_image(&resp, &dir.path().join("x.png")).unwrap_err();
        assert_eq!(exit_code_from(&err), 5);
        assert!(err.to_string().contains("data is empty"));
    }

    #[test]
    fn url_mode_rejected() {
        let resp = resp_with(vec![ImageData {
            b64_json: None,
            url: Some("https://example.com/a.png".into()),
            revised_prompt: None,
        }]);
        let dir = TempDir::new().unwrap();
        let err = save_first_image(&resp, &dir.path().join("x.png")).unwrap_err();
        assert_eq!(exit_code_from(&err), 5);
        assert!(err.to_string().contains("URL mode"));
    }

    #[test]
    fn neither_b64_nor_url_errors() {
        let resp = resp_with(vec![ImageData {
            b64_json: None,
            url: None,
            revised_prompt: None,
        }]);
        let dir = TempDir::new().unwrap();
        let err = save_first_image(&resp, &dir.path().join("x.png")).unwrap_err();
        assert_eq!(exit_code_from(&err), 5);
        assert!(err.to_string().contains("neither b64_json nor url"));
    }

    #[test]
    fn bad_base64_errors() {
        let resp = resp_with(vec![ImageData {
            b64_json: Some("!!!not base64!!!".into()),
            url: None,
            revised_prompt: None,
        }]);
        let dir = TempDir::new().unwrap();
        let err = save_first_image(&resp, &dir.path().join("x.png")).unwrap_err();
        assert_eq!(exit_code_from(&err), 5);
        assert!(err.to_string().contains("base64 decode"));
    }
}
