# Browser Launcher

A small Windows 10/11 tray URL router. It registers as an HTTP/HTTPS handler,
chooses a browser using the configured rules, and forwards the URL with
optional arguments.

Current version: **0.9.0**.

## Features

- Single instance running quietly in the tray.
- Installed browsers with names and icons.
- Ordered wildcard and regex rules; default browser as fallback.
- Drag-and-drop rule reordering.
- Custom browser arguments with `{url}`.
- In-memory URL history with click-to-copy.
- Copy the last URL from the tray menu.
- HTTP/HTTPS registration in Windows.
- Czech and English interfaces.

Settings are stored under `HKCU\Software\BrowserLauncher`; history is cleared on exit.

## Requirements

- Windows 10 or Windows 11 (64-bit),
- Rust 1.88 or newer with the `x86_64-pc-windows-msvc` toolchain,
- Visual Studio Build Tools with the **Desktop development with C++** workload.

Dependency versions are pinned in `Cargo.lock`.

## Build

Choose a build command from the project root:

```powershell
cargo build --release
.\Build-Release.ps1              # build, deploy, register, and start
.\Build-Release.ps1 -NoStart      # build and deploy without starting
```

The build output is `target\release\browser-launcher.exe`; the deployment script
installs it to `%LOCALAPPDATA%\BrowserLauncher\BrowserLauncher.exe`.

## Usage

Start the executable with a URL:

```powershell
.\target\release\browser-launcher.exe "https://example.com"
```

If already running, the URL is forwarded to that instance; otherwise the
application starts in the tray and opens it in the selected browser.

Double-click the tray icon to select a browser and routing rules, then **Save**.
Arguments can contain `{url}`; otherwise the quoted URL is appended. The first
matching rule wins; **Default** is always last. History cells copy URLs or arguments.

**Settings → Set as default browser** opens Windows settings, where you confirm
the HTTP/HTTPS association.

To register the stable copy without opening the system settings page:

```powershell
.\target\release\browser-launcher.exe --install-register
```

## License

Browser Launcher is licensed under the GNU General Public License v3.0 or later.
See [LICENSE](LICENSE).

The Czech README is available as [README.cs.md](README.cs.md).
