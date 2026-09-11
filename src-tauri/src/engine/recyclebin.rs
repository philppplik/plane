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

/// Papierkorb aller Laufwerke leeren.
///
/// Gibt die freigegebenen Bytes zurück. Die Größe wird vorher gemessen, weil
/// die Shell-API selbst nichts meldet.
pub fn empty(dry_run: bool) -> (u64, Vec<String>) {
    let (items, mut fehler) = scan();
    let groesse: u64 = items.iter().map(|i| i.size).sum();

    if dry_run || groesse == 0 {
        return (groesse, fehler);
    }

    let ausgabe = super::process::run(&[
        "powershell",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "Clear-RecycleBin -Force -ErrorAction Stop",
    ]);

    if ausgabe.code != 0 {
        // Ein leerer Papierkorb meldet ebenfalls einen Fehler – das ist keiner.
        let text = format!("{} {}", ausgabe.stderr, ausgabe.stdout).to_ascii_lowercase();
        if !text.contains("empty") && !text.contains("leer") {
            fehler.push(if ausgabe.stderr.is_empty() {
                ausgabe.stdout
            } else {
                ausgabe.stderr
            });
            return (0, fehler);
        }
    }

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

    #[test]
    fn trockenlauf_leert_nichts() {
        let (gemeldet, fehler) = empty(true);
        let (items, _) = scan();
        let erwartet: u64 = items.iter().map(|i| i.size).sum();
        assert_eq!(gemeldet, erwartet, "Trockenlauf muss die Größe melden");
        assert!(fehler.is_empty() || cfg!(not(windows)));
    }
}
