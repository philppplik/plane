//! Papierkorb: Größe ermitteln und leeren.
//!
//! Der Papierkorb ist keine gewöhnliche Ordnerstruktur, sondern eine
//! Shell-Abstraktion über `$Recycle.Bin` auf jedem Laufwerk. Für die **Analyse**
//! lesen wir diese Ordner direkt (schnell, ohne Nebenwirkung); zum **Leeren**
//! nutzen wir die offizielle Shell-API über PowerShell, damit Windows seine
//! internen Indexdateien konsistent hält.

use std::path::PathBuf;

use super::fsutil;
use super::types::ScanItem;

/// Laufwerke, auf denen ein Papierkorb liegen kann.
fn kandidaten() -> Vec<PathBuf> {
    let mut pfade = Vec::new();

    if cfg!(windows) {
        use sysinfo::Disks;
        for disk in Disks::new_with_refreshed_list().iter() {
            let wurzel = disk.mount_point().to_path_buf();
            let bin = wurzel.join("$Recycle.Bin");
            if bin.exists() {
                pfade.push(bin);
            }
        }
        if pfade.is_empty() {
            let bin = PathBuf::from(fsutil::system_drive()).join("$Recycle.Bin");
            if bin.exists() {
                pfade.push(bin);
            }
        }
    }

    pfade
}

/// Inhalt des Papierkorbs analysieren.
///
/// Liefert einen Eintrag je Laufwerk – die Einzeldateien im Papierkorb tragen
/// verschlüsselte Namen (`$R…`) und wären für den Nutzer nicht lesbar.
pub fn scan() -> (Vec<ScanItem>, Vec<String>) {
    let mut treffer = Vec::new();
    let mut warnungen = Vec::new();

    for bin in kandidaten() {
        let groesse = fsutil::dir_size(&bin);
        if groesse == 0 {
            continue;
        }
        let laufwerk = bin
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| bin.to_string_lossy().to_string());

        treffer
            .push(ScanItem::new(bin.to_string_lossy().to_string(), groesse).with_detail(laufwerk));
    }

    if treffer.is_empty() && !cfg!(windows) {
        warnungen.push("Papierkorb wird nur unter Windows unterstützt".to_string());
    }

    (treffer, warnungen)
}

// ---------------------------------------------------------------------------
// Leeren
// ---------------------------------------------------------------------------

/// Keine Rückfrage, kein Fortschrittsfenster, kein Ton.
///
/// `SHERB_NOCONFIRMATION | SHERB_NOPROGRESSUI | SHERB_NOSOUND`. Plane fragt
/// bereits selbst nach — eine zweite Rückfrage von Windows wäre lästig und
/// würde in einem Skript hängen bleiben.
#[cfg(windows)]
const SHERB_STILL: u32 = 0x1 | 0x2 | 0x4;

/// `E_UNEXPECTED` – Windows meldet das, wenn nichts zu löschen war.
#[cfg(windows)]
const E_UNEXPECTED: i32 = 0x8000_FFFFu32 as i32;

/// `HRESULT_FROM_WIN32(ERROR_FILE_NOT_FOUND)` – dieselbe Aussage, anderer Weg.
#[cfg(windows)]
const HRESULT_DATEI_NICHT_GEFUNDEN: i32 = 0x8007_0002u32 as i32;

/// Papierkorb aller Laufwerke leeren.
///
/// Gibt die freigegebenen Bytes zurück. Die Größe wird vorher gemessen, weil
/// die Shell-API selbst nichts meldet.
///
/// # Warum nicht `Clear-RecycleBin`
///
/// Bis 0.2.0 lief das über PowerShell. Zwei Probleme:
///
/// 1. **Eine irreführende Fehlermeldung.** War der Papierkorb bereits leer,
///    meldete das Cmdlet „Das System kann die angegebene Datei nicht finden"
///    mit vollständigem PowerShell-Stacktrace. Für den Nutzer sah ein
///    völlig normaler Lauf nach einem Defekt aus.
/// 2. **Ein Shell-Aufruf**, obwohl Plane an jeder anderen Stelle ohne
///    auskommt — und ein PowerShell-Start kostet rund eine Sekunde.
///
/// `SHEmptyRecycleBinW` ist genau die Funktion, die das Cmdlet intern
/// aufruft. Der Rückgabewert lässt sich eindeutig auswerten, statt ihn aus
/// übersetztem Fehlertext zu erraten.
#[cfg(windows)]
pub fn empty(dry_run: bool) -> (u64, Vec<String>) {
    let (items, mut fehler) = scan();
    let groesse: u64 = items.iter().map(|i| i.size).sum();

    if dry_run || groesse == 0 {
        return (groesse, fehler);
    }

    // SAFETY: Ein Nullzeiger als Wurzelpfad ist dokumentiert und bedeutet
    // „alle Laufwerke". Ein Nullfenster bedeutet „kein Elternfenster", was
    // zusammen mit SHERB_NOPROGRESSUI richtig ist.
    let ergebnis = unsafe {
        windows_sys::Win32::UI::Shell::SHEmptyRecycleBinW(
            std::ptr::null_mut(),
            std::ptr::null(),
            SHERB_STILL,
        )
    };

    if !leeren_gelungen(ergebnis) {
        fehler.push(format!("error.recyclebin|0x{:08X}", ergebnis as u32));
        return (0, fehler);
    }

    (groesse, fehler)
}

/// War das Leeren erfolgreich?
///
/// „Es war nichts da" zählt als Erfolg. Windows meldet diesen Fall je nach
/// Version unterschiedlich — und in beiden Fällen ist das Ergebnis genau das,
/// was der Nutzer wollte.
#[cfg(windows)]
fn leeren_gelungen(hresult: i32) -> bool {
    hresult >= 0 || hresult == E_UNEXPECTED || hresult == HRESULT_DATEI_NICHT_GEFUNDEN
}

#[cfg(not(windows))]
pub fn empty(dry_run: bool) -> (u64, Vec<String>) {
    let (items, fehler) = scan();
    let groesse: u64 = items.iter().map(|i| i.size).sum();
    let _ = dry_run;
    (groesse, fehler)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_liefert_gueltige_eintraege() {
        let (items, warnungen) = scan();
        for item in &items {
            assert!(item.path.contains("$Recycle.Bin"));
            assert!(item.size > 0, "leere Papierkörbe werden nicht gemeldet");
        }
        if cfg!(windows) {
            assert!(warnungen.is_empty());
        }
    }

    /// Der Fall, der die Fehlermeldung ausgelöst hat: ein bereits leerer
    /// Papierkorb. Windows meldet das als Fehler – es ist keiner.
    #[cfg(windows)]
    #[test]
    fn ein_leerer_papierkorb_ist_kein_fehler() {
        assert!(leeren_gelungen(0), "S_OK");
        assert!(leeren_gelungen(1), "S_FALSE");
        assert!(leeren_gelungen(E_UNEXPECTED), "war bereits leer");
        assert!(leeren_gelungen(HRESULT_DATEI_NICHT_GEFUNDEN), "nichts da");
    }

    #[cfg(windows)]
    #[test]
    fn echte_fehler_bleiben_fehler() {
        // HRESULT_FROM_WIN32(ERROR_ACCESS_DENIED)
        assert!(!leeren_gelungen(0x8007_0005u32 as i32));
    }

    #[test]
    fn trockenlauf_leert_nichts() {
        let (gemeldet, fehler) = empty(true);
        let (items, _) = scan();
        let erwartet: u64 = items.iter().map(|i| i.size).sum();
        assert_eq!(gemeldet, erwartet, "Trockenlauf muss die Größe melden");
        assert!(fehler.is_empty() || cfg!(not(windows)));
    }
}
