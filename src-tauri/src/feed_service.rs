use anyhow::{Context, Result};
use feed_rs::parser;
use reqwest::Client;
use sha2::{Digest, Sha256};
use url::Url;

use crate::models::{ParsedFeed, ParsedItem};

const MAX_FEED_BYTES: u64 = 2_000_000;
const MAX_CONTENT_CHARS: usize = 100_000;
const MAX_TEXT_CHARS: usize = 20_000;
const MAX_FETCH_ATTEMPTS: u32 = 3;

pub fn validate_http_url(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let url = Url::parse(trimmed).context("URL 格式无效")?;
    match url.scheme() {
        "http" | "https" => Ok(url.to_string()),
        _ => Err(anyhow::anyhow!("仅支持 http 或 https RSS 地址")),
    }
}

pub async fn fetch_feed(client: &Client, raw_url: &str) -> Result<ParsedFeed> {
    let url = validate_http_url(raw_url)?;
    let bytes = fetch_bytes_with_retries(client, &url).await?;

    parse_feed(&url, &bytes)
}

async fn fetch_bytes_with_retries(client: &Client, url: &str) -> Result<Vec<u8>> {
    let mut last_error = "未知网络错误".to_string();

    for attempt in 1..=MAX_FETCH_ATTEMPTS {
        let result = client
            .get(url)
            .header(
                reqwest::header::ACCEPT,
                "application/rss+xml, application/atom+xml, application/xml, text/xml;q=0.9, */*;q=0.8",
            )
            .send()
            .await;

        match result {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    last_error = format!("订阅源返回 HTTP {}", status.as_u16());
                } else if let Some(length) = response.content_length() {
                    if length > MAX_FEED_BYTES {
                        return Err(anyhow::anyhow!("订阅源内容超过 2MB 限制"));
                    }

                    match response.bytes().await {
                        Ok(bytes) if bytes.len() as u64 <= MAX_FEED_BYTES => {
                            return Ok(bytes.to_vec());
                        }
                        Ok(_) => return Err(anyhow::anyhow!("订阅源内容超过 2MB 限制")),
                        Err(error) => {
                            last_error = format!("读取订阅源内容失败：{error}");
                        }
                    }
                } else {
                    match response.bytes().await {
                        Ok(bytes) if bytes.len() as u64 <= MAX_FEED_BYTES => {
                            return Ok(bytes.to_vec());
                        }
                        Ok(_) => return Err(anyhow::anyhow!("订阅源内容超过 2MB 限制")),
                        Err(error) => {
                            last_error = format!("读取订阅源内容失败：{error}");
                        }
                    }
                }
            }
            Err(error) => {
                last_error = error.to_string();
            }
        }

        if attempt < MAX_FETCH_ATTEMPTS {
            tokio::time::sleep(retry_delay_for_attempt(attempt)).await;
        }
    }

    Err(anyhow::anyhow!("请求订阅源失败：{url}；{last_error}"))
}

fn retry_delay_for_attempt(attempt: u32) -> std::time::Duration {
    std::time::Duration::from_millis(400 * attempt as u64)
}

pub fn parse_feed(source_url: &str, bytes: &[u8]) -> Result<ParsedFeed> {
    let feed = parser::parse(bytes).context("RSS/Atom/JSON Feed 解析失败")?;
    let title = feed
        .title
        .map(|text| normalize_inline(&text.content))
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| source_url.to_string());

    let mut items = Vec::with_capacity(feed.entries.len().min(200));
    for entry in feed.entries.into_iter().take(200) {
        let feed_rs::model::Entry {
            id,
            title,
            updated,
            authors,
            content,
            links,
            summary,
            published,
            ..
        } = entry;

        let link = links
            .iter()
            .find(|link| link.rel.as_deref().unwrap_or("alternate") == "alternate")
            .or_else(|| links.first())
            .map(|link| link.href.clone());
        let title = title
            .map(|text| normalize_inline(&text.content))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| link.clone().unwrap_or_else(|| "未命名条目".to_string()));
        let author = authors
            .first()
            .map(|author| normalize_inline(&author.name))
            .filter(|value| !value.is_empty());
        let published_at = published.or(updated).map(|date| date.to_rfc3339());
        let content_html = content
            .and_then(|content| content.body)
            .or_else(|| summary.map(|text| text.content))
            .map(|value| clamp_chars(value.trim(), MAX_CONTENT_CHARS))
            .filter(|value| !value.is_empty());
        let content_text = content_html
            .as_ref()
            .map(|html| html_to_text(html))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| title.clone());
        let guid = if id.trim().is_empty() {
            stable_hash(source_url, link.as_deref(), &title)
        } else {
            id
        };

        items.push(ParsedItem {
            guid,
            title,
            link,
            author,
            published_at,
            content_html,
            content_text,
        });
    }

    Ok(ParsedFeed { title, items })
}

fn html_to_text(html: &str) -> String {
    let text = html2text::from_read(html.as_bytes(), 100).unwrap_or_else(|_| html.to_string());
    let lines = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    clamp_chars(&lines, MAX_TEXT_CHARS)
}

fn normalize_inline(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clamp_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect()
}

fn stable_hash(source_url: &str, link: Option<&str>, title: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source_url.as_bytes());
    if let Some(link) = link {
        hasher.update(link.as_bytes());
    }
    hasher.update(title.as_bytes());
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rss_extracts_summary_text() {
        let rss = br#"<?xml version="1.0" encoding="UTF-8" ?>
        <rss version="2.0">
          <channel>
            <title>Demo Feed</title>
            <link>https://example.com</link>
            <item>
              <title>Demo Item</title>
              <description><![CDATA[<p>Hello <strong>RSS</strong></p>]]></description>
              <link>https://example.com/item</link>
              <guid>demo-item</guid>
            </item>
          </channel>
        </rss>"#;

        let parsed = parse_feed("https://example.com/rss.xml", rss).unwrap();

        assert_eq!(parsed.title, "Demo Feed");
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].guid, "demo-item");
        assert!(parsed.items[0].content_text.contains("Hello"));
    }

    #[test]
    fn rejects_non_http_urls() {
        assert!(validate_http_url("file:///tmp/feed.xml").is_err());
    }

    #[test]
    #[ignore = "live network test for release verification"]
    fn live_aihot_feed_fetches() {
        tauri::async_runtime::block_on(async {
            let client = crate::state::build_http_client().unwrap();
            let parsed = fetch_feed(&client, "https://aihot.virxact.com/feed.xml")
                .await
                .unwrap();

            println!("title={}, items={}", parsed.title, parsed.items.len());
            assert!(parsed.title.contains("AI HOT"));
            assert!(!parsed.items.is_empty());
        });
    }
}
