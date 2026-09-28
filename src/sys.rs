use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use serde::Serialize;
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
pub async fn handle_sys_config(headers: HeaderMap) -> Response {
    info!("📡 [业务] 请求系统配置");

    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1:8007")
        .to_string();

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