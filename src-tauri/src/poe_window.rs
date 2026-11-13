//! Locates the Path of Exile window. Read-only Win32 queries: no hooks, no injection, no input.

pub struct Poe {
    pub foreground: bool,
    /// client area in physical screen pixels: x, y, w, h
    pub rect: (i32, i32, u32, u32),
    fg: isize,
}

impl Poe {
    pub fn foreground_is(&self, hwnd: isize) -> bool {
        self.fg == hwnd
    }
}

#[cfg(windows)]
pub fn probe() -> Option<Poe> {
    probe_class(windows::core::w!("POEWindowClass"))
}

#[cfg(windows)]
fn probe_class(class: windows::core::PCWSTR) -> Option<Poe> {
    use windows::Win32::Foundation::{POINT, RECT};
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetClientRect, GetForegroundWindow, IsIconic};
    use windows::core::PCWSTR;

    unsafe {
        let hwnd = FindWindowW(class, PCWSTR::null()).ok()?;
        if IsIconic(hwnd).as_bool() {
            return None;
        }
        let mut r = RECT::default();
        GetClientRect(hwnd, &mut r).ok()?;
        let mut origin = POINT::default();
        let _ = ClientToScreen(hwnd, &mut origin);
        let fg = GetForegroundWindow();
        Some(Poe {
            foreground: fg == hwnd,
            rect: (origin.x, origin.y, (r.right - r.left) as u32, (r.bottom - r.top) as u32),
            fg: fg.0 as isize,
        })
    }
}

#[cfg(not(windows))]
pub fn probe() -> Option<Poe> {
    None
}

/// Folder of the running game's exe, from its process image path (OS query only, the process is never read or touched).
#[cfg(windows)]
pub fn install_dir() -> Option<std::path::PathBuf> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowThreadProcessId};
    use windows::core::{w, PCWSTR, PWSTR};

    unsafe {
        let hwnd = FindWindowW(w!("POEWindowClass"), PCWSTR::null()).ok()?;
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = [0u16; 1024];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(h);
        res.ok()?;
        let exe = std::path::PathBuf::from(String::from_utf16_lossy(&buf[..len as usize]));
        exe.parent().map(|p| p.to_path_buf())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use windows::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::w;

    unsafe extern "system" fn wnd_proc(h: windows::Win32::Foundation::HWND, m: u32, w: windows::Win32::Foundation::WPARAM, l: windows::Win32::Foundation::LPARAM) -> windows::Win32::Foundation::LRESULT {
        unsafe { DefWindowProcW(h, m, w, l) }
    }

    #[test]
    fn finds_poe_class_window_and_its_client_rect() {
        assert!(probe_class(w!("PoeMateTestClass")).is_none(), "no PoE window yet");
        unsafe {
            let inst = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(wnd_proc),
                hInstance: inst.into(),
                lpszClassName: w!("PoeMateTestClass"),
                ..Default::default()
            };
            assert_ne!(RegisterClassW(&class), 0);
            // WS_POPUP: client rect == window rect, so the expected values are exact
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("PoeMateTestClass"),
                w!("Path of Exile"),
                WS_POPUP | WS_VISIBLE,
                100,
                120,
                640,
                480,
                None,
                None,
                Some(inst.into()),
                None,
            )
            .unwrap();
            let p = probe_class(w!("PoeMateTestClass")).expect("window found");
            assert_eq!(p.rect, (100, 120, 640, 480));
            DestroyWindow(hwnd).unwrap();
        }
        assert!(probe_class(w!("PoeMateTestClass")).is_none(), "gone after destroy");
    }
}
