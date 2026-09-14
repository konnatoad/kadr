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

/// Set once eframe's own shutdown starts, so the `atexit` hook can tell a
/// normal close apart from an unexpected `exit()` call.
static NORMAL_EXIT: AtomicBool = AtomicBool::new(false);

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
    snapshot_modules();

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

    install_exit_hook();

    #[cfg(windows)]
    install_seh();
}

/// Give the current thread the same stack-overflow headroom the main thread
/// gets, so `seh_filter` can still run if this thread overflows its stack.
/// Call at the top of any `thread::spawn` closure that decodes files or calls
/// into native libraries.
#[cfg(windows)]
pub fn guard_thread_stack() {
    let mut guarantee: u32 = 32 * 1024;
    unsafe {
        SetThreadStackGuarantee(&mut guarantee);
    }
}

#[cfg(not(windows))]
pub fn guard_thread_stack() {}

/// Re-arm kadr's own exception filter. Some GPU drivers install their own
/// `SetUnhandledExceptionFilter` during OpenGL context creation, silently
/// replacing ours — call this once the GL context exists to win it back.
#[cfg(windows)]
pub fn reassert() {
    install_seh();
}

#[cfg(not(windows))]
pub fn reassert() {}

/// Call when eframe's shutdown begins, so `on_process_exit` below knows the
/// close was expected instead of logging a normal quit as a crash.
pub fn mark_normal_exit() {
    NORMAL_EXIT.store(true, Ordering::SeqCst);
}

fn install_exit_hook() {
    unsafe extern "C" {
        fn atexit(cb: extern "C" fn()) -> i32;
    }
    unsafe {
        atexit(on_process_exit);
    }
}

/// Catches a native library calling `exit()` directly — no panic, no Windows
/// exception, so neither other hook ever sees it. No stack trace is possible
/// here, but it beats the process just vanishing.
extern "C" fn on_process_exit() {
    if NORMAL_EXIT.load(Ordering::SeqCst) || REPORTING.load(Ordering::SeqCst) {
        return;
    }
    let crumb = take_breadcrumbs();
    let mut full =
        String::from("kind          : process exited via exit(), not a panic or Windows exception\n");
    if let Some(what) = &crumb {
        full.push_str(&format!("last activity : {}\n", what.replace('\n', "  —  ")));
    }
    write_log(env!("CARGO_PKG_VERSION"), "kadr exited unexpectedly.", &full);
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

        let name = exception_name(code);
        let module = module_for(addr);

        // Walk the stack and gather everything else before writing or showing
        // anything — this used to happen after the popup, so a fault in the
        // walk (touching a corrupted stack) silently ate the trace every time.
        let mut frames: [*mut core::ffi::c_void; 62] = [core::ptr::null_mut(); 62];
        let n = RtlCaptureStackBackTrace(
            0,
            frames.len() as u32,
            frames.as_mut_ptr(),
            core::ptr::null_mut(),
        ) as usize;

        let mut stack = String::new();
        let _ = writeln!(stack, "---- stack ({n} frames, return addresses) ----");
        for (i, frame) in frames.iter().take(n).enumerate() {
            let ret = *frame as usize;
            match module_for(ret) {
                Some((m, off)) => {
                    let _ = writeln!(stack, "  #{i:02}  {m}+0x{off:X}");
                }
                None => {
                    let _ = writeln!(stack, "  #{i:02}  0x{ret:016X}");
                }
            }
        }

        let modules = loaded_modules();
        let crumb = take_breadcrumbs();

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
            match module_for(av_addr) {
                Some((m, off)) => {
                    let _ = writeln!(full, "  ...inside   : {m}+0x{off:X}");
                }
                None => {
                    let _ = writeln!(full, "  ...inside   : (not in any loaded module)");
                }
            }
        }
        match &module {
            Some((m, off)) => {
                let _ = writeln!(full, "faulting ip   : {m}+0x{off:X}  (0x{addr:016X})");
            }
            None => {
                let _ = writeln!(full, "faulting ip   : 0x{addr:016X}  (unknown module)");
            }
        }
        if let Some(what) = &crumb {
            let _ = writeln!(full, "last activity : {}", what.replace('\n', "  —  "));
        }
        let _ = writeln!(full);
        let _ = write!(full, "{stack}");
        let _ = writeln!(full);
        let _ = write!(full, "{modules}");

        let saved = write_log(env!("CARGO_PKG_VERSION"), headline, &full);
        let mut text = format!("{headline}\n\n{short}");
        match &saved {
            Some(path) => text.push_str(&format!("\n\nFull crash report:\n{}", path.display())),
            None => text.push_str("\n\n(could not write a crash log)"),
        }
        show_popup("kadr — crash report", &text);

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

/// Snapshot of every loaded module's (base, size, name), built by
/// `snapshot_modules` while the process is healthy. Crash-time code reads
/// this instead of asking the loader directly — see `snapshot_modules` for
/// why.
static MODULE_CACHE: std::sync::Mutex<Vec<(usize, usize, String)>> = std::sync::Mutex::new(Vec::new());

/// Record the current module list somewhere crash-time code can read it
/// without going through the loader. `GetModuleHandleExW`, `GetModuleFileNameW`
/// and `EnumProcessModules` all need the loader lock internally — if the
/// thread that's crashing happened to be mid-`LoadLibrary` (exactly how
/// `heif.dll`/`raw_r.dll` get loaded) when it died, calling any of those from
/// inside the exception handler blocks on a lock that's never coming back,
/// and the crash popup just never shows up. Call this while things are fine
/// (app startup, after the GL context exists) so crash time is a plain,
/// lock-free scan of an already-built list instead.
#[cfg(windows)]
pub fn snapshot_modules() {
    use std::os::windows::ffi::OsStringExt;
    use winapi::shared::minwindef::HMODULE;
    use winapi::um::libloaderapi::GetModuleFileNameW;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::psapi::{EnumProcessModules, GetModuleInformation, MODULEINFO};

    let mut list = Vec::new();
    unsafe {
        let process = GetCurrentProcess();
        let mut modules: [HMODULE; 256] = [core::ptr::null_mut(); 256];
        let mut needed: u32 = 0;
        let cb = (modules.len() * core::mem::size_of::<HMODULE>()) as u32;
        if EnumProcessModules(process, modules.as_mut_ptr(), cb, &mut needed) != 0 {
            let count = (needed as usize / core::mem::size_of::<HMODULE>()).min(modules.len());
            for &hmod in modules.iter().take(count) {
                let mut buf = [0u16; 260];
                let len = GetModuleFileNameW(hmod, buf.as_mut_ptr(), buf.len() as u32) as usize;
                if len == 0 {
                    continue;
                }
                let os = std::ffi::OsString::from_wide(&buf[..len.min(buf.len())]);
                let name = std::path::Path::new(&os)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .map(str::to_string)
                    .unwrap_or_default();

                let mut info: MODULEINFO = core::mem::zeroed();
                if GetModuleInformation(
                    process,
                    hmod,
                    &mut info,
                    core::mem::size_of::<MODULEINFO>() as u32,
                ) != 0
                {
                    list.push((info.lpBaseOfDll as usize, info.SizeOfImage as usize, name));
                }
            }
        }
    }

    if let Ok(mut cache) = MODULE_CACHE.lock() {
        *cache = list;
    }
}

#[cfg(not(windows))]
pub fn snapshot_modules() {}

/// Resolve an address to `(module file name, offset from module base)`
/// against the snapshot from `snapshot_modules`.
#[cfg(windows)]
fn module_for(addr: usize) -> Option<(String, usize)> {
    if addr == 0 {
        return None;
    }
    let cache = MODULE_CACHE.lock().ok()?;
    for (base, size, name) in cache.iter() {
        if addr >= *base && addr < base + size {
            return Some((name.clone(), addr - base));
        }
    }
    None
}

/// Every loaded module's name, base address and size, from the snapshot —
/// lets a stack frame or bad-access address be matched against a module by
/// hand if it doesn't resolve directly.
#[cfg(windows)]
fn loaded_modules() -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "---- loaded modules ----");
    match MODULE_CACHE.lock() {
        Ok(cache) if !cache.is_empty() => {
            for (base, size, name) in cache.iter() {
                let _ = writeln!(out, "  {name:<28} base=0x{base:016X}  size=0x{size:X}");
            }
        }
        _ => {
            let _ = writeln!(out, "(no module snapshot taken)");
        }
    }
    out
}

#[cfg(not(windows))]
fn loaded_modules() -> String {
    String::new()
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
    let crumb = take_breadcrumbs();

    let mut full = full.to_string();
    if let Some(what) = &crumb {
        full = format!("last activity : {}\n\n{full}", what.replace('\n', "  —  "));
    }
    full.push_str("\n\n");
    full.push_str(&loaded_modules());

    let version = env!("CARGO_PKG_VERSION");
    let saved = write_log(version, headline, &full);

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
