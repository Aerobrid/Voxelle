//! Windows low-level keyboard hook (WH_KEYBOARD_LL) on a dedicated thread. Unlike a registered
//! hotkey it sees key-up events, which push-to-talk needs. The hook procedure must return quickly
//! (Windows silently removes slow LL hooks), so it only runs the pure matcher and hands actions to
//! the worker; it never takes a lock that dictation holds.

use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, SetWindowsHookExW, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use super::{Action, Matcher, SharedBindings};

type Sender = Box<dyn Fn(Action) + Send + Sync>;
type Probe = Box<dyn Fn() -> bool + Send + Sync>;

struct HookCtx {
    bindings: SharedBindings,
    matcher: Mutex<Matcher>,
    send: Sender,
    session_active: Probe,
}

static CTX: OnceLock<HookCtx> = OnceLock::new();

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

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 {
        if let Some(ctx) = CTX.get() {
            let info = &*(lparam as *const KBDLLHOOKSTRUCT);
            // Ignore synthetic input — including our own typing/paste — so it can never trigger us.
            let injected = info.flags & LLKHF_INJECTED != 0;
            let down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
            let up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
            if !injected && (down || up) {
                let outcome = match (ctx.bindings.try_read(), ctx.matcher.try_lock()) {
                    (Ok(b), Ok(mut m)) => Some(m.on_key(&b, info.vkCode as u16, down, (ctx.session_active)())),
                    _ => None, // settings being swapped this instant: let the key through
                };
                if let Some(o) = outcome {
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
