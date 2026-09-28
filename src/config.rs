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
        let emby_url = env::var("EMBY_URL").unwrap_or_else(|_| "http://127.0.0.1:8096".into());
        let emby_api_key = env::var("EMBY_API_KEY").unwrap_or_default();
        let emby_user_id = env::var("EMBY_USER_ID").unwrap_or_default();
        let listen_addr = env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8007".into());

        // 启动配置自检
        if emby_url.is_empty() {
            tracing::error!("❌ EMBY_URL 未设置");
        } else {
            tracing::info!("✓ EMBY_URL     = {}", emby_url);
        }
        if emby_api_key.is_empty() {
            tracing::error!("❌ EMBY_API_KEY 未设置");
        } else {
            tracing::info!("✓ EMBY_API_KEY = {}", mask(&emby_api_key));
        }
        if emby_user_id.is_empty() {
            tracing::error!("❌ EMBY_USER_ID 未设置");
        } else {
            tracing::info!("✓ EMBY_USER_ID = {}", emby_user_id);
        }
        tracing::info!("✓ LISTEN_ADDR  = {}", listen_addr);
        tracing::info!("✓ LOG_LEVEL    = {}", env::var("RUST_LOG").unwrap_or_else(|_| "info".into()));

        Self {
            emby_url,
            emby_api_key,
            emby_user_id,
            listen_addr,
        }
    }
}

fn mask(s: &str) -> String {
    if s.len() <= 8 {
        "***".into()
    } else {
        format!("{}***{}", &s[..4], &s[s.len() - 4..])
    }
}