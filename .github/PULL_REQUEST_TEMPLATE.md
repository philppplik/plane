# Änderung

<!-- Was ändert sich und warum? Bei einem Fehler: was war kaputt? -->

Behebt #

## Art der Änderung

- [ ] Fehlerbehebung
- [ ] Neue Funktion
- [ ] Neues Reinigungsziel
- [ ] Umbau ohne Verhaltensänderung
- [ ] Dokumentation
- [ ] Übersetzung

## Prüfliste

- [ ] `npm test` läuft durch (Rust **und** Python)
- [ ] `cargo fmt` angewendet, `cargo clippy` ohne Warnungen
- [ ] Neue Funktionen haben Tests; behobene Fehler haben einen Test, der ohne
      die Korrektur fehlschlägt
- [ ] Kommentare und Testnamen auf Deutsch, mit echten Umlauten
- [ ] Dokumentation angepasst, falls sich Verhalten oder Datenmodell ändert

## Nur bei einem neuen Reinigungsziel

- [ ] Eintrag in `src-tauri/src/engine/catalog.rs` ergänzt
- [ ] Übersetzungen (`name` **und** `description`) in `src-tauri/src/i18n.rs`
      für Deutsch und Englisch ergänzt
- [ ] Risikoeinstufung begründet (siehe unten)
- [ ] Pfade ohne festen Laufwerksbuchstaben, mit `%VAR%`
- [ ] `requires_admin` gesetzt, falls das Ziel in einem Systemverzeichnis liegt
- [ ] `blocking_processes` gesetzt, falls ein laufendes Programm die Dateien
      sperrt

**Begründung der Risikoeinstufung:**

<!--
Warum ist das Ziel `Safe` / `Notice` / `Caution`?
`Safe` nur, wenn der Inhalt garantiert automatisch neu erzeugt wird.
Was merkt der Nutzer nach der Bereinigung? Was ist unwiederbringlich weg?
-->

## Geprüft auf

- [ ] Windows 11 x64
- [ ] Windows 11 ARM64
- [ ] Windows 10
- [ ] Mit Administratorrechten
- [ ] Ohne Administratorrechte
