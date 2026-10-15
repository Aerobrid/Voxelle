//! System tray icon and window lifecycle: Voxelle keeps running in the tray (so global shortcuts
//! keep working) when its window is closed, a second launch brings the existing window back, and
//! the tray menu is the way to quit.

use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, WindowEvent};

use crate::app_state::AppState;

const MAIN: &str = "main";

/// Show, un-minimize, and focus the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Voxelle", true, None::<&str>)?;
    let dictate = MenuItem::with_id(app, "dictate", "Start / stop dictation", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Voxelle", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &dictate, &PredefinedMenuItem::separator(app)?, &quit])?;

    let mut tray = TrayIconBuilder::with_id("voxelle")
        .tooltip("Voxelle — local dictation")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main(app),
            "dictate" => {
                let app = app.clone();
                // Toggle blocks (mic open / transcription); never on the event-loop thread.
                std::thread::spawn(move || {
                    if let Err(e) = crate::dictation::toggle(&app) {
                        if e != "cancelled" {
                            eprintln!("[voxelle] tray dictation: {e}");
                        }
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;

    // Closing the window hides it to the tray (unless the user turned that off), so shortcuts keep
    // working; Quit lives in the tray menu.
    if let Some(w) = app.get_webview_window(MAIN) {
        let handle = app.clone();
        w.on_window_event(move |event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let to_tray = handle.state::<AppState>().settings.lock().unwrap().close_to_tray;
                if to_tray {
                    api.prevent_close();
                    if let Some(w) = handle.get_webview_window(MAIN) {
                        let _ = w.hide();
                    }
                } else {
                    handle.exit(0);
                }
            }
        });
    }
    Ok(())
}
