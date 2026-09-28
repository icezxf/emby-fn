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

/// GET /v/api/v1/sys/version?lan=zh-CN
/// 飞牛客户端启动后第一个真正打签名的接口，用来确认服务端版本
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

/// GET /trimcon
/// 客户端用这个接口探测服务端是否可连通，返回任意成功即可
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