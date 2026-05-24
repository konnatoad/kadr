use std::path::Path;

const ALIAS: &str = "kadr_vid";

pub struct VideoPlayer {
    pub playing: bool,
    pub volume: i32,
    pub duration_ms: u64,
}

impl VideoPlayer {
    pub fn open(path: &Path) -> Option<Self> {
        #[cfg(target_os = "windows")]
        {
            send(&format!("close {ALIAS}"));
            let s = path.to_string_lossy();
            if send(&format!(r#"open "{s}" alias {ALIAS}"#)) != 0 {
                return None;
            }
            send(&format!("set {ALIAS} time format milliseconds"));
            let dur = query(&format!("status {ALIAS} length"))
                .parse::<u64>()
                .unwrap_or(0);
            let p = Self { playing: false, volume: 800, duration_ms: dur };
            send(&format!("setaudio {ALIAS} volume to {}", p.volume));
            return Some(p);
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = path;
            None
        }
    }

    pub fn play(&mut self) {
        #[cfg(target_os = "windows")]
        send(&format!("play {ALIAS}"));
        self.playing = true;
    }

    pub fn pause(&mut self) {
        #[cfg(target_os = "windows")]
        send(&format!("pause {ALIAS}"));
        self.playing = false;
    }

    pub fn toggle(&mut self) {
        if self.playing { self.pause() } else { self.play() }
    }

    pub fn seek(&mut self, delta_ms: i64) {
        #[cfg(target_os = "windows")]
        {
            let pos = self.position_ms() as i64;
            let target = (pos + delta_ms).clamp(0, self.duration_ms as i64);
            let was_playing = self.playing;
            send(&format!("seek {ALIAS} to {target}"));
            if was_playing {
                send(&format!("play {ALIAS}"));
            }
        }
        #[cfg(not(target_os = "windows"))]
        let _ = delta_ms;
    }

    pub fn position_ms(&self) -> u64 {
        #[cfg(target_os = "windows")]
        return query(&format!("status {ALIAS} position"))
            .parse::<u64>()
            .unwrap_or(0);
        #[cfg(not(target_os = "windows"))]
        0
    }

    pub fn change_volume(&mut self, delta: i32) {
        self.volume = (self.volume + delta).clamp(0, 1000);
        #[cfg(target_os = "windows")]
        send(&format!("setaudio {ALIAS} volume to {}", self.volume));
    }

    pub fn is_at_end(&self) -> bool {
        #[cfg(target_os = "windows")]
        return query(&format!("status {ALIAS} mode")) == "stopped";
        #[cfg(not(target_os = "windows"))]
        false
    }
}

impl Drop for VideoPlayer {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        send(&format!("close {ALIAS}"));
    }
}

#[cfg(target_os = "windows")]
#[link(name = "winmm")]
unsafe extern "system" {
    fn mciSendStringW(
        cmd: *const u16,
        ret: *mut u16,
        ret_len: u32,
        callback: usize,
    ) -> u32;
}

#[cfg(target_os = "windows")]
fn send(cmd: &str) -> u32 {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    let w: Vec<u16> = OsStr::new(cmd).encode_wide().chain(std::iter::once(0)).collect();
    unsafe { mciSendStringW(w.as_ptr(), std::ptr::null_mut(), 0, 0) }
}

#[cfg(target_os = "windows")]
fn query(cmd: &str) -> String {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    let w: Vec<u16> = OsStr::new(cmd).encode_wide().chain(std::iter::once(0)).collect();
    let mut buf = [0u16; 256];
    unsafe { mciSendStringW(w.as_ptr(), buf.as_mut_ptr(), 256, 0) };
    let end = buf.iter().position(|&c| c == 0).unwrap_or(256);
    String::from_utf16_lossy(&buf[..end]).trim().to_owned()
}
