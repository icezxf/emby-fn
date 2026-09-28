use axum::{
    body::Body,
    http::{header, StatusCode},
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
    pub initialized: bool,
    pub setup_completed: bool,
}

#[derive(Serialize)]
pub struct SysConfigData {
    pub initialized: bool,
    pub setup_completed: bool,
    pub need_init: bool,
    pub need_setup: bool,
    pub transcode_enabled: bool,
    pub subtitle_enabled: bool,
    pub danmaku_enabled: bool,
    pub download_enabled: bool,
    pub upload_enabled: bool,
    pub allow_register: bool,
    pub upload_max_size: u64,
    pub theme: String,
    pub default_language: String,
    pub player: PlayerConfig,
}

#[derive(Serialize)]
pub struct PlayerConfig {
    pub hardware_decode: bool,
    pub auto_play: bool,
    pub default_quality: String,
}

#[derive(Serialize)]
pub struct InitStatusData {
    pub initialized: bool,
    pub setup_completed: bool,
    pub need_init: bool,
    pub need_setup: bool,
    pub has_admin: bool,
    pub version: String,
}

/// GET /v/api/v1/sys/version
pub async fn handle_sys_version() -> Response {
    info!("📡 [业务] 请求系统版本");

    let data = VersionData {
        version: "1.0.0".into(),
        build: "100000".into(),
        build_time: "2026-01-01 00:00:00".into(),
        server_name: "fnOS-Mock".into(),
        api_version: "v1".into(),
        platform: "linux".into(),
        initialized: true,
        setup_completed: true,
    };

    let body = serde_json::to_string(&FnosResponse::ok(data)).unwrap_or_else(|_| "{}".into());
    info!("    ✓ 已返回版本 1.0.0 (initialized=true)");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// GET /v/api/v1/sys/config
pub async fn handle_sys_config() -> Response {
    info!("📡 [业务] 请求系统配置");

    let data = SysConfigData {
        initialized: true,
        setup_completed: true,
        need_init: false,
        need_setup: false,
        transcode_enabled: true,
        subtitle_enabled: true,
        danmaku_enabled: false,
        download_enabled: true,
        upload_enabled: false,
        allow_register: false,
        upload_max_size: 10 * 1024 * 1024 * 1024,
        theme: "dark".into(),
        default_language: "zh-CN".into(),
        player: PlayerConfig {
            hardware_decode: true,
            auto_play: true,
            default_quality: "auto".into(),
        },
    };

    let body = serde_json::to_string(&FnosResponse::ok(data)).unwrap_or_else(|_| "{}".into());
    info!("    ✓ 已返回系统配置 (initialized=true)");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

/// GET /v/api/v1/sys/init/status
/// 客户端可能用它检查服务端是否需要初始化
pub async fn handle_sys_init_status() -> Response {
    info!("📡 [业务] 请求初始化状态");

    let data = InitStatusData {
        initialized: true,
        setup_completed: true,
        need_init: false,
        need_setup: false,
        has_admin: true,
        version: "1.0.0".into(),
    };

    let body = serde_json::to_string(&FnosResponse::ok(data)).unwrap_or_else(|_| "{}".into());
    info!("    ✓ 已返回初始化状态 (initialized=true)");

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
            "server": "fnOS-Mock",
            "initialized": true
        }
    }))
}