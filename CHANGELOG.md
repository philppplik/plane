# Änderungsverlauf

Alle nennenswerten Änderungen an Plane werden hier festgehalten.

Das Format folgt [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
die Versionierung [Semantic Versioning](https://semver.org/lang/de/).

## [Unveröffentlicht]

Nichts.

## [0.2.0] — 2026-09-11

Der Umbau vom Prototyp zum vollwertigen Cleaner — und zwei neue Werkzeuge
daneben: **Plane Uninstaller** und **Plane Tweaker**.

> Die Installationspakete sind **nicht signiert**. Windows SmartScreen wird
> beim ersten Start warnen. Siehe [SECURITY.md](SECURITY.md).

### Hinzugefügt

- **Zweistufige Engine.** Erst analysieren (`scan`), dann bereinigen (`clean`).
  Es gibt keinen Weg, etwas zu löschen, ohne dass vorher angezeigt wurde, was
  und wie viel. Ein Simulationsmodus (`dry_run`) meldet dieselben Zahlen, ohne
  zu löschen.
- **38 Reinigungsziele in sechs Kategorien**: System, Browser, Anwendungen,
  Installationsdateien, Papierkorb, Registry. Darunter Edge, Chrome, Firefox,
  Brave, Opera und Vivaldi, Discord, Teams, Spotify, VS Code, Steam, Adobe,
  Grafik-Shadercaches, Windows-Update-Cache, Absturzabbilder, Prefetch,
  Miniaturansichten und Windows.old.
- **Erkennung alter Installationsdateien** in Downloads und auf dem
  Schreibtisch, ab 30 Tagen und 5 MB. Wird nie automatisch gelöscht — der
  Nutzer wählt einzelne Dateien aus.
- **Registry-Bereinigung** in eng begrenztem Umfang, mit automatischer
  `.reg`-Sicherung vor jeder Änderung. Ohne erfolgreiche Sicherung wird nichts
  geändert.
- **Zwei Risikoachsen je Ziel** (`requires_admin`, `irreversible`) plus eine
  Risikostufe für die Anzeige. Riskante Ziele sind nie vorausgewählt.
- **Mehrsprachigkeit Deutsch/Englisch** mit einem gemeinsamen Sprachkatalog für
  Oberfläche, Kommandozeile und TUI.
- **Plane Uninstaller.** Installierte Programme auflisten und entfernen —
  aus allen vier Quellen (`HKLM` 64-Bit und 32-Bit, `HKCU`, Store/MSIX). Plane
  löscht nichts selbst, sondern startet den Deinstaller des Herstellers und
  prüft danach, ob der Registry-Eintrag verschwunden ist. Laufzeitpakete,
  Treiber, Sicherheitssoftware, Windows-Bestandteile und Plane selbst sind
  gesperrt, jeweils mit genanntem Grund. Die msiexec-Exitcodes werden
  ausgewertet: Neustart nötig, bereits deinstalliert, vom Nutzer abgebrochen,
  durch Richtlinie verboten — statt einer pauschalen Fehlermeldung.
- **Plane Tweaker.** 19 kuratierte Windows-Einstellungen in fünf Gruppen
  (Datenschutz, Explorer, Taskleiste, Leistung, System). Jeder Punkt nennt
  seine **Nebenwirkung**, nicht nur seinen Nutzen. Zurücknehmen stellt den
  Zustand wieder her, den Plane **tatsächlich vorgefunden** hat — inklusive
  des Falls „der Wert existierte vorher gar nicht" —, nicht einen angenommenen
  Windows-Standard. Punkte, die nur unter Windows 11 wirken oder unter einer
  Gruppenrichtlinie stehen, werden als solche gekennzeichnet statt wirkungslos
  gesetzt. Was bewusst **nicht** angeboten wird und warum, steht in
  [docs/TWEAKS.md](docs/TWEAKS.md#bewusst-nicht-angeboten).
- **Fehlerprotokoll** unter `%APPDATA%\com.ppaul.plane\plane.log`, ohne
  einzelne Dateipfade, mit Rotation bei 1 MB.
- **Neustart mit Administratorrechten** aus der Anwendung heraus.
- **Kommandozeile `plane-cli`** mit den Unterbefehlen `list`, `scan`, `clean`,
  `info`, `programs`, `uninstall`, `tweaks` und `tui`, JSON-Ausgabe für
  Skripte und sinnvollen Exitcodes.
- **Installationsskripte** für die Kommandozeile (`scripts/install-cli.ps1`
  mit SHA-256-Prüfung, `scripts/install-cli.bat`).
- **Textoberfläche (TUI)** mit ASCII-Logo, Kategorien- und Zielauswahl,
  Fortschrittsbalken und vollständiger Tastaturbedienung.
- **Fortschrittsanzeige** in allen drei Oberflächen: Prozentwert, aktuelles
  Ziel, laufende Byte-Summe, aktuell bearbeiteter Pfad — und ein
  Abbrechen-Knopf, der auch greift.
- **Einstellungen**: Sprache, Erscheinungsbild (System/Hell/Dunkel), Nachfrage
  vor riskanten Schritten, Simulation als Voreinstellung. Werden als JSON im
  Benutzerprofil gespeichert.
- **Dunkles Erscheinungsbild** und responsives Layout ab 620 px Fensterbreite.
- **Schutzmechanismen**: Pfadtiefenprüfung und Sperrliste für Systemordner,
  Reparse-Point-Erkennung, keine Shell-Aufrufe, Warnung bei laufenden
  Programmen, Abbruch statt halber Bereinigung bei fehlgeschlagenem
  Dienststopp.
- **Dokumentation**: `docs/ARCHITEKTUR.md`, `docs/DATENSTRUKTUREN.md`,
  `docs/REINIGUNGSZIELE.md` (inklusive der bewussten Auslassungen),
  `docs/TWEAKS.md`, `docs/CLI.md`, `docs/TESTS.md`, `CONTRIBUTING.md`,
  `SECURITY.md`, `THIRD-PARTY-NOTICES.md`.
- **CI** für Rust, Python und Frontend; Veröffentlichungs-Workflow für
  x86-64 **und** ARM64.

### Geändert

- Reinigungslogik von Python nach Rust portiert. Die App braucht keine
  Python-Installation mehr.
- Dateien werden von Plane selbst gelöscht statt über `del`/`Remove-Item`.
  Dadurch sind freigegebene Bytes tatsächlich gemessen und Fehler überhaupt
  erkennbar.
- Der Zielkatalog ist datengetrieben: ein neues Ziel besteht aus einem Eintrag
  plus zwei Übersetzungen, ohne neuen Code.
- Oberfläche vollständig neu aufgebaut: Kategorien mit Gesamtgrößen,
  Einzelauswahl, Risikohinweise, Ergebnisansicht. Dazu zwei neue Ansichten in
  der Seitenleiste — **Programme** und **Einstellungen anpassen** —, die erst
  beim ersten Öffnen laden, weil beide die Registry abfragen.
- Anwendungslogik aus `main.rs` in die Bibliothek verschoben; Unit-Tests laufen
  dadurch einmal statt doppelt.

### Behoben

- Alle 45 im Mängelregister erfassten Defekte, dokumentiert in
  [docs/BEKANNTE_MAENGEL.md](docs/BEKANNTE_MAENGEL.md). Die schwerwiegendsten:
  - Die Bereinigung war überhaupt nicht angebunden — der Tauri-Command war ein
    Rumpf mit fester Antwort.
  - Die Oberfläche war nicht bedienbar: falscher Tauri-API-Importpfad (Version
    1 statt 2) und unerreichbare Ereignisbehandlung.
  - Der Papierkorb wurde ohne Rückfrage endgültig geleert.
  - Die Modulauflösung fiel auf einen einzigen Schlüssel zusammen; selektive
    Bereinigung war unbenutzbar.
  - Freigegebener Speicher war eine gezählte Schrittanzahl, keine Megabyte.
  - Jeder normale Lauf meldete „Some steps failed", weil übersprungene Schritte
    als Fehler zählten.
  - Windows 11 wurde als Windows 10 angezeigt.
- Analyse um etwa den Faktor 8 beschleunigt: rekursive Muster vermaßen
  denselben Verzeichnisbaum mehrfach.
- **Gesperrte Dateien sind keine Fehler mehr.** Eine Datei, die gerade von
  einem laufenden Programm benutzt wird (Windows-Fehler 32/33) oder für die
  die Rechte fehlen (Fehler 5), wurde als Fehlschlag gemeldet — ein völlig
  normaler Lauf sah dadurch kaputt aus. Beides wird jetzt getrennt gezählt und
  als Hinweis ausgewiesen; der Lauf gilt weiterhin als erfolgreich.

### Sicherheit

- Content-Security-Policy aktiviert (war abgeschaltet).
- Tauri-2-Capabilities ergänzt (fehlten vollständig).
- Keine Shell-Aufrufe mehr; externe Befehle laufen als Argumentliste.
- Registry-Sperrlisten für `Services`, `Winlogon`, `AppModel`, `PackagedCom`,
  `ActivatableClasses`, `Installer`, `CLSID` und `TypeLib`.

### Entfernt

- `utils/cleaner.py` samt zugehöriger Tests. Die Python-CLI aus der Frühphase
  deckte 8 der 38 Ziele ab und ist durch `plane-cli` vollständig abgelöst.
- Byte-identische Asset-Dubletten im Projektverzeichnis (`assets/`,
  `app-icn.png`, `bg-image.png`). Gebündelt wird ausschließlich
  `frontend/assets/`.

### Offen

- Signierung der Installationspakete, siehe [SECURITY.md](SECURITY.md).

## [0.1.0] — 2026-08-30

Erster Prototyp: Tauri-Grundgerüst, Willkommensbildschirm, Python-Skript mit
acht Reinigungsschritten.
