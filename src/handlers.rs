use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    Json,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::config::Config;
use crate::emby::{EmbyClient, EmbyItem};
use crate::fnos::{
    ItemListData, LoginData, LoginReq, MediaItem, MediaLibrary, MediaSource, PlayInfoData,
    PlayInfoReq, Response as FnosResponse,
};

const MAX_TOKENS: usize = 1000;

#[derive(Clone)]
pub struct AppState {
    pub cfg: Arc<Config>,
    pub emby: EmbyClient,
    pub tokens: Arc<RwLock<HashMap<String, String>>>,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        let emby = EmbyClient::new(
            cfg.emby_url.clone(),
            cfg.emby_api_key.clone(),
            cfg.emby_user_id.clone(),
        );
        Self {
            cfg: Arc::new(cfg),
            emby,
            tokens: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_token(&self, token: String) -> usize {
        let mut map = self.tokens.write().await;
        if map.len() >= MAX_TOKENS {
            let removed = map.len() / 2;
            let keys: Vec<String> = map.keys().take(removed).cloned().collect();
            for k in keys {
                map.remove(&k);
            }
            warn!("⚠️  Token 数量超限，清理了 {} 个旧 token", removed);
        }
        map.insert(token, self.cfg.emby_user_id.clone());
        map.len()
    }

    pub async fn remove_token(&self, token: &str) -> usize {
        let mut map = self.tokens.write().await;
        map.remove(token);
        map.len()
    }

    pub async fn token_count(&self) -> usize {
        self.tokens.read().await.len()
    }
}

fn json_response<T: serde::Serialize>(v: T) -> Response {
    let body = serde_json::to_string(&v).unwrap_or_else(|_| "{}".into());
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json; charset=utf-8")
        .body(Body::from(body))
        .unwrap()
}

fn err_response(code: i32, msg: &str) -> Response {
    json_response(FnosResponse::<()>::err(code, msg))
}

// ============================================================
// v1 登录（老客户端）
// ============================================================
pub async fn handle_login(State(state): State<AppState>, Json(req): Json<LoginReq>) -> Response {
    let start = Instant::now();
    let token = format!("fnos-{:016x}", rand::random::<u64>());
    let total = state.add_token(token.clone()).await;

    info!(
        "🔐 [业务] v1 登录成功: app={:?}, username={:?}, pwd_len={}, token={}, 当前在线 {}",
        req.app_name,
        req.username,
        req.password.len(),
        token,
        total
    );

    let mut resp = FnosResponse::<LoginData>::ok(LoginData {
        token: token.clone(),
        access_token: token,
        user_id: state.cfg.emby_user_id.clone(),
        user_guid: state.cfg.emby_user_id.clone(),
        username: "admin".into(),
        user_name: "admin".into(),
        is_admin: 1,
    });
    resp.message = Some("success".into());
    info!("    ✓ v1 登录响应已生成 ({:?})", start.elapsed());
    json_response(resp)
}

// ============================================================
// v2 登录：POST /v/api/v2/user/loginByPassword
// 新版客户端走这里，密码是 SHA256 哈希
// ============================================================
#[derive(Deserialize)]
pub struct LoginV2Req {
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub lan: String,
}

pub async fn handle_login_v2(
    State(state): State<AppState>,
    Json(req): Json<LoginV2Req>,
) -> Response {
    let start = Instant::now();
    let token = format!("fnos-{:016x}", rand::random::<u64>());
    let secret = format!(
        "{:016x}{:016x}",
        rand::random::<u64>(),
        rand::random::<u64>()
    );
    let total = state.add_token(token.clone()).await;

    info!(
        "🔐 [业务] v2 登录成功: app={:?}, device={:?}, username={:?}, pwd_len={}, token={}, 当前在线 {}",
        req.app_name,
        req.device,
        req.username,
        req.password.len(),
        token,
        total
    );

    let user_name = if req.username.is_empty() {
        "admin".to_string()
    } else {
        req.username.clone()
    };

    let data = serde_json::json!({
        "token": token,
        "access_token": token,
        "secret": secret,
        "secret_string": secret,
        "user_id": state.cfg.emby_user_id,
        "user_guid": state.cfg.emby_user_id,
        "username": user_name,
        "user_name": user_name,
        "nickname": user_name,
        "is_admin": 1,
        "is_admin_user": 1,
        "role": "admin",
        "status": 1,
        "initialized": true,
        "avatar": "",
        "email": "",
        "created_at": "2026-01-01T00:00:00Z",
        "last_login": "2026-01-01T00:00:00Z",
        "user": {
            "id": state.cfg.emby_user_id,
            "guid": state.cfg.emby_user_id,
            "username": user_name,
            "nickname": user_name,
            "is_admin": 1,
            "role": "admin",
            "status": 1
        }
    });

    let mut resp = FnosResponse::ok(data);
    resp.message = Some("success".into());
    info!("    ✓ v2 登录响应已生成 ({:?})", start.elapsed());
    json_response(resp)
}

pub async fn handle_logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = extract_token(&headers);
    match token {
        Some(t) => {
            let left = state.remove_token(&t).await;
            info!("🚪 [业务] 退出登录: token={}, 剩余 {} 个", t, left);
        }
        None => info!("🚪 [业务] 退出登录 (无 token)"),
    }
    json_response(FnosResponse::<()>::ok(()))
}

// ============================================================
// 媒体库列表
// ============================================================
pub async fn handle_mediadb_list(State(state): State<AppState>) -> Response {
    let start = Instant::now();
    info!("📚 [业务] 请求媒体库列表");

    match state.emby.get_libraries().await {
        Ok(libs) => {
            let result: Vec<MediaLibrary> = libs
                .items
                .iter()
                .map(|item| {
                    let lib_type = match item.item_type.as_str() {
                        "Series" => "TV",
                        _ => "Movie",
                    };
                    MediaLibrary {
                        guid: item.id.clone(),
                        title: item.name.clone(),
                        lib_type: lib_type.into(),
                    }
                })
                .collect();

            for lib in &result {
                info!("    · [{}] {} ({})", lib.lib_type, lib.title, lib.guid);
            }
            info!(
                "    ✓ 返回 {} 个媒体库, 总耗时 {:?}",
                result.len(),
                start.elapsed()
            );
            json_response(FnosResponse::ok(result))
        }
        Err(e) => {
            warn!(
                "    ✗ 获取 Emby 媒体库失败 ({}ms): {}",
                start.elapsed().as_millis(),
                e
            );
            err_response(500, "failed to fetch libraries")
        }
    }
}

// ============================================================
// 影片列表
// ============================================================
#[derive(Deserialize)]
pub struct ItemListQuery {
    #[serde(default)]
    pub lib_guid: String,
    #[serde(default, rename = "type")]
    pub item_type: String,
    #[serde(default)]
    pub offset: Option<u32>,
    #[serde(default)]
    pub limit: Option<u32>,
}

pub async fn handle_item_list(
    State(state): State<AppState>,
    Query(q): Query<ItemListQuery>,
) -> Response {
    let start = Instant::now();
    let item_types = if q.item_type.is_empty() {
        "Movie"
    } else {
        &q.item_type
    };
    info!(
        "🎬 [业务] 请求影片列表: lib={:?}, type={}, offset={:?}, limit={:?}",
        q.lib_guid, item_types, q.offset, q.limit
    );

    match state.emby.get_items(&q.lib_guid, item_types).await {
        Ok(items) => {
            let list: Vec<MediaItem> = items
                .items
                .iter()
                .map(|item| emby_to_fnos_item(&state.emby, item))
                .collect();

            for (i, item) in list.iter().take(5).enumerate() {
                info!(
                    "    {}. {} ({}) [{}年] 时长{}min 评分{}",
                    i + 1,
                    item.title,
                    item.guid,
                    item.production_year,
                    item.runtime,
                    item.vote_average
                );
            }
            if list.len() > 5 {
                info!("    ... 还有 {} 条未显示", list.len() - 5);
            }
            info!(
                "    ✓ 返回 {} 部影片, 总耗时 {:?}",
                list.len(),
                start.elapsed()
            );
            json_response(FnosResponse::ok(ItemListData {
                total: list.len(),
                list,
            }))
        }
        Err(e) => {
            warn!(
                "    ✗ 获取 Emby 影片列表失败 ({}ms): {}",
                start.elapsed().as_millis(),
                e
            );
            err_response(500, "failed to fetch items")
        }
    }
}

// ============================================================
// 影片详情
// ============================================================
pub async fn handle_item_detail(
    State(state): State<AppState>,
    Path(guid): Path<String>,
) -> Response {
    let start = Instant::now();
    info!("📽️  [业务] 请求影片详情: guid={}", guid);

    match state.emby.get_item(&guid).await {
        Ok(item) => {
            let f = emby_to_fnos_item(&state.emby, &item);
            info!("    · 标题:     {}", f.title);
            info!("    · GUID:     {}", f.guid);
            info!("    · 类型:     {}", f.item_type);
            info!("    · 年份:     {}", f.production_year);
            info!("    · 时长:     {} 分钟", f.runtime);
            info!("    · 评分:     {}", f.vote_average);
            info!("    · IMDb:     {}", f.imdb_id);
            info!(
                "    · 海报:     {}",
                if f.poster.is_empty() { "(无)" } else { &f.poster }
            );
            info!("    · 简介:     {}", truncate(&f.overview, 80));
            info!("    ✓ 完成, 耗时 {:?}", start.elapsed());
            json_response(FnosResponse::ok(f))
        }
        Err(e) => {
            warn!(
                "    ✗ 获取影片详情失败 ({}ms): {}",
                start.elapsed().as_millis(),
                e
            );
            err_response(404, "item not found")
        }
    }
}

// ============================================================
// 播放信息
// ============================================================
pub async fn handle_play_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let start = Instant::now();

    info!("▶️  [业务] 请求播放信息");
    info!("    body: {}", truncate(&body, 300));

    let req: PlayInfoReq = match serde_json::from_str(&body) {
        Ok(r) => r,
        Err(e) => {
            warn!("    ✗ JSON 解析失败: {}", e);
            return err_response(400, "invalid json");
        }
    };

    if req.item_guid.is_empty() {
        warn!("    ✗ item_guid 为空");
        return err_response(400, "item_guid is required");
    }
    info!("    item_guid: {}", req.item_guid);

    if let Some(authx) = headers.get("authx").and_then(|v| v.to_str().ok()) {
        info!("    Authx:         {}", truncate(authx, 100));
    }
    if let Some(auth) = headers.get(header::AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        info!("    Authorization: {}", truncate(auth, 80));
    }
    if let Some(ua) = headers.get(header::USER_AGENT).and_then(|v| v.to_str().ok()) {
        info!("    User-Agent:    {}", truncate(ua, 80));
    }

    let item = match state.emby.get_item(&req.item_guid).await {
        Ok(i) => i,
        Err(e) => {
            warn!("    ✗ 获取影片失败: {}", e);
            return err_response(404, "item not found");
        }
    };

    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("127.0.0.1:8007");
    let scheme = if headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("http")
        == "https"
    {
        "https"
    } else {
        "http"
    };
    let play_url = format!("{}://{}/Videos/{}/stream", scheme, host, req.item_guid);

    info!("    · 影片:     {} ({})", item.name, item.id);
    info!("    · 播放地址: {}", play_url);
    info!("    ✓ 完成, 耗时 {:?}", start.elapsed());

    json_response(FnosResponse::ok(PlayInfoData {
        url: play_url.clone(),
        protocol: "http".into(),
        format: "mp4".into(),
        media_source: MediaSource {
            id: format!("emby-{}", req.item_guid),
            path: play_url,
        },
    }))
}

// ============================================================
// 视频流
// ============================================================
pub async fn handle_video_stream(
    State(state): State<AppState>,
    Path(guid): Path<String>,
) -> Response {
    let stream_url = state.emby.get_stream_url(&guid);
    info!("🎥 [业务] 视频流重定向: guid={}", guid);
    Redirect::temporary(&stream_url).into_response()
}

// ============================================================
// 图片代理
// ============================================================
pub async fn handle_image_proxy(
    State(state): State<AppState>,
    Path(guid): Path<String>,
    headers: HeaderMap,
) -> Response {
    let img_type = headers
        .get("x-image-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Primary");
    let url = state.emby.get_image_url(&guid, img_type);
    info!("🖼️  [业务] 图片代理: guid={}, type={}", guid, img_type);
    Redirect::temporary(&url).into_response()
}

// ============================================================
// 任务状态
// ============================================================
pub async fn handle_task_running() -> Response {
    info!("⏳ [业务] 请求任务状态");
    json_response(FnosResponse::ok(Vec::<serde_json::Value>::new()))
}

// ============================================================
// 工具
// ============================================================
fn emby_to_fnos_item(emby: &EmbyClient, item: &EmbyItem) -> MediaItem {
    let runtime = (item.runtime_ticks / 10_000_000) as i32;
    let vote = if item.community_rating > 0.0 {
        format!("{:.1}", item.community_rating)
    } else {
        String::new()
    };
    let imdb_id = item
        .provider_ids
        .as_ref()
        .and_then(|v| v.get("Imdb"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let poster = item
        .image_tags
        .as_ref()
        .and_then(|v| v.get("Primary"))
        .map(|_| format!("/Images/{}", item.id))
        .unwrap_or_default();
    MediaItem {
        guid: item.id.clone(),
        title: item.name.clone(),
        item_type: item.item_type.clone(),
        production_year: item.production_year,
        runtime,
        overview: item.overview.clone(),
        poster,
        vote_average: vote,
        imdb_id,
    }
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim_start_matches("Bearer ").to_string())
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