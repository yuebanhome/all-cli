use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use tempfile::TempDir;

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
