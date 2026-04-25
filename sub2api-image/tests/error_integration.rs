use assert_cmd::Command;
use base64::Engine;
use httpmock::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

const PNG_1X1: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 6, 0,
    0, 0, 31, 21, 196, 137, 0, 0, 0, 13, 73, 68, 65, 84, 120, 156, 99, 0, 1, 0, 0, 5, 0, 1, 13, 10,
    45, 180, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[test]
fn api_401_authentication_error() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"base_url = "{}"
api_key  = "bad-key"
"#,
            server.base_url()
        ),
    )
    .unwrap();

    server.mock(|when, then| {
        when.method(POST).path("/v1/images/generations");
        then.status(401)
            .header("content-type", "application/json")
            .body(r#"{"error":{"code":"authentication_error","type":"authentication_error","message":"Invalid API key","param":null}}"#);
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("authentication_error"))
        .stderr(predicate::str::contains("Invalid API key"));
    assert!(
        !out_path.exists(),
        "output file should not be written on failure"
    );
}

#[test]
fn config_missing_api_key_exits_2() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"base_url = "https://example.invalid"
api_key  = ""
"#,
    )
    .unwrap();

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("api_key"));
}

#[test]
fn config_file_missing_exits_2() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("nonexistent.toml");
    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--init"));
}

#[test]
fn api_500_non_error_shape_exits_4() {
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

    server.mock(|when, then| {
        when.method(POST).path("/v1/images/generations");
        then.status(502)
            .header("content-type", "text/plain")
            .body("upstream timeout");
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("502"))
        .stderr(predicate::str::contains("upstream timeout"));
    assert!(!out_path.exists());
}

#[test]
fn api_returns_url_mode_exits_5() {
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

    server.mock(|when, then| {
        when.method(POST).path("/v1/images/generations");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"created":1,"data":[{"url":"https://example.com/a.png"}]}"#);
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("URL mode"));
    assert!(!out_path.exists());
}

#[test]
fn api_returns_invalid_base64_exits_5() {
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

    server.mock(|when, then| {
        when.method(POST).path("/v1/images/generations");
        then.status(200)
            .header("content-type", "application/json")
            .body(r#"{"created":1,"data":[{"b64_json":"!!!not base64!!!"}]}"#);
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("base64 decode"));
}

#[test]
fn mask_file_missing_exits_2() {
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        r#"base_url = "https://example.invalid"
api_key  = "test-key"
"#,
    )
    .unwrap();

    let image_path = tmp.path().join("image.png");
    std::fs::write(&image_path, PNG_1X1).unwrap();
    let mask_path = tmp.path().join("nonexistent-mask.png");

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet"])
        .arg("--image")
        .arg(&image_path)
        .arg("--mask")
        .arg(&mask_path)
        .arg("-o")
        .arg(&out_path)
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("--mask file does not exist"));
}

#[test]
fn init_writes_template_then_refuses_overwrite() {
    let tmp = TempDir::new().unwrap();
    let cfg_path = tmp.path().join("config.toml");

    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &cfg_path)
        .arg("--init")
        .assert()
        .success()
        .stdout(predicate::str::contains("Wrote config template"));

    assert!(cfg_path.exists(), "template should have been written");
    let content = std::fs::read_to_string(&cfg_path).unwrap();
    assert!(content.contains("REPLACE_ME"));

    // 二次 --init 必须报错且 exit code = 2
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &cfg_path)
        .arg("--init")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn init_creates_parent_dir() {
    let tmp = TempDir::new().unwrap();
    let nested = tmp.path().join("nested").join("dir").join("config.toml");

    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &nested)
        .arg("--init")
        .assert()
        .success();

    assert!(nested.exists());
}

#[test]
fn output_into_missing_dir_succeeds() {
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

    let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1X1);
    server.mock(|when, then| {
        when.method(POST).path("/v1/images/generations");
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"created":1,"data":[{{"b64_json":"{}"}}]}}"#,
                b64
            ));
    });

    // 三层不存在的目录，CLI 应当自动建出来
    let out_path = tmp
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "x", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .success();
    assert!(out_path.exists());
}
