# Mitwirken an Plane

Danke, dass du beitragen möchtest. Dieses Dokument erklärt, wie du das Projekt
zum Laufen bringst und worauf bei Änderungen zu achten ist.

> **Projektsprache ist Deutsch.** Kommentare, Testnamen und Dokumentation sind
> auf Deutsch, mit echten Umlauten (ö, ä, ü, ß) — nicht „oe/ae/ue". Englisch
> ist die zweite **Anzeigesprache** der Anwendung, nicht die Sprache des Codes.

## Inhalt

- [Entwicklungsumgebung](#entwicklungsumgebung)
- [Projektstruktur](#projektstruktur)
- [Tests](#tests)
- [Ein neues Reinigungsziel hinzufügen](#ein-neues-reinigungsziel-hinzufügen)
- [Übersetzungen](#übersetzungen)
- [Regeln für Code](#regeln-für-code)
- [Commits und Pull Requests](#commits-und-pull-requests)

## Entwicklungsumgebung

| Werkzeug | Version | Wofür |
|----------|---------|-------|
| [Rust](https://rustup.rs/) | stable | Engine, Backend, CLI/TUI |
| [Node.js](https://nodejs.org/) | 18+ | Frontend-Build (Vite) |
| [Python](https://www.python.org/) | 3.11+ | Vertragstests |
| Visual Studio Build Tools | aktuell | Linker für Windows |

```bash
npm install
pip install -r requirements-dev.txt
npm run tauri:dev
```

Die Kommandozeile brauchst du dafür nicht zu bauen — sie entsteht mit:

```bash
cargo run --manifest-path src-tauri/Cargo.toml --bin plane-cli -- scan
```

## Projektstruktur

Der Überblick steht in [docs/ARCHITEKTUR.md](docs/ARCHITEKTUR.md), das
Datenmodell in [docs/DATENSTRUKTUREN.md](docs/DATENSTRUKTUREN.md).

Das Wichtigste in einem Satz: **Die Engine kennt keine Oberfläche.** Alles unter
`src-tauri/src/engine/` läuft ohne Tauri, ohne HTML und ohne Terminal.
Grafische Oberfläche, Kommandozeile und Tests verwenden denselben Code.

```
src-tauri/src/engine/catalog.rs   ← WAS bereinigt wird (einziger Ort mit Pfaden)
src-tauri/src/engine/scan.rs      ← WAS WÄRE löschbar (nebenwirkungsfrei)
src-tauri/src/engine/clean.rs     ← LÖSCHEN, was ausgewählt wurde
src-tauri/src/i18n.rs             ← alle Texte, Deutsch und Englisch
src-tauri/src/commands.rs         ← Tauri-Brücke, keine Logik
src-tauri/src/cli/                ← Kommandozeile und TUI
frontend/                         ← Oberfläche (Vanilla JS, Vite)
```

## Tests

```bash
npm test
```

Das führt beide Suiten aus:

```bash
cargo test --manifest-path src-tauri/Cargo.toml   # Rust-Unit-Tests
python -m pytest                                   # statische Vertragstests
```

Beide müssen grün sein. Details und die bewusst nicht abgedeckten Bereiche
stehen in [docs/TESTS.md](docs/TESTS.md).

**Zwei Sicherheitsnetze in den Tests bitte nicht entfernen:** `conftest.py`
blockiert echte Prozessstarts und echte Löschvorgänge. Ein Test, der
versehentlich eine Bereinigung auslöst, schlägt dadurch fehl, statt Dateien auf
deinem Rechner zu löschen.

## Ein neues Reinigungsziel hinzufügen

Das ist der häufigste Beitrag — und dank des datengetriebenen Katalogs braucht
er **keinen neuen Code**.

### 1. Eintrag in `src-tauri/src/engine/catalog.rs`

```rust
const SLACK_CACHE: &[FileRule] = &[
    FileRule::new("%APPDATA%/Slack/Cache/*"),
    FileRule::new("%APPDATA%/Slack/Code Cache/**/*"),
    FileRule::new("%APPDATA%/Slack/logs/*").older_than(7),
];
```

und in `TARGETS`:

```rust
blocked_by(
    files("app.slack", Category::Apps, Risk::Safe, true, SLACK_CACHE),
    &["slack.exe"],
),
```

### 2. Risiko richtig einstufen

| Stufe | Wann | Vorausgewählt |
|-------|------|---------------|
| `Risk::Safe` | Der Inhalt wird **garantiert** automatisch neu erzeugt | ja, wenn sinnvoll |
| `Risk::Notice` | Spürbare Nebenwirkung: Abmeldung, Verlauf weg, langsamerer Start | nein |
| `Risk::Caution` | Datenverlust möglich | **niemals** |

Im Zweifel die höhere Stufe. Ein Test erzwingt, dass `Caution`-Ziele nie
vorausgewählt sind.

Zusätzlich, unabhängig von der Stufe:

- `requires_admin: true`, wenn das Ziel in einem Systemverzeichnis liegt
  (`%WINDIR%`, `%PROGRAMDATA%`). Ohne Elevation wird das Ziel dann gemeldet
  übersprungen statt still zu scheitern.
- `services`, wenn ein Dienst die Dateien hält. Scheitert der Dienststopp, wird
  gar nicht erst gelöscht — ein halb geleerter Cache ist schlimmer als ein
  voller. Dienste zu steuern setzt `requires_admin` voraus.
- `blocking_processes`, wenn ein laufendes Programm die Dateien sperrt. Plane
  warnt dann, statt weniger zu löschen als angekündigt.

### 3. Pfadregeln

- **Immer `%VAR%`**, nie ein fester Laufwerksbuchstabe. Ein Test prüft das.
- `%DOWNLOADS%`, `%DESKTOP%`, `%DOCUMENTS%` statt `%USERPROFILE%\Downloads` —
  sonst greift die Suche ins Leere, sobald OneDrive die Ordner umgeleitet hat.
- `*` gilt je Pfadsegment, `**` steigt rekursiv ab. Chromium-Profile heißen
  `Default`, `Profile 1`, … — also `User Data/*/Cache`.
- `.older_than(n)` für Protokolle und alles, was gerade entstanden sein könnte.
- `.excluding(&[…])` für Unterordner, die bestehen bleiben müssen.

### 4. Übersetzungen ergänzen

In `src-tauri/src/i18n.rs`, zwei Zeilen:

```rust
("target.app.slack.name", "Slack", "Slack"),
("target.app.slack.description",
 "Bild- und Codezwischenspeicher. Die Anmeldung bleibt bestehen.",
 "Image and code cache. You stay signed in."),
```

Vergisst du das, schlägt `jedes_reinigungsziel_ist_uebersetzt` fehl. Die
Beschreibung soll die **Nebenwirkung** nennen, nicht den Pfad — der Nutzer
entscheidet danach.

### 5. Test ergänzen

Für Besonderheiten (Risikoeinstufung, Ausschlüsse, Altersgrenzen) einen Test in
`catalog.rs` schreiben. Beispiel:

```rust
#[test]
fn slack_wartet_auf_geschlossenen_client() {
    let ziel = target_by_key("app.slack").unwrap();
    assert_eq!(ziel.blocking_processes, &["slack.exe"]);
}
```

### 6. Was NICHT aufgenommen wird

Die Liste der bewussten Auslassungen steht in
[docs/REINIGUNGSZIELE.md](docs/REINIGUNGSZIELE.md#bewusst-nicht-bereinigt) —
Browser-Passwörter, COM/CLSID-Registry-Einträge, Defender-Quarantäne,
Ereignisprotokolle und einiges mehr. Bitte dort nachlesen, bevor du einen
Vorschlag machst; jede Auslassung hat eine Begründung.

## Übersetzungen

Es gibt **einen** Sprachkatalog: `src-tauri/src/i18n.rs`. Das Frontend hat
keine eigenen Sprachdateien — es holt den Satz über `get_translations(lang)`.
So kann kein Text zwischen Oberfläche und Kommandozeile auseinanderlaufen.

- Platzhalter sind `{0}`, `{1}`, … und müssen in **beiden** Sprachen
  vorkommen; ein Test prüft das.
- Meldungen aus der Engine sind selbst Schlüssel, optional mit Argument:
  `"warn.process_running|chrome.exe"`. Damit bleibt ein Bericht sprachneutral,
  bis er angezeigt wird.
- Eine weitere Sprache ergänzt du in `LANGUAGES` und als weitere Spalte in
  `ENTRIES`.

## Regeln für Code

**Rust**

- Keine Pfade außerhalb von `catalog.rs`. `scan` und `clean` bleiben generisch.
- Keine Shell. Externe Befehle laufen als Argumentliste über
  `engine::process::run` — keine zusammengesetzten Befehlszeichenketten.
- Kein `unwrap()` auf Sperren oder E/A. Fehler gehören in den Bericht.
- Löschen ausschließlich über `fsutil::remove_entry` — dort greifen die
  Schutzregeln (Pfadtiefe, Sperrliste, Reparse-Points).
- `cargo fmt` und `cargo clippy` ohne Warnungen.

**JavaScript**

- `invoke` nur in `frontend/src/api.js`.
- Kein `innerHTML`; Pfade und Fehlermeldungen kommen aus dem Dateisystem.
- Keine Inline-`onclick`-Attribute — die Content-Security-Policy verbietet
  Inline-Skripte.
- Keine sichtbaren Texte im Markup oder im Code; alles über `t('schlüssel')`.

**Alle**

- Kommentare erklären das **Warum**, nicht das Was.
- Ein behobener Fehler bekommt einen Test, der ohne die Korrektur fehlschlägt.

## Commits und Pull Requests

[Conventional Commits](https://www.conventionalcommits.org/), Betreff auf
Deutsch:

```
feat(catalog): Slack-Zwischenspeicher als Reinigungsziel
fix(scan): Junctions nicht mehr in die Größensumme zählen
docs(tests): Sicherheitsnetze der Testsuite erklären
```

Gängige Bereiche: `catalog`, `scan`, `clean`, `registry`, `cli`, `tui`,
`frontend`, `i18n`, `docs`, `ci`.

Ablauf: Branch von `main`, Änderung mit Tests, `npm test`, Pull Request mit
ausgefüllter Vorlage. Kleine, thematisch geschlossene Pull Requests werden
deutlich schneller geprüft als große.

## Fragen

Für Unsicherheiten vor der Umsetzung gern vorab ein Issue aufmachen — das
spart Arbeit auf beiden Seiten. Sicherheitslücken bitte **nicht** als Issue,
sondern nach [SECURITY.md](SECURITY.md).
