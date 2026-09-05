# Browser Launcher

A minimalist tray URL router for Windows 10 and 11. It registers as an
HTTP/HTTPS handler, selects the configured real browser, and forwards the URL
with optional arguments.

## Features

- runs quietly as a single instance in the Windows notification area,
- forwards URLs to the running instance; if it is not running, starts in the tray,
- lists system-registered browsers with their names and native icons,
- configures one default browser and optional arguments with the `{url}`
  placeholder,
- routes URLs by ordered wildcard (`*`) or regular-expression rules,
- keeps an undeletable **Default** fallback row at the end of the rule list,
- supports drag-and-drop rule reordering,
- keeps an in-memory history of successfully opened URLs with time, browser, and
  arguments,
- copies URL or arguments by clicking the corresponding history cell; the
  context menu can copy a URL, arguments, or the complete record,
- copies the last opened URL from the tray menu,
- registers the application as a candidate default browser and opens Windows
  default-app settings,
- provides Czech and English user interfaces,
- has no .NET runtime, WebView, Electron, or other application runtime
  dependency.

History and the last URL exist only in RAM and disappear when the process exits.
Persistent settings are stored under `HKCU\\Software\\BrowserLauncher`.

## Requirements

- Windows 10 or Windows 11 (64-bit),
- Rust 1.85 or newer with the `x86_64-pc-windows-msvc` toolchain,
- Visual Studio Build Tools with the **Desktop development with C++** workload.

The project uses Rust edition 2024. Exact crate versions are pinned in
`Cargo.lock`; the main dependencies are `windows-sys`, `regex`, and the
build-time `embed-resource` crate.

## Build

Run a release build from the project root:

```powershell
cargo build --release
```

The executable is written to `target\\release\\browser-launcher.exe`. To build,
deploy to the stable per-user location, refresh registration, and replace the
running instance, use:

```powershell
.\\Build-Release.ps1
```

To build and deploy without starting the application:

```powershell
.\\Build-Release.ps1 -NoStart
```

The deployment path is
`%LOCALAPPDATA%\\BrowserLauncher\\BrowserLauncher.exe`.

## Usage

Start the executable with a URL:

```powershell
.\\target\\release\\browser-launcher.exe "https://example.com"
```

If the application is already running, the URL is forwarded to that instance.
Otherwise Windows starts it, the URL is opened in the selected browser, and the
application remains only in the tray.

Use **Settings** to select the default browser, its arguments, and routing
rules. The **Default** rule cannot be deleted or moved. Windows does not allow
an application to silently change the default handler; the registration button
therefore prepares the registration and opens the system page where the user
confirms the choice.

To register the stable copy without opening the system settings page:

```powershell
.\\target\\release\\browser-launcher.exe --install-register
```

## Verification

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

## License

Browser Launcher is licensed under the GNU General Public License v3.0 or later.
See [LICENSE](LICENSE).

The Czech README is available as [README.cs.md](README.cs.md).
