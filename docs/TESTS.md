# Testsuite

Anleitung und Konventionen. Stand: 2026-09-11.

## Ausführen

```bash
npm test
```

Einzeln:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

```bash
python -m pytest
```

Erwarteter Zustand: **beide Suiten grün**, ohne übersprungene oder erwartet
fehlschlagende Tests. 244 Rust-Testfälle, 55 Python-Testfälle, zusammen unter
30 Sekunden.

### Voraussetzungen

| Werkzeug | Zweck | Installation |
|----------|-------|--------------|
| Rust stable | Unit-Tests der Engine | https://rustup.rs/ |
| Python 3.11+, `pytest` | Vertragstests | `pip install -r requirements-dev.txt` |
| Node.js 18+ | Frontend-Build im CI | `npm ci` |

## Zwei Arten von Tests

**Rust-Unit-Tests** liegen bei ihrem Code (`#[cfg(test)] mod tests`). Sie
prüfen Verhalten: löscht `remove_entry` wirklich? Wird ein Ziel ohne
Adminrechte übersprungen? Bricht der Scan auf Signal ab?

**Python-Vertragstests** analysieren die Quelldateien statisch. Sie brauchen
weder eine laufende App noch die Rust-Toolchain und fangen genau die Fehler ab,
die sonst erst im fertigen MSI auffallen — ein Command, den das Frontend
aufruft und den es nicht gibt; ein Reinigungsziel ohne Übersetzung; eine
Element-ID, die im Markup fehlt.

```
src-tauri/src/engine/*.rs         Engine: Katalog, Scan, Clean, Registry …
src-tauri/src/cli/*.rs            Kommandozeile: Argumente, Tabellen, TUI-Zustand
src-tauri/src/{i18n,state,commands}.rs

tests/test_backend_contract.py    Commands, Katalog, Tauri-Konfiguration, Hygiene
tests/test_frontend_contract.py   Markup, JavaScript, Stylesheets
```

## Sicherheitsnetze

Die Testsuite eines Löschwerkzeugs darf nicht versehentlich löschen. Dafür
gelten zwei Regeln, die für jeden neuen Test verbindlich sind:

1. **Tests, die wirklich löschen, tun das ausschließlich in einem eigenen
   Ordner** unter `std::env::temp_dir()` und räumen hinterher auf. Kein Test
   fasst einen echten Reinigungspfad an.
2. **Jeder Test, der den gesamten Katalog berührt, läuft im Trockenlauf**
   (`dry_run: true`). Der Trockenlauf meldet exakt dieselben Zahlen wie ein
   echter Lauf, verändert aber nichts — genau dafür gibt es ihn.

Die Python-Vertragstests lesen ausschließlich Dateien; sie starten keine
Prozesse und löschen nichts.

## Was die Vertragstests abdecken

**Backend** (`test_backend_contract.py`)

- Jeder `#[tauri::command]` ist in `lib.rs` registriert — und umgekehrt
- Die Kernfunktionen (`scan`, `clean`, `cancel_run`, …) existieren
- Die Engine verwendet kein Tauri (sonst wären CLI und Tests nicht möglich)
- Jedes Reinigungsziel hat Name und Beschreibung in beiden Sprachen
- Keine Übersetzung ohne zugehöriges Ziel
- Kein Pfadmuster mit festem Laufwerksbuchstaben
- Die Registry-Regeln fassen die gefährlichen Bereiche nicht an
  (`CLSID`, `TypeLib`, `CurrentControlSet`)
- Vor dem Löschen von Registry-Einträgen wird gesichert
- CSP gesetzt, Capabilities vorhanden und auf ein existierendes Fenster bezogen
- `beforeBuildCommand` gesetzt, Builddatum nicht hart codiert
- Kein `lock().unwrap()`, kein handgebautes JSON
- `LICENSE`, `.gitignore`, gepinnte Abhängigkeiten, Doku-Verweise

**Frontend** (`test_frontend_contract.py`)

- Jeder aufgerufene Command existiert im Backend
- `invoke` läuft ausschließlich über `api.js`
- Tauri-2-Importpfad, kein Inline-Handler, kein `innerHTML`
- Jede im JavaScript referenzierte Element-ID existiert im Markup
- Fortschritts- und Navigationsereignisse werden abonniert
- Jeder verwendete Übersetzungsschlüssel existiert im Katalog
- Keine hartkodierten sichtbaren Texte im Markup
- Echte Umlaute statt „ue/oe/ae"
- Mindestens zwei Breakpoints, Dunkelmodus, `prefers-reduced-motion`,
  sichtbarer Fokusring

## Konventionen

**Testnamen sind deutsche Sätze**, die die geprüfte Eigenschaft benennen:
`uebersprungene_schritte_sind_kein_fehler`,
`riskante_ziele_sind_nie_vorausgewaehlt`. Der Name soll erklären, *warum* es
die Regel gibt.

**Jeder behobene Fehler bekommt einen Test**, der ohne die Korrektur
fehlschlägt.

**Tests prüfen das Sollverhalten.** Frühere Fassungen hielten bekannte Defekte
über `xfail(strict=True)` fest; dieses Muster gibt es nicht mehr, weil die
Defekte behoben sind. Es ist in
[BEKANNTE_MAENGEL.md](BEKANNTE_MAENGEL.md) dokumentiert, falls es wieder
gebraucht wird.

## Nicht abgedeckte Bereiche

Bewusste Lücken, damit niemand falsche Sicherheit ableitet:

| Bereich | Grund |
|---------|-------|
| Löschwirkung auf echten Systempfaden | Nicht ohne Wegwerf-VM testbar. Getestet ist der Mechanismus in `temp_dir()`, nicht das Verhalten auf `%WINDIR%` |
| Tauri-Commands mit `Window`/`State` | Benötigen eine laufende Tauri-Instanz. Getestet sind Datenstrukturen und zustandslose Commands |
| Klick-Durchlauf der Oberfläche | Kein Browser-Automationssetup. Das Markup wird nur statisch gegen das JavaScript geprüft |
| TUI-Darstellung | Getestet ist die Zustandsmaschine (Navigation, Auswahl, Modus), nicht das gezeichnete Bild |
| Verhalten **mit** Administratorrechten | Erfordert eine zweite, elevierte Testumgebung. Der Pfad ohne Rechte ist abgedeckt |
| Dienststeuerung (`wuauserv`, `FontCache`) | Wird gemockt; ein echter Dienststopp im Test wäre ein Eingriff ins Testsystem |
| Registry-**Schreib**zugriffe | Nur der Trockenlauf wird getestet. Die Sicherungs- und Löschpfade sind durch die Sperrlisten abgesichert, aber nicht durch Tests |
| MSI-Build und Installation | Wird im Release-Workflow gebaut, aber nicht automatisch geprüft |
| Windows 10 und ARM64 | Entwickelt und getestet auf Windows 11 ARM64; x64 nur über CI |

## CI

`.github/workflows/ci.yml` führt bei jedem Push und Pull Request auf `main`
drei Jobs auf Windows-Runnern aus:

| Job | Inhalt |
|-----|--------|
| `rust` | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test` |
| `python` | `pip install -r requirements-dev.txt`, `pytest` |
| `frontend` | `npm ci`, `npm run build` |

Formatierung und Clippy laufen scharf — Warnungen lassen den Lauf scheitern.
