use assert_cmd::Command;
use predicates::str;
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
