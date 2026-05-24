use std::path::Path;

/// Marker that a video was opened in the system player.
pub struct VideoPlayer;

impl VideoPlayer {
    pub fn open(path: &Path) -> Option<Self> {
        open::that(path).ok()?;
        Some(Self)
    }

    pub fn play_pause() { broadcast(14); }
    pub fn seek_back()  { broadcast(21); } // APPCOMMAND_MEDIA_REWIND
    pub fn seek_fwd()   { broadcast(49); } // APPCOMMAND_MEDIA_FAST_FORWARD
    pub fn volume_up()  { broadcast(10); }
    pub fn volume_down(){ broadcast(9);  }
}

fn broadcast(cmd: i32) {
    #[cfg(windows)]
    unsafe {
        use winapi::shared::minwindef::{LPARAM, WPARAM};
        use winapi::um::winuser::{PostMessageW, WM_APPCOMMAND};
        let hwnd_broadcast = 0xffffu16 as usize as *mut winapi::shared::windef::HWND__;
        PostMessageW(
            hwnd_broadcast,
            WM_APPCOMMAND,
            0usize as WPARAM,
            (cmd << 16) as LPARAM,
        );
    }
    #[cfg(not(windows))]
    let _ = cmd;
}
