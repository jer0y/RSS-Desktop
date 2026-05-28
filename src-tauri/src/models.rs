use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feed {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub enabled: bool,
    pub refresh_interval_minutes: i64,
    pub last_checked_at: Option<String>,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: i64,
    pub feed_id: i64,
    pub feed_title: String,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub content_text: String,
    pub content_html: Option<String>,
    pub read: bool,
    pub favorite: bool,
    pub guid: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub window_width: i64,
    pub window_height: i64,
    pub opacity: i64,
    pub margin_top: i64,
    pub margin_right: i64,
    pub global_refresh_interval_minutes: i64,
    pub max_items: i64,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            window_width: 520,
            window_height: 720,
            opacity: 86,
            margin_top: 18,
            margin_right: 18,
            global_refresh_interval_minutes: 15,
            max_items: 80,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedInput {
    pub title: Option<String>,
    pub url: String,
    pub refresh_interval_minutes: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedUpdate {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub enabled: bool,
    pub refresh_interval_minutes: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemQuery {
    pub mode: String,
    pub search: Option<String>,
    pub feed_id: Option<i64>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshSummary {
    pub feed_id: i64,
    pub feed_title: String,
    pub fetched: usize,
    pub inserted: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FeedTestResult {
    pub title: String,
    pub url: String,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshLog {
    pub id: i64,
    pub feed_id: Option<i64>,
    pub feed_title: Option<String>,
    pub status: String,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ParsedFeed {
    pub title: String,
    pub items: Vec<ParsedItem>,
}

#[derive(Debug, Clone)]
pub struct ParsedItem {
    pub guid: String,
    pub title: String,
    pub link: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<String>,
    pub content_html: Option<String>,
    pub content_text: String,
}

pub fn normalize_settings(settings: AppSettings) -> AppSettings {
    AppSettings {
        window_width: clamp(settings.window_width, 360, 900),
        window_height: clamp(settings.window_height, 420, 1100),
        opacity: clamp(settings.opacity, 45, 100),
        margin_top: clamp(settings.margin_top, 0, 200),
        margin_right: clamp(settings.margin_right, 0, 200),
        global_refresh_interval_minutes: clamp(settings.global_refresh_interval_minutes, 5, 240),
        max_items: clamp(settings.max_items, 20, 200),
    }
}

pub fn normalize_refresh_interval(value: i64) -> i64 {
    clamp(value, 5, 240)
}

fn clamp(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}
