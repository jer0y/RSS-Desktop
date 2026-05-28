use anyhow::Result;
use tauri::{
    LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::db;
use crate::models::AppSettings;
use crate::state::AppState;

pub fn position_main_window(window: &WebviewWindow, settings: &AppSettings) -> Result<()> {
    window.set_size(LogicalSize::new(
        settings.window_width as f64,
        settings.window_height as f64,
    ))?;

    let monitor = window
        .current_monitor()?
        .or_else(|| window.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        window.show()?;
        return Ok(());
    };

    let scale = window.scale_factor()?;
    let work_area = monitor.work_area();
    let width = (settings.window_width as f64 * scale).round() as i32;
    let margin_top = (settings.margin_top as f64 * scale).round() as i32;
    let margin_right = (settings.margin_right as f64 * scale).round() as i32;
    let x = work_area.position.x + work_area.size.width as i32 - width - margin_right;
    let y = work_area.position.y + margin_top;

    window.set_position(PhysicalPosition::new(x, y))?;
    window.show()?;
    Ok(())
}

pub fn open_settings_window(app: &tauri::AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window("settings") {
        window.show()?;
        window.set_focus()?;
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        "settings",
        WebviewUrl::App("index.html?window=settings".into()),
    )
    .title("RSS Desktop 设置")
    .inner_size(960.0, 780.0)
    .min_inner_size(760.0, 560.0)
    .resizable(true)
    .decorations(true)
    .visible(true)
    .build()?;

    Ok(())
}

pub fn reposition_main_from_state(app: &tauri::AppHandle) -> Result<()> {
    let Some(window) = app.get_webview_window("main") else {
        return Ok(());
    };
    let state = app.state::<AppState>();
    let settings = {
        let conn = state.conn()?;
        db::get_settings(&conn)?
    };

    position_main_window(&window, &settings)
}
