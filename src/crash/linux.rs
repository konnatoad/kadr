//! The Linux half of crash reporting.
//!
//! A native crash on Linux arrives as a signal: SIGSEGV, SIGBUS, SIGILL,
//! SIGFPE, or SIGABRT when a library calls `abort()`. A signal handler may only
//! make the few calls that are async-signal-safe: no allocating, no locks the
//! crashed thread might be holding. So [`on_signal`] does two things:
//!
//!   1. writes the facts with `write(2)` into `crash-pending.txt`: the signal,
//!      the bad address, the stack as `module+0xoffset`, the loaded libraries;
//!   2. replaces the dying process with `kadr --crash-report <that file>`.
//!
//! The new process starts with a clean heap, so it can do what the Windows
//! filter does in place: write `crash-<stamp>.log`, name kadr's own frames
//! (through `addr2line`, when there are symbols) and show the popup. If the
//! exec fails, the next launch finds `crash-pending.txt` and reports it then.
//!
//! The library list is read live with `dl_iterate_phdr` rather than from a
//! snapshot the way Windows does it: glibc's loader lock is recursive, so the
//! handler can take it even when the crash happened in the middle of a
//! `dlopen`, and libraries mpv or the GL driver open later are still listed.

use std::ffi::{CStr, CString, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

/// The handler's facts, left in the state dir until a report is made from them.
const PENDING: &str = "crash-pending.txt";

const SIGNALS: [c_int; 5] = [
    libc::SIGSEGV,
    libc::SIGBUS,
    libc::SIGILL,
    libc::SIGFPE,
    libc::SIGABRT,
];

/// Each guarded thread's signal stack. Rust gives threads 8 KiB, and the kernel's
/// saved CPU state plus the unwinder that `backtrace()` runs can need more.
const ALT_STACK_SIZE: usize = 64 * 1024;

const MAX_FRAMES: usize = 64;

const CRASH_HEADLINE: &str = "kadr crashed inside native code (segmentation fault or similar).";
const ABORT_HEADLINE: &str =
    "kadr was stopped by abort(): a library gave up on a failed check, or memory ran out.";

/// `crash-pending.txt` as a C string, made at install time because the handler can't allocate.
static PENDING_PATH: OnceLock<CString> = OnceLock::new();

static EXE_NAME: OnceLock<String> = OnceLock::new();

static BUILD_ID: OnceLock<String> = OnceLock::new();

/// Arm the crash handlers for this process. `crash::install()` calls this last.
pub fn install() {
    let dir = crate::config::state_dir();
    let _ = std::fs::create_dir_all(&dir);
    if let Ok(path) = CString::new(dir.join(PENDING).as_os_str().as_bytes()) {
        let _ = PENDING_PATH.set(path);
    }
    // the handler reads these and must not be the one to fill them
    exe_name();
    build_id();
    // the first backtrace() loads libgcc_s, which allocates: get that done now
    let mut frames = [std::ptr::null_mut(); 2];
    // SAFETY: `frames` is writable for the 2 entries passed
    unsafe { libc::backtrace(frames.as_mut_ptr(), 2) };
    guard_thread_stack();
    arm();
}

fn arm() {
    let handler: extern "C" fn(c_int, *mut libc::siginfo_t, *mut c_void) = on_signal;
    for signal in SIGNALS {
        // SAFETY: `action` is zeroed then filled in completely (handler, flags,
        // empty mask) before sigaction reads it, and `on_signal` has the
        // three-argument signature SA_SIGINFO calls for
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            action.sa_sigaction = handler as libc::sighandler_t;
            // RESETHAND: a second crash inside the handler takes the default way out
            action.sa_flags = libc::SA_SIGINFO | libc::SA_ONSTACK | libc::SA_RESETHAND;
            libc::sigemptyset(&mut action.sa_mask);
            libc::sigaction(signal, &action, std::ptr::null_mut());
        }
    }
}

/// Put kadr's handlers back in case a library replaced them while it loaded.
pub fn reassert() {
    arm();
}

// ─────────────────────────────────────────────────────────────────────────────
// Signal stacks
// ─────────────────────────────────────────────────────────────────────────────

/// Give this thread a signal stack big enough for [`on_signal`], so the report
/// still gets written when the crash is this thread running out of stack. Rust's
/// own 8 KiB one stays in place for threads that never call this.
pub fn guard_thread_stack() {
    thread_local! {
        static ALT_STACK: Option<AltStack> = AltStack::new();
    }
    ALT_STACK.with(|_| {});
}

struct AltStack {
    map: *mut c_void,
    map_len: usize,
    stack: *mut c_void,
}

impl AltStack {
    fn new() -> Option<Self> {
        // SAFETY: sysconf has no preconditions
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        let page = usize::try_from(page).ok().filter(|&p| p > 0).unwrap_or(4096);
        let map_len = page + ALT_STACK_SIZE;
        // SAFETY: a fresh anonymous mapping that nothing else points into. The
        // lowest page becomes a guard (stacks grow down), so overflowing the
        // signal stack faults instead of writing over other memory, and
        // sigaltstack only reads `alt` for the duration of the call
        unsafe {
            let map = libc::mmap(
                std::ptr::null_mut(),
                map_len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            if map == libc::MAP_FAILED {
                return None;
            }
            libc::mprotect(map, page, libc::PROT_NONE);
            let stack = map.cast::<u8>().add(page).cast::<c_void>();
            let alt = libc::stack_t {
                ss_sp: stack,
                ss_flags: 0,
                ss_size: ALT_STACK_SIZE,
            };
            if libc::sigaltstack(&alt, std::ptr::null_mut()) != 0 {
                libc::munmap(map, map_len);
                return None;
            }
            Some(Self {
                map,
                map_len,
                stack,
            })
        }
    }
}

impl Drop for AltStack {
    fn drop(&mut self) {
        // SAFETY: the stack is switched off first if it's still this thread's
        // signal stack, so the unmapped memory is never used again; the mapping
        // is the one `new` made, unmapped once
        unsafe {
            let mut current: libc::stack_t = std::mem::zeroed();
            if libc::sigaltstack(std::ptr::null(), &mut current) == 0 && current.ss_sp == self.stack
            {
                let off = libc::stack_t {
                    ss_sp: std::ptr::null_mut(),
                    ss_flags: libc::SS_DISABLE,
                    ss_size: 0,
                };
                libc::sigaltstack(&off, std::ptr::null_mut());
            }
            libc::munmap(self.map, self.map_len);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The handler
// ─────────────────────────────────────────────────────────────────────────────

/// Everything in here is async-signal-safe: plain syscalls, the already-loaded
/// unwinder, and text formatted into a stack buffer.
extern "C" fn on_signal(signal: c_int, info: *mut libc::siginfo_t, context: *mut c_void) {
    // the first crash reports; a second one (another thread, or this handler
    // failing) takes the default way out without a second report
    if !super::REPORTING.swap(true, Ordering::SeqCst)
        && let Some(pending) = PENDING_PATH.get()
    {
        // if a lock below never frees, SIGALRM ends the process instead of
        // leaving a frozen window; what was written by then is still reported
        // SAFETY: alarm has no preconditions
        unsafe { libc::alarm(10) };
        // SAFETY: the kernel hands the handler a valid siginfo_t and ucontext_t
        let fault = unsafe { Fault::read(signal, info, context) };
        if write_pending(pending, &fault) {
            exec_reporter(pending);
        }
    }
    die(signal);
}

/// What would have happened without kadr's handler: the signal's default action
/// (a core dump, and "Segmentation fault" in a terminal).
fn die(signal: c_int) {
    // SAFETY: alarm, signal and raise are async-signal-safe. The raised signal
    // is blocked while the handler runs and lands as it returns, now with the
    // default action
    unsafe {
        libc::alarm(0);
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

/// Swap the crashed process for `kadr --crash-report <pending>`.
fn exec_reporter(pending: &CStr) {
    unsafe extern "C" {
        static mut environ: *const *const libc::c_char;
    }
    let argv = [
        c"kadr".as_ptr(),
        c"--crash-report".as_ptr(),
        pending.as_ptr(),
        std::ptr::null(),
    ];
    // SAFETY: argv is a null-terminated array of C strings that outlive the
    // call, and environ is the process's own environment. A pending alarm and
    // the blocked crash signal would carry over into the new program, so
    // both are cleared first. On success execve doesn't return
    unsafe {
        libc::alarm(0);
        let mut none: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut none);
        libc::sigprocmask(libc::SIG_SETMASK, &none, std::ptr::null_mut());
        libc::execve(c"/proc/self/exe".as_ptr(), argv.as_ptr(), environ);
    }
}

/// The facts about one crash, read out of what the kernel handed the handler.
struct Fault {
    signal: c_int,
    code: c_int,
    /// The bad address for SIGSEGV/SIGBUS, the instruction for SIGILL/SIGFPE.
    addr: usize,
    ip: usize,
    sp: usize,
    /// x86-64's page-fault error code: what the bad access was trying to do.
    error: Option<usize>,
}

impl Fault {
    /// # Safety
    /// `info` and `context` are null or what the kernel passed a SA_SIGINFO handler.
    unsafe fn read(signal: c_int, info: *mut libc::siginfo_t, context: *mut c_void) -> Self {
        // SAFETY: as the caller promises; si_addr is the same field for every
        // signal and only read back for the ones where it means something
        let (code, addr) = unsafe {
            match info.as_ref() {
                Some(info) => (info.si_code, info.si_addr() as usize),
                None => (0, 0),
            }
        };
        // SAFETY: as the caller promises
        let (ip, sp, error) = unsafe { registers(context) };
        Self {
            signal,
            code,
            addr,
            ip,
            sp,
            error,
        }
    }

    /// SIGSEGV with the bad address just under the stack pointer: a thread used
    /// up its stack.
    fn stack_overflow(&self) -> bool {
        self.signal == libc::SIGSEGV
            && self.code > 0
            && self.sp != 0
            && self.addr <= self.sp.wrapping_add(256)
            && self.sp.wrapping_sub(self.addr) < 64 * 1024
    }

    /// The popup's name for it: the signal, or STACK_OVERFLOW like Windows says.
    fn name(&self) -> &'static str {
        if self.stack_overflow() {
            "STACK_OVERFLOW"
        } else {
            self.signal_name()
        }
    }

    fn signal_name(&self) -> &'static str {
        match self.signal {
            libc::SIGSEGV => "SIGSEGV",
            libc::SIGBUS => "SIGBUS",
            libc::SIGILL => "SIGILL",
            libc::SIGFPE => "SIGFPE",
            libc::SIGABRT => "SIGABRT",
            _ => "signal",
        }
    }

    fn detail(&self) -> &'static str {
        // si_code values from <asm-generic/siginfo.h>
        match (self.signal, self.code) {
            (libc::SIGABRT, _) => "abort() was called",
            (_, ..=0) => "sent with kill() or raise()",
            (libc::SIGSEGV, 1) => "address not mapped",
            (libc::SIGSEGV, 2) => "no permission for that address",
            (libc::SIGBUS, 1) => "misaligned address",
            (libc::SIGBUS, 2) => "address past the end of a mapped file",
            (libc::SIGBUS, 3) => "hardware memory error",
            (libc::SIGILL, 1) => "illegal opcode",
            (libc::SIGFPE, 1) => "integer divide by zero",
            (libc::SIGFPE, 2) => "integer overflow",
            (libc::SIGFPE, 3) => "float divide by zero",
            _ => "no detail",
        }
    }

    /// What the bad access was, for the faults that have one.
    fn access(&self) -> Option<&'static str> {
        if self.code <= 0 || !matches!(self.signal, libc::SIGSEGV | libc::SIGBUS) {
            return None;
        }
        Some(match self.error {
            Some(e) if self.signal == libc::SIGSEGV && e & 0x10 != 0 => "execute at",
            Some(e) if self.signal == libc::SIGSEGV && e & 0x2 != 0 => "write to",
            Some(_) if self.signal == libc::SIGSEGV => "read from",
            _ => "access",
        })
    }
}

/// `(instruction pointer, stack pointer, page-fault error code)` at the crash.
///
/// # Safety
/// `context` is null or the `ucontext_t` the kernel passed a SA_SIGINFO handler.
#[cfg(target_arch = "x86_64")]
unsafe fn registers(context: *mut c_void) -> (usize, usize, Option<usize>) {
    // SAFETY: as the caller promises
    let Some(context) = (unsafe { context.cast::<libc::ucontext_t>().as_ref() }) else {
        return (0, 0, None);
    };
    let regs = &context.uc_mcontext.gregs;
    (
        regs[libc::REG_RIP as usize] as usize,
        regs[libc::REG_RSP as usize] as usize,
        Some(regs[libc::REG_ERR as usize] as usize),
    )
}

/// # Safety
/// `context` is null or the `ucontext_t` the kernel passed a SA_SIGINFO handler.
#[cfg(target_arch = "aarch64")]
unsafe fn registers(context: *mut c_void) -> (usize, usize, Option<usize>) {
    // SAFETY: as the caller promises
    let Some(context) = (unsafe { context.cast::<libc::ucontext_t>().as_ref() }) else {
        return (0, 0, None);
    };
    let regs = &context.uc_mcontext;
    (regs.pc as usize, regs.sp as usize, None)
}

/// # Safety
/// Always safe; the registers just aren't read on this architecture.
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
unsafe fn registers(_context: *mut c_void) -> (usize, usize, Option<usize>) {
    (0, 0, None)
}

/// Write `crash-pending.txt`: a few `key: value` lines for the reporter, a
/// blank line, then the body of the crash log.
fn write_pending(pending: &CStr, fault: &Fault) -> bool {
    // SAFETY: `pending` is a valid C string; the descriptor is closed below
    let fd = unsafe {
        libc::open(
            pending.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | libc::O_CLOEXEC,
            0o600,
        )
    };
    if fd < 0 {
        return false;
    }
    let mut out = FdSink::new(fd);
    out.put(b"kadr-pending-crash\nsignal: ");
    put_dec(&mut out, fault.signal as i64);
    out.put(b"\ntime: ");
    // SAFETY: time() with a null pointer only returns the time
    put_dec(&mut out, unsafe { libc::time(std::ptr::null_mut()) } as i64);
    out.put(b"\nversion: ");
    out.put(env!("CARGO_PKG_VERSION").as_bytes());
    out.put(b"\nbuild: ");
    out.put(BUILD_ID.get().map_or("unknown", String::as_str).as_bytes());

    // the popup's two lines
    out.put(b"\nshort: ");
    out.put(fault.name().as_bytes());
    if let Some(kind) = fault.access() {
        out.put("  —  invalid ".as_bytes());
        out.put(kind.as_bytes());
        out.put(b" 0x");
        put_hex(&mut out, fault.addr, 0);
    }
    out.put(b"\nshort: ");
    if !with_module(fault.ip, |name, offset| {
        out.put(b"in ");
        out.put(name);
        out.put(b"+0x");
        put_hex(&mut out, offset, 0);
    }) {
        out.put(b"at 0x");
        put_hex(&mut out, fault.ip, 0);
    }
    out.put(b"\n\n");

    out.put(b"kind          : Linux signal\nsignal        : ");
    out.put(fault.signal_name().as_bytes());
    out.put(b" (");
    put_dec(&mut out, fault.signal as i64);
    out.put(b"), ");
    out.put(fault.detail().as_bytes());
    if fault.stack_overflow() {
        out.put(b"\n                (the bad address is right under the stack pointer: a thread ran out of stack)");
    }
    out.put(b"\nthread        : ");
    // SAFETY: gettid has no preconditions
    put_dec(&mut out, unsafe { libc::syscall(libc::SYS_gettid) } as i64);
    let mut thread_name = [0u8; 16];
    // SAFETY: PR_GET_NAME writes at most 16 bytes, nul included, into the buffer
    if unsafe { libc::prctl(libc::PR_GET_NAME, thread_name.as_mut_ptr()) } == 0 {
        let len = thread_name.iter().position(|&b| b == 0).unwrap_or(16);
        out.put(b" ");
        out.put(&thread_name[..len]);
    }
    if let Some(kind) = fault.access() {
        out.put(b"\ninvalid access: ");
        out.put(kind.as_bytes());
        out.put(b" 0x");
        put_hex(&mut out, fault.addr, 16);
        out.put(b"\n  ...inside   : ");
        if !with_module(fault.addr, |name, offset| {
            out.put(name);
            out.put(b"+0x");
            put_hex(&mut out, offset, 0);
        }) {
            out.put(b"(not in any loaded module)");
        }
    }
    out.put(b"\nfaulting ip   : ");
    if with_module(fault.ip, |name, offset| {
        out.put(name);
        out.put(b"+0x");
        put_hex(&mut out, offset, 0);
    }) {
        out.put(b"  (0x");
        put_hex(&mut out, fault.ip, 16);
        out.put(b")");
    } else {
        out.put(b"0x");
        put_hex(&mut out, fault.ip, 16);
        out.put(b"  (unknown module)");
    }
    out.put(b"\n\n");

    let mut frames = [std::ptr::null_mut(); MAX_FRAMES];
    // SAFETY: `frames` is writable for MAX_FRAMES entries, and libgcc_s was
    // loaded at install time so this doesn't allocate
    let count = unsafe { libc::backtrace(frames.as_mut_ptr(), MAX_FRAMES as c_int) };
    let frames = &frames[..usize::try_from(count).unwrap_or(0).min(MAX_FRAMES)];
    // the first frames are this handler; the crash starts at the faulting instruction
    let first = frames
        .iter()
        .position(|&frame| frame as usize == fault.ip)
        .unwrap_or(0);
    put_stack(&mut out, &frames[first..]);
    out.put(b"\n");
    put_modules(&mut out);
    drop(out);
    // SAFETY: `fd` came from the open above and is closed once
    unsafe { libc::close(fd) };
    true
}

// ─────────────────────────────────────────────────────────────────────────────
// Text without allocating
// ─────────────────────────────────────────────────────────────────────────────

/// Where text goes: a file descriptor from the handler (no allocation), a
/// `Vec` everywhere else. Both share one formatter, so a log looks the same
/// whichever path wrote it.
trait Sink {
    fn put(&mut self, bytes: &[u8]);
}

impl Sink for Vec<u8> {
    fn put(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

struct FdSink {
    fd: c_int,
    buf: [u8; 512],
    len: usize,
}

impl FdSink {
    fn new(fd: c_int) -> Self {
        Self {
            fd,
            buf: [0; 512],
            len: 0,
        }
    }

    fn flush(&mut self) {
        let mut done = 0;
        while done < self.len {
            // SAFETY: the range is inside `buf`
            let wrote = unsafe {
                libc::write(
                    self.fd,
                    self.buf[done..].as_ptr().cast(),
                    self.len - done,
                )
            };
            if wrote > 0 {
                done += wrote as usize;
            } else if wrote < 0
                && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted
            {
                continue;
            } else {
                break;
            }
        }
        self.len = 0;
    }
}

impl Sink for FdSink {
    fn put(&mut self, mut bytes: &[u8]) {
        while !bytes.is_empty() {
            if self.len == self.buf.len() {
                self.flush();
            }
            let n = bytes.len().min(self.buf.len() - self.len);
            self.buf[self.len..self.len + n].copy_from_slice(&bytes[..n]);
            self.len += n;
            bytes = &bytes[n..];
        }
    }
}

impl Drop for FdSink {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Upper-case hex, zero-padded to `width` digits.
fn put_hex(out: &mut impl Sink, value: usize, width: usize) {
    let mut digits = [0u8; 16];
    let mut start = digits.len();
    let mut rest = value;
    loop {
        start -= 1;
        digits[start] = b"0123456789ABCDEF"[rest & 0xF];
        rest >>= 4;
        if rest == 0 || start == 0 {
            break;
        }
    }
    for _ in (digits.len() - start)..width.min(digits.len()) {
        out.put(b"0");
    }
    out.put(&digits[start..]);
}

fn put_dec(out: &mut impl Sink, value: i64) {
    if value < 0 {
        out.put(b"-");
    }
    let mut digits = [0u8; 20];
    let mut start = digits.len();
    let mut rest = value.unsigned_abs();
    loop {
        start -= 1;
        digits[start] = b'0' + (rest % 10) as u8;
        rest /= 10;
        if rest == 0 {
            break;
        }
    }
    out.put(&digits[start..]);
}

/// `   #NN module+0xOFFSET`, the same lines the Windows log has.
fn put_stack(out: &mut impl Sink, frames: &[*mut c_void]) {
    out.put(b"---- stack (");
    put_dec(out, frames.len() as i64);
    out.put(b" frames, return addresses) ----\n");
    for (i, &frame) in frames.iter().enumerate() {
        out.put(b"   #");
        if i < 10 {
            out.put(b"0");
        }
        put_dec(out, i as i64);
        out.put(b" ");
        let addr = frame as usize;
        if !with_module(addr, |name, offset| {
            out.put(name);
            out.put(b"+0x");
            put_hex(out, offset, 0);
        }) {
            out.put(b"0x");
            put_hex(out, addr, 16);
        }
        out.put(b"\n");
    }
}

fn put_modules(out: &mut impl Sink) {
    out.put(b"---- loaded modules ----\n");
    each_module(|module| {
        out.put(b"  ");
        out.put(module.name);
        for _ in module.name.len()..28 {
            out.put(b" ");
        }
        out.put(b" base=0x");
        put_hex(out, module.bias, 16);
        out.put(b"  size=0x");
        put_hex(out, module.end - module.bias, 0);
        out.put(b"\n");
        false
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Loaded libraries
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(target_pointer_width = "64")]
type Phdr = libc::Elf64_Phdr;
#[cfg(target_pointer_width = "32")]
type Phdr = libc::Elf32_Phdr;

struct Module<'a> {
    /// File name only; kadr's own name for the main program.
    name: &'a [u8],
    /// Load address: `addr - bias` is the address inside the file, which is
    /// what `addr2line -e <file>` takes.
    bias: usize,
    start: usize,
    end: usize,
}

/// Call `visit` for every loaded library, the main program first, until it
/// returns `true`. Doesn't allocate.
fn each_module(mut visit: impl FnMut(&Module<'_>) -> bool) {
    unsafe extern "C" fn callback(
        info: *mut libc::dl_phdr_info,
        _size: usize,
        data: *mut c_void,
    ) -> c_int {
        // SAFETY: `data` is the `&mut dyn FnMut` below, alive for the whole
        // dl_iterate_phdr call; `info` and its program headers and name are
        // the loader's, valid for this callback
        unsafe {
            let visit = &mut *data.cast::<&mut dyn FnMut(&Module<'_>) -> bool>();
            let info = &*info;
            let headers: &[Phdr] = if info.dlpi_phdr.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(info.dlpi_phdr, usize::from(info.dlpi_phnum))
            };
            let (mut low, mut high) = (usize::MAX, 0);
            for header in headers.iter().filter(|h| h.p_type == libc::PT_LOAD) {
                low = low.min(header.p_vaddr as usize);
                high = high.max((header.p_vaddr + header.p_memsz) as usize);
            }
            if low > high {
                return 0;
            }
            let path = if info.dlpi_name.is_null() {
                &[][..]
            } else {
                CStr::from_ptr(info.dlpi_name).to_bytes()
            };
            let name = path.rsplit(|&b| b == b'/').next().unwrap_or(path);
            let name = if name.is_empty() {
                EXE_NAME.get().map_or(&b"kadr"[..], |n| n.as_bytes())
            } else {
                name
            };
            let bias = info.dlpi_addr as usize;
            let module = Module {
                name,
                bias,
                start: bias + low,
                end: bias + high,
            };
            c_int::from(visit(&module))
        }
    }
    let mut visit: &mut dyn FnMut(&Module<'_>) -> bool = &mut visit;
    // SAFETY: the callback only uses `data` as the closure reference passed here
    unsafe { libc::dl_iterate_phdr(Some(callback), (&raw mut visit).cast()) };
}

/// Run `found` with the library holding `addr` and the offset into it.
/// Returns whether one did.
fn with_module(addr: usize, found: impl FnOnce(&[u8], usize)) -> bool {
    if addr == 0 {
        return false;
    }
    let mut found = Some(found);
    each_module(|module| {
        if (module.start..module.end).contains(&addr) {
            if let Some(found) = found.take() {
                found(module.name, addr - module.bias);
            }
            return true;
        }
        false
    });
    found.is_none()
}

/// This program's file name, `kadr` unless it was renamed.
fn exe_name() -> &'static str {
    EXE_NAME.get_or_init(|| {
        std::env::current_exe()
            .ok()
            .and_then(|path| path.file_name()?.to_str().map(str::to_string))
            .unwrap_or_else(|| "kadr".to_string())
    })
}

/// The GNU build ID the linker put in this binary: a log only gets function
/// names from the exact build that wrote it.
pub fn build_id() -> &'static str {
    BUILD_ID.get_or_init(|| read_build_id().unwrap_or_else(|| "unknown".to_string()))
}

fn read_build_id() -> Option<String> {
    unsafe extern "C" fn main_program(
        info: *mut libc::dl_phdr_info,
        _size: usize,
        data: *mut c_void,
    ) -> c_int {
        const NT_GNU_BUILD_ID: u32 = 3;
        // SAFETY: `data` is the `Option<String>` below. The main program comes
        // first and stays mapped, and its PT_NOTE segments are loaded memory
        // made of 4-aligned (namesz, descsz, type, name, desc) records; every
        // read is bounds-checked against the segment's end first
        unsafe {
            let found = &mut *data.cast::<Option<String>>();
            let info = &*info;
            if info.dlpi_phdr.is_null() {
                return 1;
            }
            let headers = std::slice::from_raw_parts(info.dlpi_phdr, usize::from(info.dlpi_phnum));
            for header in headers.iter().filter(|h| h.p_type == libc::PT_NOTE) {
                let mut note = info.dlpi_addr as usize + header.p_vaddr as usize;
                let end = note + header.p_memsz as usize;
                while note + 12 <= end {
                    let word = |at: usize| (at as *const u32).read_unaligned() as usize;
                    let (name_len, desc_len, kind) = (word(note), word(note + 4), word(note + 8));
                    let name = note + 12;
                    let desc = name + name_len.next_multiple_of(4);
                    let next = desc + desc_len.next_multiple_of(4);
                    if next > end {
                        break;
                    }
                    let name_bytes = std::slice::from_raw_parts(name as *const u8, name_len);
                    if kind as u32 == NT_GNU_BUILD_ID && name_bytes == b"GNU\0" {
                        let id = std::slice::from_raw_parts(desc as *const u8, desc_len);
                        *found = Some(id.iter().map(|b| format!("{b:02x}")).collect());
                        return 1;
                    }
                    note = next;
                }
            }
            1
        }
    }
    let mut found: Option<String> = None;
    // SAFETY: the callback only uses `data` as the Option passed here
    unsafe { libc::dl_iterate_phdr(Some(main_program), (&raw mut found).cast()) };
    found
}

// ─────────────────────────────────────────────────────────────────────────────
// The same facts outside the handler (panics, exit())
// ─────────────────────────────────────────────────────────────────────────────

pub fn capture_stack() -> String {
    let mut frames = [std::ptr::null_mut(); MAX_FRAMES];
    // SAFETY: `frames` is writable for MAX_FRAMES entries
    let count = unsafe { libc::backtrace(frames.as_mut_ptr(), MAX_FRAMES as c_int) };
    let count = usize::try_from(count).unwrap_or(0).min(MAX_FRAMES);
    let mut out = Vec::new();
    put_stack(&mut out, &frames[..count]);
    String::from_utf8_lossy(&out).into_owned()
}

pub fn loaded_modules() -> String {
    let mut out = Vec::new();
    put_modules(&mut out);
    String::from_utf8_lossy(&out).into_owned()
}

/// A real invalid read inside a C library, the way a crash in libmpv or
/// libheif would happen.
pub fn raise_test_fault() {
    // SAFETY: deliberately not: this is `--crash-test native`. 0xDEAD is never
    // mapped, so strlen faults on its first read and the handler takes over
    unsafe {
        std::hint::black_box(libc::strlen(std::hint::black_box(0xDEAD as *const libc::c_char)));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The reporter: turns crash-pending.txt into a crash log and the popup
// ─────────────────────────────────────────────────────────────────────────────

/// Report a crash the last run couldn't hand over (its exec failed, or the
/// handler was stopped part way).
pub fn report_leftover() {
    let path = crate::config::state_dir().join(PENDING);
    if path.exists() {
        report_pending(&path);
    }
}

/// Write the crash log from the handler's facts, give kadr's own frames
/// function names, and show the popup. Runs as `kadr --crash-report <file>`
/// in the crashed process's place, or from [`report_leftover`]. Returns the
/// exit code a shell would have seen from the crash (128 + signal).
pub fn report_pending(path: &Path) -> i32 {
    let Ok(bytes) = std::fs::read(path) else {
        return 1;
    };
    let _ = std::fs::remove_file(path);
    let text = String::from_utf8_lossy(&bytes);
    let (head, body) = text.split_once("\n\n").unwrap_or((&text, ""));

    let (mut signal, mut time, mut version, mut build) = (0, None, "?", "unknown");
    let mut short = Vec::new();
    for line in head.lines() {
        match line.split_once(": ") {
            Some(("signal", value)) => signal = value.parse().unwrap_or(0),
            Some(("time", value)) => time = value.parse::<i64>().ok(),
            Some(("version", value)) => version = value,
            Some(("build", value)) => build = value,
            Some(("short", value)) => short.push(value),
            _ => {}
        }
    }
    let headline = if signal == libc::SIGABRT {
        ABORT_HEADLINE
    } else {
        CRASH_HEADLINE
    };

    let mut full = String::new();
    if let Some(what) = super::take_breadcrumbs() {
        full.push_str(&format!(
            "last activity : {}\n\n",
            what.replace('\n', "  —  ")
        ));
    }
    if body.trim().is_empty() {
        full.push_str("(the crash handler was stopped before it wrote the details)\n");
    } else {
        full.push_str(body.trim_end());
    }
    let when = time.map_or_else(super::now_parts, local_parts);
    let saved = super::write_report_at(
        "crash",
        "crash report",
        version,
        build,
        when,
        headline,
        &full,
    );
    super::symbolize_pending();

    let mut popup = format!("{headline}\n\n{}", short.join("\n"));
    match &saved {
        Some(log) => popup.push_str(&format!("\n\nFull crash report:\n{}", log.display())),
        None => popup.push_str("\n\n(could not write a crash log)"),
    }
    super::show_popup("kadr — crash report", &popup);
    128 + signal
}

// ─────────────────────────────────────────────────────────────────────────────
// Function names
// ─────────────────────────────────────────────────────────────────────────────

/// Whether this kadr has the symbols `addr2line` needs: in the binary itself
/// (a local build), or in a separate debug file a `-debug` package installs.
pub fn has_symbols() -> bool {
    static HAS: OnceLock<bool> = OnceLock::new();
    *HAS.get_or_init(|| {
        let id = build_id();
        let debug_file = id.len() > 2
            && Path::new(&format!("/usr/lib/debug/.build-id/{}/{}.debug", &id[..2], &id[2..]))
                .exists();
        debug_file
            || std::env::current_exe()
                .ok()
                .and_then(|exe| elf_has_symbols(&exe))
                .unwrap_or(false)
    })
}

/// Look through a 64-bit little-endian ELF file's section names for a symbol
/// table or line tables.
fn elf_has_symbols(path: &Path) -> Option<bool> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header).ok()?;
    if &header[..4] != b"\x7fELF" || header[4] != 2 || header[5] != 1 {
        return None;
    }
    let u16_at = |b: &[u8], at: usize| u16::from_le_bytes([b[at], b[at + 1]]) as usize;
    let u32_at = |b: &[u8], at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap()) as usize;
    let u64_at = |b: &[u8], at: usize| u64::from_le_bytes(b[at..at + 8].try_into().unwrap());
    let (offset, entry, count, names) = (
        u64_at(&header, 0x28),
        u16_at(&header, 0x3A),
        u16_at(&header, 0x3C),
        u16_at(&header, 0x3E),
    );
    if entry < 0x40 || names >= count {
        return None;
    }
    let mut sections = vec![0u8; entry * count];
    file.seek(SeekFrom::Start(offset)).ok()?;
    file.read_exact(&mut sections).ok()?;
    let strings = &sections[names * entry..];
    let mut table = vec![0u8; usize::try_from(u64_at(strings, 0x20)).ok()?.min(1 << 20)];
    file.seek(SeekFrom::Start(u64_at(strings, 0x18))).ok()?;
    file.read_exact(&mut table).ok()?;
    Some(sections.chunks_exact(entry).any(|section| {
        let name = table.get(u32_at(section, 0)..).unwrap_or(&[]);
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
        name == b".symtab" || name == b".debug_line"
    }))
}

/// The log with function names after kadr's own frames, from `addr2line`
/// (binutils). `None` when nothing could be named: no symbols, or no addr2line.
pub fn symbolize_text(text: &str) -> Option<String> {
    let exe = std::env::current_exe().ok()?;
    let marker = format!("{}+0x", exe_name());
    let lines: Vec<&str> = text.lines().collect();
    // (line, address in the file) for every place in kadr itself
    let mut wanted = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(pos) = line.find(&marker) else {
            continue;
        };
        let hex: String = line[pos + marker.len()..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        if let Ok(offset) = u64::from_str_radix(&hex, 16) {
            // a return address points after the call; one back is inside it
            let is_return = line.trim_start().starts_with('#');
            wanted.push((i, offset.saturating_sub(u64::from(is_return))));
        }
    }
    if wanted.is_empty() {
        return None;
    }
    let output = std::process::Command::new("addr2line")
        .arg("-e")
        .arg(&exe)
        .args(["-f", "-C", "-p"])
        .args(wanted.iter().map(|(_, addr)| format!("0x{addr:x}")))
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let answers = String::from_utf8_lossy(&output.stdout);
    let mut names: Vec<Option<String>> = vec![None; lines.len()];
    for ((i, _), answer) in wanted.iter().zip(answers.lines()) {
        // "function at file:line", or "?? ??:0" for an address it can't name
        let answer = answer.split(" (discriminator").next().unwrap_or(answer).trim();
        let (function, place) = answer.rsplit_once(" at ").unwrap_or((answer, "??"));
        if function == "??" {
            continue;
        }
        names[*i] = Some(if place.starts_with("??") {
            function.to_string()
        } else {
            format!("{function} ({place})")
        });
    }
    if names.iter().all(Option::is_none) {
        return None;
    }

    let build = build_id();
    let mut out = String::with_capacity(text.len() * 2);
    if !text.contains(&format!("build: {build}")) {
        out.push_str(&format!(
            "WARNING: this kadr is build {build}, which does not match this log. The names below are wrong.\n\n"
        ));
    }
    for (line, name) in lines.iter().zip(names) {
        out.push_str(line);
        if let Some(name) = name {
            out.push_str("  ");
            out.push_str(&name);
        }
        out.push('\n');
    }
    Some(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// Time and the popup
// ─────────────────────────────────────────────────────────────────────────────

/// Local `(year, month, day, hour, minute, second)` of a unix time.
pub fn local_parts(unix: i64) -> (u16, u16, u16, u16, u16, u16) {
    let time = unix as libc::time_t;
    // SAFETY: localtime_r only writes into `tm`, which it's given whole
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&time, &mut tm).is_null() {
            return (1970, 1, 1, 0, 0, 0);
        }
        let part = |value: c_int| u16::try_from(value).unwrap_or(0);
        (
            part(tm.tm_year + 1900),
            part(tm.tm_mon + 1),
            part(tm.tm_mday),
            part(tm.tm_hour),
            part(tm.tm_min),
            part(tm.tm_sec),
        )
    }
}

pub fn now_parts() -> (u16, u16, u16, u16, u16, u16) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    local_parts(now)
}

/// A dialog through zenity (what rfd uses on Linux, as plain text with
/// `--no-markup`), and the same text on stderr for a terminal or when zenity
/// isn't installed.
pub fn show_popup(title: &str, text: &str) {
    eprintln!("[{title}]\n{text}");
    let _ = rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(text)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}
