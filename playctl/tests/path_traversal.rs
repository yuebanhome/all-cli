use playctl::server::{build_router, AppState};
use std::net::TcpListener;
use std::sync::Arc;

#[test]
fn slug_path_with_dotdot_returns_404() {
    let td = tempfile::TempDir::new().unwrap();
    let state = AppState {
        project_root: Arc::new(td.path().to_path_buf()),
        port: 0,
    };
    let app = build_router(state);

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    let handle = std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async move {
            listener.set_nonblocking(true).ok();
            let l = tokio::net::TcpListener::from_std(listener).unwrap();
            let server = axum::serve(l, app);
            let _ = tokio::time::timeout(std::time::Duration::from_secs(5), server).await;
        });
    });

    wait_for_healthz(port);

    let url = format!("http://127.0.0.1:{port}/foo/../../etc/passwd");
    let resp = reqwest::blocking::get(&url).unwrap();
    assert!(
        resp.status() == reqwest::StatusCode::NOT_FOUND
            || resp.status() == reqwest::StatusCode::BAD_REQUEST,
        "expected 404/400 for path traversal, got {}",
        resp.status()
    );
    let _ = handle.join();
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
