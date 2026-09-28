use serde::Deserialize;
use std::sync::Arc;

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
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            user_id,
            http: Arc::new(http),
        }
    }

    pub async fn get_libraries(&self) -> Result<EmbyItemsResponse, String> {
        let url = format!("{}/Users/{}/Views", self.base_url, self.user_id);
        self.get_json(&url).await
    }

    pub async fn get_items(&self, parent_id: &str, item_types: &str) -> Result<EmbyItemsResponse, String> {
        let url = format!(
            "{}/Users/{}/Items?ParentId={}&Recursive=true&IncludeItemTypes={}&Fields=Overview,ProductionYear,RunTimeTicks,CommunityRating,ProviderIds,ImageTags&SortBy=SortName&SortOrder=Ascending",
            self.base_url, self.user_id,
            urlencoding::encode(parent_id),
            urlencoding::encode(item_types),
        );
        self.get_json(&url).await
    }

    pub async fn get_item(&self, item_id: &str) -> Result<EmbyItem, String> {
        let url = format!("{}/Users/{}/Items/{}", self.base_url, self.user_id, item_id);
        let resp: EmbyItemsResponse = self.get_json(&url).await?;
        resp.items.into_iter().next().ok_or_else(|| "item not found".to_string())
    }

    pub fn get_image_url(&self, item_id: &str, image_type: &str) -> String {
        format!("{}/Items/{}/Images/{}?maxWidth=500&tag=1", self.base_url, item_id, image_type)
    }

    pub fn get_stream_url(&self, item_id: &str) -> String {
        format!("{}/Videos/{}/stream?Static=true&api_key={}", self.base_url, item_id, self.api_key)
    }

    async fn get_json(&self, url: &str) -> Result<EmbyItemsResponse, String> {
        let resp = self.http.get(url)
            .header("X-Emby-Token", &self.api_key)
            .send().await
            .map_err(|e| format!("request error: {}", e))?;
        let status = resp.status();
        let body = resp.text().await.map_err(|e| format!("read body error: {}", e))?;
        if !status.is_success() {
            return Err(format!("emby api error: {} {}", status, body));
        }
        if let Ok(list) = serde_json::from_str::<EmbyItemsResponse>(&body) {
            if !list.items.is_empty() {
                return Ok(list);
            }
        }
        if let Ok(single) = serde_json::from_str::<EmbyItem>(&body) {
            if !single.id.is_empty() {
                return Ok(EmbyItemsResponse { items: vec![single], total: 1 });
            }
        }
        Ok(EmbyItemsResponse { items: vec![], total: 0 })
    }
}
