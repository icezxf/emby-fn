mod config;
mod emby;
mod fnos;
mod handlers;

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use std::time::Instant;
use tokio::signal;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use config::Config;
use handlers::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .with_level(true)
        .init();

    let cfg = Config::load();
    let listen_addr = cfg.listen_addr.clone();
    let emby_url = cfg.emby_url.clone();
    let emby_user = cfg.emby_user_id.clone();
    let state = AppState::new(cfg);

    let app = Router::new()
        .route("/v/api/v1/login", post(handlers::handle_login))
        .route("/v/api/v1/logout", post(handlers::handle_logout))
        .route("/v/api/v1/mediadb/list", get(handlers::handle_mediadb_list))
        .route("/v/api/v1/mediadb/sum", get(handlers::handle_mediadb_list))
        .route("/v/api/v1/item/list", get(handlers::handle_item_list))
        .route("/v/api/v1/item/:guid", get(handlers::handle_item_detail))
        .route("/v/api/v1/play/info", post(handlers::handle_play_info))
        .route("/v/api/v1/task/running", get(handlers::handle_task_running))
        .route("/Videos/:guid/stream", get(handlers::handle_video_stream))
        .route("/", get(index))
        .fallback(not_found)
        .layer(middleware::from_fn(log_middleware))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen_addr).await.expect("failed to bind address");
    info!("🚀 飞牛影视 → Emby 协议网关启动");
    info!("   监听:       {}", listen_addr);
    info!("   Emby 地址:  {}", emby_url);
    info!("   Emby User:  {}", emby_user);
    info!("   飞牛客户端填写: http://<本机IP>:8007");

    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await.expect("server error");
    info!("✅ 已关闭");
}

async fn shutdown_signal() {
    let ctrl_c = async { signal::ctrl_c().await.expect("install Ctrl+C handler"); };
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler").recv().await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    info!("🛑 收到退出信号，正在关闭...");
}

async fn log_middleware(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|s| s.to_string());
    let ua = req.headers().get("user-agent").and_then(|v| v.to_str().ok()).map(|s| s.to_string());
    let is_video = path.starts_with("/Videos/");
    if !is_video {
        info!("➡️  {} {}", method, path);
        if let Some(q) = &query { info!("    Query: {}", q); }
        if let Some(ua) = &ua { info!("    UA: {}", truncate(ua, 60)); }
    }
    let resp = next.run(req).await;
    if is_video { info!("🎬 {} {} ({:?})", method, path, start.elapsed()); }
    resp
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n { s.to_string() } else { format!("{}...", &s[..n]) }
}

async fn index() -> impl IntoResponse {
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")], INDEX_HTML)
}

async fn not_found(req: Request<Body>) -> impl IntoResponse {
    tracing::warn!("❓ 未匹配的请求: {} {}", req.method(), req.uri().path());
    (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "application/json")], r#"{"code":404,"msg":"not found"}"#)
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
<a href="/v/api/v1/mediadb/list">/v/api/v1/mediadb/list</a><br><br>
<a href="/v/api/v1/item/list?lib_guid=&type=Movie">/v/api/v1/item/list</a>
</div></div><script>document.getElementById('srv').textContent=location.origin;</script>
</body></html>"#;
