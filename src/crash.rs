//! Crash reporting — turn a silent disappearance into a popup that says *why*.
//!
//! kadr links a lot of native code: libmpv (video), libheif (HEIC), libraw
//! (`raw_r.dll`), mozjpeg (JPEG) and the OpenGL driver. When any of those
//! walks off a cliff the result is a *Windows structured exception* (access
//! violation, stack overflow, …) — **not** a Rust panic. The release build is
//! `panic = "abort"` with `windows_subsystem = "windows"` (no console), so
//! today such a fault just makes the process vanish with no hint of the cause.
//!
//! `install()` wires up two catch-alls that both funnel into one reporter:
//!
//!   * `std::panic::set_hook`         — Rust panics, on any thread
//!   * `SetUnhandledExceptionFilter`  — native / OS structured exceptions
//!
//! The reporter writes a full log next to `config.toml` (fault address,
//! faulting module, and every stack frame as `module+0xoffset` so a stripped
//! release binary can still be symbolised offline), shows a modal message box
//! with the short reason, then hard-exits the process.
//!
//! Some deaths reach *neither* hook: an allocation-failure abort (a file that
//! claims impossible dimensions), a stack overflow (both are printed to a
//! stderr that a windowed build doesn't have, then `abort()`), or a C library
//! calling `exit()`. For those, [`Breadcrumb`] leaves a note on disk saying
//! what kadr was doing; if it's still there next launch, `install()` shows it.
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Flipped to `true` the moment we start reporting a crash, so the
/// panic-hook → `abort()` → structured-exception cascade only ever produces
/// one dialog.
static REPORTING: AtomicBool = AtomicBool::new(false);

/// Per-process counter so concurrent [`Breadcrumb`]s don't share a file.
static CRUMB_SEQ: AtomicU64 = AtomicU64::new(0);

// ─────────────────────────────────────────────────────────────────────────────
// Breadcrumbs — the catch-all for deaths that bypass both hooks
// ─────────────────────────────────────────────────────────────────────────────

/// While alive, a file on disk records what the app is attempting. Dropped on
/// normal completion (file removed); left behind only if the process dies hard
/// without unwinding. `install()` surfaces a leftover on the next launch.
///
/// ```ignore
/// let _crumb = crash::Breadcrumb::new(format!("opening {}", path.display()));
/// // …risky decode…
/// ```
#[must_use = "bind to a variable (`let _crumb = …`); dropping it immediately clears the breadcrumb"]
pub struct Breadcrumb {
    path: PathBuf,
}

impl Breadcrumb {
    pub fn new(what: impl AsRef<str>) -> Self {
        let id = CRUMB_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = crate::config::kadr_dir();
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("activity-{id}.txt"));
        let _ = std::fs::write(&path, format!("{}\n{}", stamp_human(), what.as_ref()));
        Self { path }
    }
}

impl Drop for Breadcrumb {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Delete every breadcrumb file, returning the text of the most recent one.
fn take_breadcrumbs() -> Option<String> {
    let dir = crate::config::kadr_dir();
    let mut newest: Option<(std::time::SystemTime, String)> = None;
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !(name.starts_with("activity-") && name.ends_with(".txt")) {
            continue;
        }
        let when = entry
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
        let _ = std::fs::remove_file(entry.path());
        if newest.as_ref().is_none_or(|(t, _)| when >= *t) {
            newest = Some((when, text));
        }
    }
    newest.map(|(_, text)| text)
}

/// Install the panic hook and (on Windows) the unhandled-exception filter.
/// Call this as the very first thing in `main()`.
pub fn install() {
    // A leftover breadcrumb means the last run died in a way neither hook can
    // catch. Report it before arming anything for this run.
    if let Some(activity) = take_breadcrumbs() {
        let (when, what) = activity.split_once('\n').unwrap_or(("", activity.as_str()));
        let when = when.trim();
        let what = what.trim();
        show_popup(
            "kadr — previous session ended unexpectedly",
            &format!(
                "Last time, kadr closed with no warning while:\n\n\
                 {what}\n\n\
                 (started {when})\n\n\
                 This is almost always a corrupt or unsupported file — most \
                 often one claiming impossible dimensions, which runs the \
                 machine out of memory. The same file will do it every time."
            ),
        );
    }

    std::panic::set_hook(Box::new(move |info| {
        let (short, full) = format_panic(info);
        // Still print — helps when a console *is* attached (debug builds).
        eprintln!("\n=== kadr panic ===\n{full}\n");
        report(
            "kadr stopped because of an internal error (panic).",
            &short,
            &full,
            101,
        );
    }));

    #[cfg(windows)]
    install_seh();
}

// ─────────────────────────────────────────────────────────────────────────────
// Panic path
// ─────────────────────────────────────────────────────────────────────────────

fn format_panic(info: &std::panic::PanicHookInfo<'_>) -> (String, String) {
    let payload = info.payload();
    let msg = payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_string());

    let location = info
        .location()
        .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
        .unwrap_or_else(|| "unknown location".to_string());

    let thread = std::thread::current();
    let thread = thread.name().unwrap_or("<unnamed>").to_string();

    // `force_capture` works even without RUST_BACKTRACE=1. Symbol quality
    // depends on the build (release is `strip = "symbols"`), but the frame
    // addresses are always there.
    let backtrace = std::backtrace::Backtrace::force_capture();

    let short = format!("{msg}\n\nat {location}\nthread: {thread}");
    let full = format!(
        "kind          : Rust panic\n\
         message       : {msg}\n\
         location      : {location}\n\
         thread        : {thread}\n\
         \n---- backtrace ----\n{backtrace}"
    );
    (short, full)
}

// ─────────────────────────────────────────────────────────────────────────────
// Windows structured-exception path
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    // kernel32 exports declared here directly to keep the `winapi` feature
    // surface (and any per-version module churn) small.
    fn RtlCaptureStackBackTrace(
        frames_to_skip: u32,
        frames_to_capture: u32,
        back_trace: *mut *mut core::ffi::c_void,
        back_trace_hash: *mut u32,
    ) -> u16;
    fn SetThreadStackGuarantee(stack_size_in_bytes: *mut u32) -> i32;
    fn SetErrorMode(u_mode: u32) -> u32;
}

#[cfg(windows)]
fn install_seh() {
    use winapi::um::errhandlingapi::SetUnhandledExceptionFilter;

    // Suppress the default "kadr.exe has stopped working" WER dialog — ours
    // replaces it.
    const SEM_FAILCRITICALERRORS: u32 = 0x0001;
    const SEM_NOGPFAULTERRORBOX: u32 = 0x0002;

    // Reserve stack for the filter to run in even when the crash *is* a stack
    // overflow. Per-thread; covers the main/UI thread where most faults land.
    let mut guarantee: u32 = 32 * 1024;

    unsafe {
        SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX);
        SetThreadStackGuarantee(&mut guarantee);
        SetUnhandledExceptionFilter(Some(seh_filter));
    }
}

#[cfg(windows)]
unsafe extern "system" fn seh_filter(info: *mut winapi::um::winnt::EXCEPTION_POINTERS) -> i32 {
    use std::fmt::Write as _;
    use winapi::um::processthreadsapi::GetCurrentThreadId;

    let headline = "kadr crashed inside native code (access violation or similar).";

    unsafe {
        // Pull the few fields we need out of the EXCEPTION_RECORD up front.
        let (code, addr, av_kind, av_addr) = if info.is_null() || (*info).ExceptionRecord.is_null()
        {
            (0u32, 0usize, None, 0usize)
        } else {
            let rec = &*(*info).ExceptionRecord;
            let code = rec.ExceptionCode;
            let addr = rec.ExceptionAddress as usize;
            // For access violations / in-page errors, ExceptionInformation holds
            // [operation, faulting-address], operation 0=read 1=write 8=exec.
            let (kind, bad) =
                if matches!(code, 0xC000_0005 | 0xC000_0006) && rec.NumberParameters >= 2 {
                    let kind = match rec.ExceptionInformation[0] {
                        0 => "read from",
                        1 => "write to",
                        8 => "execute (DEP) at",
                        _ => "access",
                    };
                    (Some(kind), rec.ExceptionInformation[1])
                } else {
                    (None, 0)
                };
            (code, addr, kind, bad)
        };

        // Another thread already reporting (or a fault inside this handler)?
        // Bail before doing any allocation.
        if REPORTING.swap(true, Ordering::SeqCst) {
            hard_exit(code.max(1));
        }
        let _ = take_breadcrumbs();

        let name = exception_name(code);
        let module = module_for(addr);

        // ── Short reason (message box) + the cheap half of the log ───────────
        let mut short = String::new();
        let _ = write!(short, "{name}");
        if let Some(kind) = av_kind {
            let _ = write!(short, "  —  invalid {kind} 0x{av_addr:X}");
        }
        match &module {
            Some((m, off)) => {
                let _ = write!(short, "\nin {m}+0x{off:X}");
            }
            None => {
                let _ = write!(short, "\nat 0x{addr:X}");
            }
        }

        let mut full = String::with_capacity(4096);
        let _ = writeln!(full, "kind          : Windows structured exception");
        let _ = writeln!(full, "exception     : {name} (0x{code:08X})");
        let _ = writeln!(full, "thread id     : {}", GetCurrentThreadId());
        if let Some(kind) = av_kind {
            let _ = writeln!(full, "invalid access: {kind} 0x{av_addr:016X}");
        }
        match &module {
            Some((m, off)) => {
                let _ = writeln!(full, "faulting ip   : {m}+0x{off:X}  (0x{addr:016X})");
            }
            None => {
                let _ = writeln!(full, "faulting ip   : 0x{addr:016X}  (unknown module)");
            }
        }

        // ── Persist + notify BEFORE the stack walk ──────────────────────────
        // The walk touches a corrupted stack and can fault again; do it last
        // so the user has already seen the popup and a log by then.
        let saved = write_log(env!("CARGO_PKG_VERSION"), headline, &full);
        let mut text = format!("{headline}\n\n{short}");
        match &saved {
            Some(path) => text.push_str(&format!("\n\nFull crash report:\n{}", path.display())),
            None => text.push_str("\n\n(could not write a crash log)"),
        }
        show_popup("kadr — crash report", &text);

        // ── Best-effort stack walk, appended to the log ─────────────────────
        if let Some(path) = &saved {
            let mut walk = String::new();
            let mut frames: [*mut core::ffi::c_void; 62] = [core::ptr::null_mut(); 62];
            let n = RtlCaptureStackBackTrace(
                0,
                frames.len() as u32,
                frames.as_mut_ptr(),
                core::ptr::null_mut(),
            ) as usize;
            let _ = writeln!(walk, "\n---- stack ({n} frames, return addresses) ----");
            for (i, frame) in frames.iter().take(n).enumerate() {
                let ret = *frame as usize;
                match module_for(ret) {
                    Some((m, off)) => {
                        let _ = writeln!(walk, "  #{i:02}  {m}+0x{off:X}");
                    }
                    None => {
                        let _ = writeln!(walk, "  #{i:02}  0x{ret:016X}");
                    }
                }
            }
            let _ = writeln!(
                walk,
                "\nResolve the `module+0xoffset` entries against a matching build \
                 (map file or unstripped binary) for function names."
            );
            use std::io::Write as _;
            let _ = std::fs::OpenOptions::new()
                .append(true)
                .open(path)
                .map(|mut f| f.write_all(walk.as_bytes()));
        }

        hard_exit(code.max(1));
    }
}

/// Human name for a Windows exception / NTSTATUS code.
#[cfg(windows)]
fn exception_name(code: u32) -> &'static str {
    match code {
        0xC000_0005 => "ACCESS_VIOLATION",
        0xC000_0006 => "IN_PAGE_ERROR",
        0xC000_001D => "ILLEGAL_INSTRUCTION",
        0xC000_0096 => "PRIVILEGED_INSTRUCTION",
        0xC000_00FD => "STACK_OVERFLOW",
        0xC000_0094 => "INTEGER_DIVIDE_BY_ZERO",
        0xC000_0095 => "INTEGER_OVERFLOW",
        0xC000_008C => "ARRAY_BOUNDS_EXCEEDED",
        0xC000_008E => "FLOAT_DIVIDE_BY_ZERO",
        0xC000_0090 => "FLOAT_INVALID_OPERATION",
        0xC000_00FE => "TIMEOUT",
        0x8000_0002 => "DATATYPE_MISALIGNMENT",
        0x8000_0003 => "BREAKPOINT",
        0x8000_0004 => "SINGLE_STEP",
        0xC000_0409 => "STACK_BUFFER_OVERRUN / __fastfail",
        0xC000_0374 => "HEAP_CORRUPTION",
        0xC000_0017 => "NO_MEMORY",
        0xC000_0025 => "NONCONTINUABLE_EXCEPTION",
        0xC000_001E => "INVALID_DISPOSITION",
        0xE06D_7363 => "unhandled C++ exception",
        0 => "no exception record available",
        _ => "unrecognised exception",
    }
}

/// Resolve an address to `(module file name, offset from module base)`.
#[cfg(windows)]
fn module_for(addr: usize) -> Option<(String, usize)> {
    use std::os::windows::ffi::OsStringExt;
    use winapi::shared::minwindef::HMODULE;
    use winapi::um::libloaderapi::{GetModuleFileNameW, GetModuleHandleExW};

    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;
    const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x0000_0002;

    if addr == 0 {
        return None;
    }

    let mut hmodule: HMODULE = core::ptr::null_mut();
    let mut buf = [0u16; 260];

    let len = unsafe {
        let ok = GetModuleHandleExW(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            addr as *const u16,
            &mut hmodule,
        );
        if ok == 0 || hmodule.is_null() {
            return None;
        }
        GetModuleFileNameW(hmodule, buf.as_mut_ptr(), buf.len() as u32) as usize
    };

    let base = hmodule as usize;
    let len = len.min(buf.len());

    let name = if len == 0 {
        format!("0x{base:X}")
    } else {
        let os = std::ffi::OsString::from_wide(&buf[..len]);
        std::path::Path::new(&os)
            .file_name()
            .and_then(|f| f.to_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("0x{base:X}"))
    };

    Some((name, addr.wrapping_sub(base)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared reporter
// ─────────────────────────────────────────────────────────────────────────────

/// `headline` — one plain-language line. `short` — reason for the message box.
/// `full` — everything, written to the crash log. Never returns.
fn report(headline: &str, short: &str, full: &str, exit_code: u32) -> ! {
    if REPORTING.swap(true, Ordering::SeqCst) {
        // A crash is already being reported (e.g. the panic hook fired, then
        // `panic = "abort"` re-entered us as a structured exception). Leave
        // now without a second dialog.
        hard_exit(exit_code);
    }
    // We're handling it here, so don't also report it as a breadcrumb next launch.
    let _ = take_breadcrumbs();

    let version = env!("CARGO_PKG_VERSION");
    let saved = write_log(version, headline, full);

    let mut text = format!("{headline}\n\n{short}");
    match &saved {
        Some(path) => {
            text.push_str(&format!("\n\nFull crash report:\n{}", path.display()));
        }
        None => text.push_str("\n\n(could not write a crash log)"),
    }

    show_popup("kadr — crash report", &text);
    hard_exit(exit_code);
}

fn write_log(version: &str, headline: &str, full: &str) -> Option<std::path::PathBuf> {
    let dir = crate::config::kadr_dir();
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("crash-{}.log", stamp_compact()));

    let when = stamp_human();
    let rule = "-".repeat(64);
    let contents = format!(
        "kadr {version} — crash report\n\
         when : {when}\n\
         {headline}\n\
         {rule}\n\n\
         {full}\n"
    );

    std::fs::write(&path, contents).ok()?;
    Some(path)
}

fn show_popup(title: &str, text: &str) {
    #[cfg(windows)]
    unsafe {
        use std::os::windows::ffi::OsStrExt;
        use winapi::um::winuser::{
            MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MB_SYSTEMMODAL, MB_TOPMOST, MessageBoxW,
        };

        let text: Vec<u16> = std::ffi::OsStr::new(text)
            .encode_wide()
            .chain(core::iter::once(0))
            .collect();
        let title: Vec<u16> = std::ffi::OsStr::new(title)
            .encode_wide()
            .chain(core::iter::once(0))
            .collect();

        MessageBoxW(
            core::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SYSTEMMODAL | MB_SETFOREGROUND | MB_TOPMOST,
        );
    }

    #[cfg(not(windows))]
    eprintln!("[{title}]\n{text}");
}

fn hard_exit(code: u32) -> ! {
    #[cfg(windows)]
    unsafe {
        use winapi::um::processthreadsapi::{GetCurrentProcess, TerminateProcess};
        TerminateProcess(GetCurrentProcess(), code);
    }
    std::process::exit(code as i32);
}

// ── timestamps (no chrono dependency) ────────────────────────────────────────

#[cfg(windows)]
fn now_parts() -> (u16, u16, u16, u16, u16, u16) {
    unsafe {
        use winapi::um::minwinbase::SYSTEMTIME;
        use winapi::um::sysinfoapi::GetLocalTime;
        let mut st: SYSTEMTIME = core::mem::zeroed();
        GetLocalTime(&mut st);
        (
            st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond,
        )
    }
}

#[cfg(windows)]
fn stamp_compact() -> String {
    let (y, mo, d, h, mi, s) = now_parts();
    format!("{y:04}{mo:02}{d:02}-{h:02}{mi:02}{s:02}")
}

#[cfg(windows)]
fn stamp_human() -> String {
    let (y, mo, d, h, mi, s) = now_parts();
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

#[cfg(not(windows))]
fn stamp_compact() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        .to_string()
}

#[cfg(not(windows))]
fn stamp_human() -> String {
    stamp_compact()
}
