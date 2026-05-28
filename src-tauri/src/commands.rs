use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

use crate::db;
use crate::feed_service;
use crate::models::{
    AppSettings, Feed, FeedInput, FeedTestResult, FeedUpdate, Item, ItemQuery, RefreshLog,
    RefreshSummary,
};
use crate::refresh;
use crate::state::AppState;
use crate::windowing;

type CommandResult<T> = Result<T, String>;

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> CommandResult<AppSettings> {
    let conn = state.conn().map_err(to_command_error)?;
    db::get_settings(&conn).map_err(to_command_error)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> CommandResult<AppSettings> {
    let saved = {
        let mut conn = state.conn().map_err(to_command_error)?;
        db::save_settings(&mut conn, settings).map_err(to_command_error)?
    };

    if let Err(error) = windowing::reposition_main_from_state(&app) {
        eprintln!("failed to reposition main window: {error}");
    }

    Ok(saved)
}

#[tauri::command]
pub fn list_feeds(state: State<'_, AppState>) -> CommandResult<Vec<Feed>> {
    let conn = state.conn().map_err(to_command_error)?;
    db::list_feeds(&conn).map_err(to_command_error)
}

#[tauri::command]
pub async fn create_feed(
    state: State<'_, AppState>,
    input: FeedInput,
) -> CommandResult<Feed> {
    let mut input = input;
    input.url = feed_service::validate_http_url(&input.url).map_err(to_command_error)?;

    if input.refresh_interval_minutes.is_none() {
        let conn = state.conn().map_err(to_command_error)?;
        input.refresh_interval_minutes = Some(
            db::get_settings(&conn)
                .map_err(to_command_error)?
                .global_refresh_interval_minutes,
        );
    }

    let parsed = feed_service::fetch_feed(&state.http, &input.url)
        .await
        .map_err(to_command_error)?;
    let conn = state.conn().map_err(to_command_error)?;
    db::create_feed(&conn, input, parsed.title).map_err(to_command_error)
}

#[tauri::command]
pub fn update_feed(state: State<'_, AppState>, feed: FeedUpdate) -> CommandResult<Feed> {
    if feed.title.trim().is_empty() {
        return Err("订阅源名称不能为空".to_string());
    }
    feed_service::validate_http_url(&feed.url).map_err(to_command_error)?;

    let conn = state.conn().map_err(to_command_error)?;
    db::update_feed(&conn, feed).map_err(to_command_error)
}

#[tauri::command]
pub fn delete_feed(state: State<'_, AppState>, id: i64) -> CommandResult<()> {
    let conn = state.conn().map_err(to_command_error)?;
    db::delete_feed(&conn, id).map_err(to_command_error)
}

#[tauri::command]
pub async fn test_feed(
    state: State<'_, AppState>,
    url: String,
) -> CommandResult<FeedTestResult> {
    let url = feed_service::validate_http_url(&url).map_err(to_command_error)?;
    let parsed = feed_service::fetch_feed(&state.http, &url)
        .await
        .map_err(to_command_error)?;

    Ok(FeedTestResult {
        title: parsed.title,
        url,
        item_count: parsed.items.len(),
    })
}

#[tauri::command]
pub fn list_items(state: State<'_, AppState>, query: ItemQuery) -> CommandResult<Vec<Item>> {
    let conn = state.conn().map_err(to_command_error)?;
    db::list_items(&conn, query).map_err(to_command_error)
}

#[tauri::command]
pub fn set_item_read(state: State<'_, AppState>, id: i64, read: bool) -> CommandResult<()> {
    let conn = state.conn().map_err(to_command_error)?;
    db::set_item_read(&conn, id, read).map_err(to_command_error)
}

#[tauri::command]
pub fn set_item_favorite(
    state: State<'_, AppState>,
    id: i64,
    favorite: bool,
) -> CommandResult<()> {
    let conn = state.conn().map_err(to_command_error)?;
    db::set_item_favorite(&conn, id, favorite).map_err(to_command_error)
}

#[tauri::command]
pub async fn refresh_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Vec<RefreshSummary>> {
    let summaries = refresh::refresh_all_enabled(state.inner())
        .await
        .map_err(to_command_error)?;

    if summaries.iter().any(|summary| summary.inserted > 0) {
        let _ = app.emit("items_updated", ());
    }

    Ok(summaries)
}

#[tauri::command]
pub async fn refresh_feed(
    app: AppHandle,
    state: State<'_, AppState>,
    feed_id: i64,
) -> CommandResult<RefreshSummary> {
    let summary = refresh::refresh_feed_by_id(state.inner(), feed_id)
        .await
        .map_err(to_command_error)?;

    if summary.inserted > 0 {
        let _ = app.emit("items_updated", ());
    }

    Ok(summary)
}

#[tauri::command]
pub fn open_item(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> CommandResult<()> {
    let url = {
        let conn = state.conn().map_err(to_command_error)?;
        db::get_item_link(&conn, id).map_err(to_command_error)?
    };
    open_external_url(app, url)
}

#[tauri::command]
pub fn open_external_url(app: AppHandle, url: String) -> CommandResult<()> {
    let url = feed_service::validate_http_url(&url).map_err(to_command_error)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(to_command_error)
}

#[tauri::command]
pub async fn open_settings_window(app: AppHandle) -> CommandResult<()> {
    windowing::open_settings_window(&app).map_err(to_command_error)
}

#[tauri::command]
pub fn reposition_main_window(app: AppHandle) -> CommandResult<()> {
    windowing::reposition_main_from_state(&app).map_err(to_command_error)
}

#[tauri::command]
pub fn list_refresh_logs(state: State<'_, AppState>) -> CommandResult<Vec<RefreshLog>> {
    let conn = state.conn().map_err(to_command_error)?;
    db::list_refresh_logs(&conn).map_err(to_command_error)
}

fn to_command_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}
