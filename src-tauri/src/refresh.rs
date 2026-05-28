use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use tauri::{AppHandle, Emitter, Manager};

use crate::db;
use crate::feed_service;
use crate::models::{Feed, RefreshSummary};
use crate::state::AppState;

pub async fn refresh_feed_by_id(state: &AppState, feed_id: i64) -> Result<RefreshSummary> {
    let feed = {
        let conn = state.conn()?;
        db::get_feed(&conn, feed_id)?
    };

    refresh_feed_record(state, feed).await
}

pub async fn refresh_all_enabled(state: &AppState) -> Result<Vec<RefreshSummary>> {
    let feeds = {
        let conn = state.conn()?;
        db::list_enabled_feeds(&conn)?
    };

    let mut summaries = Vec::with_capacity(feeds.len());
    for feed in feeds {
        summaries.push(refresh_feed_record(state, feed).await?);
    }

    Ok(summaries)
}

pub async fn refresh_due_feeds(app: &AppHandle) -> Result<()> {
    let state = app.state::<AppState>();
    let feeds = {
        let conn = state.conn()?;
        db::list_enabled_feeds(&conn)?
    };

    let due = feeds
        .into_iter()
        .filter(is_due)
        .collect::<Vec<_>>();

    if due.is_empty() {
        return Ok(());
    }

    let mut had_insert = false;
    for feed in due {
        let summary = refresh_feed_record(state.inner(), feed).await?;
        had_insert = had_insert || summary.inserted > 0;
    }

    if had_insert {
        let _ = app.emit("items_updated", ());
    }

    Ok(())
}

pub fn spawn_scheduler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            if let Err(error) = refresh_due_feeds(&app).await {
                if let Some(state) = app.try_state::<AppState>() {
                    if let Ok(conn) = state.conn() {
                        let _ = db::add_refresh_log(&conn, None, "error", Some(&error.to_string()));
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}

async fn refresh_feed_record(state: &AppState, feed: Feed) -> Result<RefreshSummary> {
    let client = state.http.clone();
    let parsed = feed_service::fetch_feed(&client, &feed.url).await;

    match parsed {
        Ok(parsed) => {
            let fetched = parsed.items.len();
            let inserted = {
                let mut conn = state.conn()?;
                let inserted = db::insert_parsed_items(&mut conn, feed.id, &parsed.items)?;
                db::mark_feed_refresh_success(&conn, feed.id)?;
                db::add_refresh_log(
                    &conn,
                    Some(feed.id),
                    "success",
                    Some(&format!("解析 {fetched} 条，新增 {inserted} 条")),
                )?;
                inserted
            };

            Ok(RefreshSummary {
                feed_id: feed.id,
                feed_title: feed.title,
                fetched,
                inserted,
                error: None,
            })
        }
        Err(error) => {
            let message = error.to_string();
            {
                let conn = state.conn()?;
                db::mark_feed_refresh_error(&conn, feed.id, &message)?;
                db::add_refresh_log(&conn, Some(feed.id), "error", Some(&message))?;
            }

            Ok(RefreshSummary {
                feed_id: feed.id,
                feed_title: feed.title,
                fetched: 0,
                inserted: 0,
                error: Some(message),
            })
        }
    }
}

fn is_due(feed: &Feed) -> bool {
    let Some(last_checked_at) = &feed.last_checked_at else {
        return true;
    };

    let Ok(last_checked_at) = DateTime::parse_from_rfc3339(last_checked_at) else {
        return true;
    };

    let interval = Duration::minutes(feed.refresh_interval_minutes.max(5));
    Utc::now().signed_duration_since(last_checked_at.with_timezone(&Utc)) >= interval
}
