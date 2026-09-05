# Zadání aplikace Browser Launcher

## Cíl

Vytvořit minimalistickou aplikaci pro Windows 10 a 11, která se v systému nabídne jako webový prohlížeč a příchozí URL přesměruje do zvoleného skutečného prohlížeče. Aplikace poběží v oznamovací oblasti, bude malá na disku a bude mít nízkou spotřebu paměti.

## Požadované chování

- Aplikace přijme URL z příkazové řádky a předá ji cílovému prohlížeči.
- Běží pouze jedna tray instance. Další spuštění předá URL existující instanci a skončí.
- Tray menu obsahuje:
  - **Settings** – nastavení výchozího cílového prohlížeče, jeho argumentů a později pravidel směrování.
  - **Log** – dočasný seznam URL otevřených za běhu aplikace; nic se nepersistuje.
  - **Copy last URL to clipboard** – zkopíruje poslední přijatou URL.
  - **Exit** – aplikaci ukončí.
- Pravidlo směrování může být plný regulární výraz nebo jednoduchý vzor s `*` a má přiřazený prohlížeč a argumenty.
- Pokud žádné pravidlo neodpovídá, použije se výchozí cílový prohlížeč.
- Nastavení nabídne registraci aplikace jako kandidáta na výchozí prohlížeč pro `http` a `https` a otevření systémové stránky Výchozí aplikace.

## Priorita první verze

První použitelná verze musí umět vybrat jeden cílový prohlížeč, volitelně nastavit jeho argumenty, otevřít v něm přijatou URL a z tray menu zkopírovat poslední URL. Následující etapy doplňují pravidla, RAM historii, registraci do Windows a lokalizované záložkové UI.

## Akceptační kritéria první verze

1. Uživatel vybere existující `.exe` prohlížeče a nastavení se zachová po restartu.
2. Spuštění `browser-launcher.exe https://example.com` otevře URL ve zvoleném prohlížeči.
3. Je-li tray instance spuštěná, další proces jí URL předá a ukončí se.
4. Položka **Kopírovat poslední URL** vloží přesnou poslední přijatou URL do schránky.
5. Poslední URL se neukládá na disk ani do registru.
