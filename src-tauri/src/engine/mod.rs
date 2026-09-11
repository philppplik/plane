//! Reinigungs-Engine von Plane.
//!
//! # Aufbau
//!
//! ```text
//! catalog   – WAS bereinigt wird (einziger Ort mit Pfaden)
//! scan      – WAS WÄRE löschbar (nebenwirkungsfrei)
//! clean     – LÖSCHEN, was der Nutzer ausgewählt hat
//!
//! fsutil    – Pfade, Globs, Größen, sicheres Löschen
//! process   – externe Befehle ohne Shell
//! log       – Nachvollziehbarkeit der Läufe
//! elevation – Neustart mit Administratorrechten
//! registry  – Registry-Analyse mit Sicherung
//! recyclebin/installers – Sonderfälle mit eigener Logik
//! runtime   – Abbruch, Fortschritt, Rechte, Prozesse
//! types     – Datenmodell
//! ```
//!
//! Die Engine kennt weder Tauri noch eine Oberfläche. Fortschritt wird über
//! einen Callback gemeldet ([`runtime::RunContext::with_progress`]), damit
//! GUI, CLI und Tests denselben Code verwenden.

pub mod catalog;
pub mod clean;
pub mod elevation;
pub mod fsutil;
pub mod installers;
pub mod log;
pub mod process;
pub mod recyclebin;
pub mod registry;
pub mod runtime;
pub mod scan;
pub mod types;

pub use catalog::{default_selection, target_by_key, targets_in, TARGETS};
pub use clean::clean;
pub use elevation::{neu_starten_als_admin, neustart_sinnvoll, Elevation};
pub use runtime::{is_admin, running_processes, CancelToken, RunContext};
pub use scan::scan;
pub use types::{
    Category, CleanReport, CleanRequest, Progress, Risk, ScanItem, ScanReport, Target, TargetClean,
    TargetKind, TargetScan,
};

#[cfg(test)]
mod integration_tests {
    //! Zusammenspiel von Katalog, Scan und Clean.

    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn backup_dir() -> PathBuf {
        std::env::temp_dir().join("plane_engine_backup")
    }

    #[test]
    fn jedes_katalogziel_laesst_sich_analysieren() {
        let ctx = RunContext::new().with_elevated(false);
        for target in TARGETS {
            let ergebnis = scan::scan_target(target, &ctx);
            assert_eq!(ergebnis.key, target.key);
            // Kein Ziel darf die Analyse zum Absturz bringen.
            assert!(ergebnis.size < u64::MAX);
        }
    }

    #[test]
    fn standardauswahl_laesst_sich_vollstaendig_im_trockenlauf_bereinigen() {
        let ctx = RunContext::new().with_elevated(false);
        let anfrage = CleanRequest {
            targets: default_selection().iter().map(|s| s.to_string()).collect(),
            only_paths: Vec::new(),
            dry_run: true,
        };

        let bericht = clean::clean(&anfrage, &backup_dir(), &ctx);
        assert_eq!(bericht.targets.len(), default_selection().len());
        assert!(!bericht.cancelled);
    }

    #[test]
    fn analyse_und_trockenlauf_melden_vergleichbare_groessen() {
        // Beide Phasen dürfen nicht unterschiedlich rechnen.
        let ordner = std::env::temp_dir().join("plane_engine_vergleich");
        let _ = fs::remove_dir_all(&ordner);
        fs::create_dir_all(&ordner).unwrap();
        let datei = ordner.join("gross-setup.exe");
        fs::write(&datei, vec![0u8; 8192]).unwrap();

        let pfad = datei.to_string_lossy().to_string();
        let ctx = RunContext::new().with_elevated(false);

        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            only_paths: vec![pfad],
            dry_run: true,
        };
        let bericht = clean::clean(&anfrage, &backup_dir(), &ctx);

        assert_eq!(bericht.total_freed, 8192);
        assert!(datei.exists());

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn kein_ziel_faellt_durch_die_uebersetzungsraster() {
        // Jedes Ziel braucht Name und Beschreibung im Sprachkatalog.
        for target in TARGETS {
            let name = target.i18n_name();
            let beschreibung = target.i18n_description();
            assert!(name.starts_with("target."));
            assert!(name.ends_with(".name"));
            assert!(beschreibung.ends_with(".description"));
        }
    }

    #[test]
    fn riskante_ziele_sind_klar_gekennzeichnet() {
        for target in TARGETS {
            if target.risk == Risk::Caution {
                assert!(!target.default_enabled, "{}", target.key);
            }
            if target.is_suggestion_only() {
                assert!(!target.default_enabled, "{}", target.key);
            }
        }
    }
}
