//! Windows low-level keyboard hook (WH_KEYBOARD_LL) on a dedicated thread. Unlike a registered
//! hotkey it sees key-up events, which push-to-talk needs, and it sees the Win key before the shell
//! does (so Win chords work and the recorder can capture them). The hook procedure must return
//! quickly (Windows silently removes slow LL hooks), so it only runs the pure matcher and hands
//! actions to the worker; it never takes a lock that dictation holds.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_SZ};
use windows_sys::Win32::System::Threading::{GetCurrentThread, GetCurrentThreadId, SetThreadPriority, THREAD_PRIORITY_TIME_CRITICAL};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayoutList, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetMessageW, PostThreadMessageW, SetTimer, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_APP, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
    WM_TIMER,
};

use super::capture::{Capture, CaptureUpdate};
use super::{vk, Action, LayoutHotkeys, Matcher, Role, SharedBindings};

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
                // Windows silently removes an LL hook that misses its timeout once, e.g. while a
                // CPU transcription saturates every core. This thread only wakes per key, so
                // running it at top priority costs nothing and keeps it on time.
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_TIME_CRITICAL);
                let mut hook = install_hook();
                if hook.is_null() {
                    let _ = ready_tx.send(Err("could not install the keyboard hook".into()));
                    return;
                }
                HOOK_THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
                let _ = ready_tx.send(Ok(()));
                // A removed hook is never reported, so re-install it periodically and on request.
                SetTimer(std::ptr::null_mut(), 0, REHOOK_EVERY_MS, None);
                let mut msg: MSG = std::mem::zeroed();
                // LL hooks are dispatched through this thread's message loop.
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    if msg.message == WM_TIMER || msg.message == WM_REHOOK {
                        hook = reinstall(hook);
                    }
                }
            }
        })
        .map_err(|e| e.to_string())?;
    ready_rx.recv().map_err(|e| e.to_string())?
}

const WM_REHOOK: u32 = WM_APP + 1;
const REHOOK_EVERY_MS: u32 = 30_000;
static HOOK_THREAD: AtomicU32 = AtomicU32::new(0);

unsafe fn install_hook() -> HHOOK {
    SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), GetModuleHandleW(std::ptr::null()), 0)
}

/// New hook first, then remove the old one. Both calls run on the hook thread without pumping
/// messages in between, so no key event is lost or seen twice. If the new hook fails, keep the old.
unsafe fn reinstall(old: HHOOK) -> HHOOK {
    let new = install_hook();
    if new.is_null() {
        return old;
    }
    UnhookWindowsHookEx(old); // fails harmlessly if Windows already removed it
    new
}

/// Re-install the keyboard hook now (in case Windows removed it). Cheap; safe to call any time.
pub fn refresh() {
    let thread = HOOK_THREAD.load(Ordering::SeqCst);
    if thread != 0 {
        // SAFETY: posting a plain message to our own hook thread's queue.
        unsafe { PostThreadMessageW(thread, WM_REHOOK, 0, 0) };
    }
}

/// Which Shift chords switch layouts here. Windows only acts on them when more than one keyboard
/// layout is installed; then the user's Toggle settings say which chord does what (defaults:
/// Alt+Shift switches language, Ctrl+Shift switches layout).
pub fn layout_hotkeys() -> LayoutHotkeys {
    // SAFETY: a zero-length query only returns the count.
    let layouts = unsafe { GetKeyboardLayoutList(0, std::ptr::null_mut()) };
    if layouts <= 1 {
        return LayoutHotkeys::NONE;
    }
    let language = toggle_setting("Language Hotkey").or_else(|| toggle_setting("Hotkey")).unwrap_or('1');
    let layout = toggle_setting("Layout Hotkey").unwrap_or('2');
    LayoutHotkeys {
        ctrl_shift: language == '2' || layout == '2',
        alt_shift: language == '1' || layout == '1',
    }
}

/// First character of a `HKCU\Keyboard Layout\Toggle` value ("1" Alt+Shift, "2" Ctrl+Shift,
/// "3" none, "4" grave accent).
fn toggle_setting(name: &str) -> Option<char> {
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (key, value) = (wide("Keyboard Layout\\Toggle"), wide(name));
    let mut buf = [0u16; 8];
    let mut size = std::mem::size_of_val(&buf) as u32;
    // SAFETY: valid NUL-terminated names and an output buffer of `size` bytes.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut size,
        )
    };
    (status == 0).then(|| char::from_u32(buf[0] as u32)).flatten()
}

pub fn begin_capture(role: Role, sink: CaptureSink) -> Result<(), String> {
    if CTX.get().is_none() {
        return Err("global shortcuts are unavailable, so keys cannot be recorded".into());
    }
    // Make sure the hook is really there: a removed hook would leave the dialog seeing no keys.
    refresh();
    let capture = Capture::with_layouts(role, layout_hotkeys());
    let previous = CAPTURE.lock().unwrap().replace(Recording { capture, sink, last_key: Instant::now() });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hook_install_and_reinstall() {
        unsafe {
            let h1 = install_hook();
            println!("Initial hook: {:?}", h1);
            assert!(!h1.is_null(), "install_hook failed");
            let h2 = reinstall(h1);
            println!("Reinstalled hook: {:?}", h2);
            assert!(!h2.is_null(), "reinstall failed");
            UnhookWindowsHookEx(h2);
        }
    }
}
