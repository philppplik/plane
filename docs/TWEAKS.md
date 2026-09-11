# Windows-Einstellungen

Was der „Plane Tweaker" anbietet, warum — und was er bewusst **nicht** anbietet.
Stand: 2026-09-11.

Der Katalog steht in
[`src-tauri/src/engine/tweaks.rs`](../src-tauri/src/engine/tweaks.rs).

## Haltung

Es gibt viele Werkzeuge, die Dutzende Registry-Werte auf einmal umstellen.
Einige davon haben nachweislich Systeme beschädigt. Plane bietet deshalb eine
**kleine, kuratierte Auswahl** an. Jeder Punkt erfüllt drei Bedingungen:

1. **Umkehrbar.** Plane merkt sich den Zustand, den es vorgefunden hat.
2. **Erklärbar.** Jeder Punkt nennt seine Nebenwirkung, nicht nur seinen
   Nutzen.
3. **Belegbar.** Der Wert stammt aus der Microsoft-Dokumentation oder aus
   einer geprüften Sammlung, nicht aus einem Forenbeitrag.

## Warum nicht einfach Standardwerte zurückschreiben

Verbreitete Werkzeuge hinterlegen je Tweak einen festen „Originalwert" und
schreiben ihn beim Zurücknehmen zurück. Das ist falsch.

Beispiel: Sie haben die Menüverzögerung selbst auf `0` gestellt. Ein Werkzeug
mit fest hinterlegtem Originalwert setzt beim Zurücknehmen den
Windows-Standard `400` — nicht Ihren Wert. Ihre Einstellung ist weg, ohne dass
es jemand merkt.

Plane liest deshalb **vor** jeder Änderung den tatsächlichen Zustand und legt
ihn in einem Journal ab (`%APPDATA%\com.ppaul.plane\tweaks.json`).
Zurücknehmen heißt: genau diesen Zustand wiederherstellen — einschließlich des
wichtigsten Falls, „der Wert existierte vorher gar nicht".

Ein Test hält das fest:
`anwenden_und_zuruecknehmen_stellt_den_ist_zustand_wieder_her`.

## Drei Zustände statt zwei

Ein Tweak kann mehrere Registry-Werte umfassen. Stehen nur einige davon wie
erwartet, meldet Plane **Teilweise** statt „aus". Verbreitete Werkzeuge melden
hier schlicht „aus" und verleiten damit zum wiederholten Anwenden.

Verglichen wird außerdem **typkonform**: `"0"` als Zeichenkette und `0` als
Zahl sind zwei verschiedene Dinge.

---

## Katalog

Legende: **Adm** = Administratorrechte nötig · **Wirkt** = sofort (S),
Explorer-Neustart (E), Neustart/Anmeldung (N)

### Datenschutz

| Punkt | Was passiert | Nebenwirkung | Adm | Wirkt |
|-------|--------------|--------------|-----|-------|
| Werbe-ID abschalten | Apps bekommen keine geräteweite Kennung mehr | Werbung bleibt, wird weniger passgenau | – | S |
| Zugeschnittene Erlebnisse | Diagnosedaten werden nicht mehr für Tipps und Werbung ausgewertet | Weniger personalisierte Vorschläge | – | S |
| Zuletzt geöffnete Apps | Windows merkt sich nicht mehr, was Sie wie oft starten | „Meistverwendet" im Startmenü bleibt leer | – | E |
| Tipp-Telemetrie | Keine Daten mehr darüber, wie Sie tippen | Eingabevorschläge werden mit der Zeit schlechter | – | S |
| Feedback-Nachfragen | Windows fragt nicht mehr nach Ihrer Meinung | keine | – | S |
| Vorgeschlagene Apps | Windows installiert keine Spiele mehr von selbst ins Startmenü | keine; installierte Apps bleiben | ✔ | N |

### Explorer

| Punkt | Was passiert | Nebenwirkung | Adm | Wirkt |
|-------|--------------|--------------|-----|-------|
| Dateiendungen anzeigen | `.pdf`, `.exe` werden wieder angezeigt | keine | – | E |
| Versteckte Dateien | Als versteckt markierte Einträge werden sichtbar | Ordner wirken unaufgeräumter | – | E |
| Klassisches Kontextmenü | Rechtsklick zeigt das volle Menü ohne Umweg | Menü ist länger | – | E |
| Explorer öffnet „Dieser PC" | Startet mit der Laufwerksübersicht | keine | – | E |
| „Task beenden" im Rechtsklick | Programme direkt aus der Taskleiste beenden | Ein beendetes Programm speichert nicht | – | E |

**Dateiendungen anzeigen ist auch eine Sicherheitsfrage.** `Rechnung.pdf.exe`
ist ohne sichtbare Endung nicht als Programm erkennbar — genau darauf setzen
Schadprogramme in E-Mail-Anhängen.

### Taskleiste

| Punkt | Was passiert | Nebenwirkung | Adm | Wirkt |
|-------|--------------|--------------|-----|-------|
| Linksbündig | Symbole beginnen wieder links | keine | – | E |
| Suchfeld ausblenden | Das breite Feld verschwindet | Suche bleibt über die Windows-Taste erreichbar | – | E |
| Task-Ansicht ausblenden | Symbol verschwindet | Funktion bleibt über Win+Tab | – | E |
| Keine Websuche im Startmenü | Startmenü durchsucht nur den Rechner | Keine Webergebnisse mehr | – | E |

Die ersten beiden Punkte sowie das klassische Kontextmenü und „Task beenden"
wirken **nur unter Windows 11**. Plane erkennt das an der Buildnummer und
markiert sie auf Windows 10 als wirkungslos, statt sie wirkungslos zu setzen.

### Leistung

| Punkt | Was passiert | Nebenwirkung | Adm | Wirkt |
|-------|--------------|--------------|-----|-------|
| Spielaufzeichnung abschalten | Hintergrundaufnahme der Xbox Game Bar endet | „Letzte 30 Sekunden aufzeichnen" geht nicht mehr | – | N |
| Schnellstart abschalten | Windows fährt beim Herunterfahren wirklich herunter | Start dauert einige Sekunden länger | ✔ | N |

**Schnellstart** ist der Punkt, der bei Treiberproblemen am häufigsten hilft
und für Dual-Boot nötig ist: mit aktiviertem Schnellstart schreibt Windows
beim Herunterfahren einen Ruhezustand und gibt die Datenträger nicht frei.

### System

| Punkt | Was passiert | Nebenwirkung | Adm | Wirkt |
|-------|--------------|--------------|-----|-------|
| Lange Dateipfade | Hebt die Grenze von 260 Zeichen auf | Ältere Programme kommen trotzdem nicht damit zurecht | ✔ | N |
| Ausführliche Statusmeldungen | Windows zeigt beim Starten, was es tut | keine; nützlich bei hängendem Herunterfahren | ✔ | N |

---

## Bewusst nicht angeboten

Diese Liste ist genauso wichtig wie der Katalog.

| Nicht angeboten | Begründung |
|-----------------|------------|
| **Windows Update deaktivieren** | Der am besten belegte Schadensfall. Das Abschalten der Dienste `BITS`, `wuauserv` und `InstallService` bricht den Microsoft Store und `winget` — betroffene Nutzer berichten von dauerhaft in „Queued" hängenden Installationen. In mehreren Berichten ließ sich Windows Update danach nicht wieder aktivieren. |
| **Microsoft Defender deaktivieren** | Wird seit 2020 durch die Manipulationsschutz-Funktion ignoriert. Der Wert würde gesetzt, aber wirkungslos bleiben — die Oberfläche würde also lügen. Und selbst wenn er wirkte: ein Aufräumwerkzeug schaltet keinen Virenschutz ab. |
| **Microsoft Edge entfernen** | Bricht `.pdf`- und `.html`-Zuordnungen sowie alle Anwendungen, die auf WebView2 aufsetzen. Wird von Updates teilweise wiederhergestellt. In der EU ist das Entfernen über die Windows-Einstellungen ohnehin vorgesehen. |
| **OneDrive entfernen** | **Datenverlustrisiko.** Desktop, Dokumente und Bilder können nach OneDrive umgeleitet sein. Ein Entfernen ohne vorheriges Zurückverschieben kostet Dateien. |
| **BitLocker deaktivieren** | Entschlüsselt die Systemplatte. Ein Abbruch mitten in der Entschlüsselung kann katastrophal enden. |
| **Windows Recall / AI entfernen** | In Windows 11 zunehmend Teil der Shell. Das Entfernen kann Startmenü und Suche beschädigen, und es gibt keinen zuverlässigen Rückweg. |
| **IPv6 global abschalten** | Microsoft rät ausdrücklich davon ab. Bricht VPNs und zunehmend auch normale Verbindungen. |
| **Dienste massenhaft abschalten** | Der klassische Weg, ein System unreparierbar zu machen. Plane fasst keine Dienste an — auch nicht die scheinbar harmlosen. |
| **Reservierten Speicher abschalten** | Spart 7–10 GB, lässt aber Funktionsupdates fehlschlagen, solange er aus ist. |
| **„Visuelle Effekte auf Leistung"** | Elf Einzelwerte plus eine Binärmaske, deren Rücknahme in verbreiteten Werkzeugen fehlerhaft ist. Der messbare Gewinn ist auf heutiger Hardware vernachlässigbar. |
| **Store-Empfehlungen per Dateirechten blockieren** | Ein ACL-Eingriff in eine Store-Datenbank, kein sauberer Mechanismus. Kann Store-Funktionen stören. |

Wenn Sie einen dieser Eingriffe brauchen: Es gibt Werkzeuge dafür, und die
Registry-Werte sind dokumentiert. Plane ist nicht das richtige Programm dafür.

---

## Gruppenrichtlinien

Werte unter `HKLM\SOFTWARE\Policies\...` haben Vorrang vor den normalen
Einstellungen. Plane prüft das und markiert betroffene Punkte als **„Von einer
Richtlinie Ihrer Organisation überlagert"** — eine Änderung daneben wäre
wirkungslos und würde beim nächsten Richtlinienabgleich ohnehin überschrieben.

Auf einem Firmen- oder Schulgerät sollten Sie die Punkte, die Adminrechte
verlangen, ohnehin mit Ihrer IT abstimmen.

---

## In der Oberfläche

Seitenleiste → **Windows Tweaks**. Die Punkte sind nach den fünf
Gruppen sortiert; jeder zeigt seinen Zustand als Abzeichen, seine
Nebenwirkung als eigene Zeile und wann die Änderung greift.

Der Schalter kennt drei Stellungen: an, aus und **unbestimmt** — letzteres für
„Teilweise". Neben jedem Punkt steht **Zurücknehmen**; das ist nicht dasselbe
wie Ausschalten (siehe unten). Punkte, die auf diesem System wirkungslos sind
oder unter einer Gruppenrichtlinie stehen, sind gesperrt und gedämpft
dargestellt statt versteckt — wer sie sucht, soll erfahren, warum sie fehlen.

Punkte, die Administratorrechte brauchen, fragen vorher nach und starten
Plane auf Wunsch erhöht neu — Windows zeigt dabei seine UAC-Rückfrage. Einen
einzelnen Registry-Zugriff nachträglich zu erhöhen sieht Windows nicht vor;
es geht nur über den Prozess.

## Kommandozeile

```bash
plane-cli tweaks
```

```bash
plane-cli tweaks --on explorer.file_extensions
```

```bash
plane-cli tweaks --off taskbar.hide_search
```

```bash
plane-cli tweaks --revert explorer.file_extensions
```

`--revert` stellt den Zustand wieder her, den Plane vor der eigenen Änderung
vorgefunden hat. `--off` setzt dagegen den Windows-Standardzustand — das ist
nicht dasselbe. Im Zweifel `--revert`.

`--json` liefert den Zustand aller Punkte maschinenlesbar.

---

## Herkunft

Die Ausgangsliste stammt aus der Recherche zu
[winutil](https://github.com/ChrisTitusTech/winutil) (MIT). Von dessen rund 67
Einträgen bietet Plane 19 an; die Auswahl und alle Beschreibungstexte sind
eigenständig. Einzelheiten in
[THIRD-PARTY-NOTICES.md](../THIRD-PARTY-NOTICES.md).
