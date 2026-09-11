# Reinigungsziele

Was Plane bereinigt, warum – und was bewusst **nicht**. Stand: 2026-09-10.

Der maßgebliche Katalog steht in
[`src-tauri/src/engine/catalog.rs`](../src-tauri/src/engine/catalog.rs). Dieses
Dokument erklärt die Entscheidungen dahinter. Wer ein Ziel ergänzen will, findet
die Schritt-für-Schritt-Anleitung in [CONTRIBUTING.md](../CONTRIBUTING.md).

## Das Sicherheitsmodell

Plane löscht Dateien. Das ist unumkehrbar, also ist die Frage nicht „was können
wir alles löschen", sondern „was können wir *verantworten* zu löschen".

### Zwei Stufen statt einer

```
Analysieren  →  Auswählen  →  Bereinigen
(ändert nichts)  (Nutzer)     (nur Ausgewähltes)
```

Es gibt keinen Weg, über die Oberfläche oder die CLI etwas zu löschen, ohne dass
vorher angezeigt wurde, was und wie viel. Das ist der wichtigste Unterschied zu
einem naiven „Aufräumen"-Knopf.

### Zwei Risikoachsen statt eines Schalters

Ein einzelnes `safe`-Flag reicht nicht, weil „braucht Administratorrechte" und
„ist unumkehrbar" verschiedene Dinge sind:

| Feld | Bedeutung |
|------|-----------|
| `requires_admin` | Ohne erhöhte Rechte nicht durchführbar. Wird gemeldet übersprungen, nicht still ignoriert. |
| `irreversible` | Löscht endgültig, ohne Papierkorb. |

`safe` ist daraus abgeleitet. Zusätzlich trägt jedes Ziel eine Risikostufe für
die Anzeige:

| Stufe | Bedeutung | Vorausgewählt? |
|-------|-----------|----------------|
| 🟢 `Safe` | Reiner Cache, wird bei Bedarf neu erzeugt. | ja, wenn sinnvoll |
| 🟡 `Notice` | Spürbare Nebenwirkung (Abmeldung, längere Startzeit, Verlauf weg). | nein |
| 🔴 `Caution` | Datenverlust möglich. | **niemals** |

### Technische Schutzmechanismen

| Mechanismus | Wogegen |
|-------------|---------|
| `path_is_allowed()` – mindestens zwei Pfadebenen, Sperrliste für `System32`, `WinSxS`, `Program Files` | Ein fehlerhaftes Muster löscht eine Partition |
| Reparse-Point-Erkennung vor jedem rekursiven Abstieg | Eine Junction führt den Lauf aus dem Zielbaum heraus; gelöscht wird der Verweis, nie sein Ziel |
| Keine Shell – Befehle als Argumentliste | Command-Injection, Abhängigkeit von cmd/PowerShell-Expansionsregeln |
| Nicht auflösbare `%VAR%` = Fehler, nicht Fortsetzung | Löschen eines wörtlichen `%LOCALAPPDATA%`-Ordners |
| Dienststopp scheitert ⇒ Ziel wird übersprungen | Halb geleerter, inkonsistenter Update-Cache |
| Registry-Sicherung als `.reg` vor jeder Änderung | Siehe unten |
| `min_age_days` bei Installationsdateien und Protokollen | Löschen eines Downloads, der gerade läuft |
| `exclude`-Listen (z. B. `%TEMP%\Low`) | Ordner entfernen, die Windows braucht |

---

## Katalog

### System

| Schlüssel | Was | Risiko | Admin | Nebenwirkung |
|-----------|-----|--------|-------|--------------|
| `system.temp.user` | `%TEMP%` | 🟢 | – | keine. `%TEMP%\Low` bleibt erhalten |
| `system.temp.windows` | `%WINDIR%\Temp` | 🟢 | ✔ | keine |
| `system.thumbnails` | `thumbcache_*.db`, `iconcache_*.db` | 🟢 | – | erster Ordneraufruf kurz langsamer |
| `system.inetcache` | `INetCache` | 🟢 | – | keine |
| `system.errorreports` | WER-Berichte (Nutzer und System) | 🟢 | – | Absturzberichte weg |
| `system.crashdumps` | `MEMORY.DMP`, Minidumps, LiveKernelReports | 🟢 | – | keine BSOD-Analyse mehr möglich. Oft mehrere GB |
| `system.logs` | CBS-, DISM-, Update-, Setup-Protokolle ab 7 bzw. 30 Tagen | 🟢 | ✔ | die *aktive* `CBS.log` bleibt |
| `system.dns` | `ipconfig /flushdns` | 🟢 | – | kein Speichergewinn, hilft bei hängenden Verbindungen |
| `system.prefetch` | `%WINDIR%\Prefetch\*.pf` | 🟡 | ✔ | Programme starten übergangsweise langsamer |
| `system.windowsupdate` | `SoftwareDistribution\Download` | 🟡 | ✔ | stoppt `wuauserv`+`bits`. Update-Historie (`DataStore.edb`) bleibt |
| `system.deliveryoptimization` | DO-Cache | 🟢 | ✔ | stoppt `DoSvc` |
| `system.fontcache` | FontCache-Dateien | 🟢 | ✔ | stoppt `FontCache`; Neustart empfohlen |
| `system.recentdocs` | Zuletzt-verwendet-Verknüpfungen | 🟡 | – | Sprunglisten leer. Dateien bleiben |
| `system.searchcache` | Symbolcache der Windows-Suche | 🟢 | – | keine |
| `system.windowsold` | `Windows.old`, `$Windows.~BT`, `$Windows.~WS` | 🔴 | ✔ | **kein Rollback zur vorherigen Windows-Version mehr**. 8–40 GB |

### Papierkorb

| Schlüssel | Risiko | Nebenwirkung |
|-----------|--------|--------------|
| `recyclebin.all` | 🔴 | Endgültig auf allen Laufwerken. Genau hier findet man versehentlich Gelöschtes wieder |

Die Größe wird direkt aus `$Recycle.Bin` je Laufwerk ermittelt; geleert wird
über `Clear-RecycleBin`, damit Windows seine Indexdateien konsistent hält.

### Browser

Unterstützt: Edge, Chrome, Firefox, Brave, Opera (inkl. GX), Vivaldi.

Chromium-Profile heißen `Default`, `Profile 1`, … – die Muster nutzen `*`.
Firefox-Profile tragen Zufallsnamen, ebenfalls `*`. Opera legt Profil
(`%APPDATA%`) und Cache (`%LOCALAPPDATA%`) getrennt ab.

| Schlüssel | Risiko | Nebenwirkung |
|-----------|--------|--------------|
| `browser.<name>.cache` | 🟢 | keine. Anmeldungen und Verlauf bleiben |
| `browser.predictor` | 🟢 | Adressleiste rät kurzzeitig schlechter |
| `browser.history` | 🟡 | Verlauf und Vervollständigung weg. **Lesezeichen bleiben** |
| `browser.cookies` | 🔴 | **Überall abgemeldet** |

Läuft der Browser, sind seine Dateien gesperrt. Plane erkennt das (`chrome.exe`,
`msedge.exe`, `firefox.exe`, …) und zeigt eine Warnung am Ziel an, statt still
weniger zu löschen als angekündigt.

### Anwendungen

| Schlüssel | Risiko | Anmerkung |
|-----------|--------|-----------|
| `app.discord`, `app.teams` | 🟢 | Cache und Protokolle. Anmeldung bleibt |
| `app.spotify` | 🟢 | **ohne** `Storage` – dort liegen Offline-Downloads |
| `app.vscode` | 🟢 | **ohne** `User\History` – das ist die lokale Datei-Zeitleiste |
| `app.gpushadercache` | 🟢 | NVIDIA, AMD, Intel, DirectX. Spiele laden einmalig länger |
| `app.steam` | 🟢 | Web- und Protokollcache. Spiele und Spielstände unberührt |
| `app.adobe` | 🟡 | Medien-Cache – Premiere/After Effects müssen neu analysieren |
| `app.java` | 🟢 | – |
| `app.office` | 🟢 | **ohne** `OfficeFileCache` (siehe unten) |
| `app.packagemanagers` | 🟡 | npm/pip laden beim nächsten Mal erneut aus dem Netz |

### Installationsdateien

`installers.downloads` durchsucht Downloads, Schreibtisch und öffentliche
Downloads nach Setups. Diese Ordner werden über die **Shell-Ordnerdefinition**
aufgelöst, nicht als `%USERPROFILE%\Downloads` – sonst greift die Suche ins
Leere, sobald OneDrive die Ordner umgeleitet hat.

Erkennung, absichtlich zurückhaltend:

- eindeutige Endungen (`.msi`, `.msu`, `.msix`, `.appx`) gelten immer
- `.exe` nur mit Hinweis im Namen (`setup`, `install`, `-x64`, `driver`, …),
  weil portable Programme ebenfalls `.exe` heißen
- Archive nur mit solchem Hinweis
- mindestens 30 Tage alt und 5 MB groß

Plane löscht hier **nie** von selbst: das Ziel ist `suggestion_only`, der Nutzer
wählt einzelne Dateien aus. Eine `.iso` kann ein Offline-Installer sein, den es
nirgends sonst gibt.

### Registry

> Microsoft unterstützt die Verwendung von Registry-Cleanern ausdrücklich nicht
> und weist darauf hin, dass fehlerhafte Änderungen eine Neuinstallation
> erfordern können (KB2563254). Der messbare Geschwindigkeitsgewinn ist
> praktisch null.

Plane bietet die Funktion trotzdem an, weil Nutzer sie von anderen Werkzeugen
kennen – aber in der kleinstmöglichen verantwortbaren Ausprägung:

| Schlüssel | Was | Risiko |
|-----------|-----|--------|
| `registry.privacy` | MUICache – Zuletzt-gestartet-Listen | 🟡 |
| `registry.orphans` | SharedDLLs, Autostart, App Paths, Uninstall-Einträge mit fehlender Zieldatei | 🔴 |

`registry.privacy` ist ausdrücklich eine **Datenschutz**funktion, keine
Reparatur. Beide Ziele sind nie vorausgewählt.

**Vier Regeln:**

1. Vor jeder Änderung wird eine `.reg`-Sicherung geschrieben (über
   `reg export`, damit das Format garantiert wieder importierbar ist).
   Scheitert die Sicherung, wird **nichts** gelöscht.
2. Als verwaist gilt nur ein absoluter Pfad mit Laufwerksbuchstaben *und*
   Dateiendung, der nicht existiert. UNC-Pfade, URLs, MSI-GUIDs,
   `rundll32`-Aufrufe und Ordner ohne Endung bleiben unangetastet.
3. Wert- und Schlüssel-Sperrlisten: OneDrive, Defender, `svchost`, `dllhost`
   und alles unter `Services`, `Winlogon`, `AppModel`, `PackagedCom`,
   `ActivatableClasses`, `Installer`.
4. Nicht auflösbare Umgebungsvariablen ⇒ Eintrag bleibt.

---

## Bewusst NICHT bereinigt

Diese Liste ist genauso wichtig wie der Katalog. Jeder Punkt ist eine
Entscheidung, keine Lücke.

| Nicht bereinigt | Begründung |
|-----------------|------------|
| **COM/CLSID, TypeLib, Interface, AppID** | Die dokumentierte Hauptursache für Systemschäden durch Registry-Cleaner. Viele Einträge zeigen absichtlich auf verzögert registrierte oder erst bei Bedarf bereitgestellte Komponenten und sehen für eine Existenzprüfung „verwaist" aus. |
| **MSI-Installer-Komponenten** (`Classes\Installer`, `CurrentVersion\Installer`) | Beschädigt Reparatur und Deinstallation von Office und Visual Studio. |
| **Dienste** (`SYSTEM\CurrentControlSet\Services`) | Ein entfernter Boot-Treiber macht Windows nicht mehr startfähig. |
| **Dateizuordnungen** (`HKCR\.ext`, `FileExts\UserChoice`) | „Öffnen mit" bricht; `UserChoice` ist zusätzlich hash-geschützt. |
| **Firewall-Regeln** | Legacy-Anwendungen verlieren ihre Freigaben. |
| **Ereignisprotokolle** (`winevt\Logs\*.evtx`) | Audit- und Diagnoseverlust. Korrekt wäre `EvtClearLog`, nicht Dateilöschung – bis dahin gar nicht. |
| **Defender-Quarantäne und Signaturen** | Fälschlich erkannte Dateien wären unwiederbringlich weg; Tamper Protection blockiert den Zugriff ohnehin. |
| **Browser-Passwörter** (`Login Data`, `logins.json`) | Kein Cleaner-Anwendungsfall. Unwiederbringlich. |
| **Browser-Site-Data** (`Local Storage`, `IndexedDB`, Service Worker) | Enthält Offline-Dokumente und Entwürfe von Web-Apps. |
| **`OfficeFileCache`** | Enthält noch nicht hochgeladene Änderungen an Cloud-Dokumenten. |
| **Spotify `Storage`** | Heruntergeladene Titel für die Offline-Wiedergabe. |
| **VS Code `User\History`** | Lokale Datei-Zeitleiste – ein Sicherungsnetz. |
| **Steam `steamapps`** | Spiele und Spielstände. Redistributables wären löschbar, brechen aber Steams Integritätsprüfung. |
| **Freien Speicher überschreiben** | Auf SSDs wirkungslos bis schädlich (Schreibzyklen), auf verschlüsselten Laufwerken sinnlos. |
| **Fremde Benutzerprofile** | Bräuchte Administratorrechte und würde fremde Daten anfassen. |

---

## Windows on ARM

Plane wird nativ für `x86_64-pc-windows-msvc` **und** `aarch64-pc-windows-msvc`
gebaut. Das ist keine Kosmetik, sondern Korrektheit: ein 32-Bit-Prozess unterliegt
der WOW64-Dateisystem- und Registry-Umleitung und würde 64-Bit-Dateien
systematisch für fehlend halten – und damit gültige Einträge als „verwaist"
melden.

Weitere Konsequenzen im Code:

- Kein hart verdrahtetes Laufwerk und kein hart verdrahteter `Program
  Files`-Pfad. `%PROGRAMFILES(ARM)%` kann als Variable existieren, ohne dass der
  Ordner da ist – Muster ohne Treffer sind deshalb kein Fehler.
- Die Anwendungsdaten liegen architekturunabhängig: Chrome, Edge und Firefox
  legen ihre Profile auf ARM64 exakt dort ab wie auf x64. Die Browser-Ziele
  gelten unverändert.
- Grafik-Shadercaches unterscheiden sich: auf Snapdragon-Geräten fehlen die
  NVIDIA- und AMD-Ordner, `%LOCALAPPDATA%\D3DSCache` existiert aber weiterhin.
  Deshalb enthält `app.gpushadercache` beides.
- Auf ARM64 existiert zusätzlich die Registry-Sicht `WowAA32Node`. Plane liest
  bislang nur die native Sicht – für die angebotenen Ziele ausreichend, für
  eine spätere Erweiterung um 32-Bit-Uninstall-Einträge zu beachten.

---

## Quellen

Die Pfade und Risikoeinschätzungen stützen sich auf:

- Microsoft: *Support policy for the use of registry cleaning utilities*
  (KB2563254)
- Microsoft Learn: *Registry Redirector*, *Accessing an Alternate Registry
  View*, *Windows on Arm FAQ*, *Arm64X PE Files*
- BleachBit (GPL, quelloffen): `cleaners/*.xml` und `bleachbit/Cleaner.py` –
  die umfangreichste öffentlich geprüfte Sammlung von Windows-Reinigungszielen
- Winapp2 – deklarative Zieldatenbank mit mehreren tausend Einträgen
- NVIDIA Support: *Deleting NVIDIA Shader Cache files*
