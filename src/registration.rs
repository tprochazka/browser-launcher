use std::io;
use std::path::Path;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegSetValueExW,
};
use windows_sys::Win32::UI::Shell::{SHCNE_ASSOCCHANGED, SHCNF_IDLIST, SHChangeNotify};

pub const REGISTERED_APP_NAME: &str = "Browser Launcher";
const PROG_ID: &str = "BrowserLauncherURL";

pub fn register_current_user(executable: &Path) -> io::Result<()> {
    let executable = executable.to_string_lossy();
    let icon = format!("\"{executable}\",0");
    let command = format!("\"{executable}\" \"%1\"");

    set_value(
        r"Software\Classes\BrowserLauncherURL",
        None,
        "Browser Launcher URL",
    )?;
    set_value(
        r"Software\Classes\BrowserLauncherURL",
        Some("URL Protocol"),
        "",
    )?;
    set_value(
        r"Software\Classes\BrowserLauncherURL\DefaultIcon",
        None,
        &icon,
    )?;
    set_value(
        r"Software\Classes\BrowserLauncherURL\shell\open\command",
        None,
        &command,
    )?;

    set_value(
        r"Software\BrowserLauncher\Capabilities",
        Some("ApplicationName"),
        REGISTERED_APP_NAME,
    )?;
    set_value(
        r"Software\BrowserLauncher\Capabilities",
        Some("ApplicationDescription"),
        "Směruje webové odkazy do zvoleného prohlížeče.",
    )?;
    set_value(
        r"Software\BrowserLauncher\Capabilities",
        Some("ApplicationIcon"),
        &icon,
    )?;
    set_value(
        r"Software\BrowserLauncher\Capabilities\UrlAssociations",
        Some("http"),
        PROG_ID,
    )?;
    set_value(
        r"Software\BrowserLauncher\Capabilities\UrlAssociations",
        Some("https"),
        PROG_ID,
    )?;
    set_value(
        r"Software\RegisteredApplications",
        Some(REGISTERED_APP_NAME),
        r"Software\BrowserLauncher\Capabilities",
    )?;

    unsafe { SHChangeNotify(SHCNE_ASSOCCHANGED as i32, SHCNF_IDLIST, null(), null()) };
    Ok(())
}

fn set_value(key_path: &str, value_name: Option<&str>, value: &str) -> io::Result<()> {
    let path = wide(key_path);
    let mut key: HKEY = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let name = value_name.map(wide);
    let name_pointer = name.as_ref().map_or(null(), |value| value.as_ptr());
    let value = wide(value);
    let status = unsafe {
        RegSetValueExW(
            key,
            name_pointer,
            0,
            REG_SZ,
            value.as_ptr().cast::<u8>(),
            (value.len() * 2) as u32,
        )
    };
    unsafe { RegCloseKey(key) };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
