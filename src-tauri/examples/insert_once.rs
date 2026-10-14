//! Insert text into the foreground window with the production inserter, but ONLY if that window's
//! title is exactly `--expect-title` (a harness-owned test window), so a stray run can never type
//! into someone's real document. Used by scripts/insert-probe.ps1.
//!
//!   cargo run --example insert_once -- --expect-title "Voxelle insertion probe" --text "hello"

#[cfg(windows)]
fn main() {
    use voxelle_core::insert::{sanitize_for_insertion, TextInserter, WindowsInserter};
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

    let mut args = std::env::args().skip(1);
    let (mut expect, mut text) = (None, None);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--expect-title" => expect = args.next(),
            "--text" => text = args.next(),
            "--text-file" => text = Some(std::fs::read_to_string(args.next().unwrap()).unwrap()),
            other => panic!("unknown arg {other}"),
        }
    }
    let (expect, text) = (expect.expect("--expect-title"), text.expect("--text"));
    let title = unsafe {
        let mut buf = [0u16; 512];
        let n = GetWindowTextW(GetForegroundWindow(), buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    };
    if title != expect {
        println!("RESULT {{\"outcome\":\"refused\",\"foreground\":{title:?}}}");
        std::process::exit(3);
    }
    let t = std::time::Instant::now();
    let r = WindowsInserter::new().insert(&sanitize_for_insertion(&text));
    println!(
        "RESULT {{\"outcome\":{:?},\"method\":{:?},\"ms\":{},\"message\":{:?}}}",
        format!("{:?}", r.outcome),
        r.method,
        t.elapsed().as_millis(),
        r.message.unwrap_or_default()
    );
}

#[cfg(not(windows))]
fn main() {
    eprintln!("insert_once is Windows-only");
}
