use assert_cmd::Command;
use predicates::str;
use serial_test::serial;
use std::fs;
use tempfile::TempDir;

fn bin() -> Command {
    Command::cargo_bin("playctl").unwrap()
}

#[test]
fn version_prints() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(str::contains("playctl"));
}

#[test]
fn print_template_known_succeeds() {
    bin()
        .args(["print-template", "design-playground"])
        .assert()
        .success()
        .stdout(str::contains("# design-playground"));
}

#[test]
fn print_template_unknown_fails_exit_1() {
    bin()
        .args(["print-template", "nope-nope"])
        .assert()
        .failure()
        .code(1)
        .stderr(str::contains("unknown template"));
}

#[test]
fn status_when_no_state_is_stopped() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .arg("status")
        .assert()
        .success()
        .stdout(str::contains("stopped"));
}

#[test]
fn list_empty_message() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .arg("list")
        .assert()
        .success()
        .stdout(str::contains("no playgrounds"));
}

#[test]
fn invalid_slug_rejected() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .args(["new", "BAD_SLUG"])
        .assert()
        .failure()
        .code(1)
        .stderr(str::contains("invalid slug"));
}

#[test]
fn unknown_template_rejected() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .args(["new", "ok-slug", "--template", "no-such"])
        .assert()
        .failure()
        .code(1)
        .stderr(str::contains("unknown template"));
}

#[test]
fn open_rejects_invalid_slug() {
    // 防 cmd.exe / xdg-open / wslview 参数注入：传 "../foo" 这种非法 slug
    // 必须在拼 URL 前被拒绝，退出码 1，不能去 spawn 任何 opener。
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .args(["open", "../foo"])
        .assert()
        .failure()
        .code(1)
        .stderr(str::contains("invalid slug"));
}

#[test]
fn open_rejects_slug_with_slash() {
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    bin()
        .args(["--root"])
        .arg(td.path())
        .args(["open", "foo/bar"])
        .assert()
        .failure()
        .code(1)
        .stderr(str::contains("invalid slug"));
}

#[test]
#[serial]
fn new_persists_description_to_index_json() {
    // `playctl new <slug> --description "..."` 必须把 description 写进
    // .playgrounds/index.json 对应条目。spec §14.4 #9 要求的 e2e 覆盖。
    // 用高位非默认端口避开和并发 sub2api-image 测试 / 其他 4747 占用者
    // 的端口竞争；测试结束显式 stop 兜底，TempDir 再 drop 掉 .runtime/。
    let td = TempDir::new().unwrap();
    fs::create_dir_all(td.path().join(".git")).unwrap();
    let port = "5347";
    let slug = "desc-test";
    let desc = "spec §14.4 #9 e2e";

    bin()
        .args(["--root"])
        .arg(td.path())
        .args(["--port", port])
        .args([
            "new",
            slug,
            "--template",
            "design-playground",
            "--description",
            desc,
        ])
        .assert()
        .success()
        .stdout(str::contains(slug));

    // index.json 落盘后读回校验
    let body = fs::read_to_string(td.path().join(".playgrounds/index.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    let entry = v["playgrounds"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["slug"] == slug)
        .expect("slug not in index.json");
    assert_eq!(entry["description"], desc);
    assert_eq!(entry["template"], "design-playground");

    // 清理 cmd_new 自启动的 daemon，避免泄漏到后续测试 / 后续 cargo test
    let _ = bin()
        .args(["--root"])
        .arg(td.path())
        .arg("stop")
        .assert()
        .try_success();
}
