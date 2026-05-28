mod commands;
mod db;
mod feed_service;
mod models;
mod refresh;
mod state;
mod tray;
mod windowing;

use anyhow::Result;
use tauri::Manager;

use state::AppState;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            setup_app(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_main_window_opacity,
            commands::list_feeds,
            commands::create_feed,
            commands::update_feed,
            commands::delete_feed,
            commands::clear_cached_items,
            commands::test_feed,
            commands::list_items,
            commands::set_item_read,
            commands::set_item_favorite,
            commands::refresh_all,
            commands::refresh_feed,
            commands::open_item,
            commands::open_external_url,
            commands::open_settings_window,
            commands::reposition_main_window,
            commands::hide_main_window,
            commands::list_refresh_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tauri application");
}

fn setup_app(app: &mut tauri::App) -> Result<()> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("rss_desktop.sqlite3");

    let state = AppState::new(db_path)?;
    app.manage(state);

    if let Some(window) = app.get_webview_window("main") {
        let state = app.state::<AppState>();
        let settings = {
            let conn = state.conn()?;
            db::get_settings(&conn)?
        };
        windowing::position_main_window(&window, &settings)?;
        windowing::bind_main_window_events(&window);
    }

    tray::setup_tray(app)?;
    refresh::spawn_scheduler(app.handle().clone());
    Ok(())
}
