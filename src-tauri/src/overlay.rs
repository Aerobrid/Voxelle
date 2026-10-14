//! Recording indicator window (P0 §0.1). Non-focusable (WS_EX_NOACTIVATE on Windows) and
//! click-through, so showing it never moves focus away from the app being dictated into.

use std::time::Duration;

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

use crate::session::{SessionSnapshot, SessionState};

pub const LABEL: &str = "overlay";
const WIDTH: f64 = 260.0;
const HEIGHT: f64 = 52.0;
/// Distance from the bottom of the work area, in logical pixels.
const BOTTOM_MARGIN: f64 = 72.0;
/// How long a finished/failed/cancelled result stays visible.
const LINGER: Duration = Duration::from_millis(1600);

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let win = WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("overlay.html".into()))
        .title("Voxelle recording indicator")
        .inner_size(WIDTH, HEIGHT)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .focused(false)
        .focusable(false)
        .visible(false)
        .build()?;
    win.set_ignore_cursor_events(true)?;
    if let Some(m) = win.primary_monitor()? {
        let scale = m.scale_factor();
        let area = m.work_area();
        let x = area.position.x + ((area.size.width as f64 - WIDTH * scale) / 2.0) as i32;
        let y = area.position.y + (area.size.height as f64 - (HEIGHT + BOTTOM_MARGIN) * scale) as i32;
        win.set_position(PhysicalPosition::new(x, y))?;
    }
    Ok(())
}

/// Show while a session is live; keep the final state up briefly, then hide.
pub fn sync(app: &AppHandle, snap: &SessionSnapshot, generation: u64) {
    let Some(win) = app.get_webview_window(LABEL) else { return };
    match snap.state {
        SessionState::Recording | SessionState::Processing | SessionState::Inserting => {
            let _ = win.show();
        }
        SessionState::Completed | SessionState::Cancelled | SessionState::Failed => {
            let app = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(LINGER);
                let st = app.state::<crate::app_state::AppState>();
                let session = st.session.lock().unwrap();
                // Only hide if no newer session started meanwhile.
                if session.generation() == generation && session.state().is_terminal() {
                    if let Some(w) = app.get_webview_window(LABEL) {
                        let _ = w.hide();
                    }
                }
            });
        }
        SessionState::Idle => {
            let _ = win.hide();
        }
    }
}
