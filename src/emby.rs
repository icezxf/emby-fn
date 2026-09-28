use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, info, warn};

#[derive(Clone)]
pub struct EmbyClient {
    pub base_url: String,
    pub api_key: String,
    pub user_id: String,
    pub http: Arc<reqwest::Client>,
}

#[derive(Debug, Deserialize)]
pub struct EmbyItemsResponse {
    #[serde(rename = "Items", default)]
    pub items: Vec<EmbyItem>,
    #[serde(rename = "TotalRecordCount", default)]
    pub total: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EmbyItem {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Type", default)]
    pub item_type: String,
    #[serde(rename = "ProductionYear", default)]
    pub production_year: i32,
    #[serde(rename = "RunTimeTicks", default)]
    pub runtime_ticks: i64,
    #[serde(rename = "Overview", default)]
    pub overview: String,
    #[serde(rename = "ImageTags", default)]
    pub image_tags: Option<serde_json::Value>,
    #[serde(rename = "CommunityRating", default)]
    pub community_rating: f64,
    #[serde(rename = "ProviderIds", default)]
    pub provider_ids: Option<serde_json::Value>,
}

impl EmbyClient {
    pub fn new(base_url: String, api_key: String, user_id: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");
        info!(
            "🔧 EmbyClient 初始化: base_url={}, user_id={}, api_key={}",
            base_url,
            user_id,
            mask_bare_key(&api_key)
        );
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            user_id,
            http: Arc::new(http),
        }
    }

    pub async fn get_libraries(&self) -> Result<EmbyItemsResponse, String> {
        let url = format!("{}/Users/{}/Views", self.base_url, self.user_id);
        info!("🌐 [Emby] GET Views (媒体库列表)");
        self.get_json(&url).await
    }

    pub async fn get_items(
        &self,
        parent_id: &str,
        item_types: &str,
    ) -> Result<EmbyItemsResponse, String> {
        let url = format!(
            "{}/Users/{}/Items?ParentId={}&Recursive=true&IncludeItemTypes={}&Fields=Overview,ProductionYear,RunTimeTicks,CommunityRating,ProviderIds,ImageTags&SortBy=SortName&SortOrder=Ascending",
            self.base_url,
            self.user_id,
            urlencoding::encode(parent_id),
            urlencoding::encode(item_types),
        );
        info!(
            "🌐 [Emby] GET Items (parent={}, types={})",
            parent_id, item_types
        );
        self.get_json(&url).await
    }

    pub async fn get_item(&self, item_id: &str) -> Result<EmbyItem, String> {
        let url = format!("{}/Users/{}/Items/{}", self.base_url, self.user_id, item_id);
        info!("🌐 [Emby] GET Item detail (id={})", item_id);
        let resp: EmbyItemsResponse = self.get_json(&url).await?;
        resp.items
            .into_iter()
            .next()
            .ok_or_else(|| "item not found".to_string())
    }

    pub fn get_image_url(&self, item_id: &str, image_type: &str) -> String {
        format!(
            "{}/Items/{}/Images/{}?maxWidth=500&tag=1",
            self.base_url, item_id, image_type
        )
    }

    pub fn get_stream_url(&self, item_id: &str) -> String {
        let url = format!(
            "{}/Videos/{}/stream?Static=true&api_key={}",
            self.base_url, item_id, self.api_key
        );
        info!(
            "🎬 [Emby] 生成流地址: item_id={}, url={}",
            item_id,
            mask_key(&url)
        );
        url
    }

    async fn get_json(&self, url: &str) -> Result<EmbyItemsResponse, String> {
        let start = Instant::now();
        let masked = mask_key(url);
        debug!("    → 请求 URL: {}", masked);

        let resp = match self
            .http
            .get(url)
            .header("X-Emby-Token", &self.api_key)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    "    ✗ Emby 请求失败 ({}ms): {}",
                    start.elapsed().as_millis(),
                    e
                );
                return Err(format!("request error: {}", e));
            }
        };

        let status = resp.status();
        let body = match resp.text().await {
            Ok(b) => b,
            Err(e) => {
                warn!("    ✗ 读取响应体失败: {}", e);
                return Err(format!("read body error: {}", e));
            }
        };
        let elapsed = start.elapsed().as_millis();
        let body_len = body.len();

        if !status.is_success() {
            warn!(
                "    ✗ Emby 返回错误 ({}ms): HTTP {} — {}",
                elapsed,
                status,
                truncate(&body, 300)
            );
            return Err(format!("emby api error: {} {}", status, body));
        }

        debug!(
            "    ← Emby 响应 ({}ms, {}B): {}",
            elapsed,
            body_len,
            truncate(&body, 400)
        );

        if let Ok(list) = serde_json::from_str::<EmbyItemsResponse>(&body) {
            if !list.items.is_empty() {
                debug!(
                    "    ✓ 解析到 {} 个条目 (总数 {}), 耗时 {}ms",
                    list.items.len(),
                    list.total,
                    elapsed
                );
                return Ok(list);
            }
        }

        if let Ok(single) = serde_json::from_str::<EmbyItem>(&body) {
            if !single.id.is_empty() {
                debug!(
                    "    ✓ 解析到单个条目: {} ({}), 耗时 {}ms",
                    single.name, single.id, elapsed
                );
                return Ok(EmbyItemsResponse {
                    items: vec![single],
                    total: 1,
                });
            }
        }

        warn!(
            "    ⚠ Emby 返回无法解析为空结果 ({}ms, {}B): {}",
            elapsed,
            body_len,
            truncate(&body, 200)
        );
        Ok(EmbyItemsResponse {
            items: vec![],
            total: 0,
        })
    }
}

/// 把 URL 里的 api_key=xxx 替换成 api_key=***
fn mask_key(s: &str) -> String {
    if let Some(pos) = s.find("api_key=") {
        let (head, tail) = s.split_at(pos + 8);
        let end = tail.find('&').unwrap_or(tail.len());
        let (_, rest) = tail.split_at(end);
        format!("{}***{}", head, rest)
    } else {
        s.to_string()
    }
}

/// 掩码裸 API Key，保留前 4 后 4 字符
fn mask_bare_key(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= 8 {
        "***".into()
    } else {
        format!(
            "{}***{}",
            chars[..4].iter().collect::<String>(),
            chars[chars.len() - 4..].iter().collect::<String>()
        )
    }
}

fn truncate(s: &str, n: usize) -> String {
    let s = s.replace('\n', " ").replace('\r', " ");
    if s.chars().count() <= n {
        s
    } else {
        let t: String = s.chars().take(n).collect();
        format!("{}...", t)
    }
}