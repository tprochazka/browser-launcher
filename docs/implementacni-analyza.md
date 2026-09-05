# Implementační analýza

## Volba technologie

Pro tuto aplikaci je vhodnější **Rust s přímým Win32 API** než .NET/WinForms nebo WPF. Výsledkem je jeden nativní EXE bez požadavku na .NET Desktop Runtime, s malou pracovní sadou a bez paměťové režie managed runtime. Nevýhodou je pracnější tvorba UI; vzhledem k několika formulářovým prvkům je tento kompromis přijatelný.

GUI toolkit typu Tauri, WebView nebo Electron by byl vzhledem k cíli zbytečně velký. První implementace proto používá pouze systémové Win32 ovládací prvky a crate `windows-sys`, který nepřidává vlastní runtime.

## Návrh běhu aplikace

První proces vytvoří skryté Win32 okno, tray ikonu a message loop. Nastavení se při startu automaticky nezobrazuje; okno se otevře až z tray menu nebo dvojklikem na ikonu. Další proces najde okno podle privátní window class, předá oprávnění k foreground aktivaci, synchronně mu pošle URL přes `WM_COPYDATA` a skončí. Tray instance po spuštění cílového prohlížeče vyhledá jeho viditelné hlavní okno podle skutečné cesty procesu, obnoví je a přesune do popředí. Díky tomu existuje jediný RAM log i jediná poslední URL.

Úspěšná otevření se ukládají pouze v paměti jako čas, URL, název cílového prohlížeče a skutečně předané argumenty. Seznam je omezen na 500 nejnovějších záznamů a při ukončení procesu zanikne. Nastavení používá čtyři nativní Win32 záložky; společné tlačítko Uložit je mimo jejich obsah a zůstává vždy dole. Poslední řádek pokročilého filtrování reprezentuje stejný výchozí fallback jako první záložka a nelze jej odstranit ani přesunout.

Nastavení výchozího cílového prohlížeče se ukládá pro aktuálního uživatele do `HKCU\Software\BrowserLauncher`. URL a budoucí log se do registru ani na disk neukládají.

Argumenty cílového prohlížeče podporují zástupný text `{url}`. Není-li uveden, bezpečně uvozená URL se připojí na konec argumentů.

Seznam cílových prohlížečů se načítá pro aktuálního uživatele i celý počítač ze standardních klíčů `Software\Clients\StartMenuInternet`. Používá registrovaný název, spouštěcí příkaz a `DefaultIcon`; duplicitní instalace se sloučí podle cesty EXE. Ruční výběr zůstává fallbackem.

## Registrace jako výchozí prohlížeč

Desktopová aplikace se musí zaregistrovat jako kandidát pro protokoly `http` a `https`: vytvoří vlastní ProgID a command, zapíše `Capabilities\UrlAssociations` a položku v `HKCU\Software\RegisteredApplications`. Následně oznámí změnu asociací shellu.

Windows 10 a 11 nedovolují aplikaci tiše se sama nastavit jako výchozí handler. Konečnou volbu provede uživatel v systémovém UI. Tlačítko v nastavení proto provede registraci a otevře `ms-settings:defaultapps`; na podporovaných Windows 11 lze stránku zacílit přímo na registrovanou aplikaci.

Podklady: [Windows app defaults platform](https://learn.microsoft.com/en-us/windows/apps/develop/windows-integration/default-apps-platform), [Default Programs pro Win32](https://learn.microsoft.com/en-us/windows/win32/shell/default-programs) a [otevření stránky Default Apps](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-default-apps-settings).

Registrace musí používat stabilní umístění EXE. Před implementací této části je proto vhodné doplnit instalační nebo per-user instalační krok; registrace EXE přímo z build adresáře by po přesunu přestala fungovat.

## Směrovací pravidla

Budoucí pravidlo bude obsahovat typ (`wildcard` nebo `regex`), vzor, cestu prohlížeče, argumenty, pořadí a příznak aktivace. Jednoduchý vzor escapuje běžné znaky a `*` převede na libovolnou posloupnost. Pravidla se vyhodnotí shora dolů; první shoda vítězí, jinak se použije výchozí prohlížeč.

Neplatný regulární výraz musí být odmítnut už při ukládání. Předávání URL se nesmí skládat do jednoho příkazu pro shell; cesta programu a argumenty se předávají odděleně.

## Rizika a rozhodnutí

- Samotný název okna není kryptografická ochrana single-instance komunikace. Pro lokální desktopovou utilitu je dostačující; později lze přejít na pojmenovaný mutex a pipe s omezeným ACL.
- Registry nastavení je malé a nevyžaduje parser ani další závislost.
- Ukončení tray procesu smaže poslední URL a celý budoucí log, jak zadání požaduje.
- Vlastní ikona, automatický start a instalátor nejsou součástí prvního řezu.
