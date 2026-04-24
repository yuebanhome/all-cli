use assert_cmd::Command;
use base64::Engine;
use httpmock::prelude::*;
use tempfile::TempDir;

const PNG_1X1: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn edit_with_mask_multipart() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"base_url = "{}"
api_key  = "test-key"
"#,
            server.base_url()
        ),
    )
    .unwrap();

    // 本地假 image + mask（内容只要存在可读即可；服务器是 mock，不校验图像有效性）
    let image_path = tmp.path().join("origin.png");
    let mask_path = tmp.path().join("mask.png");
    std::fs::write(&image_path, PNG_1X1).unwrap();
    std::fs::write(&mask_path, PNG_1X1).unwrap();

    let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1X1);
    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/images/edits")
            .header("authorization", "Bearer test-key")
            // multipart body 里应含各字段名
            .body_contains(r#"name="prompt""#)
            .body_contains(r#"name="model""#)
            .body_contains(r#"name="n""#)
            .body_contains(r#"name="quality""#)
            .body_contains(r#"name="size""#)
            .body_contains(r#"name="image""#)
            .body_contains(r#"name="mask""#);
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"created":1,"data":[{{"b64_json":"{}"}}]}}"#,
                b64
            ));
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "starry sky", "--quiet"])
        .arg("--image")
        .arg(&image_path)
        .arg("--mask")
        .arg(&mask_path)
        .arg("-o")
        .arg(&out_path)
        .assert()
        .success();

    mock.assert();
    let bytes = std::fs::read(&out_path).unwrap();
    assert_eq!(&bytes[..4], &[137, 80, 78, 71]);
}

#[test]
fn edit_without_mask_ok() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"base_url = "{}"
api_key  = "test-key"
"#,
            server.base_url()
        ),
    )
    .unwrap();

    let image_path = tmp.path().join("origin.png");
    std::fs::write(&image_path, PNG_1X1).unwrap();

    let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1X1);
    let mock = server.mock(|when, then| {
        when.method(POST).path("/v1/images/edits");
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"created":1,"data":[{{"b64_json":"{}"}}]}}"#,
                b64
            ));
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "watercolor", "--quiet"])
        .arg("--image")
        .arg(&image_path)
        .arg("-o")
        .arg(&out_path)
        .assert()
        .success();
    mock.assert();
}
