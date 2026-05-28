use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::models::{
    normalize_refresh_interval, normalize_settings, AppSettings, Feed, FeedInput, FeedUpdate, Item,
    ItemQuery, ParsedItem, RefreshLog,
};

pub fn configure_connection(conn: &Connection) -> Result<()> {
    conn.busy_timeout(Duration::from_secs(5))?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    Ok(())
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            url TEXT NOT NULL UNIQUE,
            enabled INTEGER NOT NULL DEFAULT 1,
            refresh_interval_minutes INTEGER NOT NULL DEFAULT 15,
            last_checked_at TEXT,
            last_error TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            feed_id INTEGER NOT NULL,
            guid TEXT NOT NULL,
            title TEXT NOT NULL,
            link TEXT,
            author TEXT,
            published_at TEXT,
            content_text TEXT NOT NULL,
            content_html TEXT,
            read INTEGER NOT NULL DEFAULT 0,
            favorite INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY(feed_id) REFERENCES feeds(id) ON DELETE CASCADE,
            UNIQUE(feed_id, guid)
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS refresh_logs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            feed_id INTEGER,
            status TEXT NOT NULL,
            message TEXT,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_items_feed_time ON items(feed_id, published_at);
        CREATE INDEX IF NOT EXISTS idx_items_read ON items(read);
        CREATE INDEX IF NOT EXISTS idx_items_favorite ON items(favorite);
        CREATE INDEX IF NOT EXISTS idx_logs_time ON refresh_logs(created_at);
        "#,
    )
    .context("failed to initialize database schema")?;

    ensure_default_settings(conn)
}

pub fn get_settings(conn: &Connection) -> Result<AppSettings> {
    let default = AppSettings::default();
    Ok(normalize_settings(AppSettings {
        window_width: get_i64_setting(conn, "window_width")?.unwrap_or(default.window_width),
        window_height: get_i64_setting(conn, "window_height")?.unwrap_or(default.window_height),
        opacity: get_i64_setting(conn, "opacity")?.unwrap_or(default.opacity),
        margin_top: get_i64_setting(conn, "margin_top")?.unwrap_or(default.margin_top),
        margin_right: get_i64_setting(conn, "margin_right")?.unwrap_or(default.margin_right),
        global_refresh_interval_minutes: get_i64_setting(conn, "global_refresh_interval_minutes")?
            .unwrap_or(default.global_refresh_interval_minutes),
        max_items: get_i64_setting(conn, "max_items")?.unwrap_or(default.max_items),
    }))
}

pub fn save_settings(conn: &mut Connection, settings: AppSettings) -> Result<AppSettings> {
    let settings = normalize_settings(settings);
    let tx = conn.transaction()?;
    set_setting(&tx, "window_width", settings.window_width)?;
    set_setting(&tx, "window_height", settings.window_height)?;
    set_setting(&tx, "opacity", settings.opacity)?;
    set_setting(&tx, "margin_top", settings.margin_top)?;
    set_setting(&tx, "margin_right", settings.margin_right)?;
    set_setting(
        &tx,
        "global_refresh_interval_minutes",
        settings.global_refresh_interval_minutes,
    )?;
    set_setting(&tx, "max_items", settings.max_items)?;
    tx.commit()?;
    Ok(settings)
}

pub fn list_feeds(conn: &Connection) -> Result<Vec<Feed>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, title, url, enabled, refresh_interval_minutes, last_checked_at, last_error,
               created_at, updated_at
        FROM feeds
        ORDER BY enabled DESC, title COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_feed)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read feeds")
}

pub fn list_enabled_feeds(conn: &Connection) -> Result<Vec<Feed>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT id, title, url, enabled, refresh_interval_minutes, last_checked_at, last_error,
               created_at, updated_at
        FROM feeds
        WHERE enabled = 1
        ORDER BY title COLLATE NOCASE ASC
        "#,
    )?;
    let rows = stmt.query_map([], row_to_feed)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read enabled feeds")
}

pub fn get_feed(conn: &Connection, id: i64) -> Result<Feed> {
    conn.query_row(
        r#"
        SELECT id, title, url, enabled, refresh_interval_minutes, last_checked_at, last_error,
               created_at, updated_at
        FROM feeds
        WHERE id = ?1
        "#,
        [id],
        row_to_feed,
    )
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("feed not found"))
}

pub fn create_feed(conn: &Connection, input: FeedInput, inferred_title: String) -> Result<Feed> {
    let now = now_rfc3339();
    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(inferred_title.trim())
        .to_string();
    let interval = normalize_refresh_interval(input.refresh_interval_minutes.unwrap_or(15));

    conn.execute(
        r#"
        INSERT INTO feeds (title, url, enabled, refresh_interval_minutes, created_at, updated_at)
        VALUES (?1, ?2, 1, ?3, ?4, ?4)
        "#,
        params![title, input.url.trim(), interval, now],
    )
    .context("failed to create feed")?;

    get_feed(conn, conn.last_insert_rowid())
}

pub fn update_feed(conn: &Connection, feed: FeedUpdate) -> Result<Feed> {
    let now = now_rfc3339();
    conn.execute(
        r#"
        UPDATE feeds
        SET title = ?1,
            url = ?2,
            enabled = ?3,
            refresh_interval_minutes = ?4,
            updated_at = ?5
        WHERE id = ?6
        "#,
        params![
            feed.title.trim(),
            feed.url.trim(),
            bool_to_i64(feed.enabled),
            normalize_refresh_interval(feed.refresh_interval_minutes),
            now,
            feed.id
        ],
    )
    .context("failed to update feed")?;

    get_feed(conn, feed.id)
}

pub fn delete_feed(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM feeds WHERE id = ?1", [id])
        .context("failed to delete feed")?;
    Ok(())
}

pub fn list_items(conn: &Connection, query: ItemQuery) -> Result<Vec<Item>> {
    let limit = query.limit.clamp(1, 200);
    let offset = query.offset.max(0);
    let mode = match query.mode.as_str() {
        "unread" | "favorite" => query.mode,
        _ => "all".to_string(),
    };
    let search = query
        .search
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let mut stmt = conn.prepare(
        r#"
        SELECT i.id, i.feed_id, f.title AS feed_title, i.title, i.link, i.author,
               i.published_at, i.content_text, i.content_html, i.read, i.favorite,
               i.guid, i.created_at
        FROM items i
        JOIN feeds f ON f.id = i.feed_id
        WHERE (?1 IS NULL OR i.feed_id = ?1)
          AND (?2 != 'unread' OR i.read = 0)
          AND (?2 != 'favorite' OR i.favorite = 1)
          AND (
            ?3 IS NULL
            OR i.title LIKE '%' || ?3 || '%'
            OR i.content_text LIKE '%' || ?3 || '%'
            OR f.title LIKE '%' || ?3 || '%'
          )
        ORDER BY COALESCE(i.published_at, i.created_at) DESC, i.id DESC
        LIMIT ?4 OFFSET ?5
        "#,
    )?;

    let rows = stmt.query_map(params![query.feed_id, mode, search, limit, offset], row_to_item)?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read items")
}

pub fn set_item_read(conn: &Connection, id: i64, read: bool) -> Result<()> {
    conn.execute(
        "UPDATE items SET read = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(read), now_rfc3339(), id],
    )
    .context("failed to update read state")?;
    Ok(())
}

pub fn set_item_favorite(conn: &Connection, id: i64, favorite: bool) -> Result<()> {
    conn.execute(
        "UPDATE items SET favorite = ?1, updated_at = ?2 WHERE id = ?3",
        params![bool_to_i64(favorite), now_rfc3339(), id],
    )
    .context("failed to update favorite state")?;
    Ok(())
}

pub fn get_item_link(conn: &Connection, id: i64) -> Result<String> {
    conn.query_row("SELECT link FROM items WHERE id = ?1", [id], |row| {
        row.get::<_, Option<String>>(0)
    })
    .optional()?
    .flatten()
    .ok_or_else(|| anyhow::anyhow!("item has no link"))
}

pub fn insert_parsed_items(
    conn: &mut Connection,
    feed_id: i64,
    items: &[ParsedItem],
) -> Result<usize> {
    let now = now_rfc3339();
    let tx = conn.transaction()?;
    let mut inserted = 0;

    for item in items {
        let changed = tx
            .execute(
                r#"
                INSERT OR IGNORE INTO items (
                    feed_id, guid, title, link, author, published_at,
                    content_text, content_html, read, favorite, created_at, updated_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0, 0, ?9, ?9)
                "#,
                params![
                    feed_id,
                    item.guid,
                    item.title,
                    item.link,
                    item.author,
                    item.published_at,
                    item.content_text,
                    item.content_html,
                    now
                ],
            )
            .context("failed to insert item")?;
        inserted += changed;
    }

    tx.execute(
        r#"
        DELETE FROM items
        WHERE id NOT IN (
            SELECT id FROM items
            ORDER BY COALESCE(published_at, created_at) DESC, id DESC
            LIMIT 5000
        )
        "#,
        [],
    )?;

    tx.commit()?;
    Ok(inserted)
}

pub fn mark_feed_refresh_success(conn: &Connection, feed_id: i64) -> Result<()> {
    conn.execute(
        r#"
        UPDATE feeds
        SET last_checked_at = ?1,
            last_error = NULL,
            updated_at = ?1
        WHERE id = ?2
        "#,
        params![now_rfc3339(), feed_id],
    )
    .context("failed to update feed refresh state")?;
    Ok(())
}

pub fn mark_feed_refresh_error(conn: &Connection, feed_id: i64, message: &str) -> Result<()> {
    conn.execute(
        r#"
        UPDATE feeds
        SET last_checked_at = ?1,
            last_error = ?2,
            updated_at = ?1
        WHERE id = ?3
        "#,
        params![now_rfc3339(), message, feed_id],
    )
    .context("failed to update feed error state")?;
    Ok(())
}

pub fn add_refresh_log(
    conn: &Connection,
    feed_id: Option<i64>,
    status: &str,
    message: Option<&str>,
) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO refresh_logs (feed_id, status, message, created_at)
        VALUES (?1, ?2, ?3, ?4)
        "#,
        params![feed_id, status, message, now_rfc3339()],
    )
    .context("failed to write refresh log")?;

    conn.execute(
        r#"
        DELETE FROM refresh_logs
        WHERE id NOT IN (
            SELECT id FROM refresh_logs ORDER BY id DESC LIMIT 80
        )
        "#,
        [],
    )?;
    Ok(())
}

pub fn list_refresh_logs(conn: &Connection) -> Result<Vec<RefreshLog>> {
    let mut stmt = conn.prepare(
        r#"
        SELECT l.id, l.feed_id, f.title, l.status, l.message, l.created_at
        FROM refresh_logs l
        LEFT JOIN feeds f ON f.id = l.feed_id
        ORDER BY l.id DESC
        LIMIT 50
        "#,
    )?;

    let rows = stmt.query_map([], |row| {
        Ok(RefreshLog {
            id: row.get(0)?,
            feed_id: row.get(1)?,
            feed_title: row.get(2)?,
            status: row.get(3)?,
            message: row.get(4)?,
            created_at: row.get(5)?,
        })
    })?;

    rows.collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read refresh logs")
}

fn ensure_default_settings(conn: &Connection) -> Result<()> {
    let settings = AppSettings::default();
    set_setting_if_missing(conn, "window_width", settings.window_width)?;
    set_setting_if_missing(conn, "window_height", settings.window_height)?;
    set_setting_if_missing(conn, "opacity", settings.opacity)?;
    set_setting_if_missing(conn, "margin_top", settings.margin_top)?;
    set_setting_if_missing(conn, "margin_right", settings.margin_right)?;
    set_setting_if_missing(
        conn,
        "global_refresh_interval_minutes",
        settings.global_refresh_interval_minutes,
    )?;
    set_setting_if_missing(conn, "max_items", settings.max_items)?;
    Ok(())
}

fn get_i64_setting(conn: &Connection, key: &str) -> Result<Option<i64>> {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .optional()?
    .map(|value| value.parse::<i64>().context("invalid numeric setting"))
    .transpose()
}

fn set_setting(conn: &Connection, key: &str, value: i64) -> Result<()> {
    conn.execute(
        r#"
        INSERT INTO settings (key, value)
        VALUES (?1, ?2)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
        params![key, value.to_string()],
    )?;
    Ok(())
}

fn set_setting_if_missing(conn: &Connection, key: &str, value: i64) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
        params![key, value.to_string()],
    )?;
    Ok(())
}

fn row_to_feed(row: &Row<'_>) -> rusqlite::Result<Feed> {
    Ok(Feed {
        id: row.get(0)?,
        title: row.get(1)?,
        url: row.get(2)?,
        enabled: row.get::<_, i64>(3)? != 0,
        refresh_interval_minutes: row.get(4)?,
        last_checked_at: row.get(5)?,
        last_error: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

fn row_to_item(row: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get(0)?,
        feed_id: row.get(1)?,
        feed_title: row.get(2)?,
        title: row.get(3)?,
        link: row.get(4)?,
        author: row.get(5)?,
        published_at: row.get(6)?,
        content_text: row.get(7)?,
        content_html: row.get(8)?,
        read: row.get::<_, i64>(9)? != 0,
        favorite: row.get::<_, i64>(10)? != 0,
        guid: row.get(11)?,
        created_at: row.get(12)?,
    })
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn connection() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        configure_connection(&conn).unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn settings_round_trip_normalizes_values() {
        let mut conn = connection();
        let saved = save_settings(
            &mut conn,
            AppSettings {
                window_width: 100,
                window_height: 2000,
                opacity: 10,
                margin_top: -1,
                margin_right: 500,
                global_refresh_interval_minutes: 1,
                max_items: 500,
            },
        )
        .unwrap();

        assert_eq!(saved.window_width, 360);
        assert_eq!(saved.window_height, 1100);
        assert_eq!(saved.opacity, 45);
        assert_eq!(saved.margin_top, 0);
        assert_eq!(saved.margin_right, 200);
        assert_eq!(saved.global_refresh_interval_minutes, 5);
        assert_eq!(saved.max_items, 200);
    }

    #[test]
    fn item_insert_deduplicates_by_feed_and_guid() {
        let mut conn = connection();
        let feed = create_feed(
            &conn,
            FeedInput {
                title: Some("Feed".to_string()),
                url: "https://example.com/rss.xml".to_string(),
                refresh_interval_minutes: Some(15),
            },
            "Feed".to_string(),
        )
        .unwrap();
        let item = ParsedItem {
            guid: "one".to_string(),
            title: "Title".to_string(),
            link: Some("https://example.com/1".to_string()),
            author: None,
            published_at: None,
            content_html: None,
            content_text: "Body".to_string(),
        };

        assert_eq!(insert_parsed_items(&mut conn, feed.id, &[item.clone()]).unwrap(), 1);
        assert_eq!(insert_parsed_items(&mut conn, feed.id, &[item]).unwrap(), 0);
    }

    #[test]
    fn refresh_log_keeps_error_message() {
        let conn = connection();
        add_refresh_log(&conn, None, "error", Some("network timeout")).unwrap();
        let logs = list_refresh_logs(&conn).unwrap();

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].status, "error");
        assert_eq!(logs[0].message.as_deref(), Some("network timeout"));
    }
}
