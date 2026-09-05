use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

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
    let temporary = directory.join("BrowserLauncher.new.exe");
    fs::copy(&current, &temporary)?;
    if destination.exists() {
        fs::remove_file(&destination).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!(
                    "Nainstalovanou aplikaci nelze aktualizovat. Ukončete její běžící instanci a zkuste to znovu: {error}"
                ),
            )
        })?;
    }
    fs::rename(&temporary, &destination)?;
    Ok(destination)
}

fn install_path_from(local_app_data: OsString) -> PathBuf {
    PathBuf::from(local_app_data)
        .join(INSTALL_DIRECTORY)
        .join(INSTALL_EXECUTABLE)
}

fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::{install_path_from, same_path};

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
}
