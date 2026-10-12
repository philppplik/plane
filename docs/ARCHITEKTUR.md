# Architektur

Wie Plane aufgebaut ist und warum. Stand: 2026-09-10.

Für das Datenmodell im Detail siehe [DATENSTRUKTUREN.md](DATENSTRUKTUREN.md),
für den Zielkatalog [REINIGUNGSZIELE.md](REINIGUNGSZIELE.md).

## Leitgedanke

> Die Engine kennt keine Oberfläche.

Alles, was Plane tut, passiert in `src-tauri/src/engine/`. Dieses Modul kennt
weder Tauri noch HTML noch ein Terminal. Fortschritt meldet es über einen
Callback, Abbruch empfängt es über ein Token. Dadurch verwenden die grafische
Oberfläche, die Kommandozeile, die TUI und die Tests **denselben** Code – es
gibt keine zweite Wahrheit, die auseinanderlaufen könnte.

Ein Vertragstest hält das fest: `test_engine_kennt_die_oberflaeche_nicht`
schlägt fehl, sobald jemand `tauri::` in ein Engine-Modul schreibt.

## Schichten

```
┌─────────────────────┐   ┌──────────────────────┐
│  GUI (Tauri)        │   │  CLI / TUI           │
│  frontend/ + JS     │   │  src/bin/plane-cli   │
└──────────┬──────────┘   └──────────┬───────────┘
           │ invoke()                │ direkter Aufruf
┌──────────▼──────────┐              │
│  commands.rs        │              │
│  sperren, delegieren│              │
│  Fortschritt senden │              │
└──────────┬──────────┘              │
           │                         │
┌──────────▼─────────────────────────▼───────────┐
│  engine/                                        │
│  ┌──────────┐  ┌────────┐  ┌─────────────────┐ │
│  │ catalog  │→ │ scan   │→ │ clean           │ │
│  │ WAS      │  │ WAS WÄRE│ │ TATSÄCHLICH TUN │ │
│  └──────────┘  └────────┘  └─────────────────┘ │
│  fsutil · process · registry · recyclebin ·     │
│  installers · runtime · types · log · elevation  │
│                                                  │
│  Daneben, ohne Bezug zur Reinigung:              │
│  uninstall · tweaks · icons · update             │
└─────────────────────────────────────────────────┘
           │
┌──────────▼──────────┐   ┌──────────────────────┐
│  state.rs           │   │  i18n.rs             │
│  Einstellungen,JSON │   │  Deutsch + Englisch  │
└─────────────────────┘   └──────────────────────┘
```

Vier Module gehören nicht zum Reinigungsablauf, teilen sich aber dieselbe
Regel „kennt keine Oberfläche":

| Modul | Aufgabe | Besonderheit |
|-------|---------|--------------|
| `uninstall` | Programme auflisten und entfernen | startet fremde Deinstaller, löscht selbst nichts |
| `tweaks` | Windows-Einstellungen | Journal mit dem **vorgefundenen** Zustand |
| `icons` | Programmsymbole aus `.exe`/`.dll`/`.ico` | liefert rohe RGBA-Punkte, kein PNG |
| `update` | Suche nach neuen Versionen | **der einzige Netzwerkzugriff**, standardmäßig aus |

`update` ist die eine Ausnahme vom Offline-Versprechen. Ein Vertragstest
(`test_netzwerkzugriff_gibt_es_nur_an_einer_stelle`) verhindert, dass
anderswo ein zweiter Zugang entsteht.

## Die zwei Phasen

Der wichtigste Entwurfsentscheid. Professionelle Cleaner trennen Analyse und
Bereinigung; Plane erzwingt diese Trennung im Typsystem:

| Phase | Funktion | Rückgabe | Nebenwirkung |
|-------|----------|----------|--------------|
| Analyse | `engine::scan(&[keys], &ctx)` | `ScanReport` | **keine** |
| Bereinigung | `engine::clean(&request, &backup_dir, &ctx)` | `CleanReport` | löscht |

`clean` nimmt eine explizite Zielliste entgegen. Es gibt keinen Aufruf, der
„alles" bereinigt, ohne dass ein Aufrufer die Ziele benannt hat. Eine leere
Liste bereinigt nichts.

`CleanRequest.dry_run` liefert exakt dieselben Zahlen wie ein echter Lauf, ohne
zu löschen – gemessen wird vor dem Löschen, nicht danach.

## Warum der Katalog datengetrieben ist

`catalog.rs` ist die einzige Stelle mit Pfaden. `scan` und `clean` sind
vollständig generisch über `TargetKind`:

```rust
enum TargetKind {
    Files(&'static [FileRule]),   // Glob-Muster mit %VAR%
    RecycleBin,                   // Shell-API
    Command(&'static [&str]),     // externer Befehl, ohne Shell
    Installers,                   // Heuristik über Downloads/Desktop
    Registry(&'static [RegistryRule]),
}
```

Ein neues Reinigungsziel besteht damit aus **einem Eintrag im Katalog plus zwei
Übersetzungen** – kein neuer Code, keine neue Verzweigung. Das ist der Grund,
weshalb der Katalog von 8 auf 38 Ziele wachsen konnte, ohne dass `scan.rs` oder
`clean.rs` länger wurden.

## Fortschritt und Abbruch

```rust
let token = CancelToken::new();
let ctx = RunContext::new()
    .with_cancel(token.clone())
    .with_progress(Box::new(|p: Progress| { /* Event, Zeile, TUI-Balken */ }));
```

- `RunContext` bündelt Abbruchsignal, Fortschrittsempfänger und Rechtestatus.
- Die GUI leitet `Progress` als Tauri-Event `plane://progress` weiter, die CLI
  schreibt eine sich selbst überschreibende Zeile, die TUI zeichnet einen
  Balken. Die Engine weiß von all dem nichts.
- Abbruch wird zwischen den Einzelschritten geprüft. Ein laufender
  Löschvorgang wird nie mittendrin abgeschnitten – der Bericht bleibt
  konsistent.
- `RunContext::with_elevated(bool)` erlaubt Tests, den Rechtestatus
  vorzutäuschen, ohne die Testumgebung zu verändern.

## Nebenläufigkeit

Analyse und Bereinigung sind rechen- und E/A-lastig. Die Tauri-Commands lagern
sie über `spawn_blocking` aus, damit die Oberfläche bedienbar bleibt und der
Abbrechen-Knopf reagiert:

```rust
tauri::async_runtime::spawn_blocking(move || engine::scan(&targets, &ctx)).await
```

Der Zustand liegt hinter einem `Mutex`, der nie mit `unwrap()` gesperrt wird –
eine vergiftete Sperre würde sonst jeden weiteren Aufruf abstürzen lassen.

## Sprache

`i18n.rs` ist eine einzige Tabelle `(Schlüssel, Deutsch, Englisch)`. Der Grund
für Rust statt JSON im Frontend: Zielnamen werden von der Oberfläche **und** von
der Kommandozeile gebraucht. Zwei Sprachdateien wären zwei Wahrheiten.

Das Frontend holt sich den kompletten Satz über `get_translations(lang)`.
Meldungen aus der Engine sind selbst Schlüssel, teils mit Argument:

```
"skip.needs_admin"              → einfacher Schlüssel
"warn.process_running|chrome.exe" → Schlüssel mit Argument
```

Damit bleiben Berichte sprachneutral, bis sie angezeigt werden – ein
JSON-Bericht aus der CLI ist in jeder Sprache derselbe.

## Persistenz

`state.json` im App-Config-Verzeichnis (`%APPDATA%\com.ppaul.plane\`).
Bewusst kein SQLite: für eine Handvoll Einstellungen wäre eine C-Abhängigkeit
im Build unverhältnismäßig. Beim Einführen einer Reinigungs-Historie ist der
Wechsel neu zu bewerten.

Alle Felder tragen `serde(default)`, damit eine ältere Zustandsdatei nach einem
Update lesbar bleibt. Eine beschädigte Datei führt zum Standardzustand, nicht
zum Startabbruch.

## Bauen für zwei Architekturen

Plane wird nativ für `x86_64-pc-windows-msvc` und `aarch64-pc-windows-msvc`
gebaut. Das ist Korrektheit, nicht Kosmetik: ein 32-Bit-Prozess unterliegt der
WOW64-Umleitung und würde 64-Bit-Dateien für fehlend halten – und damit gültige
Registry-Einträge als „verwaist" melden. Details in
[REINIGUNGSZIELE.md](REINIGUNGSZIELE.md#windows-on-arm).

## Verzeichnisüberblick

| Pfad | Inhalt |
|------|--------|
| `src-tauri/src/engine/` | Analyse und Bereinigung. Kein UI-Bezug |
| `src-tauri/src/commands.rs` | Tauri-Brücke. Keine Logik |
| `src-tauri/src/state.rs` | Einstellungen, Persistenz |
| `src-tauri/src/i18n.rs` | Sprachkatalog |
| `src-tauri/src/cli/` | Kommandozeile und TUI |
| `src-tauri/src/bin/` | Einstiegspunkt von `plane-cli` |
| `src-tauri/capabilities/` | Tauri-2-Berechtigungen |
| `frontend/` | Oberfläche (Vanilla JS, Vite) |
| `tests/` | Statische Vertragstests (pytest) |
| `docs/` | Diese Dokumentation |
