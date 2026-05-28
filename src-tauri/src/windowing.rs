use anyhow::Result;
use tauri::{
    LogicalSize, Manager, PhysicalPosition, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};

use crate::db;
use crate::models::AppSettings;
use crate::state::AppState;

pub fn position_main_window(window: &WebviewWindow, settings: &AppSettings) -> Result<()> {
    window.set_size(LogicalSize::new(
        settings.window_width as f64,
        settings.window_height as f64,
    ))?;
    apply_window_opacity(window, settings.opacity)?;

    let (x, y) = match (settings.window_x, settings.window_y) {
        (Some(x), Some(y)) => (x as i32, y as i32),
        _ => {
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
            (
                work_area.position.x + work_area.size.width as i32 - width - margin_right,
                work_area.position.y + margin_top,
            )
        }
    };

    window.set_position(PhysicalPosition::new(x, y))?;
    window.show()?;
    Ok(())
}

pub fn apply_main_window_opacity(app: &tauri::AppHandle, opacity: i64) -> Result<()> {
    if let Some(window) = app.get_webview_window("main") {
        apply_window_opacity(&window, opacity)?;
    }
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

pub fn bind_main_window_events(window: &WebviewWindow) {
    let app = window.app_handle().clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(position) => {
            let state = app.state::<AppState>();
            if let Ok(conn) = state.conn() {
                let _ = db::save_window_position(&conn, position.x as i64, position.y as i64);
            };
        }
        WindowEvent::Resized(size) => {
            let scale = app
                .get_webview_window("main")
                .and_then(|window| window.scale_factor().ok())
                .unwrap_or(1.0);
            let width = (size.width as f64 / scale).round() as i64;
            let height = (size.height as f64 / scale).round() as i64;
            let state = app.state::<AppState>();
            if let Ok(conn) = state.conn() {
                let _ = db::save_window_size(&conn, width, height);
            };
        }
        WindowEvent::CloseRequested { api, .. } => {
            api.prevent_close();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        _ => {}
    });
}

#[cfg(windows)]
fn apply_window_opacity(window: &WebviewWindow, opacity: i64) -> Result<()> {
    use windows::Win32::Foundation::COLORREF;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA,
        WS_EX_LAYERED,
    };

    let hwnd = window.hwnd()?;
    let alpha = ((opacity.clamp(45, 100) as f64 / 100.0) * 255.0).round() as u8;

    unsafe {
        let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as isize);
        SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA)?;
    }

    Ok(())
}

#[cfg(not(windows))]
fn apply_window_opacity(_window: &WebviewWindow, _opacity: i64) -> Result<()> {
    Ok(())
}
