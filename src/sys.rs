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

/// GET /v/api/v1/sys/version?lan=zh-CN
/// 只返回版本基础信息，不要加 initialized/setup_completed，会干扰客户端流程
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
/// initialized/setup_completed 只在这里出现
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