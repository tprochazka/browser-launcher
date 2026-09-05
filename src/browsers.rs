use std::ffi::c_void;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_NO_MORE_ITEMS, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, RRF_RT_REG_SZ, RegCloseKey,
    RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
};

const BROWSERS_KEY: &str = r"SOFTWARE\Clients\StartMenuInternet";

#[derive(Clone, Debug)]
pub struct BrowserInfo {
    pub name: String,
    pub executable: String,
    pub icon: String,
}

pub fn installed_browsers() -> Vec<BrowserInfo> {
    let mut browsers = Vec::new();
    for root in [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE] {
        enumerate_root(root, &mut browsers);
    }
    browsers.sort_by_key(|browser| browser.name.to_lowercase());
    browsers.dedup_by(|left, right| left.executable.eq_ignore_ascii_case(&right.executable));
    browsers
}

fn enumerate_root(root: HKEY, browsers: &mut Vec<BrowserInfo>) {
    let key_path = wide(BROWSERS_KEY);
    let mut parent = null_mut();
    let status = unsafe { RegOpenKeyExW(root, key_path.as_ptr(), 0, KEY_READ, &mut parent) };
    if status != ERROR_SUCCESS {
        return;
    }

    let mut index = 0;
    loop {
        let mut name = [0u16; 256];
        let mut name_length = name.len() as u32;
        let status = unsafe {
            RegEnumKeyExW(
                parent,
                index,
                name.as_mut_ptr(),
                &mut name_length,
                null(),
                null_mut(),
                null_mut(),
                null_mut(),
            )
        };
        if status == ERROR_NO_MORE_ITEMS {
            break;
        }
        if status == ERROR_SUCCESS {
            let subkey = String::from_utf16_lossy(&name[..name_length as usize]);
            if let Some(browser) = read_browser(parent, &subkey) {
                browsers.push(browser);
            }
        }
        index += 1;
    }
    unsafe { RegCloseKey(parent) };
}

fn read_browser(parent: HKEY, subkey: &str) -> Option<BrowserInfo> {
    let subkey_wide = wide(subkey);
    let mut key = null_mut();
    let status = unsafe { RegOpenKeyExW(parent, subkey_wide.as_ptr(), 0, KEY_READ, &mut key) };
    if status != ERROR_SUCCESS {
        return None;
    }
    let name = read_string(key, None).unwrap_or_else(|| subkey.to_owned());
    let command = read_string(key, Some(r"shell\open\command"));
    let icon = read_string(key, Some("DefaultIcon")).unwrap_or_default();
    unsafe { RegCloseKey(key) };

    let executable = command.and_then(|value| executable_from_command(&value))?;
    if !Path::new(&executable).is_file() {
        return None;
    }
    Some(BrowserInfo {
        name,
        executable,
        icon,
    })
}

fn read_string(key: HKEY, subkey: Option<&str>) -> Option<String> {
    let subkey_wide = subkey.map(wide);
    let subkey_pointer = subkey_wide.as_ref().map_or(null(), |value| value.as_ptr());
    let mut bytes = 0u32;
    let status = unsafe {
        RegGetValueW(
            key,
            subkey_pointer,
            null(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_FILE_NOT_FOUND || status != ERROR_SUCCESS || bytes < 2 {
        return None;
    }
    let mut value = vec![0u16; bytes as usize / 2];
    let status = unsafe {
        RegGetValueW(
            key,
            subkey_pointer,
            null(),
            RRF_RT_REG_SZ,
            null_mut(),
            value.as_mut_ptr().cast::<c_void>(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    while value.last() == Some(&0) {
        value.pop();
    }
    Some(String::from_utf16_lossy(&value))
}

fn executable_from_command(command: &str) -> Option<String> {
    let command = command.trim();
    if let Some(rest) = command.strip_prefix('"') {
        return rest.find('"').map(|end| rest[..end].to_owned());
    }
    let lowercase = command.to_lowercase();
    let end = lowercase.find(".exe")? + 4;
    Some(command[..end].to_owned())
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::executable_from_command;

    #[test]
    fn extracts_quoted_executable() {
        assert_eq!(
            executable_from_command(r#""C:\Program Files\Browser\browser.exe" --flag"#),
            Some(r"C:\Program Files\Browser\browser.exe".to_owned())
        );
    }

    #[test]
    fn extracts_unquoted_executable_with_spaces() {
        assert_eq!(
            executable_from_command(r"C:\Program Files\Browser\browser.exe --flag"),
            Some(r"C:\Program Files\Browser\browser.exe".to_owned())
        );
    }
}
