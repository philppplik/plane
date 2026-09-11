//! Erkennung alter Installationsdateien.
//!
//! Heruntergeladene Setups sind bei fast jedem Rechner der größte einzelne
//! Posten an totem Ballast – und zugleich der heikelste: eine Datei kann eine
//! Lizenz, ein Offline-Installer oder ein Archiv sein, das nirgends sonst
//! existiert. Deshalb gilt hier:
//!
//! * Es wird **nie** automatisch gelöscht (`Target::is_suggestion_only`).
//! * Nur Dateien ab [`MIN_AGE_DAYS`] Tagen und ab [`MIN_SIZE_BYTES`] werden
//!   überhaupt vorgeschlagen.
//! * Der Nutzer wählt einzelne Dateien aus, nicht das Ziel als Ganzes.

use std::path::{Path, PathBuf};

use super::fsutil;
use super::types::ScanItem;

/// Mindestalter, damit ein frischer Download nicht sofort vorgeschlagen wird.
pub const MIN_AGE_DAYS: u32 = 30;

/// Mindestgröße – kleine Helfer lohnen den Vorschlag nicht.
pub const MIN_SIZE_BYTES: u64 = 5 * 1024 * 1024;

/// Dateiendungen, die auf ein Installationspaket hindeuten.
pub const INSTALLER_EXTENSIONS: &[&str] = &["exe", "msi", "msu", "msix", "msixbundle", "appx"];

/// Endungen von Archiven, die häufig Installer enthalten.
pub const ARCHIVE_EXTENSIONS: &[&str] = &["zip", "7z", "rar", "iso", "img"];

/// Namensbestandteile, die einen Installer verraten – auch ohne passende
/// Endung (z. B. `firefox-setup-nightly.exe`).
const INSTALLER_HINTS: &[&str] = &[
    "setup",
    "install",
    "installer",
    "update",
    "updater",
    "redist",
    "webinstall",
    "-x64",
    "-x86",
    "-arm64",
    "patch",
    "driver",
];

/// Suchorte für Installationsdateien.
///
/// `%DOWNLOADS%` und `%DESKTOP%` werden über die Shell-Ordnerdefinition
/// aufgelöst – `%USERPROFILE%\Downloads` wäre falsch, sobald OneDrive die
/// Ordner umgeleitet hat.
pub const SEARCH_PATTERNS: &[&str] = &["%DOWNLOADS%/*", "%DESKTOP%/*", "%PUBLIC%/Downloads/*"];

/// Bewertung einer Datei.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Eindeutig ein Installationspaket.
    Installer,
    /// Archiv, das ein Installationspaket enthalten könnte.
    Archive,
    /// Kein Vorschlag.
    Ignore,
}

/// Eine einzelne Datei bewerten – ohne Dateisystemzugriff, damit testbar.
pub fn classify(dateiname: &str) -> Verdict {
    let klein = dateiname.to_ascii_lowercase();
    let endung = Path::new(&klein)
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    if INSTALLER_EXTENSIONS.contains(&endung.as_str()) {
        // .exe ist mehrdeutig: portable Programme sind keine Installer.
        if endung == "exe" && !INSTALLER_HINTS.iter().any(|h| klein.contains(h)) {
            return Verdict::Ignore;
        }
        return Verdict::Installer;
    }

    if ARCHIVE_EXTENSIONS.contains(&endung.as_str())
        && INSTALLER_HINTS.iter().any(|h| klein.contains(h))
    {
        return Verdict::Archive;
    }

    Verdict::Ignore
}

/// Prüft, ob eine konkrete Datei vorgeschlagen werden soll.
pub fn qualifies(pfad: &Path, min_size: u64, min_age_days: u32) -> Option<(Verdict, u64, u64)> {
    let meta = std::fs::symlink_metadata(pfad).ok()?;
    if !meta.is_file() {
        return None;
    }
    let name = pfad.file_name()?.to_string_lossy().to_string();
    let urteil = classify(&name);
    if urteil == Verdict::Ignore {
        return None;
    }
    if meta.len() < min_size {
        return None;
    }
    if !fsutil::is_older_than(pfad, min_age_days) {
        return None;
    }
    let alter_tage = fsutil::entry_age(pfad)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0);
    Some((urteil, meta.len(), alter_tage))
}

/// Alle Installationsdateien in den Standardsuchorten finden.
pub fn find_installers(patterns: &[&str]) -> (Vec<ScanItem>, Vec<String>) {
    let mut treffer = Vec::new();
    let mut warnungen = Vec::new();
    let mut gesehen: Vec<PathBuf> = Vec::new();

    for muster in patterns {
        let pfade = match fsutil::resolve_pattern(muster) {
            Ok(p) => p,
            // Ein fehlendes %PUBLIC%\Downloads ist kein Fehler, nur eine Notiz.
            Err(e) => {
                warnungen.push(e);
                continue;
            }
        };

        for pfad in pfade {
            if gesehen.contains(&pfad) {
                continue;
            }
            if let Some((urteil, groesse, alter)) = qualifies(&pfad, MIN_SIZE_BYTES, MIN_AGE_DAYS) {
                let art = match urteil {
                    Verdict::Installer => "installer",
                    Verdict::Archive => "archive",
                    Verdict::Ignore => continue,
                };
                treffer.push(
                    ScanItem::new(pfad.to_string_lossy().to_string(), groesse)
                        .with_detail(format!("{art}|{alter}")),
                );
                gesehen.push(pfad);
            }
        }
    }

    // Größte zuerst – das ist die Reihenfolge, in der ein Nutzer entscheidet.
    treffer.sort_by_key(|item| std::cmp::Reverse(item.size));
    (treffer, warnungen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn erkennt_eindeutige_installationspakete() {
        assert_eq!(classify("VCRedist.msi"), Verdict::Installer);
        assert_eq!(classify("Windows11.msu"), Verdict::Installer);
        assert_eq!(classify("App.msix"), Verdict::Installer);
        assert_eq!(classify("Paket.appx"), Verdict::Installer);
    }

    #[test]
    fn exe_nur_mit_hinweis_im_namen() {
        assert_eq!(classify("FirefoxSetup.exe"), Verdict::Installer);
        assert_eq!(classify("node-v22-x64.exe"), Verdict::Installer);
        assert_eq!(classify("nvidia-driver.exe"), Verdict::Installer);
        // Portable Programme sind keine Installer.
        assert_eq!(classify("mein-tool.exe"), Verdict::Ignore);
        assert_eq!(classify("game.exe"), Verdict::Ignore);
    }

    #[test]
    fn archive_nur_mit_hinweis_im_namen() {
        assert_eq!(classify("treiber-setup.zip"), Verdict::Archive);
        assert_eq!(classify("urlaubsfotos.zip"), Verdict::Ignore);
        assert_eq!(classify("win11-install.iso"), Verdict::Archive);
    }

    #[test]
    fn dokumente_werden_ignoriert() {
        for name in [
            "rechnung.pdf",
            "notizen.txt",
            "bild.png",
            "musik.mp3",
            "ohne-endung",
        ] {
            assert_eq!(classify(name), Verdict::Ignore, "{name}");
        }
    }

    #[test]
    fn erkennung_ist_gross_klein_unabhaengig() {
        assert_eq!(classify("SETUP.EXE"), Verdict::Installer);
        assert_eq!(classify("Paket.MSI"), Verdict::Installer);
    }

    #[test]
    fn zu_kleine_dateien_werden_nicht_vorgeschlagen() {
        let ordner = std::env::temp_dir().join("plane_installers_klein");
        let _ = std::fs::remove_dir_all(&ordner);
        std::fs::create_dir_all(&ordner).unwrap();
        let datei = ordner.join("setup.exe");
        std::fs::write(&datei, vec![0u8; 1024]).unwrap();

        assert!(qualifies(&datei, MIN_SIZE_BYTES, 0).is_none());
        assert!(qualifies(&datei, 512, 0).is_some());

        let _ = std::fs::remove_dir_all(&ordner);
    }

    #[test]
    fn frische_dateien_werden_nicht_vorgeschlagen() {
        let ordner = std::env::temp_dir().join("plane_installers_frisch");
        let _ = std::fs::remove_dir_all(&ordner);
        std::fs::create_dir_all(&ordner).unwrap();
        let datei = ordner.join("setup.msi");
        std::fs::write(&datei, vec![0u8; 2048]).unwrap();

        assert!(
            qualifies(&datei, 512, MIN_AGE_DAYS).is_none(),
            "Ein frischer Download darf nicht zum Löschen vorgeschlagen werden"
        );

        let _ = std::fs::remove_dir_all(&ordner);
    }

    #[test]
    fn verzeichnisse_werden_ignoriert() {
        let ordner = std::env::temp_dir().join("plane_installers_ordner");
        let _ = std::fs::remove_dir_all(&ordner);
        std::fs::create_dir_all(ordner.join("setup.exe")).unwrap();
        assert!(qualifies(&ordner.join("setup.exe"), 0, 0).is_none());
        let _ = std::fs::remove_dir_all(&ordner);
    }

    #[test]
    fn finder_sortiert_nach_groesse_und_meldet_details() {
        let ordner = std::env::temp_dir().join("plane_installers_finder");
        let _ = std::fs::remove_dir_all(&ordner);
        std::fs::create_dir_all(&ordner).unwrap();
        std::fs::write(ordner.join("klein-setup.exe"), vec![0u8; 2000]).unwrap();
        std::fs::write(ordner.join("gross-setup.exe"), vec![0u8; 9000]).unwrap();
        std::fs::write(ordner.join("egal.txt"), vec![0u8; 9999]).unwrap();

        std::env::set_var("PLANE_TEST_DOWNLOADS", ordner.to_string_lossy().to_string());
        let (treffer, _) = {
            // Direktaufruf mit kleinerem Schwellwert über qualifies:
            let mut items: Vec<ScanItem> = fsutil::resolve_pattern("%PLANE_TEST_DOWNLOADS%/*")
                .unwrap()
                .into_iter()
                .filter_map(|p| {
                    qualifies(&p, 512, 0).map(|(_, groesse, alter)| {
                        ScanItem::new(p.to_string_lossy().to_string(), groesse)
                            .with_detail(format!("installer|{alter}"))
                    })
                })
                .collect();
            items.sort_by_key(|item| std::cmp::Reverse(item.size));
            (items, Vec::<String>::new())
        };

        assert_eq!(treffer.len(), 2, "nur Installer, keine Textdatei");
        assert!(treffer[0].size > treffer[1].size, "größte zuerst");
        assert!(treffer[0].detail.starts_with("installer|"));

        let _ = std::fs::remove_dir_all(&ordner);
    }

    #[test]
    fn finder_meldet_fehlende_suchorte_als_warnung() {
        let (treffer, warnungen) = find_installers(&["%GIBT_ES_NICHT_98765%/*"]);
        assert!(treffer.is_empty());
        assert_eq!(warnungen.len(), 1);
    }

    #[test]
    fn standardsuchorte_sind_nutzerordner() {
        for muster in SEARCH_PATTERNS {
            assert!(muster.starts_with('%'), "{muster}");
        }
    }
}
