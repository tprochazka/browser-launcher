# Browser Launcher

Malý tray router URL pro Windows 10 a 11. Přijme URL jako handler pro
`http`/`https`, podle pravidel vybere prohlížeč a předá mu URL i volitelné
argumenty.

Aktuální verze: **0.9.1**.

## Funkce

- Jediná instance běžící tiše v tray.
- Nainstalované prohlížeče s názvy a ikonami.
- Wildcard a regex pravidla podle pořadí; výchozí prohlížeč jako záloha.
- Změna pořadí pravidel přetažením.
- Vlastní argumenty prohlížeče s `{url}`.
- Historie URL v paměti s kopírováním kliknutím.
- Kopírování poslední URL z tray menu.
- Registrace HTTP/HTTPS ve Windows.
- České a anglické rozhraní.

Nastavení se ukládá do `HKCU\Software\BrowserLauncher`; historie se při ukončení vymaže.

## Požadavky

- Windows 10 nebo Windows 11 (64bit),
- Rust 1.88 nebo novější, toolchain `x86_64-pc-windows-msvc`,
- Visual Studio Build Tools s workloadem **Desktop development with C++**.

Verze závislostí jsou v `Cargo.lock`.

## Sestavení

V kořeni projektu zvolte jeden z příkazů:

```powershell
cargo build --release           # pouze sestavení
.\Build-Release.ps1              # sestavení, nasazení, registrace a spuštění
.\Build-Release.ps1 -NoStart      # nasazení bez spuštění
```

Výstup je `target\release\browser-launcher.exe`; skript jej nasadí do
`%LOCALAPPDATA%\BrowserLauncher\BrowserLauncher.exe`.

## Použití

Spuštění s URL:

```powershell
.\target\release\browser-launcher.exe "https://example.com"
```

Pokud aplikace běží, URL předá existující instanci; jinak se spustí v tray a URL
otevře ve vybraném prohlížeči.

Dvojklikem na tray ikonu otevřete výběr prohlížeče a pravidel, poté klikněte na
**Uložit**. Argumenty mohou obsahovat `{url}`; jinak se URL v uvozovkách připojí
na konec. Vyhrává první odpovídající pravidlo, **Výchozí** je vždy poslední.
Kliknutím na buňku historie zkopírujete URL nebo argumenty.

**Nastavení → Nastavit jako výchozí prohlížeč** otevře nastavení Windows,
kde potvrdíte přiřazení HTTP/HTTPS.

Ruční registrace stabilní kopie bez otevření systémového nastavení:

```powershell
.\target\release\browser-launcher.exe --install-register
```

## Licence

Browser Launcher je licencován pod GNU General Public License v3.0 nebo novější.
Viz [LICENSE](LICENSE).
