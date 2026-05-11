use crate::error::ExitError;
use crate::index;
use crate::templates;
use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::{Path as AxumPath, State},
    http::{header, HeaderValue, Response, StatusCode, Uri},
    response::{IntoResponse, Redirect},
    routing::get,
    Router,
};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct AppState {
    pub project_root: Arc<PathBuf>,
    pub port: u16,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_page))
        .route("/healthz", get(healthz))
        .route("/_internal/index.json", get(index_json))
        .route("/_internal/style.css", get(style_css))
        .route("/:slug", get(slug_redirect))
        .fallback(get(slug_static))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn healthz() -> &'static str {
    "OK"
}

async fn index_page(State(s): State<AppState>) -> Response<Body> {
    let html_bytes = match templates::read_asset("index.html") {
        Ok(b) => b,
        Err(_) => return server_error("index template missing"),
    };
    let mut html = String::from_utf8_lossy(&html_bytes).into_owned();
    let project_name = s
        .project_root
        .file_name()
        .map(|x| x.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    html = html.replace("{{project_name}}", &html_escape(&project_name));
    html = html.replace("{{port}}", &s.port.to_string());
    html = html.replace("{{version}}", VERSION);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(html))
        .unwrap()
}

async fn style_css() -> Response<Body> {
    match templates::read_asset("style.css") {
        Ok(b) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/css; charset=utf-8")
            .header(header::CACHE_CONTROL, "public, max-age=86400")
            .body(Body::from(b))
            .unwrap(),
        Err(_) => server_error("style.css missing"),
    }
}

async fn index_json(State(s): State<AppState>) -> Response<Body> {
    let body = match index::read_index(&s.project_root) {
        Ok(idx) => match serde_json::to_vec(&idx) {
            Ok(b) => b,
            Err(e) => return server_error(&format!("encode index.json: {e}")),
        },
        Err(e) => return server_error(&format!("read index.json: {e}")),
    };
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(body))
        .unwrap()
}

async fn slug_redirect(AxumPath(slug): AxumPath<String>) -> Response<Body> {
    if !is_valid_slug(&slug) {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("not found"))
            .unwrap();
    }
    Redirect::permanent(&format!("/{slug}/")).into_response()
}

async fn slug_static(State(s): State<AppState>, uri: Uri) -> Response<Body> {
    // 路径形如 /<slug>/<file...>；slug 必须先校验。
    let path = uri.path().trim_start_matches('/');
    let mut parts = path.splitn(2, '/');
    let slug = match parts.next() {
        Some(x) if is_valid_slug(x) => x,
        _ => return not_found(),
    };
    let rest = parts.next().unwrap_or("");
    let serve_root = s.project_root.join(".playgrounds").join(slug);
    if !serve_root.exists() {
        return not_found();
    }

    let mut svc = ServeDir::new(&serve_root).append_index_html_on_directories(true);
    let req_path = if rest.is_empty() {
        "/".to_string()
    } else {
        format!("/{rest}")
    };
    let req = http::Request::builder()
        .uri(req_path)
        .body(Body::empty())
        .unwrap();
    match tower::ServiceExt::oneshot(&mut svc, req).await {
        Ok(resp) => {
            let (parts, body): (_, tower_http::services::fs::ServeFileSystemResponseBody) =
                resp.into_parts();
            let mut resp = Response::from_parts(parts, axum::body::Body::new(body));
            resp.headers_mut()
                .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
            resp
        }
        Err(_) => server_error("static serve failed"),
    }
}

fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn not_found() -> Response<Body> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("not found"))
        .unwrap()
}

fn server_error(msg: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(Body::from(msg.to_string()))
        .unwrap()
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// 在已绑定的 listener 上跑 axum，直到 ctrl-c。同步外壳，内部启动 tokio runtime。
pub fn run(listener: TcpListener, project_root: PathBuf) -> Result<()> {
    let port = listener
        .local_addr()
        .with_context(|| ExitError::Internal("listener has no addr".into()))?
        .port();
    let state = AppState {
        project_root: Arc::new(project_root),
        port,
    };
    let app = build_router(state);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .with_context(|| ExitError::Internal("tokio runtime".into()))?;
    rt.block_on(async move {
        listener.set_nonblocking(true).ok();
        let l = tokio::net::TcpListener::from_std(listener).context("convert listener")?;
        axum::serve(l, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .context("axum serve")
    })?;
    Ok(())
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM");
    let mut int = signal(SignalKind::interrupt()).expect("install SIGINT");
    tokio::select! {
        _ = term.recv() => {},
        _ = int.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_validates() {
        assert!(is_valid_slug("foo"));
        assert!(is_valid_slug("foo-bar"));
        assert!(is_valid_slug("a1b2"));
        assert!(!is_valid_slug(""));
        assert!(!is_valid_slug("Foo"));
        assert!(!is_valid_slug("-foo"));
        assert!(!is_valid_slug("foo/bar"));
        assert!(!is_valid_slug(".."));
        assert!(!is_valid_slug("a".repeat(65).as_str()));
    }
}
