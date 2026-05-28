use anyhow::Result;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Emitter, Manager};

use crate::refresh;
use crate::state::AppState;
use crate::windowing;

const MENU_SHOW: &str = "show";
const MENU_SETTINGS: &str = "settings";
const MENU_REFRESH: &str = "refresh";
const MENU_QUIT: &str = "quit";

pub fn setup_tray(app: &mut App) -> Result<()> {
    let show = MenuItem::with_id(app, MENU_SHOW, "显示主窗口", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, MENU_SETTINGS, "设置", true, None::<&str>)?;
    let refresh = MenuItem::with_id(app, MENU_REFRESH, "刷新全部", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, MENU_QUIT, "退出", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&show, &settings, &refresh, &separator, &quit])?;

    let mut builder = TrayIconBuilder::with_id("rss-desktop-tray")
        .tooltip("RSS Desktop Widget")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_SHOW => show_main(app),
            MENU_SETTINGS => {
                let _ = windowing::open_settings_window(app);
            }
            MENU_REFRESH => refresh_all(app),
            MENU_QUIT => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            let should_show = matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                    | TrayIconEvent::DoubleClick {
                        button: MouseButton::Left,
                        ..
                    }
            );

            if should_show {
                show_main(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }

    builder.build(app)?;
    Ok(())
}

fn show_main(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
        let _ = windowing::reposition_main_from_state(app);
    }
}

fn refresh_all(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = app.state::<AppState>();
        match refresh::refresh_all_enabled(state.inner()).await {
            Ok(summaries) => {
                if summaries.iter().any(|summary| summary.inserted > 0) {
                    let _ = app.emit("items_updated", true);
                }
            }
            Err(error) => {
                eprintln!("tray refresh failed: {error}");
            }
        }
    });
}
