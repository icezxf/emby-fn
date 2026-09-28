use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Serialize;
use std::net::SocketAddr;
use tracing::info;

use crate::fnos::Response as FnosResponse;

#[derive(Serialize)]
pub struct VersionData {
    pub version: String,
    pub build: String,
    pub build_time: String,
    pub server_name: String,
    pub api_version: String,
    pub platform: String,
}

/// GET /v/api/v1/sys/version?lan=zh-CN
/// 只返回版本基础信息，不要加 initialized/setup_completed
pub async fn handle_sys_version() -> Response {
    info!("📡 [业务] 请求系统版本");

    let data = VersionData {
        version: "1.0.0".into(),
        build: "100000".into(),
        build_time: "2026-01-01 00:00:00".into(),
        server_name: "fnOS-Mock".into(),
        api_version: "v1".into(),
        platform: "linux".into(),
    };

    let resp = FnosResponse::ok(data);
    let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());

    info!("    ✓ 已返回版本 1.0.0");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// GET /v/api/v1/sys/config?lan=zh-CN
/// 塞入多个候选字段用于探测客户端在哪里拿 WS 地址
pub async fn handle_sys_config(
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    info!("📡 [业务] 请求系统配置");

    // 优先从 Host 头拿，拿不到用请求来源 IP
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}:8007", addr.ip()));

    let data = serde_json::json!({
        "initialized": true,
        "setup_completed": true,
        "need_init": false,
        "need_setup": false,
        "transcode_enabled": true,
        "subtitle_enabled": true,
        "danmaku_enabled": false,
        "download_enabled": true,
        "upload_enabled": false,
        "allow_register": false,
        "upload_max_size": 10737418240u64,
        "theme": "dark",
        "default_language": "zh-CN",
        "player": {
            "hardware_decode": true,
            "auto_play": true,
            "default_quality": "auto"
        },

        // ---- 候选 WS 地址字段 ----
        "ws_url": format!("ws://{}/websocket", host),
        "websocket": format!("ws://{}/websocket", host),
        "websocket_url": format!("ws://{}/websocket", host),
        "ws": format!("ws://{}/websocket", host),
        "websocket_port": 8007,
        "ws_port": 8007,
        "server_url": format!("http://{}", host),
    });

    let resp = FnosResponse::ok(data);
    let body = serde_json::to_string(&resp).unwrap_or_else(|_| "{}".into());

    info!("    ✓ 已返回系统配置");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// GET /trimcon
pub async fn handle_trimcon() -> Json<serde_json::Value> {
    info!("📡 [业务] 连接探测 /trimcon");
    Json(serde_json::json!({
        "code": 0,
        "msg": "",
        "data": {
            "ok": true,
            "server": "fnOS-Mock"
        }
    }))
}