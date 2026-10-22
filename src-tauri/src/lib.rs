mod capture;
mod commands;
mod history;
mod live;
mod mock_feed;
mod window;

use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use tauri::Manager;

use commands::AppState;
use live::Capturer;

/// EXP region capture rate (~2 fps).
const CAPTURE_EVERY: Duration = Duration::from_millis(500);
const MOCK_EVERY: Duration = Duration::from_millis(250);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(all(target_os = "macos", debug_assertions))]
    window::log_objc_exceptions();
    let mock = std::env::var_os("BATTLE_MOCK").is_some_and(|v| v == "1");
    tauri::Builder::default()
        // Managed before any window loads; the capturer needs the app's data dir, so it's added in setup.
        .manage(Mutex::new(AppState::new(mock)))
        .invoke_handler(tauri::generate_handler![
            commands::get_view,
            commands::session_command,
            commands::open_calibration,
            commands::setup_status,
            commands::request_permission,
            commands::open_permission_settings,
            commands::choose_window,
            commands::window_preview,
            commands::region_preview,
            commands::teach_sample,
            commands::reset_calibration,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event
                && window.label() == "main"
                && let Err(e) = window::platform().before_close(window)
            {
                eprintln!("[window] couldn't prepare the panel to close: {e}");
            }
        })
        .setup(move |app| {
            // An accessory app (no Dock icon) can put its panel over another app's fullscreen Space.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if let Some(main) = app.get_webview_window("main") {
                window::platform().make_floating(&main)?;
            }
            let data_dir = app.path().app_data_dir()?;
            app.state::<Mutex<AppState>>().lock().unwrap().set_history_dir(data_dir.clone());
            app.manage(Mutex::new(Capturer::new(data_dir)));

            let handle = app.handle().clone();
            thread::spawn(move || {
                let state = handle.state::<Mutex<AppState>>();
                let cap = handle.state::<Mutex<Capturer>>();
                loop {
                    if mock {
                        thread::sleep(MOCK_EVERY);
                        state.lock().unwrap().tick_mock();
                        continue;
                    }
                    thread::sleep(CAPTURE_EVERY);
                    let recording = state.lock().unwrap().recording();
                    // Capture without holding the session lock, so the panel stays responsive.
                    let (status, tick) = {
                        let mut cap = cap.lock().unwrap();
                        let tick = if recording { cap.tick() } else { cap.scan(false); None };
                        cap.learn();
                        (cap.status(), tick)
                    };
                    state.lock().unwrap().on_capture(status, tick);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
