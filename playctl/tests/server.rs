use playctl::index::{write_index, Index, Playground};
use playctl::server::{build_router, AppState};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::oneshot;

/// 在专用线程里跑 axum；测试函数 drop 返回的 guard 时通过 graceful_shutdown
/// 让服务器立即退出，避免测试结束后 tokio runtime 还要硬撑到 timeout 兜底。
struct ServerGuard {
    shutdown: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(h) = self.thread.take() {
            let _ = h.join();
        }
    }
}

fn spawn_server(project_root: std::path::PathBuf) -> (u16, ServerGuard) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = oneshot::channel::<()>();
    let thread = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            listener.set_nonblocking(true).ok();
            let l = tokio::net::TcpListener::from_std(listener).unwrap();
            let app = build_router(AppState {
                project_root: Arc::new(project_root),
                port,
            });
            let _ = axum::serve(l, app)
                .with_graceful_shutdown(async move {
                    let _ = rx.await;
                })
                .await;
        });
    });
    wait_for_healthz(port);
    (
        port,
        ServerGuard {
            shutdown: Some(tx),
            thread: Some(thread),
        },
    )
}

fn wait_for_healthz(port: u16) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let url = format!("http://127.0.0.1:{port}/healthz");
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(200))
        .build()
        .unwrap();
    while std::time::Instant::now() < deadline {
        if let Ok(r) = client.get(&url).send() {
            if r.status().is_success() {
                return;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("server on port {port} did not respond to /healthz within 5s");
}

#[test]
fn healthz_returns_ok() {
    let td = tempfile::TempDir::new().unwrap();
    let (port, _g) = spawn_server(td.path().to_path_buf());
    let r = reqwest::blocking::get(format!("http://127.0.0.1:{port}/healthz")).unwrap();
    assert_eq!(r.status(), 200);
    assert_eq!(r.text().unwrap(), "OK");
}

#[test]
fn index_page_contains_project_name() {
    let td = tempfile::TempDir::new().unwrap();
    let name = td
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let (port, _g) = spawn_server(td.path().to_path_buf());
    let body = reqwest::blocking::get(format!("http://127.0.0.1:{port}/"))
        .unwrap()
        .text()
        .unwrap();
    assert!(body.contains(&name), "expected {name} in body");
    assert!(body.contains(&format!("port {port}")));
}

#[test]
fn internal_index_json_passthrough() {
    let td = tempfile::TempDir::new().unwrap();
    let mut idx = Index::empty("p");
    idx.playgrounds.push(Playground {
        slug: "card-tuner".into(),
        title: "Card Tuner".into(),
        description: "demo".into(),
        template: "design-playground".into(),
        created_at: "2026-05-09T00:00:00Z".into(),
    });
    write_index(td.path(), &idx).unwrap();
    let (port, _g) = spawn_server(td.path().to_path_buf());
    let body = reqwest::blocking::get(format!("http://127.0.0.1:{port}/_internal/index.json"))
        .unwrap()
        .text()
        .unwrap();
    assert!(body.contains("card-tuner"));
    assert!(body.contains("\"version\":1"));
}

#[test]
fn slug_serves_index_html() {
    let td = tempfile::TempDir::new().unwrap();
    let pg = td.path().join(".playgrounds/foo");
    std::fs::create_dir_all(&pg).unwrap();
    std::fs::write(pg.join("index.html"), "<html><body>HELLO-FOO</body></html>").unwrap();
    let (port, _g) = spawn_server(td.path().to_path_buf());

    // 不带尾斜杠 → 308 (axum::Redirect::permanent)
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    let r = client
        .get(format!("http://127.0.0.1:{port}/foo"))
        .send()
        .unwrap();
    assert_eq!(r.status(), 308);

    // 带尾斜杠 → 200 + 内容
    let body = reqwest::blocking::get(format!("http://127.0.0.1:{port}/foo/"))
        .unwrap()
        .text()
        .unwrap();
    assert!(body.contains("HELLO-FOO"));
}

#[test]
fn unknown_slug_returns_404() {
    let td = tempfile::TempDir::new().unwrap();
    let (port, _g) = spawn_server(td.path().to_path_buf());
    let r = reqwest::blocking::get(format!("http://127.0.0.1:{port}/nonexistent/")).unwrap();
    assert_eq!(r.status(), 404);
}
