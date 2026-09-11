<div align="center">

# Plane

**Ein schlanker PC-Cleaner für Windows.**
Lokal, schnell, ohne Datenübertragung — mit grafischer Oberfläche, Kommandozeile und TUI.

[![CI](https://github.com/philppplik/plane/actions/workflows/ci.yml/badge.svg)](https://github.com/philppplik/plane/actions/workflows/ci.yml)
[![Lizenz: MIT](https://img.shields.io/badge/Lizenz-MIT-A7EC5B.svg)](LICENSE)
[![Plattform](https://img.shields.io/badge/Windows-10%20%7C%2011%20%C2%B7%20x64%20%7C%20ARM64-143302.svg)](#systemanforderungen)

</div>

---

> **Vorabversion.** Plane funktioniert, ist getestet und wird auf einem echten
> System benutzt — aber es gibt noch keine signierte Veröffentlichung. Bis
> dahin: aus dem Quelltext bauen. Siehe [SECURITY.md](SECURITY.md).

## Was Plane macht

Plane räumt auf, was sich über die Zeit auf einem Windows-Rechner ansammelt:
temporäre Dateien, Browser-Zwischenspeicher, Absturzabbilder, alte
Update-Pakete, Grafik-Shadercaches, vergessene Installationsdateien.

Der Unterschied zu einem naiven „Aufräumen"-Knopf liegt im Ablauf:

```
Analysieren  →  Auswählen  →  Bereinigen
(ändert nichts)   (Sie)       (nur Ausgewähltes)
```

**Es gibt keinen Weg, etwas zu löschen, ohne dass vorher angezeigt wurde, was
und wie viel.** Riskante Ziele sind nie vorausgewählt, und ein
Simulationsmodus zeigt jederzeit, was passieren *würde*.

## Funktionen

- **38 Reinigungsziele** in sechs Kategorien: System, Browser, Anwendungen,
  Installationsdateien, Papierkorb, Registry
- **Browser:** Edge, Chrome, Firefox, Brave, Opera (inkl. GX), Vivaldi —
  inklusive aller Profile
- **Anwendungen:** Discord, Teams, Spotify, VS Code, Steam, Adobe, Java,
  Office, npm/pip sowie Grafik-Shadercaches von NVIDIA, AMD, Intel und DirectX
- **Alte Installationsdateien** in Downloads und auf dem Schreibtisch werden
  gefunden und *vorgeschlagen* — gelöscht wird nur, was Sie einzeln anhaken
- **Registry-Bereinigung** in eng begrenztem Umfang, mit automatischer
  `.reg`-Sicherung vor jeder Änderung
- **Tatsächlich gemessene Megabyte**, nicht geschätzt
- **Deutsch und Englisch**, umschaltbar im laufenden Betrieb
- **Helles und dunkles Erscheinungsbild**, dem System folgend oder fest
- **Fortschrittsanzeige** mit Prozentwert, aktuellem Ziel, laufender
  Byte-Summe und einem Abbrechen-Knopf, der auch wirkt
- **Programme deinstallieren** — installierte Software auflisten und entfernen,
  mit Programmsymbol, Größe und Herkunft, und mit Schutz für Laufzeitpakete,
  Treiber und Systembestandteile
- **Windows-Einstellungen** — 19 kuratierte, umkehrbare Punkte zu Datenschutz,
  Explorer, Taskleiste und Leistung
- **Kommandozeile und TUI** für alles, was die Oberfläche kann
- **Keine Telemetrie.** Plane zählt nichts, meldet nichts und legt kein Profil
  an — weder anonym noch sonstwie
- **Offline, sofern Sie nichts anderes wollen.** Der einzige Netzwerkzugriff
  ist die Suche nach neuen Versionen. Sie ist **standardmäßig aus** und lässt
  sich in den Einstellungen einschalten; siehe
  [Aktuell bleiben](#aktuell-bleiben)

## Sicherheit

Plane löscht Dateien. Das ist der Zweck — und das Risiko. Die eingebauten
Schutzmechanismen:

| Mechanismus | Wogegen |
|-------------|---------|
| Zweistufiger Ablauf | Löschen ohne vorherige Anzeige |
| Pfadtiefenprüfung und Sperrliste für `System32`, `WinSxS`, `Program Files` | Ein fehlerhaftes Muster löscht eine Partition |
| Reparse-Point-Erkennung | Eine Junction führt den Lauf aus dem Zielbaum heraus |
| Keine Shell — Befehle als Argumentliste | Command-Injection |
| Registry-Sicherung vor jeder Änderung | Nicht rückholbare Registry-Schäden |
| Tweaks merken den **vorgefundenen** Zustand, nicht einen angenommenen Standard | Zurücknehmen überschreibt eigene Einstellungen |
| Geschützte Programme können nicht deinstalliert werden | Entfernte Laufzeitpakete brechen andere Software |
| Abbruch statt halber Bereinigung bei fehlgeschlagenem Dienststopp | Inkonsistente Caches |
| Warnung bei laufenden Programmen | Weniger löschen als angekündigt |

Was Plane **bewusst nicht** anfasst, steht jeweils mit Begründung in der
Dokumentation: beim Reinigen etwa Browser-Passwörter, COM-Registrierungen und
Spotify-Downloads ([Details](docs/REINIGUNGSZIELE.md#bewusst-nicht-bereinigt)),
bei den Einstellungen etwa das Abschalten von Windows Update und Defender
([Details](docs/TWEAKS.md#bewusst-nicht-angeboten)).

## Systemanforderungen

Windows 10 oder 11, **x64 oder ARM64**. Plane wird nativ für beide
Architekturen gebaut; das ist keine Kosmetik, sondern Korrektheit — ein
32-Bit-Prozess unterliegt der WOW64-Umleitung und würde gültige Einträge
fälschlich für verwaist halten.

Administratorrechte sind **optional**. Ohne sie werden die betroffenen Ziele
gemeldet übersprungen, statt still zu scheitern.

## Aktuell bleiben

Plane kann nachsehen, ob eine neuere Version erschienen ist. Das ist der
einzige Netzwerkzugriff des Programms, und er ist **standardmäßig
ausgeschaltet**.

Einschalten: **Einstellungen → Nach neuen Versionen suchen**. Daneben steht
ein Knopf **Jetzt prüfen**, der auch ohne die Einstellung funktioniert — wer
ihn drückt, hat sich ja gerade entschieden.

In der Kommandozeile:

```bash
plane-cli update
```

Was dabei passiert, vollständig:

| | |
|-|-|
| **Wohin** | eine Anfrage an `api.github.com`, sonst nirgendwohin |
| **Was hin** | nichts außer dem `User-Agent` `plane/<version>`, den GitHub verlangt. GitHub sieht Ihre IP-Adresse, wie beim Aufruf jeder Webseite |
| **Was zurück** | die Nummer der jüngsten Veröffentlichung |
| **Was danach** | nichts. Plane lädt nichts herunter und installiert nichts |

Ist etwas Neueres da, erscheint ein Hinweis mit drei Möglichkeiten:
**Herunterladen** öffnet die Veröffentlichungsseite im Browser, **Später**
blendet den Hinweis bis zum nächsten Start aus, **Diese Version überspringen**
dauerhaft für genau diese Version.

**Warum Plane sich nicht selbst aktualisiert:** ein Programm, das sich selbst
ersetzen kann, lässt sich auch durch etwas anderes ersetzen. Solange die
Pakete nicht signiert sind ([SECURITY.md](SECURITY.md)), wäre das ein
Angriffsweg und kein Komfortgewinn.

## Bauen

```bash
npm install
npm run tauri:dev
```

Produktionsbuild (MSI-Installer):

```bash
npm run tauri:build
```

Voraussetzungen: [Rust](https://rustup.rs/) (stable),
[Node.js](https://nodejs.org/) 18+, Visual Studio Build Tools.
Python 3.11+ nur für die Tests.

## Kommandozeile

Installieren — ein Befehl in PowerShell, keine Administratorrechte nötig:

```powershell
irm https://raw.githubusercontent.com/philppplik/plane/main/scripts/install-cli.ps1 | iex
```

Erkennt die Architektur, prüft die SHA256-Summe und ergänzt den PATH. Aus einem
ausgecheckten Repository geht auch `scripts\install-cli.bat`.

Danach in einem **neuen** Terminal:

```bash
plane-cli                     # Textoberfläche (Standard im Terminal)
plane-cli list                # alle Reinigungsziele
plane-cli scan                # analysieren, ohne etwas zu verändern
plane-cli scan --json         # dasselbe für Skripte
plane-cli clean --dry-run     # simulieren
plane-cli clean -y            # empfohlene Auswahl bereinigen
plane-cli clean browser.chrome.cache app.discord
plane-cli info                # System, Rechtestatus, Laufwerk
plane-cli programs            # installierte Programme
plane-cli uninstall Discord   # ein Programm entfernen
plane-cli tweaks              # Windows-Einstellungen anzeigen
plane-cli tweaks --on explorer.file_extensions
```

Global: `--lang de|en`, `--no-color`.
Exitcodes: `0` Erfolg, `1` mindestens ein Ziel fehlgeschlagen, `2` Bedienfehler,
`130` abgebrochen.

Die TUI bietet Kategorien- und Zielauswahl, Fortschrittsbalken und
vollständige Tastaturbedienung (`?` zeigt die Belegung).

Vollständige Referenz mit allen Zielschlüsseln, Exitcodes und der
JSON-Ausgabe: [docs/CLI.md](docs/CLI.md).

## Tests

```bash
npm test
```

299 Rust-Unit-Tests und 55 statische Vertragstests, zusammen unter 30
Sekunden. Die Vertragstests prüfen ohne laufende App, dass Frontend, Backend
und Sprachkatalog zusammenpassen — etwa dass jeder aufgerufene Command
existiert und jedes Reinigungsziel übersetzt ist.

Details und die bewusst nicht abgedeckten Bereiche: [docs/TESTS.md](docs/TESTS.md).

## Aufbau

Der Leitgedanke: **die Engine kennt keine Oberfläche.** Alles unter
`src-tauri/src/engine/` läuft ohne Tauri, ohne HTML und ohne Terminal — GUI,
CLI und Tests verwenden denselben Code.

```
src-tauri/src/engine/catalog.rs   WAS bereinigt wird (einziger Ort mit Pfaden)
src-tauri/src/engine/scan.rs      WAS WÄRE löschbar (nebenwirkungsfrei)
src-tauri/src/engine/clean.rs     LÖSCHEN, was ausgewählt wurde
src-tauri/src/engine/uninstall.rs Programme auflisten und deinstallieren
src-tauri/src/engine/tweaks.rs    Windows-Einstellungen, umkehrbar
src-tauri/src/engine/update.rs    der einzige Netzwerkzugriff, standardmäßig aus
src-tauri/src/engine/icons.rs     Programmsymbole aus .exe/.dll/.ico
src-tauri/src/i18n.rs             alle Texte, Deutsch und Englisch
src-tauri/src/commands.rs         Tauri-Brücke, keine Logik
src-tauri/src/cli/                Kommandozeile und TUI
frontend/                         Oberfläche (Vanilla JS, Vite)
scripts/                          Installation der CLI
tests/                            statische Vertragstests (pytest)
```

Ein neues Reinigungsziel besteht aus **einem Katalogeintrag plus zwei
Übersetzungen** — kein neuer Code.

## Dokumentation

| Dokument | Inhalt |
|----------|--------|
| [docs/CLI.md](docs/CLI.md) | Kommandozeile und TUI: alle Befehle, Zielschlüssel, Exitcodes, JSON |
| [docs/ARCHITEKTUR.md](docs/ARCHITEKTUR.md) | Aufbau, Schichten, Entwurfsentscheidungen |
| [docs/REINIGUNGSZIELE.md](docs/REINIGUNGSZIELE.md) | Jedes Ziel mit Risiko und Nebenwirkung — und was bewusst fehlt |
| [docs/TWEAKS.md](docs/TWEAKS.md) | Windows-Einstellungen: jeder Punkt mit Nebenwirkung — und was bewusst fehlt |
| [docs/DATENSTRUKTUREN.md](docs/DATENSTRUKTUREN.md) | Datenmodell, Command-Verträge, Konventionen |
| [docs/TESTS.md](docs/TESTS.md) | Testsuite, Sicherheitsnetze, Lücken |
| [docs/BEKANNTE_MAENGEL.md](docs/BEKANNTE_MAENGEL.md) | Mängelregister der Frühphase |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Mitwirken, neues Reinigungsziel hinzufügen |
| [SECURITY.md](SECURITY.md) | Sicherheitsmodell, Lücken melden |
| [CHANGELOG.md](CHANGELOG.md) | Änderungsverlauf |
| [THIRD-PARTY-NOTICES.md](THIRD-PARTY-NOTICES.md) | Projekte, auf deren Arbeit Plane aufbaut |

## Dank

Die Pfade und Risikoeinschätzungen stützen sich unter anderem auf die Arbeit
von [BleachBit](https://www.bleachbit.org/) und
[Winapp2](https://github.com/MoscaDotTo/Winapp2) — beide quelloffen und
deutlich länger im Geschäft.

## Lizenz

MIT — siehe [LICENSE](LICENSE).
