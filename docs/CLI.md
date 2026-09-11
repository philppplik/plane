# Kommandozeile und Textoberfläche

`plane-cli` kann alles, was die grafische Oberfläche kann — sie nutzt dieselbe
Engine. Stand: 2026-09-11.

## Installation

```bat
scripts\install-cli.bat
```

Kopiert `plane-cli.exe` nach `%LOCALAPPDATA%\Programs\Plane` und nimmt den
Ordner in den Benutzer-PATH auf. **Keine Administratorrechte nötig** — es wird
nur ins eigene Profil geschrieben. Fehlt das Binary, baut das Skript es vorher
(braucht dann [Rust](https://rustup.rs/)).

Danach ein **neues** Terminal öffnen; PATH-Änderungen gelten nicht rückwirkend.

Entfernen:

```bat
scripts\uninstall-cli.bat
```

Ohne Installation lässt sich die CLI auch direkt aufrufen:

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
  info    Systeminformationen, Rechtestatus und Laufwerksbelegung
  tui     Die textbasierte Oberfläche starten

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
