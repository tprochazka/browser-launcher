# Browser Launcher

Minimalistický tray „prohlížeč“ pro Windows 10 a 11. Přijme URL jako výchozí
handler pro `http`/`https`, vybere podle pravidel skutečný prohlížeč a předá mu
URL i volitelné argumenty.

## Funkce

- běží tiše jako jediná instance v oznamovací oblasti Windows,
- při spuštění s URL předá požadavek běžící instanci; pokud neběží, spustí se a zůstane v tray,
- seznam systémově registrovaných prohlížečů včetně jejich názvů a ikon,
- výchozí prohlížeč a argumenty se zástupným textem `{url}`,
- směrování podle pořadí wildcard pravidel (`*`) nebo regulárních výrazů,
- poslední nesmazatelný řádek **Výchozí** jako fallback pro pravidla,
- změna pořadí pravidel přetažením,
- RAM historie úspěšně otevřených URL s časem, prohlížečem a argumenty,
- kliknutí na URL nebo argumenty v historii kopíruje příslušnou hodnotu,
- kontextové menu historie umí kopírovat URL, argumenty nebo celý záznam,
- zkopírování poslední URL z tray menu,
- registrace aplikace jako kandidáta výchozího prohlížeče a otevření systémového nastavení,
- české a anglické uživatelské rozhraní,
- bez .NET runtime, WebView, Electronu nebo jiného aplikačního runtime.

Historie a poslední URL se drží pouze v RAM a při ukončení procesu zaniknou.
Trvalé nastavení se ukládá do `HKCU\Software\BrowserLauncher`.

## Požadavky

- Windows 10 nebo Windows 11 (64bit),
- Rust 1.85 nebo novější, toolchain `x86_64-pc-windows-msvc`,
- Visual Studio Build Tools s workloadem **Desktop development with C++**.

Projekt používá Rust edition 2024. Přesné verze crate závislostí jsou v
`Cargo.lock`; hlavními závislostmi jsou `windows-sys`, `regex` a build-time
`embed-resource`.

## Sestavení

```powershell
cargo build --release
```

Výstup je `target\release\browser-launcher.exe`. Pro sestavení, nasazení do
stabilní per-user cesty a restart běžící aplikace použijte skript v rootu:

```powershell
.\Build-Release.ps1
```

Skript nasadí EXE do `%LOCALAPPDATA%\BrowserLauncher\BrowserLauncher.exe`,
obnoví registraci a spustí novou tray instanci. Pro build a nasazení bez startu:

```powershell
.\Build-Release.ps1 -NoStart
```

## Použití

Spuštění s URL:

```powershell
.\target\release\browser-launcher.exe "https://example.com"
```

Pokud aplikace běží, URL předá existující instanci. Pokud neběží, Windows ji
spustí, URL se otevře ve vybraném prohlížeči a aplikace zůstane pouze v tray.

V nastavení lze vybrat výchozí prohlížeč, jeho argumenty a pravidla směrování.
Výchozí řádek pravidel nelze smazat ani přesunout. Windows neumožňuje aplikaci
tiše změnit výchozího handlera; tlačítko registrace proto připraví registraci a
otevře systémovou stránku, kde volbu potvrdí uživatel.

Ruční registrace stabilní kopie bez otevření systémového nastavení:

```powershell
.\target\release\browser-launcher.exe --install-register
```

## Kontrola

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

## Dokumentace

- [Zadání](docs/zadani.md)
- [Implementační analýza](docs/implementacni-analyza.md)
- [Plán implementace](docs/tasks.md)

Build výstupy v `target/` a lokální vývojové artefakty nejsou součástí
repozitáře.

## Licence

Browser Launcher je licencován pod GNU General Public License v3.0 nebo novější.
Viz [LICENSE](LICENSE).
