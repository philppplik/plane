# Kommandozeile und Textoberfläche

`plane-cli` kann alles, was die grafische Oberfläche kann — sie nutzt dieselbe
Engine. Stand: 2026-09-11.

## Installation

### Empfohlen: ein Befehl im Terminal

PowerShell öffnen und einfügen:

```powershell
irm https://raw.githubusercontent.com/philppplik/plane/main/scripts/install-cli.ps1 | iex
```

Das Skript erkennt die Architektur (x64 oder ARM64), lädt `plane-cli.exe` aus
der neuesten Veröffentlichung, **prüft die SHA256-Summe** gegen die
veröffentlichte `SHA256SUMS.txt`, legt die Datei in
`%LOCALAPPDATA%\Programs\Plane` ab und ergänzt den Benutzer-PATH.

Keine Administratorrechte nötig — geschrieben wird nur ins eigene Profil.

> Die Prüfsummenkontrolle ist kein Beiwerk. Plane löscht Dateien; ein
> manipuliertes Binary wäre ein Totalschaden. Stimmt die Summe nicht, bricht
> das Skript ab und installiert nichts.

Eine bestimmte Version:

```powershell
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/philppplik/plane/main/scripts/install-cli.ps1))) -Version v0.3.0
```

### Aus dem Quelltext

Wenn du das Repository ohnehin ausgecheckt hast:

```bat
scripts\install-cli.bat
```

Baut das Binary bei Bedarf selbst (braucht dann [Rust](https://rustup.rs/)) und
installiert es an dieselbe Stelle.

### Von Hand

`plane-cli.exe` aus der [Veröffentlichung](https://github.com/philppplik/plane/releases)
laden und irgendwohin legen, das im PATH liegt. Mehr braucht es nicht — die CLI
ist eine einzelne Datei ohne Abhängigkeiten.

### Prüfen

**Neues Terminal öffnen** (PATH-Änderungen gelten nicht rückwirkend), dann:

```bash
plane-cli --version
```

### Entfernen

```bat
scripts\uninstall-cli.bat
```

Oder von Hand: `%LOCALAPPDATA%\Programs\Plane` löschen und den Eintrag aus dem
Benutzer-PATH nehmen.

### Ohne Installation

Direkt aus dem Buildverzeichnis:

```bat
src-tauri\target\release\plane-cli.exe scan
```

## Überblick

```
plane-cli [OPTIONEN] [BEFEHL]

Befehle:
  list    Alle Reinigungsziele auflisten
  scan    Analysieren, ohne etwas zu verändern
  clean   Bereinigen
  info      Systeminformationen, Rechtestatus und Laufwerksbelegung
  programs  Installierte Programme auflisten
  uninstall Ein Programm deinstallieren
  tweaks    Windows-Einstellungen anzeigen und ändern
  update    Nachsehen, ob eine neuere Version erschienen ist
  tui       Die textbasierte Oberfläche starten

Globale Optionen:
  --lang <de|en>   Sprache der Ausgabe (Standard: Systemsprache)
  --no-color       Keine Farben und keine Fortschrittsanimation
  -h, --help       Hilfe
  -V, --version    Version
```

Ohne Befehl startet die TUI — aber nur, wenn ein Terminal zuschaut. In einer
Pipeline wäre das eine Falle, dort meldet sich stattdessen ein Hinweis.

`--help` gibt es auch je Befehl: `plane-cli clean --help`.

## Befehle

### `list` — was Plane kennt

```bash
plane-cli list
```

```bash
plane-cli list --category browsers
```

Kategorien: `system`, `browsers`, `apps`, `installers`, `recyclebin`,
`registry`.

Die Risikospalte zeigt `+` unbedenklich, `!` mit Nebenwirkung, `#` Vorsicht.

### `scan` — analysieren

Verändert **nichts**. Zeigt je Ziel die gefundene Größe und Anzahl, darunter
eine Zusammenfassung je Kategorie.

```bash
plane-cli scan
```

```bash
plane-cli scan browser.chrome.cache app.discord
```

| Option | Wirkung |
|--------|---------|
| `--all` | Auch Ziele ohne Treffer anzeigen |
| `--json` | Maschinenlesbare Ausgabe |

### `clean` — bereinigen

Ohne Zielangabe wird die **empfohlene Auswahl** verwendet (alle unbedenklichen
Ziele). Riskante Ziele sind dort nie enthalten.

```bash
plane-cli clean --dry-run
```

```bash
plane-cli clean -y
```

```bash
plane-cli clean browser.chrome.cache app.teams
```

| Option | Wirkung |
|--------|---------|
| `--dry-run` | Nichts löschen, nur berichten, was passieren würde |
| `-y`, `--yes` | Ohne Rückfrage ausführen |
| `--json` | Maschinenlesbare Ausgabe |

Ohne `--yes` und ohne `--json` zeigt Plane erst die Analyse und fragt nach.
Riskante Ziele werden dabei zusätzlich einzeln benannt.

> **`--dry-run` meldet exakt dieselben Zahlen wie ein echter Lauf.** Gemessen
> wird vor dem Löschen, nicht danach. Im Zweifel also immer erst simulieren.

### `info` — System und Rechte

```bash
plane-cli info
```

Zeigt Betriebssystem, CPU, Architektur, RAM, Laufwerksbelegung und ob Plane
mit Administratorrechten läuft.

### `programs` — installierte Programme

```bash
plane-cli programs
```

```bash
plane-cli programs --filter discord
```

| Option | Wirkung |
|--------|---------|
| `--filter <TEXT>` | Nur Namen, die diesen Text enthalten |
| `--all` | Auch geschützte Einträge zeigen |
| `--json` | Maschinenlesbare Ausgabe |

Standardmäßig werden nur entfernbare Einträge gezeigt. `--all` blendet auch die
geschützten ein — mit Begründung in der Spalte „Hinweis".

### `uninstall` — ein Programm entfernen

```bash
plane-cli uninstall Discord
```

Die Kennung stammt aus `programs`. **Plane löscht nichts selbst** — es startet
den Deinstaller des jeweiligen Herstellers.

| Option | Wirkung |
|--------|---------|
| `-y`, `--yes` | Ohne Rückfrage |
| `--quiet` | Nach Möglichkeit ohne Dialog |
| `--json` | Maschinenlesbare Ausgabe |

`--quiet` wirkt nur, wenn der Hersteller einen stillen Schalter hinterlegt hat
oder es ein MSI-Paket ist. Plane **rät keine Schalter** — halb entfernte
Software ist schlimmer als ein Klick mehr.

Ohne Terminal (also in einem Skript) ist `--yes` zwingend; sonst bricht Plane
ab, statt auf eine Eingabe zu warten, die nie kommt.

### `tweaks` — Windows-Einstellungen

```bash
plane-cli tweaks
```

Ohne Schalter zeigt der Befehl alle Punkte mit ihrem tatsächlichen Zustand
(`Aktiv`, `Inaktiv`, `Teilweise`) und den Hinweisen, warum ein Punkt auf
diesem System wirkungslos oder von einer Richtlinie überlagert ist.

```bash
plane-cli tweaks --on explorer.file_extensions
```

```bash
plane-cli tweaks --off taskbar.hide_search
```

```bash
plane-cli tweaks --revert explorer.file_extensions
```

`--revert` stellt den Zustand wieder her, den Plane **vorgefunden** hat.
`--off` setzt dagegen den Windows-Standard. Das ist nicht dasselbe — im
Zweifel `--revert`.

Der Katalog, jeder Punkt mit seiner Nebenwirkung, und was bewusst fehlt:
[TWEAKS.md](TWEAKS.md).

### `update` — nach einer neueren Version sehen

```bash
plane-cli update
```

Der **einzige** Befehl, der das Netzwerk berührt. Er fragt
`api.github.com` nach der jüngsten Veröffentlichung und nennt die Nummer.
Er lädt nichts herunter, installiert nichts und schickt nichts mit außer dem
`User-Agent` `plane/<version>`, den die GitHub-API verlangt.

Anders als in der Oberfläche gibt es hier keinen Schalter in den
Einstellungen: wer den Befehl eintippt, hat sich bereits entschieden.

Der Exitcode ist auch dann `0`, wenn eine neuere Version vorliegt — „es gibt
ein Update" ist kein Fehler. Skripte sollen `--json` auswerten und auf `newer`
sehen, statt einen Exitcode umzudeuten:

```bash
plane-cli update --json
```

```json
{"current":"0.2.0","latest":"0.3.0","newer":true,"url":"https://github.com/philppplik/plane/releases/latest","published":"2026-09-11"}
```

Schlägt die Abfrage fehl, ist der Exitcode `1` und die Ausgabe nennt den
Grund — im JSON-Fall als maschinenlesbarer Schlüssel:

```json
{"error":"update.error.offline","message":"GitHub ist nicht erreichbar. Besteht eine Internetverbindung?"}
```

### `tui` — Textoberfläche

```bash
plane-cli tui
```

Braucht mindestens 80×20 Zeichen; darunter erscheint ein Hinweis statt eines
kaputten Layouts.

| Taste | Wirkung |
|-------|---------|
| `↑` `↓` / `j` `k` | Navigieren |
| `←` `→` / `h` `l` | Spalte wechseln |
| `Leertaste` | Ziel auswählen (in der Kategoriespalte: ganze Kategorie) |
| `a` / `n` / `r` | Alles / nichts / empfohlene Auswahl |
| `s` | Analysieren |
| `c` | Bereinigen |
| `d` | Trockenlauf umschalten |
| `L` | Sprache umschalten |
| `?` / `F1` | Tastaturhilfe |
| `q` / `Esc` | Beenden (während eines Laufs: abbrechen) |
| `Strg+C` | Beenden bzw. abbrechen |

## Zielschlüssel

Alle 38 Schlüssel, wie `list` sie ausgibt:

**System** — `system.temp.user`, `system.temp.windows`, `system.thumbnails`,
`system.inetcache`, `system.errorreports`, `system.crashdumps`, `system.logs`,
`system.dns`, `system.prefetch`, `system.windowsupdate`,
`system.deliveryoptimization`, `system.fontcache`, `system.recentdocs`,
`system.searchcache`, `system.windowsold`

**Papierkorb** — `recyclebin.all`

**Browser** — `browser.edge.cache`, `browser.chrome.cache`,
`browser.firefox.cache`, `browser.brave.cache`, `browser.opera.cache`,
`browser.vivaldi.cache`, `browser.cookies`, `browser.history`,
`browser.predictor`

**Anwendungen** — `app.discord`, `app.teams`, `app.spotify`, `app.vscode`,
`app.gpushadercache`, `app.steam`, `app.adobe`, `app.java`, `app.office`,
`app.packagemanagers`

**Installationsdateien** — `installers.downloads`

**Registry** — `registry.privacy`, `registry.orphans`

Was jedes Ziel genau anfasst und welche Nebenwirkung es hat, steht in
[REINIGUNGSZIELE.md](REINIGUNGSZIELE.md).

## Exitcodes

| Code | Bedeutung |
|------|-----------|
| `0` | Erfolg |
| `1` | Mindestens ein Ziel ist fehlgeschlagen |
| `2` | Bedienfehler (unbekanntes Ziel, ungültige Kategorie) |
| `130` | Vom Nutzer abgebrochen (Strg+C) |

## JSON-Ausgabe

`--json` schreibt ausschließlich gültiges JSON nach stdout — keine
Fortschrittszeile, keine Farben. Dasselbe gilt automatisch, wenn stdout keine
Konsole ist, damit eine Pipe nicht verunreinigt wird.

```bash
plane-cli scan --json
```

```json
{
  "targets": [
    {
      "key": "system.dns",
      "category": "system",
      "risk": "safe",
      "requires_admin": false,
      "default_enabled": true,
      "suggestion_only": false,
      "item_count": 1,
      "size": 0,
      "items": [],
      "skipped": false
    }
  ],
  "total_size": 0,
  "total_items": 1,
  "duration_ms": 0,
  "cancelled": false
}
```

`clean --json` liefert zusätzlich `success`, `total_freed`, `total_removed`
und — falls die Registry bereinigt wurde — `registry_backup` mit dem Pfad der
Sicherung.

Meldungen wie `skip_reason` und `warnings` sind **Übersetzungsschlüssel**, teils
mit Argument nach einem `|` (etwa `warn.process_running|chrome.exe`). Ein
JSON-Bericht ist dadurch sprachneutral.

## Administratorrechte

Plane fordert **keine** Elevation an. Ziele, die erhöhte Rechte brauchen,
werden gemeldet übersprungen:

```
Übersprungen: Plane läuft ohne Administratorrechte
```

Betroffen sind `system.temp.windows`, `system.logs`, `system.prefetch`,
`system.windowsupdate`, `system.deliveryoptimization`, `system.fontcache` und
`system.windowsold`.

Für diese Ziele ein Terminal als Administrator öffnen und `plane-cli` dort
starten.

**Wichtig:** Bei Dateien, die ein laufendes Programm geöffnet hält, helfen
Administratorrechte **nicht**. Eine exklusiv geöffnete Datei lässt sich auch
als Administrator nicht löschen — dagegen hilft nur, das Programm zu
schließen. Plane unterscheidet die beiden Fälle und sagt jeweils, was hilft.

Auch die Deinstallation maschinenweit installierter Programme braucht erhöhte
Rechte; `plane-cli programs` markiert das je Eintrag.

## Beispiele

Wöchentlich simulieren und das Ergebnis protokollieren:

```bash
plane-cli clean --dry-run --json > plane-%DATE%.json
```

Nur Browser-Zwischenspeicher, ohne Rückfrage:

```bash
plane-cli clean browser.edge.cache browser.chrome.cache browser.firefox.cache -y
```

Prüfen, ob sich eine Bereinigung überhaupt lohnt (Exitcode-Auswertung):

```bash
plane-cli scan --json | python -c "import json,sys; d=json.load(sys.stdin); print(d['total_size'])"
```
