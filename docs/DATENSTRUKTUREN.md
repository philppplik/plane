# Datenstrukturen

Referenz des Datenmodells. Stand: 2026-09-10.

Architekturüberblick: [ARCHITEKTUR.md](ARCHITEKTUR.md).
Zielkatalog und Begründungen: [REINIGUNGSZIELE.md](REINIGUNGSZIELE.md).

---

## 1. Engine (`src-tauri/src/engine/types.rs`)

### `Category`

Oberkategorie eines Ziels, bestimmt die Gruppierung in Oberfläche und CLI.
Serialisiert kleingeschrieben.

`System` · `Browsers` · `Apps` · `Installers` · `RecycleBin` · `Registry`

`Category::i18n_key()` liefert `category.<schlüssel>`.

### `Risk`

Risikostufe für Anzeige und Vorauswahl. Geordnet: `Safe < Notice < Caution`.

| Stufe | Bedeutung |
|-------|-----------|
| `Safe` | Reiner Cache, wird bei Bedarf neu erzeugt |
| `Notice` | Spürbare Nebenwirkung (Abmeldung, Verlauf weg, längere Startzeit) |
| `Caution` | Datenverlust möglich. **Nie vorausgewählt** |

### `Target`

Ein Reinigungsziel. Enthält **keine** Texte – Name und Beschreibung kommen über
`i18n_name()` / `i18n_description()` aus dem Sprachkatalog.

| Feld | Typ | Bedeutung |
|------|-----|-----------|
| `key` | `&'static str` | Stabiler Identifier, kleingeschrieben, gegliedert (`browser.chrome.cache`) |
| `category` | `Category` | Gruppierung |
| `kind` | `TargetKind` | Was zu tun ist |
| `risk` | `Risk` | Risikostufe |
| `requires_admin` | `bool` | Ohne Elevation nicht durchführbar |
| `default_enabled` | `bool` | Bei der Standardauswahl vorausgewählt |
| `services` | `&[&str]` | Dienste, die vorher gestoppt und danach gestartet werden |
| `blocking_processes` | `&[&str]` | Programme, die die Dateien sperren – es wird gewarnt |

Abgeleitet: `is_safe()` = `risk == Safe && !requires_admin`,
`is_suggestion_only()` = Ziel wird nur vorgeschlagen, nie automatisch bereinigt.

### `TargetKind`

```rust
Files(&[FileRule])        // Glob-Muster
RecycleBin                // Shell-API über alle Laufwerke
Command(&[&str])          // externer Befehl als Argumentliste, ohne Shell
Installers                // Heuristik über Downloads/Desktop
Registry(&[RegistryRule]) // Verwaisungsprüfung
```

### `FileRule`

| Feld | Typ | Bedeutung |
|------|-----|-----------|
| `pattern` | `&'static str` | Glob mit `%VAR%`. `*` je Segment, `**` rekursiv |
| `min_age_days` | `u32` | Nur ältere Einträge (0 = egal) |
| `exclude` | `&[&str]` | Teilzeichenketten, die einen Treffer ausschließen |

Aufbau verkettet: `FileRule::new("%TEMP%/*").older_than(7).excluding(&["\\Temp\\Low"])`

Neben den Windows-Umgebungsvariablen kennt Plane die Kürzel `%DOWNLOADS%`,
`%DESKTOP%` und `%DOCUMENTS%`. Diese werden über die Shell-Ordnerdefinition
aufgelöst – `%USERPROFILE%\Downloads` wäre falsch, sobald OneDrive umleitet.

### `RegistryRule` und `RegistryCheck`

| Feld | Bedeutung |
|------|-----------|
| `hive` | `"HKCU"` \| `"HKLM"` \| `"HKCR"` |
| `path` | Unterschlüssel |
| `check` | Art der Verwaisungsprüfung |

```rust
ValueNameIsPath              // Wertname ist ein Dateipfad (MUICache, SharedDLLs)
ValueIsPath { value }        // Wert enthält einen Pfad (Autostart)
SubkeyProgId                 // Standardwert eines Unterschlüssels (App Paths)
UninstallEntry               // Uninstall-Eintrag mit fehlendem Programm
```

---

## 2. Analyseergebnis

### `ScanItem`

| Feld | Typ | Bedeutung |
|------|-----|-----------|
| `path` | `String` | Dateipfad oder Registry-Schlüssel |
| `size` | `u64` | Bytes. Registry-Treffer zählen 0 |
| `detail` | `String` | Zusatz für die Anzeige (z. B. `installer\|214` = Art und Alter in Tagen) |
| `value_name` | `Option<String>` | Registry-Wertname, falls ein Wert statt eines Schlüssels entfernt wird |

### `TargetScan`

| Feld | Typ | Bedeutung |
|------|-----|-----------|
| `key`, `category`, `risk`, `requires_admin`, `default_enabled`, `suggestion_only` | | Spiegel der Katalogdaten, damit die UI nicht nachschlagen muss |
| `item_count` | `usize` | **Vollständige** Trefferzahl |
| `size` | `u64` | **Vollständige** Summe in Bytes |
| `items` | `Vec<ScanItem>` | Einzeltreffer, begrenzt auf `MAX_ITEMS_PER_TARGET` (200) |
| `skipped` | `bool` | Konnte nicht analysiert werden |
| `skip_reason` | `String` | Übersetzungsschlüssel |
| `warnings` | `Vec<String>` | Übersetzungsschlüssel, blockieren nicht |

Die Begrenzung von `items` ist bewusst: ein Browser-Cache hat schnell 50 000
Dateien. `item_count` und `size` bleiben vollständig, nur die Einzelliste ist
gekappt.

### `ScanReport`

`targets` · `total_size` · `total_items` · `duration_ms` · `cancelled`

---

## 3. Bereinigung

### `CleanRequest`

| Feld | Typ | Bedeutung |
|------|-----|-----------|
| `targets` | `Vec<String>` | Zielschlüssel. **Leer = nichts** |
| `only_paths` | `Vec<String>` | Nur diese Pfade (leer = alle Treffer des Ziels) |
| `dry_run` | `bool` | Nur berichten, nichts löschen |

`only_paths` ist der Mechanismus für die Einzelauswahl bei Installationsdateien.
Für `TargetKind::Installers` ist eine leere Liste ein Abbruchgrund – dieses Ziel
löscht nie pauschal.

### `TargetClean` und `CleanReport`

| Feld | Bedeutung |
|------|-----------|
| `ok` | `true` nur, wenn kein Fehler auftrat |
| `skipped` / `skip_reason` | Bewusst nicht ausgeführt – **kein** Fehlschlag |
| `freed` | Tatsächlich freigegebene Bytes, vor dem Löschen gemessen |
| `removed_items` | Anzahl entfernter Einträge |
| `errors` | Fehlermeldungen, teils als Übersetzungsschlüssel |

`CleanReport` ergänzt: `success` · `total_freed` · `total_removed` ·
`duration_ms` · `cancelled` · `registry_backup` (Pfad der `.reg`-Sicherung) ·
`error`.

`success` ist `false`, sobald ein Ziel **fehlgeschlagen** ist. Übersprungene
Ziele zählen ausdrücklich nicht als Fehler.

### `Progress`

Wird während beider Phasen laufend gemeldet.

| Feld | Bedeutung |
|------|-----------|
| `phase` | `"scan"` oder `"clean"` |
| `target_key` / `target_i18n` | Aktuelles Ziel und sein Übersetzungsschlüssel |
| `index` / `total` | Position im Lauf |
| `percent` | 0–100 über den **gesamten** Lauf |
| `bytes` | Bisher gefunden bzw. freigegeben |
| `current_path` | Aktuell bearbeiteter Pfad (Detailzeile) |
| `done` | Ziel abgeschlossen |

In der GUI kommt das als Tauri-Event `plane://progress` an.

---

## 4. Laufzeitkontext (`engine/runtime.rs`)

| Typ | Zweck |
|-----|-------|
| `CancelToken` | Teilbares Abbruchsignal (`Arc<AtomicBool>`). `cancel()`, `reset()`, `is_cancelled()` |
| `ProgressSink` | `Box<dyn Fn(Progress) + Send + Sync>` |
| `RunContext` | Bündelt Abbruch, Fortschritt und Rechtestatus |

`RunContext::with_elevated(bool)` erlaubt Tests, den Rechtestatus
vorzutäuschen.

---

## 5. Zustand (`src-tauri/src/state.rs`)

### `Settings`

| Feld | Typ | Standard | Bedeutung |
|------|-----|----------|-----------|
| `language` | `String` | Systemsprache, sonst `en` | `de` oder `en` |
| `theme` | `Theme` | `System` | `System` \| `Light` \| `Dark` |
| `confirm_risky` | `bool` | **`true`** | Vor `Caution`-Zielen nachfragen |
| `dry_run_default` | `bool` | `false` | Läufe standardmäßig simulieren |
| `selected_targets` | `Vec<String>` | leer | Zuletzt gewählte Ziele |

### `AppState`

| Feld | Persistiert | Bedeutung |
|------|-------------|-----------|
| `has_seen_welcome` | ✔ | Willkommensbildschirm nur beim ersten Start |
| `settings` | ✔ | siehe oben |
| `current_screen` | ✘ | Reiner Laufzeitzustand |

Ablage: `%APPDATA%\com.ppaul.plane\state.json`, Registry-Sicherungen unter
`backups/`. Alle Felder mit `serde(default)`; eine beschädigte Datei führt zum
Standardzustand, nicht zum Startabbruch.

### Zustandsübergänge

```
   [Start] ──get_start_screen()──┐
                                 │
   has_seen_welcome == false ──► welcome ──start_app()──┐
   has_seen_welcome == true  ───────────────────────────┤
                                                        ▼
                                     about ◄──show_about()── home
                                           ──show_home()───►
```

`welcome` ist nach dem ersten `start_app()` nur über die Einstellung
„Willkommensbildschirm erneut zeigen" erreichbar.

---

## 6. Tauri-Commands (`src-tauri/src/commands.rs`)

| Command | Argumente | Rückgabe |
|---------|-----------|----------|
| `get_start_screen` | – | `String` |
| `start_app` / `show_home` / `show_about` | – | `String` |
| `get_settings` | – | `Settings` |
| `set_settings` | `settings` | `Settings` |
| `reset_welcome` | – | – |
| `get_languages` | – | `[{code, label}]` |
| `get_translations` | `language` | `{ schlüssel: text }` |
| `get_brand` | – | `BrandInfo` |
| `list_targets` | – | `TargetInfo[]` |
| `get_disk_stats` | – | `DiskStats` |
| `get_system_info` | – | `SystemInfo` |
| `scan` | `targets: string[]` | `ScanReport` |
| `clean` | `request: CleanRequest` | `CleanReport` |
| `cancel_run` | – | – |

Alle geben `Result<T, String>` zurück. `scan` und `clean` laufen über
`spawn_blocking`; währenddessen kommen `plane://progress`-Events.

### Antworttypen

`BrandInfo` — `name`, `version` (aus `CARGO_PKG_VERSION`), `build_date` (aus
`PLANE_BUILD_DATE`, gesetzt von `build.rs`), `developer`, `colors`.

`DiskStats` — `drive`, `total_gb`, `free_gb`, `used_percent`. Der Prozentwert
wird **berechnet**, nie separat gepflegt; Division durch null ist abgefangen.

`SystemInfo` — `os` (mit Buildnummer, Windows 11 korrekt erkannt), `cpu`,
`arch`, `ram_gb`, `is_admin`, `disk`.

`TargetInfo` — `key`, `category`, `name_key`, `description_key`, `risk`,
`requires_admin`, `default_enabled`, `suggestion_only`. Die Oberfläche bekommt
**Schlüssel**, keine Texte – so wechselt die Sprache ohne Neuladen der Ziele.

---

## 7. Sprachkatalog (`src-tauri/src/i18n.rs`)

Eine Tabelle `(Schlüssel, Deutsch, Englisch)`.

| Funktion | Zweck |
|----------|-------|
| `t(lang, key)` | Text nachschlagen. Unbekannte Schlüssel geben sich selbst zurück |
| `format(lang, key, &[args])` | Platzhalter `{0}`, `{1}`, … füllen |
| `catalog(lang)` | Alle Texte einer Sprache – für das Frontend |
| `normalize(lang)` | `de-DE` → `de`, Unbekanntes → `en` |

Schlüsselschema:

```
app.*  nav.*  home.*  disk.*  settings.*  about.*  confirm.*  cli.*
category.<name>
risk.<stufe>[.hint]
status.*  skip.*  warn.*  error.*  clean.*
target.<zielschlüssel>.name
target.<zielschlüssel>.description
```

Meldungen aus der Engine sind selbst Schlüssel, optional mit Argument nach
einem `|`:

```
"skip.needs_admin"                → t(lang, "skip.needs_admin")
"warn.process_running|chrome.exe" → format(lang, "warn.process_running", &["chrome.exe"])
```

Damit bleibt ein Bericht sprachneutral, bis er angezeigt wird.

---

## 8. Konventionen für Erweiterungen

1. **Neues Reinigungsziel:** Eintrag in `catalog.rs` plus zwei Übersetzungen in
   `i18n.rs`. Kein neuer Code. Schritt-für-Schritt in
   [CONTRIBUTING.md](../CONTRIBUTING.md).
2. **Neuer Command:** in `commands.rs` definieren *und* in `lib.rs`
   registrieren – ein Vertragstest prüft beide Richtungen.
3. **Neues Feld in einer Antwort:** typisiert am Struct ergänzen, nie als freies
   JSON.
4. **Kein Text im Katalog:** Namen und Beschreibungen gehören nach `i18n.rs`.
   Ein Test findet jedes Ziel ohne Übersetzung.
5. **Keine Pfade außerhalb des Katalogs.** `scan` und `clean` bleiben generisch.
