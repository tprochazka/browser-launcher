use std::ffi::c_void;
use std::io;
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};

use crate::routing::{RoutingRule, decode_rules, encode_rules};

const KEY_PATH: &str = r"Software\BrowserLauncher";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    pub browser_path: String,
    pub browser_arguments: String,
    pub routing_rules: Vec<RoutingRule>,
    pub language: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            browser_path: String::new(),
            browser_arguments: String::new(),
            routing_rules: Vec::new(),
            language: "cs".to_owned(),
        }
    }
}

impl Config {
    pub fn load() -> io::Result<Self> {
        let Some(key) = open_existing_key()? else {
            return Ok(Self::default());
        };
        let encoded_rules = read_string(key, "RoutingRules")?.unwrap_or_default();
        let routing_rules = decode_rules(&encoded_rules).map_err(|message| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Pravidla nelze načíst: {message}"),
            )
        })?;
        let result = Ok(Self {
            browser_path: read_string(key, "BrowserPath")?.unwrap_or_default(),
            browser_arguments: read_string(key, "BrowserArguments")?.unwrap_or_default(),
            routing_rules,
            language: read_string(key, "Language")?.unwrap_or_else(|| "cs".to_owned()),
        });
        unsafe { RegCloseKey(key) };
        result
    }

    pub fn save(&self) -> io::Result<()> {
        let key = create_key(KEY_WRITE)?;
        let result = write_string(key, "BrowserPath", &self.browser_path)
            .and_then(|_| write_string(key, "BrowserArguments", &self.browser_arguments))
            .and_then(|_| write_string(key, "RoutingRules", &encode_rules(&self.routing_rules)))
            .and_then(|_| write_string(key, "Language", &self.language));
        unsafe { RegCloseKey(key) };
        result
    }

    pub fn is_configured(&self) -> bool {
        !self.browser_path.trim().is_empty()
    }
}

fn open_existing_key() -> io::Result<Option<HKEY>> {
    let path = wide(KEY_PATH);
    let mut key: HKEY = null_mut();
    let status = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, path.as_ptr(), 0, KEY_READ, &mut key) };
    if status == ERROR_FILE_NOT_FOUND {
        Ok(None)
    } else if status == ERROR_SUCCESS {
        Ok(Some(key))
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn create_key(access: u32) -> io::Result<HKEY> {
    let path = wide(KEY_PATH);
    let mut key: HKEY = null_mut();
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            path.as_ptr(),
            0,
            null_mut(),
            REG_OPTION_NON_VOLATILE,
            access,
            null(),
            &mut key,
            null_mut(),
        )
    };
    if status == ERROR_SUCCESS {
        Ok(key)
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn read_string(key: HKEY, name: &str) -> io::Result<Option<String>> {
    let name = wide(name);
    let mut bytes = 0u32;
    let status = unsafe {
        RegGetValueW(
            key,
            null(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            null_mut(),
            &mut bytes,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return Ok(None);
    }
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let mut value = vec![0u16; bytes as usize / 2];
    let status = unsafe {
        RegGetValueW(
            key,
            null(),
            name.as_ptr(),
            RRF_RT_REG_SZ,
            null_mut(),
            value.as_mut_ptr().cast::<c_void>(),
            &mut bytes,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    while value.last() == Some(&0) {
        value.pop();
    }
    Ok(Some(String::from_utf16_lossy(&value)))
}

fn write_string(key: HKEY, name: &str, value: &str) -> io::Result<()> {
    let name = wide(name);
    let value = wide(value);
    let status = unsafe {
        RegSetValueExW(
            key,
            name.as_ptr(),
            0,
            REG_SZ,
            value.as_ptr().cast::<u8>(),
            (value.len() * 2) as u32,
        )
    };
    if status == ERROR_SUCCESS {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(status as i32))
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
