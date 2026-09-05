use std::ptr::null_mut;
use std::thread;
use std::time::Duration;

use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    ASFW_ANY, AllowSetForegroundWindow, BringWindowToTop, EnumWindows, GW_OWNER, GetWindow,
    GetWindowTextLengthW, GetWindowThreadProcessId, IsIconic, IsWindowVisible, SW_RESTORE,
    SetForegroundWindow, ShowWindowAsync,
};

pub fn grant_foreground_activation() {
    unsafe {
        // The URL-activation process is normally started by the foreground application.
        // Passing this privilege on lets both the tray instance and the real browser activate.
        AllowSetForegroundWindow(ASFW_ANY);
    }
}

pub fn bring_browser_to_front(executable: String) {
    thread::spawn(move || {
        // Existing browsers usually receive the URL over IPC. A short delay lets that hand-off
        // finish; repeated attempts also cover a newly created browser window.
        for attempt in 0..12 {
            thread::sleep(Duration::from_millis(if attempt == 0 { 80 } else { 75 }));
            let Some(window) = find_browser_window(&executable) else {
                continue;
            };
            unsafe {
                // Only restore if the window is minimized: SW_RESTORE on an already-visible
                // (e.g. maximized) window would reset it to its non-maximized size/position.
                if IsIconic(window) != 0 {
                    ShowWindowAsync(window, SW_RESTORE);
                }
                BringWindowToTop(window);
                if SetForegroundWindow(window) != 0 {
                    return;
                }
            }
        }
    });
}

struct WindowSearch<'a> {
    executable: &'a str,
    result: HWND,
}

fn find_browser_window(executable: &str) -> Option<HWND> {
    let mut search = WindowSearch {
        executable,
        result: null_mut(),
    };
    unsafe {
        EnumWindows(
            Some(enum_window),
            &mut search as *mut WindowSearch<'_> as LPARAM,
        );
    }
    (!search.result.is_null()).then_some(search.result)
}

unsafe extern "system" fn enum_window(window: HWND, parameter: LPARAM) -> i32 {
    let search = &mut *(parameter as *mut WindowSearch<'_>);
    if IsWindowVisible(window) == 0
        || !GetWindow(window, GW_OWNER).is_null()
        || GetWindowTextLengthW(window) == 0
    {
        return 1;
    }

    let mut process_id = 0;
    GetWindowThreadProcessId(window, &mut process_id);
    if process_id == 0 {
        return 1;
    }
    let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id);
    if process.is_null() {
        return 1;
    }
    let mut path = [0u16; 32768];
    let mut length = path.len() as u32;
    let queried = QueryFullProcessImageNameW(process, 0, path.as_mut_ptr(), &mut length) != 0;
    CloseHandle(process);
    if queried {
        let candidate = String::from_utf16_lossy(&path[..length as usize]);
        if candidate.eq_ignore_ascii_case(search.executable) {
            search.result = window;
            return 0;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    #[test]
    fn executable_comparison_is_case_insensitive() {
        assert!(
            r"C:\Program Files\Chrome\chrome.exe"
                .eq_ignore_ascii_case(r"c:\PROGRAM FILES\Chrome\CHROME.EXE")
        );
    }
}
