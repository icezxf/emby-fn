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
use tokio::sync::RwLock;
use tracing::info;

use crate::config::Config;
use crate::emby::{EmbyClient, EmbyItem};
use crate::fnos::{
    ItemListData, LoginData, LoginReq, MediaItem, MediaLibrary, MediaSource, PlayInfoData,
    PlayInfoReq, Response as FnosResponse,
};

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

pub async fn handle_login(State(state): State<AppState>, Json(req): Json<LoginReq>) -> Response {
    let token = format!("fnos-{:016x}", rand::random::<u64>());
    state
        .tokens
        .write()
        .await
        .insert(token.clone(), state.cfg.emby_user_id.clone());
    info!(
        "🔐 飞牛客户端登录: app_name={}, username={} -> token={}",
        req.app_name, req.username, token
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
    json_response(resp)
}

pub async fn handle_logout() -> Response {
    json_response(FnosResponse::<()>::ok(()))
}

pub async fn handle_mediadb_list(State(state): State<AppState>) -> Response {
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
            info!("📚 返回媒体库列表: {} 个", result.len());
            json_response(FnosResponse::ok(result))
        }
        Err(e) => {
            tracing::error!("❌ 获取 Emby 媒体库失败: {}", e);
            err_response(500, "failed to fetch libraries")
        }
    }
}

#[derive(Deserialize)]
pub struct ItemListQuery {
    #[serde(default)]
    pub lib_guid: String,
    #[serde(default, rename = "type")]
    pub item_type: String,
}

pub async fn handle_item_list(
    State(state): State<AppState>,
    Query(q): Query<ItemListQuery>,
) -> Response {
    let item_types = if q.item_type.is_empty() {
        "Movie"
    } else {
        &q.item_type
    };
    match state.emby.get_items(&q.lib_guid, item_types).await {
        Ok(items) => {
            let list: Vec<MediaItem> = items
                .items
                .iter()
                .map(|item| emby_to_fnos_item(&state.emby, item))
                .collect();
            info!(
                "🎬 返回影片列表: {} 部 (lib={}, type={})",
                list.len(),
                q.lib_guid,
                item_types
            );
            json_response(FnosResponse::ok(ItemListData {
                total: list.len(),
                list,
            }))
        }
        Err(e) => {
            tracing::error!("❌ 获取 Emby 影片列表失败: {}", e);
            err_response(500, "failed to fetch items")
        }
    }
}

pub async fn handle_item_detail(
    State(state): State<AppState>,
    Path(guid): Path<String>,
) -> Response {
    match state.emby.get_item(&guid).await {
        Ok(item) => {
            info!("🎬 返回影片详情: {} ({})", item.name, item.id);
            json_response(FnosResponse::ok(emby_to_fnos_item(&state.emby, &item)))
        }
        Err(e) => {
            tracing::error!("❌ 获取影片详情失败: {}", e);
            err_response(404, "item not found")
        }
    }
}

pub async fn handle_play_info(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let req: PlayInfoReq = serde_json::from_str(&body).unwrap_or(PlayInfoReq {
        item_guid: String::new(),
    });
    if req.item_guid.is_empty() {
        return err_response(400, "item_guid is required");
    }
    let item = match state.emby.get_item(&req.item_guid).await {
        Ok(i) => i,
        Err(e) => {
            tracing::error!("❌ 获取播放信息失败: {}", e);
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
    info!("▶️  返回播放信息: {} -> {}", item.name, play_url);
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

pub async fn handle_video_stream(
    State(state): State<AppState>,
    Path(guid): Path<String>,
) -> Response {
    let stream_url = state.emby.get_stream_url(&guid);
    info!("🎬 视频流重定向: {} -> {}", guid, stream_url);
    Redirect::temporary(&stream_url).into_response()
}

pub async fn handle_task_running() -> Response {
    json_response(FnosResponse::ok(Vec::<serde_json::Value>::new()))
}

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
        .map(|_| emby.get_image_url(&item.id, "Primary"))
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