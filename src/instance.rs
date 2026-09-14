//! Single-instance startup and bounded inter-process URL forwarding.
//!
//! The mutex is deliberately held only while the caller probes for, and creates,
//! the primary window.  It must be dropped after the window and tray icon are
//! ready; it must not be held while forwarding a URL to that window.

use std::ffi::{OsStr, c_void};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, HANDLE, HWND};
use windows_sys::Win32::Security::{
    GetLengthSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser,
};
use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
use windows_sys::Win32::System::Threading::{
    CreateMutexW, GetCurrentProcess, OpenProcessToken, ReleaseMutex, WaitForSingleObject,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SMTO_ABORTIFHUNG, SMTO_BLOCK, SendMessageTimeoutW, WM_COPYDATA,
};

/// Keep this in sync with the receiver's maximum WM_COPYDATA payload.
pub const MAX_URL_BYTES: usize = 64 * 1024;
const MUTEX_WAIT_MS: u32 = 5_000;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_ABANDONED: u32 = 0x0000_0080;
const ERROR_TIMEOUT: u32 = 1460;

pub enum StartupState {
    /// Another ready (or currently initializing) instance owns the window.
    Existing(HWND),
    /// The caller is responsible for creating the primary window while this
    /// guard remains alive.
    Primary(StartupGuard),
}

pub struct StartupGuard {
    handle: HANDLE,
}

impl Drop for StartupGuard {
    fn drop(&mut self) {
        unsafe {
            ReleaseMutex(self.handle);
            CloseHandle(self.handle);
        }
    }
}

/// Serializes startup per interactive user/session and probes by the stable
/// window class (the title contains the version and is not an identity key).
pub unsafe fn begin(class_name: &str) -> Result<StartupState, String> {
    let mutex_name = wide(&format!("{}.{}", mutex_name()?, class_name));
    // Create it without initial ownership, then acquire it exactly once. Passing TRUE
    // here and waiting again would recursively acquire the Win32 mutex.
    let handle = CreateMutexW(null(), 0, mutex_name.as_ptr());
    if handle.is_null() {
        return Err(format!(
            "single-instance mutex creation failed ({})",
            GetLastError()
        ));
    }

    let wait_result = WaitForSingleObject(handle, MUTEX_WAIT_MS);
    if wait_result != WAIT_OBJECT_0 && wait_result != WAIT_ABANDONED {
        CloseHandle(handle);
        return Err(if wait_result == 0x0000_0102 {
            "timed out waiting for another Browser Launcher instance to finish starting".to_owned()
        } else {
            format!("single-instance mutex wait failed ({wait_result:#x})")
        });
    }

    let class_name = wide(class_name);
    let existing = FindWindowW(class_name.as_ptr(), null());
    if existing.is_null() {
        Ok(StartupState::Primary(StartupGuard { handle }))
    } else {
        ReleaseMutex(handle);
        CloseHandle(handle);
        Ok(StartupState::Existing(existing))
    }
}

/// Forward one UTF-16, NUL-terminated URL without allowing a hung primary
/// process to block the newly launched process indefinitely.
pub unsafe fn send_url(hwnd: HWND, url: &str) -> Result<(), String> {
    if url.encode_utf16().any(|unit| unit == 0) {
        return Err("URL contains an embedded NUL character".to_owned());
    }
    let encoded = wide(url);
    let byte_count = encoded
        .len()
        .checked_mul(std::mem::size_of::<u16>())
        .ok_or_else(|| "URL is too large to forward".to_owned())?;
    if byte_count > MAX_URL_BYTES || byte_count > u32::MAX as usize {
        return Err(format!(
            "URL exceeds the {} KiB forwarding limit",
            MAX_URL_BYTES / 1024
        ));
    }
    let data = COPYDATASTRUCT {
        dwData: 1,
        cbData: byte_count as u32,
        lpData: encoded.as_ptr().cast::<c_void>() as *mut c_void,
    };
    let mut result = 0usize;
    if SendMessageTimeoutW(
        hwnd,
        WM_COPYDATA,
        0,
        &data as *const _ as isize,
        SMTO_ABORTIFHUNG | SMTO_BLOCK,
        2_000,
        &mut result,
    ) == 0
    {
        let error = GetLastError();
        return Err(if error == ERROR_TIMEOUT {
            "the primary Browser Launcher instance did not respond in time".to_owned()
        } else {
            format!("URL forwarding failed ({error})")
        });
    }
    if result == 0 {
        return Err("the primary Browser Launcher instance rejected the URL".to_owned());
    }
    Ok(())
}

unsafe fn mutex_name() -> Result<String, String> {
    let mut token = null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
        return Err(format!(
            "current-user token query failed ({})",
            GetLastError()
        ));
    }
    let result = token_sid_key(token);
    CloseHandle(token);
    result.map(|sid| format!("Local\\BrowserLauncher.SingleInstance.{sid}"))
}

unsafe fn token_sid_key(token: HANDLE) -> Result<String, String> {
    let mut required = 0u32;
    GetTokenInformation(token, TokenUser, null_mut(), 0, &mut required);
    if required == 0 {
        return Err(format!(
            "token information sizing failed ({})",
            GetLastError()
        ));
    }
    // TOKEN_USER contains pointers and must be read from suitably aligned
    // storage; Vec<u8> does not provide that guarantee.
    let words = (required as usize).div_ceil(std::mem::size_of::<usize>());
    let mut buffer = vec![0usize; words];
    if GetTokenInformation(
        token,
        TokenUser,
        buffer.as_mut_ptr().cast(),
        (buffer.len() * std::mem::size_of::<usize>()) as u32,
        &mut required,
    ) == 0
    {
        return Err(format!(
            "token information query failed ({})",
            GetLastError()
        ));
    }
    let user = &*(buffer.as_ptr().cast::<TOKEN_USER>());
    if user.User.Sid.is_null() {
        return Err("current-user token has no SID".to_owned());
    }
    let length = GetLengthSid(user.User.Sid) as usize;
    let bytes = std::slice::from_raw_parts(user.User.Sid.cast::<u8>(), length);
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::{StartupState, begin};

    #[test]
    fn startup_guard_is_released_for_the_next_start() {
        let class_name = format!("BrowserLauncherTestRelease_{}", std::process::id());
        let first = unsafe { begin(&class_name).expect("first startup probe") };
        assert!(matches!(first, StartupState::Primary(_)));
        drop(first);

        let second = unsafe { begin(&class_name).expect("second startup probe") };
        assert!(matches!(second, StartupState::Primary(_)));
    }

    #[test]
    fn concurrent_start_waits_until_primary_releases_guard() {
        let class_name = format!("BrowserLauncherTestConcurrent_{}", std::process::id());
        let first = unsafe { begin(&class_name).expect("first startup probe") };
        assert!(matches!(first, StartupState::Primary(_)));
        let thread_class = class_name.clone();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel();
        let waiter = std::thread::spawn(move || unsafe {
            started_tx.send(()).unwrap();
            match begin(&thread_class) {
                Ok(StartupState::Primary(guard)) => {
                    acquired_tx.send(()).unwrap();
                    // HANDLE-bearing state must not cross thread boundaries.
                    drop(guard);
                    true
                }
                Ok(StartupState::Existing(_)) | Err(_) => false,
            }
        });
        started_rx.recv().unwrap();
        assert!(matches!(
            acquired_rx.recv_timeout(std::time::Duration::from_millis(100)),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout)
        ));
        drop(first);
        acquired_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(waiter.join().expect("startup waiter thread"));
    }

    #[test]
    fn send_url_rejects_embedded_nul_and_oversized_payload() {
        let embedded_nul = unsafe { super::send_url(std::ptr::null_mut(), "https://a\0b") };
        assert!(embedded_nul.is_err());

        let oversized = "x".repeat(super::MAX_URL_BYTES / 2);
        let oversized = unsafe { super::send_url(std::ptr::null_mut(), &oversized) };
        assert!(oversized.is_err());
    }
}
