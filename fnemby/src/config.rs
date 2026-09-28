use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub emby_url: String,
    pub emby_api_key: String,
    pub emby_user_id: String,
    pub listen_addr: String,
}

impl Config {
    pub fn load() -> Self {
        let emby_api_key = env::var("EMBY_API_KEY").unwrap_or_default();
        if emby_api_key.is_empty() {
            tracing::warn!("EMBY_API_KEY 未设置，无法连接 Emby");
        }
        Self {
            emby_url: env::var("EMBY_URL").unwrap_or_else(|_| "http://127.0.0.1:8096".into()),
            emby_api_key,
            emby_user_id: env::var("EMBY_USER_ID").unwrap_or_default(),
            listen_addr: env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8007".into()),
        }
    }
}
