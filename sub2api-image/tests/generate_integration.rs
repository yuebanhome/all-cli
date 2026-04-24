use assert_cmd::Command;
use base64::Engine;
use httpmock::prelude::*;
use tempfile::TempDir;

const PNG_1x1: &[u8] = &[
    137,80,78,71,13,10,26,10,0,0,0,13,73,72,68,82,0,0,0,1,0,0,0,1,
    8,6,0,0,0,31,21,196,137,0,0,0,13,73,68,65,84,120,156,99,0,1,0,
    0,5,0,1,13,10,45,180,0,0,0,0,73,69,78,68,174,66,96,130,
];

#[test]
fn generate_happy_path() {
    let server = MockServer::start();
    let tmp = TempDir::new().unwrap();
    let config_path = tmp.path().join("config.toml");
    std::fs::write(
        &config_path,
        format!(
            r#"base_url = "{}"
api_key  = "test-key"

[defaults]
model   = "gpt-image-2"
size    = "auto"
quality = "auto"
"#,
            server.base_url()
        ),
    )
    .unwrap();

    let b64 = base64::engine::general_purpose::STANDARD.encode(PNG_1x1);

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/v1/images/generations")
            .header("authorization", "Bearer test-key")
            .header_exists("content-type")
            .body_contains(r#""prompt":"a red apple""#)
            .body_contains(r#""model":"gpt-image-2""#)
            .body_contains(r#""n":1"#);
        then.status(200)
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"created":1700000000,"data":[{{"b64_json":"{}"}}],"usage":{{"total_tokens":100,"input_tokens":10,"output_tokens":90}}}}"#,
                b64
            ));
    });

    let out_path = tmp.path().join("out.png");
    Command::cargo_bin("sub2api-image")
        .unwrap()
        .env("SUB2API_IMAGE_CONFIG", &config_path)
        .args(["--prompt", "a red apple", "--quiet", "-o"])
        .arg(&out_path)
        .assert()
        .success();

    mock.assert();
    let bytes = std::fs::read(&out_path).unwrap();
    assert_eq!(&bytes[..4], &[137, 80, 78, 71], "PNG magic bytes");
}
