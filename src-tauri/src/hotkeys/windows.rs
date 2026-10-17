//! Windows low-level keyboard hook (WH_KEYBOARD_LL) on a dedicated thread. Unlike a registered
//! hotkey it sees key-up events, which push-to-talk needs, and it sees the Win key before the shell
//! does (so Win chords work and the recorder can capture them). The hook procedure must return
//! quickly (Windows silently removes slow LL hooks), so it only runs the pure matcher and hands
//! actions to the worker; it never takes a lock that dictation holds.

use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::capture::{Capture, CaptureUpdate};
use super::{vk, Action, Matcher, Role, SharedBindings};

type Sender = Box<dyn Fn(Action) + Send + Sync>;
type Probe = Box<dyn Fn() -> bool + Send + Sync>;
pub type CaptureSink = Box<dyn Fn(CaptureUpdate) + Send + Sync>;

struct HookCtx {
    bindings: SharedBindings,
    matcher: Mutex<Matcher>,
    send: Sender,
    session_active: Probe,
}

static CTX: OnceLock<HookCtx> = OnceLock::new();

struct Recording {
    capture: Capture,
    sink: CaptureSink,
    last_key: Instant,
}

/// While set, physical keys go to the recorder and nowhere else.
static CAPTURE: Mutex<Option<Recording>> = Mutex::new(None);
/// Keys still held when recording ended: the recorder swallowed their key-down, so their key-up
/// must be swallowed too, or the focused app (and the shortcut matcher) would see half a press.
static DRAIN: Mutex<Vec<u16>> = Mutex::new(Vec::new());

pub fn install(bindings: SharedBindings, send: Sender, session_active: Probe) -> Result<(), String> {
    CTX.set(HookCtx {
        bindings,
        matcher: Mutex::new(Matcher::default()),
        send,
        session_active,
    })
    .map_err(|_| "keyboard hook already installed".to_string())?;

    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    std::thread::Builder::new()
        .name("voxelle-key-hook".into())
        .spawn(move || {
            // SAFETY: standard LL-hook setup; the hook lives as long as this thread's message loop.
            unsafe {
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0);
                if hook.is_null() {
                    let _ = ready_tx.send(Err("could not install the keyboard hook".into()));
                    return;
                }
                let _ = ready_tx.send(Ok(()));
                let mut msg: MSG = std::mem::zeroed();
                // LL hooks are dispatched through this thread's message loop.
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
            }
        })
        .map_err(|e| e.to_string())?;
    ready_rx.recv().map_err(|e| e.to_string())?
}

pub fn begin_capture(role: Role, sink: CaptureSink) -> Result<(), String> {
    if CTX.get().is_none() {
        return Err("global shortcuts are unavailable, so keys cannot be recorded".into());
    }
    let previous = CAPTURE.lock().unwrap().replace(Recording { capture: Capture::new(role), sink, last_key: Instant::now() });
    if let Some(p) = previous {
        DRAIN.lock().unwrap().extend_from_slice(p.capture.held());
    }
    Ok(())
}

pub fn end_capture() -> bool {
    match CAPTURE.lock().unwrap().take() {
        Some(r) => {
            DRAIN.lock().unwrap().extend_from_slice(r.capture.held());
            true
        }
        None => false,
    }
}

/// How long the recorder has gone without a key event, or None if it is not recording. Never
/// idle while a key is held, so a timeout cannot cut a press in half.
pub fn capture_idle() -> Option<Duration> {
    let guard = CAPTURE.lock().unwrap();
    let r = guard.as_ref()?;
    Some(if r.capture.held().is_empty() { r.last_key.elapsed() } else { Duration::ZERO })
}

/// Tap an unassigned key so that releasing a held Win key does not open the Start menu. Sent from
/// a short-lived thread: never call SendInput from inside the hook procedure.
fn mask_win() {
    std::thread::spawn(|| {
        let key = |flags| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk::MASK, wScan: 0, dwFlags: flags, time: 0, dwExtraInfo: 0 } },
        };
        let inputs = [key(0), key(KEYEVENTF_KEYUP)];
        // SAFETY: valid INPUT array for the duration of the call.
        unsafe { SendInput(inputs.len() as u32, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    });
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(ctx) = CTX.get() {
            let info = &*(lparam as *const KBDLLHOOKSTRUCT);
            // Ignore synthetic input — including our own typing/paste — so it can never trigger us.
            let injected = info.flags & LLKHF_INJECTED != 0;
            let down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
            if !injected && (down || up) {
                let code = info.vkCode as u16;
                {
                    let mut capture = CAPTURE.lock().unwrap_or_else(|e| e.into_inner());
                    // Key-ups of keys pressed before recording began pass through untouched.
                    if let Some(rec) = capture.as_mut().filter(|r| down || r.capture.held().contains(&code)) {
                        rec.last_key = Instant::now();
                        let update = rec.capture.on_key(code, down);
                        let finished = update.as_ref().is_some_and(|u| !matches!(u, CaptureUpdate::Progress { .. }));
                        if let Some(u) = update {
                            (rec.sink)(u); // only queues: emitting happens on another thread
                        }
                        if finished {
                            *capture = None;
                        }
                        return 1; // recording: the key does nothing else
                    }
                }
                if up {
                    let mut drain = DRAIN.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(i) = drain.iter().position(|c| *c == code) {
                        drain.remove(i);
                        return 1;
                    }
                }
                let outcome = match (ctx.bindings.try_read(), ctx.matcher.try_lock()) {
                    (Ok(b), Ok(mut m)) => Some(m.on_key(&b, info.vkCode as u16, down, (ctx.session_active)())),
                    _ => None, // settings being swapped this instant: let the key through
                };
                if let Some(o) = outcome {
                    if o.mask_win {
                        mask_win();
                    }
                    for a in o.actions {
                        (ctx.send)(a);
                    }
                    if o.swallow {
                        return 1;
                    }
                }
            }
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}
