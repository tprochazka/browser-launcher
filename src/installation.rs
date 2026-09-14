use std::ffi::OsString;
use std::fs;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Storage::FileSystem::{
    MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
};
use windows_sys::core::BOOL;

const INSTALL_DIRECTORY: &str = "BrowserLauncher";
const INSTALL_EXECUTABLE: &str = "BrowserLauncher.exe";

pub fn install_current_user() -> io::Result<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Windows neposkytl cestu LOCALAPPDATA.",
        )
    })?;
    let destination = install_path_from(local_app_data);
    let current = std::env::current_exe()?;
    if same_path(&current, &destination) {
        return Ok(destination);
    }

    let directory = destination
        .parent()
        .expect("the installation path always has a parent");
    fs::create_dir_all(directory)?;
    let temporary = directory.join(format!("BrowserLauncher.{}.new.exe", std::process::id()));
    fs::copy(&current, &temporary)?;
    if destination.exists() {
        if let Err(error) = replace_file(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(io::Error::new(
                error.kind(),
                format!(
                    "Nainstalovanou aplikaci nelze aktualizovat. Ukončete její běžící instanci a zkuste to znovu: {error}"
                ),
            ));
        }
    } else {
        if let Err(error) = fs::rename(&temporary, &destination) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
    }
    Ok(destination)
}

fn replace_file(source: &Path, destination: &Path) -> io::Result<()> {
    let source_wide = wide_path(source);
    let destination_wide = wide_path(destination);
    let replaced: BOOL = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            destination_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn install_path_from(local_app_data: OsString) -> PathBuf {
    PathBuf::from(local_app_data)
        .join(INSTALL_DIRECTORY)
        .join(INSTALL_EXECUTABLE)
}

pub(crate) fn same_path(left: &Path, right: &Path) -> bool {
    normalize_path(left).eq_ignore_ascii_case(&normalize_path(right))
}

fn normalize_path(path: &Path) -> String {
    let mut value = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = value.strip_prefix(r"\\?\UNC\") {
        value = format!(r"\\{rest}");
    } else if let Some(rest) = value.strip_prefix(r"\\?\") {
        value = rest.to_owned();
    }
    value
}

fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[cfg(windows)]
    use std::fs::OpenOptions;
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;

    use super::{install_path_from, replace_file, same_path};

    fn test_directory(suffix: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "browser-launcher-test-{suffix}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir(&directory).unwrap();
        directory
    }

    #[test]
    fn uses_stable_per_user_location() {
        assert_eq!(
            install_path_from(OsString::from(r"C:\Users\Test\AppData\Local")),
            PathBuf::from(r"C:\Users\Test\AppData\Local\BrowserLauncher\BrowserLauncher.exe")
        );
    }

    #[test]
    fn windows_path_comparison_is_case_insensitive() {
        assert!(same_path(
            &PathBuf::from(r"C:\App\BrowserLauncher.exe"),
            &PathBuf::from(r"c:\APP\browserlauncher.EXE")
        ));
    }

    #[test]
    fn windows_path_comparison_ignores_extended_prefix() {
        assert!(same_path(
            &PathBuf::from(r"\\?\C:\App\BrowserLauncher.exe"),
            &PathBuf::from(r"c:\APP\browserlauncher.EXE")
        ));
    }

    #[test]
    fn atomically_replaces_existing_file() {
        let directory = test_directory("replace");
        let source = directory.join("new.exe");
        let destination = directory.join("installed.exe");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old").unwrap();
        replace_file(&source, &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"new");
        assert!(!source.exists());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_replacement_keeps_source_and_existing_destination() {
        let directory = test_directory("locked");
        let source = directory.join("new.exe");
        let destination = directory.join("installed.exe");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"old").unwrap();
        let destination_handle = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&destination)
            .unwrap();
        assert!(replace_file(&source, &destination).is_err());
        drop(destination_handle);
        assert_eq!(fs::read(&source).unwrap(), b"new");
        assert_eq!(fs::read(&destination).unwrap(), b"old");
        fs::remove_dir_all(directory).unwrap();
    }
}
