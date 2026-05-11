use crate::error::ExitError;
use crate::index;
use crate::slug::is_valid_slug;
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
    let template = String::from_utf8_lossy(&html_bytes);
    let project_name = s
        .project_root
        .file_name()
        .map(|x| x.to_string_lossy().into_owned())
        .unwrap_or_else(|| "project".into());
    let port_str = s.port.to_string();
    let html = render_index_template(&template, &project_name, &port_str, VERSION);

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(html))
        .unwrap()
}

/// 一次性扫描 `{{token}}` 占位符并按位替换，避免顺序 `String::replace` 导致
/// 用户值（如目录名 `foo{{port}}`）被二次替换。
///
/// 识别的 token 只有 `project_name` / `port` / `version`；未识别的 `{{...}}`
/// 原样保留。`project_name` 在写入前经过 `html_escape`。
fn render_index_template(template: &str, project_name: &str, port: &str, version: &str) -> String {
    let mut out = String::with_capacity(template.len() + 64);
    let mut rest = template;
    while let Some(idx) = rest.find("{{") {
        out.push_str(&rest[..idx]);
        let after = &rest[idx + 2..];
        let Some(end) = after.find("}}") else {
            // 模板里有未闭合的 `{{`，原样输出剩余内容并退出。
            out.push_str("{{");
            out.push_str(after);
            return out;
        };
        let token = &after[..end];
        match token {
            "project_name" => out.push_str(&html_escape(project_name)),
            "port" => out.push_str(port),
            "version" => out.push_str(version),
            _ => {
                out.push_str("{{");
                out.push_str(token);
                out.push_str("}}");
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
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

    // Symlink escape 兜底：tower-http 0.5 ServeDir 没有 follow_symlinks(false)。
    // 在 ServeDir 之前先按 rest 的清理后路径做 canonicalize，确认最终目标仍位于
    // serve_root 之内；ServeDir 自己也会再 open 一次，开销可忽略。
    if !rest.is_empty() && !is_inside_serve_root(&serve_root, rest) {
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

/// 在执行 ServeDir 之前，确认 `serve_root / rest` 的 canonical 路径仍位于
/// `serve_root` 的 canonical 路径之内，防止 .playgrounds/<slug>/ 下的 symlink
/// 逃逸到宿主文件系统（tower-http 0.5 ServeDir 没有 follow_symlinks 选项）。
///
/// rest 取自 URL path，未做 percent-decode；ServeDir 自身已经拒绝 `..` /
/// 绝对根 / Windows 盘符等组件，这里只额外验 canonical 仍位于 serve_root 内。
///
/// **TOCTOU 取舍**：本函数在 canonicalize 失败时 fail-open（返回 true），把判断
/// 让给后续 ServeDir。这覆盖了"路径不存在 → 正常 404"的常态；但也意味着
/// 在 canonicalize 与 ServeDir open 之间存在窗口，被攻击者补出文件即逃逸。
/// 本服务定位为单用户本地 dev server（仅绑定 127.0.0.1），该 TOCTOU 接受。
fn is_inside_serve_root(serve_root: &std::path::Path, rest: &str) -> bool {
    let rel = std::path::Path::new(rest);
    let target = serve_root.join(rel);
    let target_can = match std::fs::canonicalize(&target) {
        Ok(p) => p,
        // 路径不存在 → 留给 ServeDir 走正常 404；canonical 校验不在此处提前拒。
        Err(_) => return true,
    };
    let root_can = match std::fs::canonicalize(serve_root) {
        Ok(p) => p,
        Err(_) => return false,
    };
    target_can.starts_with(&root_can)
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
    fn render_replaces_known_tokens() {
        let out = render_index_template(
            "<a>{{project_name}}</a> port {{port}} v{{version}}",
            "demo",
            "4747",
            "0.1.0",
        );
        assert_eq!(out, "<a>demo</a> port 4747 v0.1.0");
    }

    #[test]
    fn render_html_escapes_project_name() {
        let out = render_index_template("{{project_name}}", "a<b>&c\"", "0", "0");
        assert_eq!(out, "a&lt;b&gt;&amp;c&quot;");
    }

    #[test]
    fn render_does_not_re_substitute_user_value() {
        // 用户的项目目录名恰好叫 "foo{{port}}"：第一遍替换 project_name 后，
        // 顺序 String::replace 会把目录名里的 {{port}} 再当作模板 token 替换，
        // 输出 "foo4747"。一次性扫描实现必须保留原样 "foo{{port}}"
        // (html_escape 不动 `{` `}` 字符)。
        let out =
            render_index_template("{{project_name}}-{{port}}", "foo{{port}}", "4747", "0.1.0");
        assert_eq!(out, "foo{{port}}-4747");
    }

    #[test]
    fn render_unknown_token_kept_verbatim() {
        let out = render_index_template("a{{nope}}b", "x", "0", "0");
        assert_eq!(out, "a{{nope}}b");
    }

    #[test]
    fn render_unclosed_brace_kept() {
        let out = render_index_template("a{{port", "x", "0", "0");
        assert_eq!(out, "a{{port");
    }
}
