//! Windows low-level keyboard hook (WH_KEYBOARD_LL) on a dedicated thread. Unlike a registered
//! hotkey it sees key-up events, which push-to-talk needs, and it sees the Win key before the shell
//! does (so Win chords work and the recorder can capture them). The hook procedure must return
//! quickly (Windows silently removes slow LL hooks), so it only runs the pure matcher and hands
//! actions to the worker; it never takes a lock that dictation holds.

use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::capture::{Capture, CaptureUpdate};
use super::{vk, Action, Matcher, SharedBindings};

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
/// While set, every physical key goes to the recorder and nowhere else.
static CAPTURE: Mutex<Option<(Capture, CaptureSink)>> = Mutex::new(None);

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

pub fn begin_capture(sink: CaptureSink) -> Result<(), String> {
    if CTX.get().is_none() {
        return Err("global shortcuts are unavailable, so keys cannot be recorded".into());
    }
    *CAPTURE.lock().unwrap() = Some((Capture::default(), sink));
    Ok(())
}

pub fn end_capture() -> bool {
    CAPTURE.lock().unwrap().take().is_some()
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
                if let Ok(mut capture) = CAPTURE.try_lock() {
                    if let Some((recorder, sink)) = capture.as_mut() {
                        let update = recorder.on_key(info.vkCode as u16, down);
                        let finished = matches!(update, Some(CaptureUpdate::Done { .. } | CaptureUpdate::Rejected { .. }));
                        if let Some(u) = update {
                            sink(u);
                        }
                        if finished {
                            *capture = None;
                        }
                        return 1; // recording: the key does nothing else
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
