#[derive(Clone, Debug)]
pub struct MonitorInfo {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub is_primary: bool,
}

impl MonitorInfo {
    pub fn label(&self) -> String {
        if self.is_primary {
            format!("Monitor {} — {}×{} (Primary)", self.index + 1, self.width, self.height)
        } else {
            format!("Monitor {} — {}×{}", self.index + 1, self.width, self.height)
        }
    }
}

#[cfg(windows)]
pub fn enumerate() -> Vec<MonitorInfo> {
    use std::mem;
    use winapi::shared::minwindef::{BOOL, LPARAM};
    use winapi::shared::windef::{HDC, HMONITOR, LPRECT};
    use winapi::um::winuser::{EnumDisplayMonitors, GetMonitorInfoW, MONITORINFO, MONITORINFOF_PRIMARY};

    unsafe extern "system" fn enum_proc(
        hmonitor: HMONITOR,
        _: HDC,
        _: LPRECT,
        lparam: LPARAM,
    ) -> BOOL {
        unsafe {
            let list = &mut *(lparam as *mut Vec<HMONITOR>);
            list.push(hmonitor);
        }
        1
    }

    let mut handles: Vec<HMONITOR> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            Some(enum_proc),
            &mut handles as *mut _ as LPARAM,
        );
    }

    // Primary monitor first
    let mut infos: Vec<MonitorInfo> = handles
        .iter()
        .enumerate()
        .filter_map(|(i, &handle)| unsafe {
            let mut info: MONITORINFO = mem::zeroed();
            info.cbSize = mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(handle, &mut info) != 0 {
                Some(MonitorInfo {
                    index: i,
                    x: info.rcMonitor.left,
                    y: info.rcMonitor.top,
                    width: (info.rcMonitor.right - info.rcMonitor.left) as u32,
                    height: (info.rcMonitor.bottom - info.rcMonitor.top) as u32,
                    is_primary: info.dwFlags & MONITORINFOF_PRIMARY != 0,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort so primary is always index 0
    infos.sort_by_key(|m| if m.is_primary { 0 } else { 1 });
    for (i, m) in infos.iter_mut().enumerate() {
        m.index = i;
    }
    infos
}

#[cfg(not(windows))]
pub fn enumerate() -> Vec<MonitorInfo> {
    vec![]
}

/// Returns the position to place a window of `win_w × win_h` centered on the preferred monitor.
/// `preferred` is 1-based (1 = first/primary). Returns None if 0 (OS default) or index out of range.
pub fn initial_position(preferred: usize, win_w: f32, win_h: f32) -> Option<egui::Pos2> {
    if preferred == 0 {
        return None;
    }
    let monitors = enumerate();
    let m = monitors.get(preferred - 1)?;
    let x = m.x as f32 + (m.width as f32 - win_w) / 2.0;
    let y = m.y as f32 + (m.height as f32 - win_h) / 2.0;
    Some(egui::pos2(x.max(m.x as f32), y.max(m.y as f32)))
}
