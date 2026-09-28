mod config;
mod emby;
mod fnos;
mod handlers;
mod sys;

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::{fmt, EnvFilter};

use config::Config;
use handlers::AppState;

static REQ_COUNTER: AtomicU64 = AtomicU64::new(1);
static VIDEO_COUNTER: AtomicU64 = AtomicU64::new(0);
static IMAGE_COUNTER: AtomicU64 = AtomicU64::new(0);

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(false)
        .init();

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🔧 加载配置");
    let cfg = Config::load();

    let listen_addr = cfg.listen_addr.clone();
    let emby_url = cfg.emby_url.clone();
    let emby_user = cfg.emby_user_id.clone();

    info!("🔧 初始化 AppState");
    let state = AppState::new(cfg);

    info!("🔧 注册路由");
    let app = Router::new()
        .route("/trimcon", get(sys::handle_trimcon))
        .route("/v/api/v1/sys/version", get(sys::handle_sys_version))
        .route("/v/api/v1/sys/config", get(sys::handle_sys_config))
        .route("/v/api/v1/sys/init/status", get(sys::handle_sys_init_status))
        .route("/v/api/v2/sys/init/status", get(sys::handle_sys_init_status))
        .route("/v/api/v2/user/loginByPassword", post(handlers::handle_login_v2))
        .route("/v/api/v1/login", post(handlers::handle_login))
        .route("/v/api/v1/logout", post(handlers::handle_logout))
        .route("/v/api/v1/mediadb/list", get(handlers::handle_mediadb_list))
        .route("/v/api/v1/mediadb/sum", get(handlers::handle_mediadb_list))
        .route("/v/api/v1/item/list", get(handlers::handle_item_list))
        .route("/v/api/v1/item/:guid", get(handlers::handle_item_detail))
        .route("/v/api/v1/play/info", post(handlers::handle_play_info))
        .route("/v/api/v1/task/running", get(handlers::handle_task_running))
        .route("/Videos/:guid/stream", get(handlers::handle_video_stream))
        .route("/Images/:guid", get(handlers::handle_image_proxy))
        .route("/", get(index))
        .fallback(not_found)
        .layer(middleware::from_fn(log_middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("failed to bind address");

    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    info!("🚀 飞牛影视 → Emby 协议网关启动");
    info!("   监听:        {}", listen_addr);
    info!("   Emby 地址:   {}", emby_url);
    info!("   Emby User:   {}", emby_user);
    info!("   客户端填写:  http://<本机IP>:8007");
    info!("   日志级别:    RUST_LOG=info|debug|warn");
    info!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .expect("server error");

    info!("✅ 已关闭");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install Ctrl+C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    info!("🛑 收到退出信号，正在关闭...");
}

async fn log_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let req_id = REQ_COUNTER.fetch_add(1, Ordering::Relaxed);

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|s| s.to_string());
    let version = req.version();
    let is_video = path.starts_with("/Videos/");
    let is_image = path.starts_with("/Images/");

    let ua = header_str(req.headers(), "user-agent");
    let auth = header_str(req.headers(), "authorization");
    let authx = header_str(req.headers(), "authx");
    let referer = header_str(req.headers(), "referer");
    let range = header_str(req.headers(), "range");
    let content_type = header_str(req.headers(), "content-type");

    let client_ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("-").trim().to_string())
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.to_string())
        })
        .unwrap_or_else(|| "-".into());

    let (parts, body) = req.into_parts();
    let body_bytes = if is_video || is_image {
        bytes::Bytes::new()
    } else {
        axum::body::to_bytes(body, 8192)
            .await
            .unwrap_or_else(|_| bytes::Bytes::new())
    };
    let body_len = body_bytes.len();
    let body_preview = if body_len > 0 {
        Some(truncate(&String::from_utf8_lossy(&body_bytes), 500))
    } else {
        None
    };

    let req = Request::from_parts(parts, Body::from(body_bytes));

    let video_sample = env_u64("VIDEO_LOG_SAMPLE", 100).max(1);
    let image_sample = env_u64("IMAGE_LOG_SAMPLE", 50).max(1);

    let log_request = if is_video {
        VIDEO_COUNTER.fetch_add(1, Ordering::Relaxed) % video_sample == 0
    } else if is_image {
        IMAGE_COUNTER.fetch_add(1, Ordering::Relaxed) % image_sample == 0
    } else {
        true
    };

    if log_request && !is_image {
        info!("");
        info!("┌─ [req#{}] {} {} {:?}", req_id, method, path, version);
        info!("│  from        : {}", client_ip);
        info!("│  user-agent  : {}", ua.as_deref().unwrap_or("-"));
        if let Some(q) = &query {
            info!("│  query       : {}", q);
        }
        if let Some(ct) = &content_type {
            info!("│  content-type: {}", ct);
        }
        if let Some(a) = &auth {
            info!("│  authorization: {}", truncate(a, 100));
        }
        if let Some(a) = &authx {
            info!("│  authx       : {}", truncate(a, 120));
        }
        if let Some(r) = &referer {
            info!("│  referer     : {}", truncate(r, 80));
        }
        if let Some(b) = &body_preview {
            info!("│  body ({}B)  : {}", body_len, b);
        }
    }

    let resp = next.run(req).await;

    let status = resp.status().as_u16();
    let elapsed = start.elapsed();

    if is_video {
        if log_request {
            info!(
                "🎬 [req#{}] {} {} → {} ({:?}) Range={}",
                req_id,
                method,
                path,
                status,
                elapsed,
                range.as_deref().unwrap_or("-")
            );
        }
    } else if is_image {
        if log_request {
            info!("🖼️  [req#{}] Images → {} ({:?})", req_id, status, elapsed);
        }
    } else {
        info!("│  → status: {} {} ({:?})", status, reason(status), elapsed);
        info!("└─ [req#{}] done", req_id);
    }

    resp
}

fn header_str(headers: &axum::http::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn truncate(s: &str, n: usize) -> String {
    let s = s.replace('\n', "\\n").replace('\r', "\\r");
    if s.chars().count() <= n {
        s
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{}...", t)
    }
}

fn reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        504 => "Gateway Timeout",
        _ => "",
    }
}

async fn index() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
        INDEX_HTML,
    )
}

async fn not_found(req: Request<Body>) -> impl IntoResponse {
    let ua = header_str(req.headers(), "user-agent").unwrap_or_else(|| "-".into());
    let auth = header_str(req.headers(), "authorization").unwrap_or_else(|| "-".into());
    warn!(
        "❓ 未匹配路由: {} {}  UA={}  Auth={}",
        req.method(),
        req.uri().path(),
        truncate(&ua, 60),
        truncate(&auth, 40)
    );
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        r#"{"code":404,"msg":"not found"}"#,
    )
}

const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="zh-CN"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>飞牛影视 → Emby 网关</title>
<style>
body{margin:0;font-family:-apple-system,"PingFang SC",sans-serif;background:#0f1115;color:#e6e8eb;min-height:100vh}
.wrap{max-width:720px;margin:0 auto;padding:48px 24px}
h1{font-size:22px;margin:0 0 4px}.sub{color:#8a90a0;font-size:13px;margin-bottom:32px}
.status{display:inline-flex;align-items:center;gap:8px;background:#16261a;color:#7ee2a8;padding:6px 14px;border-radius:20px;font-size:13px;margin-bottom:24px}
.dot{width:8px;height:8px;border-radius:50%;background:#3ddc84;animation:p 1.6s infinite}
@keyframes p{50%{opacity:.4}}
.card{background:#161922;border:1px solid #232838;border-radius:12px;padding:20px 22px;margin-bottom:16px}
.card h2{font-size:13px;margin:0 0 12px;color:#9aa2b1;text-transform:uppercase;letter-spacing:.5px}
code{background:#0b0d12;padding:2px 6px;border-radius:4px;color:#7cc7ff;font-size:12px}
a{color:#7cc7ff}
</style></head><body><div class="wrap">
<h1>飞牛影视 → Emby 协议网关</h1>
<div class="sub">fnOS Media Client → Emby Protocol Adapter (Rust)</div>
<div class="status"><span class="dot"></span> 运行中</div>
<div class="card"><h2>飞牛影视客户端配置</h2>
<table style="width:100%;font-size:13px;border-collapse:collapse;">
<tr><td style="color:#8a90a0;padding:6px 0;">服务器地址</td><td><code id="srv"></code></td></tr>
<tr><td style="color:#8a90a0;padding:6px 0;">账号</td><td><code>admin</code>（任意密码）</td></tr>
</table></div>
<div class="card"><h2>自测</h2>
<a href="/trimcon">/trimcon</a><br><br>
<a href="/v/api/v1/sys/version?lan=zh-CN">/v/api/v1/sys/version</a><br><br>
<a href="/v/api/v1/sys/config?lan=zh-CN">/v/api/v1/sys/config</a><br><br>
<a href="/v/api/v1/sys/init/status">/v/api/v1/sys/init/status</a><br><br>
<a href="/v/api/v1/mediadb/list">/v/api/v1/mediadb/list</a><br><br>
<a href="/v/api/v1/item/list?lib_guid=&type=Movie">/v/api/v1/item/list</a>
</div></div><script>document.getElementById('srv').textContent=location.origin;</script>
</body></html>"#;