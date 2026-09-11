# Sicherheit

## Eine Lücke melden

**Bitte kein öffentliches Issue.** Nutze die privaten
[GitHub Security Advisories](https://docs.github.com/de/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability)
(Reiter „Security" → „Report a vulnerability").

Erwartete Reaktion:

| Schritt | Zeitrahmen |
|---------|------------|
| Eingangsbestätigung | 72 Stunden |
| Erste Einschätzung | 7 Tage |
| Korrektur oder Zeitplan | 30 Tage |

Plane ist ein Freizeitprojekt einer einzelnen Person — es gibt kein
Prämienprogramm, aber jede ernsthafte Meldung wird ernsthaft behandelt und auf
Wunsch in den Anmerkungen zur Veröffentlichung genannt.

## Unterstützte Versionen

| Version | Unterstützt |
|---------|-------------|
| 0.1.x | ✅ (Vorabversion) |

Vor der ersten stabilen Veröffentlichung wird nur die jeweils neueste Version
gepflegt.

## Sicherheitsmodell

Plane löscht Dateien und ändert die Registry. Diese Fähigkeiten sind der
Zweck des Programms — und zugleich das, was ein Angreifer missbrauchen würde.
Die folgenden Mechanismen sind deshalb keine Nebensache, sondern Kern der
Architektur.

### Was Plane tut

- Löscht Dateien in Benutzer- und Systemverzeichnissen
- Leert den Papierkorb über die Shell-API
- Entfernt Registry-Werte in einer eng begrenzten Auswahl von Schlüsseln
- Startet eine Handvoll Windows-Dienste neu (`wuauserv`, `bits`, `FontCache`,
  `DoSvc`)
- Führt `ipconfig /flushdns` und `Clear-RecycleBin` aus

### Was Plane nicht tut

- **Keine Telemetrie.** Plane zählt keine Starts, meldet keine Nutzung und
  legt kein Profil an.
- **Kein Netzwerkverkehr ohne Ihr Zutun.** Es gibt genau einen Netzzugriff,
  und er ist standardmäßig **aus**: die Suche nach neuen Versionen. Sie ist in
  `src-tauri/src/engine/update.rs` gekapselt, ein Vertragstest verhindert,
  dass an anderer Stelle Netzwerkcode entsteht. Eingeschaltet stellt Plane
  eine einzige Anfrage an `api.github.com` und schickt dabei nichts mit außer
  dem `User-Agent` `plane/<version>`, den die GitHub-API verlangt. GitHub
  sieht Ihre IP-Adresse — wie beim Aufruf jeder Webseite.
- **Kein Selbstaktualisieren.** Plane lädt nichts herunter und installiert
  nichts. Es öffnet die Veröffentlichungsseite im Browser; alles Weitere ist
  Ihre Handlung. Solange die Pakete nicht signiert sind, wäre alles andere
  ein Angriffsweg.
- **Keine Shell.** Externe Befehle laufen als Argumentliste ohne
  `cmd.exe`/PowerShell-Zeichenkettenauswertung. Es gibt keine Stelle, an der
  ein Dateiname in eine Befehlszeile eingesetzt wird.
- **Keine dauerhafte Rechteerhöhung.** Plane fordert keine Elevation an.
  Adminpflichtige Ziele werden gemeldet übersprungen.
- **Kein Nachladen von Code.** Der Zielkatalog ist einkompiliert, es gibt keine
  Definitionsdateien, die zur Laufzeit gelesen würden.

### Schutzmechanismen

| Mechanismus | Wogegen |
|-------------|---------|
| Zweistufiger Ablauf: erst analysieren, dann bereinigen | Es gibt keinen Weg, etwas zu löschen, ohne dass vorher angezeigt wurde, was und wie viel |
| `path_is_allowed()` — mindestens zwei Pfadebenen, Sperrliste für `System32`, `WinSxS`, `Program Files` | Ein fehlerhaftes Muster löscht eine Partition |
| Reparse-Point-Erkennung vor jedem rekursiven Abstieg | Eine Junction führt den Lauf aus dem Zielbaum heraus; gelöscht wird der Verweis, nie sein Ziel |
| Nicht auflösbare `%VAR%` ist ein Fehler, kein Grund weiterzumachen | Löschen eines wörtlich genommenen `%LOCALAPPDATA%`-Ordners |
| Registry-Sicherung als `.reg` **vor** jeder Änderung; ohne Sicherung keine Änderung | Nicht rückholbare Registry-Schäden |
| Registry-Sperrliste: `Services`, `Winlogon`, `AppModel`, `PackagedCom`, `ActivatableClasses`, `Installer`, `CLSID`, `TypeLib` | Die dokumentierten Hauptursachen für Systemschäden durch Registry-Cleaner |
| Content-Security-Policy mit `script-src 'self'`, keine Inline-Skripte | Eingeschleuster Code in der Oberfläche |
| Tauri-Capabilities auf `core:default` begrenzt | Unnötige API-Fläche im Frontend |
| Kein `innerHTML` im Frontend | Dateinamen und Fehlermeldungen als Markup interpretiert |

### Restrisiko

Ehrlich benannt:

1. **Ein Fehler im Zielkatalog löscht die falschen Dateien.** Der Katalog ist
   die gefährlichste Datei im Projekt. Änderungen daran brauchen eine
   Begründung und eine Quelle — siehe [CONTRIBUTING.md](CONTRIBUTING.md).
2. **Mit Administratorrechten gestartet, kann Plane Systemdateien löschen.**
   Die Sperrlisten fangen die bekannten Fälle ab, aber sie sind eine Liste,
   keine Garantie.
3. **Registry-Bereinigung bleibt riskant.** Microsoft rät von Registry-Cleanern
   ausdrücklich ab (KB2563254). Plane bietet nur die kleinstmögliche
   verantwortbare Teilmenge an, nie vorausgewählt, immer mit Sicherung. Wer
   das Risiko nicht braucht, lässt die Kategorie einfach aus.
4. **Keine Wiederherstellung für gelöschte Dateien.** Anders als bei der
   Registry gibt es keine Sicherung. Deshalb die zweistufige Anzeige und der
   Simulationsmodus.

## Lieferkette

Ein Aufräumwerkzeug mit Löschrechten ist ein attraktives Ziel für einen
Lieferketten-Angriff — CCleaner wurde 2017 auf genau diesem Weg kompromittiert
und verteilte Schadsoftware an über zwei Millionen Nutzer.

Daraus folgt für dieses Projekt:

- Abhängigkeiten werden bewusst klein gehalten und über Dependabot verfolgt.
- Veröffentlichungen entstehen aus einem nachvollziehbaren CI-Lauf, nicht auf
  einem Entwicklerrechner.
- **Vor der ersten stabilen Version:** Signierung der Installationspakete und
  veröffentlichte Prüfsummen. Bis dahin gilt: nur aus dem Quelltext bauen oder
  Vorabversionen ausschließlich zum Ausprobieren verwenden.
- Lade Plane niemals von einer anderen Stelle als dem offiziellen Repository
  herunter.
