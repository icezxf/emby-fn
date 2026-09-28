use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct Response<T> {
    pub code: i32,
    pub msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
}

impl<T> Response<T> {
    pub fn ok(data: T) -> Self {
        Self { code: 0, msg: String::new(), message: None, data: Some(data) }
    }
    pub fn err(code: i32, msg: &str) -> Response<()> {
        Response { code, msg: msg.to_string(), message: None, data: None }
    }
}

#[derive(Serialize)]
pub struct LoginData {
    pub token: String,
    pub access_token: String,
    pub user_id: String,
    pub user_guid: String,
    pub username: String,
    pub user_name: String,
    pub is_admin: i32,
}

#[derive(Serialize)]
pub struct MediaLibrary {
    pub guid: String,
    pub title: String,
    #[serde(rename = "type")]
    pub lib_type: String,
}

#[derive(Serialize, Clone)]
pub struct MediaItem {
    pub guid: String,
    pub title: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub production_year: i32,
    pub runtime: i32,
    pub overview: String,
    pub poster: String,
    pub vote_average: String,
    pub imdb_id: String,
}

#[derive(Serialize)]
pub struct ItemListData {
    pub list: Vec<MediaItem>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct PlayInfoData {
    pub url: String,
    pub protocol: String,
    pub format: String,
    pub media_source: MediaSource,
}

#[derive(Serialize)]
pub struct MediaSource {
    pub id: String,
    pub path: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
}

#[derive(Deserialize)]
pub struct PlayInfoReq {
    #[serde(default)]
    pub item_guid: String,
}
