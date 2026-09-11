# Mängelregister

Stand: 2026-09-10, Version 0.1.0.

Am 2026-09-10 wurden 45 Defekte erfasst und **alle behoben**. Dieses
Dokument bleibt als Nachweis erhalten: es zeigt, was falsch war, wie es gelöst
wurde und welcher Test die Lösung absichert. Damit ist nachvollziehbar, warum
der Code an einigen Stellen aufwendiger ist, als er auf den ersten Blick sein
müsste.

| Status | Anzahl |
|--------|--------|
| ✅ Behoben | 45 |
| ⏸ Offen | 0 |

Die Testsuite ist grün und enthält **keine** `xfail`-Marker oder
Characterization-Tests mehr — jeder Test prüft jetzt das Sollverhalten.
Konventionen: [TESTS.md](TESTS.md). Datenmodelle: [DATENSTRUKTUREN.md](DATENSTRUKTUREN.md).

---

## Behoben — Reinigungskern

> Die Lösungsspalte beschreibt den Stand zum Zeitpunkt der Behebung. Der
> damalige Python-Kern wurde später vollständig nach Rust portiert und
> entfernt; die Korrekturen sind dort unverändert enthalten.

| ID | Mangel | Lösung |
|----|--------|--------|
| 01 | Modulschlüssel aus dem ersten Wort des Namens ⇒ alle kollidierten zu `"clear"`, selektive Bereinigung unbenutzbar | Neues Feld `CleanStep.key`; Register `STEPS_BY_KEY` mit Kollisionsprüfung beim Import |
| 15 | `"clear"` bildete stillschweigend auf den zuletzt definierten Schritt ab | entfällt mit 01; unbekannte Schlüssel werden gemeldet |
| 18 | Kein stabiler Modul-Identifier — Umbenennung im UI brach die Auflösung | `key` ist vom Anzeigenamen entkoppelt |
| 04 | „Clear Recycle Bin" löschte unwiederbringlich, war aber `safe=True` | Zwei Risikoachsen `requires_admin`/`irreversible`; `safe` daraus abgeleitet. Papierkorb ist `irreversible` |
| 02 | Nie ersetzter Platzhalter `[App]` im Befehl | Schritt `appcache` nutzt konkrete Pfade (`INetCache`, `CrashDumps`) |
| 03 | `%VAR%` in PowerShell-Befehlen wird dort nicht expandiert | Pfade werden nicht mehr an eine Shell übergeben; `expand()` löst sie in Python auf |
| 05 | Update-Cache wurde bei laufendem Dienst gelöscht | Feld `services`; `wuauserv`+`bits` werden gestoppt und danach gestartet. Scheitert der Stopp, wird gar nicht gelöscht |
| 08 | Freigabe kam aus `sys.argv` — als Bibliothek nicht steuerbar | `clean_operation(step, *, yolo=False)`; `sys.argv` nur noch in `main()` |
| 14 | `clean_all(yolo=…)` wirkungslos | Parameter wird durchgereicht |
| 09 | Ergebnis-Dict mit drei verschiedenen Schemata | Dataclass `StepResult` — alle Felder auf jedem Pfad gesetzt |
| 10 | `ok=True` bedeutete nur „Prozess gestartet" (`2>nul`, `del` liefert immer 0) | Löschen erfolgt in Python mit echter Fehlerbehandlung; `ok` = keine Fehler |
| 11 | `success` hart auf `True` | `_build_result()` leitet `success` aus den Schritten ab |
| 12 | `total_freed_mb` zählte Schritte statt Megabyte | Bytes werden je gelöschter Datei gemessen; bei externen Befehlen über die Differenz des freien Speichers |
| 13 | Übersprungene Schritte galten als Fehler ⇒ jeder Lauf meldete „Some steps failed" | `_build_result()` wertet nur `not ok and not skipped` als Fehlschlag |
| 16 | `clean_specific()` meldete immer `success=True` | unbekannte Module setzen `success=False` |
| 17 | `clean_specific()` befüllte `CleanResult` inkonsistent | beide Einstiegspunkte nutzen `_build_result()` |
| 06 | `CREATE_NO_WINDOW` wurde plattformunabhängig gesetzt | nur noch unter `sys.platform == "win32"` |
| 07 | `except Exception` verschluckte Programmierfehler | nur `OSError` und `subprocess.SubprocessError` |
| 19 | Windows 11 wurde als „Windows 10" angezeigt | `windows_edition()` wertet die Buildnummer aus (≥ 22000 ⇒ 11) |
| 21 | Systemlaufwerk `C:\` hart verdrahtet | `system_drive()` liest `%SystemDrive%` |
| 20 | `psutil` nirgends deklariert | `requirements.txt` und `requirements-dev.txt`, beide mit gepinnten Versionen |
| 33 | Zwei abweichende Timeout-Defaults (60 / 120) | eine Konstante `DEFAULT_TIMEOUT = 120` |
| 38 | Keine Prüfung auf Administratorrechte | `is_admin()`; adminpflichtige Schritte werden gemeldet übersprungen statt still zu scheitern |

**Zusätzlich, über das Register hinaus:** Schutz gegen Muster, die auf eine
Laufwerkswurzel zeigen (`_path_is_allowed`), Ablehnung nicht auflösbarer
Umgebungsvariablen statt literalem Löschen, `CleanStep` ist unveränderlich
(`frozen`), und die CLI hat eine echte `argparse`-Schnittstelle
(`--modules`, `--list`, `--info`, `--verbose`) mit Exitcode 1 bei Fehlschlag.

## Behoben — Rust-Backend (`src-tauri/`)

| ID | Mangel | Lösung |
|----|--------|--------|
| 27 | `clean_all` war ein Stub, der Kern war nicht angebunden | Reinigungs-Engine nach Rust portiert (`src/engine/`), Command ruft sie über `spawn_blocking` auf |
| 28 | `get_disk_stats` lieferte fest 256/200 GB | echte Werte über `sysinfo`; `used_percent` wird berechnet |
| 31 | Antwortschema wich vom Python-`CleanResult` ab | identische Feldnamen, Vertragstest über beide Fassungen |
| 32 | `clean_all` gab handgebautes JSON als `String` zurück | typisiertes `CleanResult` |
| 26 | Tauri-2-Capabilities fehlten vollständig | `capabilities/default.json` mit `core:default`, auf Fenster `main` begrenzt |
| 30 | `build_date` hart codiert | `build.rs` setzt `PLANE_BUILD_DATE` |
| 34 | `lock().unwrap()` — vergifteter Mutex hätte die App dauerhaft blockiert | Hilfsfunktion `lock()` mit `map_err` |
| 35 | Modul in zwei Crates ⇒ Tests liefen doppelt | Logik in `lib.rs` mit `pub fn run()`, `main.rs` ruft nur `plane_lib::run()` |
| 36 | CSP deaktiviert (`"csp": null`) | restriktive Policy mit `script-src 'self'` |
| 37 | `beforeBuildCommand` leer ⇒ `tauri build` bündelte veraltetes Frontend | `npm run build` bzw. `npm run dev` eingetragen |
| 40 | `$schema` zeigte auf die Tauri-1-Domain | `https://schema.tauri.app/config/2` |
| 42 | `clean_specific`/`get_system_info` waren nicht exponiert | als Commands verfügbar und im Dashboard genutzt |

**Zusätzlich:** `AppState.current_screen` ist ein `Screen`-Enum statt eines
Strings, Fenster hat ein `label`, Lizenz in `Cargo.toml`/`bundle`, Buildlauf
ist warnungsfrei (`crate-type = ["rlib"]`, eigener Lib-Name gegen die
PDB-Kollision).

## Behoben — Frontend (`frontend/`)

| ID | Mangel | Lösung |
|----|--------|--------|
| 22 | Tauri-1-Importpfad `@tauri-apps/api/tauri` ⇒ `invoke` zur Laufzeit nicht verfügbar | `@tauri-apps/api/core` |
| 23 | `plane` war für die Inline-`onclick` nicht erreichbar ⇒ nichts reagierte | keine Inline-Handler mehr, alles über `addEventListener` in `main.js` |
| 24 | Unverarbeitete EJS-Syntax `<% … %>` vor dem Doctype | entfernt |
| 25 | `#home-screen` existierte nicht ⇒ kein Dashboard | Dashboard gebaut: Seitenleiste mit Laufwerksbelegung, Hero mit „Alles bereinigen", Modulraster, Ergebnisliste |
| 39 | Welcome-Screen doppelt in `index.html` und `welcome.html` | Single-Page-Anwendung; `welcome.html` und `about.html` entfernt, ihr Inhalt lebt in `index.html` |
| 29 | README/`about.html` versprachen SQLite, pyo3, ARM64, „React-like UI", Cross-Platform | Info-Ansicht nennt die tatsächlich verwendeten Bibliotheken; Zustand wird als JSON persistiert (siehe unten) |

**Zusätzlich:** Bestätigungsdialog vor unsicheren Schritten (nichts
Unwiderrufliches ohne ausdrückliche Zustimmung), Risikohinweise auf den
Modulkarten, Backend-Events werden abonniert statt ignoriert, Fehler erscheinen
sichtbar in der Oberfläche statt nur in der Konsole, Tastaturbedienbarkeit
(`role`, `tabindex`, Enter/Space), `@font-face` für die mitgelieferten
Schriften — vorher fielen alle Texte auf die Systemschrift zurück.

## Behoben — Projekt und Dokumentation

| ID | Mangel | Lösung |
|----|--------|--------|
| 29 | Doku beschrieb ein anderes Produkt als der Code | README auf den Ist-Stand gebracht; Persistenz tatsächlich implementiert |
| 41 | README nannte eine nicht existierende Funktion `welcome()` | Command-Tabelle mit den echten Commands |
| 43 | `LICENSE` fehlte, obwohl die README darauf verwies | MIT-Lizenz ergänzt, auch in `Cargo.toml` und `bundle` |
| 45 | Keine `.gitignore`, `target/`/`node_modules/` im Projekt | `.gitignore` für Node, Rust, Python, Editor |

---

## Bewusst nicht geändert

| Punkt | Begründung |
|-------|------------|
| SQLite statt JSON für den Zustand | Für zwei Zustandsfelder wäre eine C-Abhängigkeit unverhältnismäßig. Beim Einführen einer Reinigungs-Historie neu bewerten. |
| Keine Elevation über ein Manifest | Plane läuft bewusst ohne Adminrechte und meldet, welche Schritte deshalb übersprungen wurden. Ein dauerhaft erhöhter Cleaner ist die größere Angriffsfläche. |
| `freed_mb` bei externen Befehlen ist eine Schätzung | `Clear-RecycleBin` und `ipconfig` melden keine Bytes; die Differenz des freien Speichers ist die beste verfügbare Näherung und als solche dokumentiert. |
