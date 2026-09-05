# Plán implementace

## P0 – první použitelná verze

- [x] Založit minimalistický Rust/Win32 projekt a release profil zaměřený na velikost.
- [x] Vytvořit single-instance tray aplikaci.
- [x] Přijmout URL z příkazové řádky a předat ji běžící instanci.
- [x] Přidat nastavení jednoho výchozího cílového prohlížeče.
- [x] Přidat volitelné argumenty se zástupným textem `{url}`.
- [x] Persistovat pouze nastavení, ne URL.
- [x] Otevřít přijatou URL ve vybraném prohlížeči.
- [x] Předat foreground oprávnění a přenést cílové okno prohlížeče do popředí.
- [x] Držet poslední URL pouze v RAM.
- [x] Přidat tray akci **Kopírovat poslední URL**.
- [x] Přidat tray akce **Nastavení** a **Ukončit**.
- [x] Spouštět aplikaci tiše bez automatického otevření nastavení.
- [x] Přidat vlastní vícevelikostní tray a EXE ikonu.
- [x] Vypsat registrované prohlížeče včetně systémových ikon a zachovat ruční výběr.
- [x] Trvale zvýraznit vybraný prohlížeč a zobrazit jeho název pod seznamem.
- [x] Automaticky ověřit testy, release build a ukončení druhé instance.
- [x] Ručně ověřit výběr reálného prohlížeče, předání URL mezi procesy a schránku.

## P1 – registrace ve Windows

- [x] Navrhnout stabilní per-user instalaci a umístění EXE.
- [x] Zapsat per-user ProgID, `http`/`https` capabilities a `RegisteredApplications`.
- [x] Oznámit shellu změnu asociací.
- [x] Přidat tlačítko pro registraci a otevření systémového nastavení Výchozí aplikace.
- [ ] Ověřit celý tok na Windows 10 a Windows 11 bez pokusu obejít uživatelskou volbu.

## P2 – pravidla směrování

- [x] Přidat datový model a editor uspořádaných pravidel.
- [x] Implementovat jednoduché `*` vzory.
- [x] Implementovat plné regulární výrazy a validaci.
- [x] Umožnit každému pravidlu vybrat prohlížeč a argumenty.
- [x] Přidat testy priority pravidel, wildcard převodu a fallbacku.

## P3 – RAM log a dokončení UI

- [x] Přidat omezený RAM log úspěšně otevřených URL bez persistence.
- [x] Rozdělit okno na záložky **Výchozí prohlížeč**, **Pokročilé filtrování**, **Historie** a **Nastavení**.
- [x] Zobrazit v historii čas, URL, použitý prohlížeč a argumenty, nejnovější záznam nahoře.
- [x] Umožnit zkopírovat URL kliknutím na řádek historie.
- [x] Zobrazit výchozí fallback jako poslední nesmazatelný řádek pravidel.
- [x] Přidat změnu pořadí pravidel přetažením a ikony prohlížečů ve výběru.
- [x] Přidat volbu českého a anglického uživatelského rozhraní.
- [x] Doplnit vlastní ikonu a tooltip/stavové hlášky bez rušivých dialogů.
- [ ] Změřit release velikost, idle working set a start aplikace.
- [x] Přidat root PowerShell skript pro release build, nasazení do `%LOCALAPPDATA%` a výměnu běžící instance.
- [ ] Přidat instalaci, odinstalaci a úklid registrace.
