mod commands;
mod mock_feed;
mod window;

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::Manager;

use commands::AppState;
use window::{BasicFloating, FloatingWindow};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![commands::get_view, commands::session_command])
        .setup(|app| {
            if let Some(main) = app.get_webview_window("main") {
                BasicFloating.make_floating(&main)?;
            }
            // Milestone 1: drive the real stats engine from a synthetic feed.
            // Replaced by the capture loop in Milestone 3.
            let handle = app.handle().clone();
            thread::spawn(move || loop {
                thread::sleep(Duration::from_millis(250));
                handle.state::<Mutex<AppState>>().lock().unwrap().tick_mock();
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
