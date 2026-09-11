# Fremde Arbeit, auf der Plane aufbaut

Plane steht unter der MIT-Lizenz (siehe [LICENSE](LICENSE)). Dieses Dokument
nennt die Projekte, deren Arbeit in Plane eingeflossen ist — als Quelle von
Wissen, nicht von Quelltext. **Plane enthält keinen kopierten Code aus diesen
Projekten.**

Registry-Pfade, Dateipfade und Wertnamen sind Tatsachen über Windows und als
solche nicht schutzfähig. Schutzfähig sind Auswahl, Anordnung und die
beschreibenden Texte — deshalb stehen hier die Projekte, deren Sammlungen als
Ausgangspunkt gedient haben.

---

## Reinigungsziele

### BleachBit

<https://www.bleachbit.org/> · <https://github.com/bleachbit/bleachbit>
GNU General Public License v3.0

Die umfangreichste öffentlich geprüfte Sammlung von Windows-Reinigungszielen.
Die Cleaner-Definitionen (`cleaners/*.xml`) und der System-Cleaner
(`bleachbit/Cleaner.py`) dienten als Referenz für die Pfade in
`src-tauri/src/engine/catalog.rs` — insbesondere für die Browser-Profile,
die Windows-Protokollpfade und die Erkenntnis, welche Ordner man *nicht*
anfassen sollte.

> **Wichtig:** BleachBit steht unter der GPL-3.0. Aus einem MIT-Projekt darf
> kein GPL-Quelltext übernommen werden. Plane hat deshalb ausschließlich
> **Pfadangaben und Verhaltensregeln** übernommen, keinen Code und keine
> übersetzten Beschreibungstexte. Die Implementierung ist vollständig
> eigenständig in Rust entstanden.

### Winapp2

<https://github.com/MoscaDotTo/Winapp2>

Deklarative Datenbank mit mehreren tausend Reinigungszielen. Diente als
Gegenprobe für Anwendungspfade (Discord, Teams, Spotify, Steam, Adobe) und
für die Einschätzung, welche Ordner Nutzerdaten enthalten.

---

## Windows-Einstellungen („Plane Tweaker")

### winutil

<https://github.com/ChrisTitusTech/winutil>
MIT License · Copyright (c) 2022 CT Tech Group LLC

```
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

winutils `config/tweaks.json` war die Ausgangsliste für die Recherche zu
`src-tauri/src/engine/tweaks.rs`. Zwei Dinge macht Plane bewusst anders:

1. **Nur eine kuratierte Teilmenge.** Von winutils rund 67 Einträgen bietet
   Plane 19 an. Nicht übernommen wurden unter anderem: Windows Update
   deaktivieren, Defender abschalten, Edge oder OneDrive entfernen, BitLocker
   deaktivieren, IPv6 global abschalten, massenhaftes Deaktivieren von
   Diensten. Die Begründungen stehen in
   [docs/TWEAKS.md](docs/TWEAKS.md#bewusst-nicht-angeboten).

2. **Echter Vorher-Zustand statt angenommener Standardwerte.** winutil
   hinterlegt je Tweak einen festen `OriginalValue`. Wer den betreffenden Wert
   selbst gesetzt hatte, bekommt beim Zurücknehmen den Windows-Standard —
   nicht seinen eigenen Wert. Plane liest den tatsächlichen Zustand vor der
   Änderung und legt ihn in einem Journal ab.

Die Beschreibungstexte in `src-tauri/src/i18n.rs` sind eigene Formulierungen,
keine Übersetzungen der winutil-Texte.

> Diese Nennung bedeutet keine Billigung durch Chris Titus Tech oder CT Tech
> Group LLC. Plane ist ein unabhängiges Projekt und verwendet deren Marken
> nicht.

---

## Deinstallation

### Bulk Crap Uninstaller

<https://www.bcuninstaller.com/> · <https://github.com/Klocman/Bulk-Crap-Uninstaller>
GPL-3.0 (Anwendung) / Apache-2.0 (Bibliothek `UninstallTools`)

Referenz für die Filterregeln, mit denen sich die Liste in „Apps & Features"
nachbilden lässt, und für das Konzept geschützter Einträge.

> **Kein Quelltext übernommen.** Die GPL-Anteile wären mit der MIT-Lizenz von
> Plane unvereinbar. Übernommen wurden ausschließlich Konzepte und
> Filterkriterien, die sich aus der Windows-Dokumentation ohnehin ableiten
> lassen.

---

## Microsoft-Dokumentation

Die Grundlage für Registry-Sichten (`KEY_WOW64_*`), msiexec-Exitcodes,
MSIX-Paketverwaltung, WOW64-Umleitung und Windows on Arm stammt aus
Microsoft Learn. Besonders relevant:

- *Microsoft support policy for the use of registry cleaning utilities*
  (KB2563254) — die Grundlage für Planes zurückhaltenden Umgang mit der
  Registry
- *Registry Redirector* und *Accessing an Alternate Registry View*
- *Windows Installer error codes*
- *Windows on Arm FAQ*

---

## Softwareabhängigkeiten

Die Lizenzen der verwendeten Rust-Kisten und npm-Pakete lassen sich erzeugen
mit:

```bash
cargo tree --manifest-path src-tauri/Cargo.toml
```

```bash
npm ls --all
```

Die wichtigsten: Tauri (MIT/Apache-2.0), serde (MIT/Apache-2.0), sysinfo (MIT),
ratatui (MIT), clap (MIT/Apache-2.0), winreg (MIT), glob (MIT/Apache-2.0),
walkdir (MIT/Unlicense), Vite (MIT).

Die Schriften **Young Serif** und **Elms Sans** liegen unter `frontend/fonts/`
und unterliegen den Lizenzbedingungen ihrer jeweiligen Urheber.
