//! Protokollierung der Läufe.
//!
//! Jede Analyse und jede Bereinigung wird als Zeile mitgeschrieben. Das ist
//! kein Debug-Log für Entwickler, sondern eine **Nachvollziehbarkeit für den
//! Nutzer**: Plane löscht Dateien, und wer später wissen will, was wann
//! verschwunden ist, findet es hier.
//!
//! Grundsätze:
//!
//! * **Keine Einzelpfade.** Protokolliert werden Ziele, Zahlen und Fehler –
//!   nicht, welche Datei in welchem Ordner lag. Ein Protokoll, das man in ein
//!   Issue kopiert, darf keine persönlichen Pfade ausplaudern.
//! * **Begrenzte Größe.** Wird die Datei zu groß, beginnt sie von vorn. Ein
//!   Aufräumwerkzeug, das selbst den Datenträger vollschreibt, wäre absurd.
//! * **Fehler beim Protokollieren sind egal.** Sie dürfen einen Lauf niemals
//!   abbrechen.

use std::fmt::Write as _;
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::{CleanReport, ScanReport};

/// Dateiname im App-Config-Verzeichnis.
pub const LOG_FILE: &str = "plane.log";

/// Ab dieser Größe wird die Datei zurückgesetzt (1 MB).
pub const MAX_GROESSE: u64 = 1024 * 1024;

/// Pfad der Protokolldatei.
pub fn log_path(config_dir: &Path) -> PathBuf {
    config_dir.join(LOG_FILE)
}

/// Zeitstempel als `YYYY-MM-DD HH:MM:SS` (UTC).
///
/// Bewusst ohne `chrono`: eine Datumsformatierung rechtfertigt keine weitere
/// Abhängigkeit. Umrechnung nach dem Verfahren von Howard Hinnant.
fn zeitstempel() -> String {
    let sekunden = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let tage = (sekunden / 86_400) as i64;
    let rest = sekunden % 86_400;
    let (jahr, monat, tag) = civil_from_days(tage);

    format!(
        "{jahr:04}-{monat:02}-{tag:02} {:02}:{:02}:{:02}",
        rest / 3600,
        (rest % 3600) / 60,
        rest % 60
    )
}

fn civil_from_days(tage: i64) -> (i64, u32, u32) {
    let z = tage + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Eine Zeile anhängen. Fehler werden bewusst verschluckt.
fn schreibe(config_dir: &Path, zeile: &str) {
    let _ = schreibe_intern(config_dir, zeile);
}

fn schreibe_intern(config_dir: &Path, zeile: &str) -> std::io::Result<()> {
    fs::create_dir_all(config_dir)?;
    let pfad = log_path(config_dir);

    // Zu groß gewordene Datei zurücksetzen, statt sie unbegrenzt wachsen zu
    // lassen.
    if let Ok(meta) = fs::metadata(&pfad) {
        if meta.len() > MAX_GROESSE {
            let _ = fs::remove_file(&pfad);
        }
    }

    let mut datei = OpenOptions::new().create(true).append(true).open(&pfad)?;
    writeln!(datei, "{} {zeile}", zeitstempel())
}

/// Eine Analyse protokollieren.
pub fn scan(config_dir: &Path, bericht: &ScanReport) {
    let uebersprungen = bericht.targets.iter().filter(|t| t.skipped).count();
    schreibe(
        config_dir,
        &format!(
            "ANALYSE  ziele={} gefunden={} eintraege={} uebersprungen={} dauer={}ms{}",
            bericht.targets.len(),
            super::fsutil::format_bytes(bericht.total_size),
            bericht.total_items,
            uebersprungen,
            bericht.duration_ms,
            if bericht.cancelled {
                " ABGEBROCHEN"
            } else {
                ""
            }
        ),
    );
}

/// Eine Bereinigung protokollieren – Kopfzeile plus eine Zeile je Ziel.
pub fn clean(config_dir: &Path, bericht: &CleanReport, trockenlauf: bool) {
    schreibe(
        config_dir,
        &format!(
            "{}  freigegeben={} entfernt={} gesperrt={} rechte={} dauer={}ms{}",
            if trockenlauf {
                "SIMULATION"
            } else {
                "BEREINIGUNG"
            },
            super::fsutil::format_bytes(bericht.total_freed),
            bericht.total_removed,
            bericht.total_locked,
            bericht.total_denied,
            bericht.duration_ms,
            if bericht.cancelled {
                " ABGEBROCHEN"
            } else {
                ""
            }
        ),
    );

    for ziel in &bericht.targets {
        let mut zeile = String::new();
        let _ = write!(
            zeile,
            "  {:<28} {}",
            ziel.key,
            if ziel.skipped {
                "uebersprungen"
            } else if ziel.ok {
                "ok"
            } else {
                "FEHLER"
            }
        );

        if !ziel.skipped {
            let _ = write!(
                zeile,
                " freigegeben={} entfernt={}",
                super::fsutil::format_bytes(ziel.freed),
                ziel.removed_items
            );
            if ziel.locked_items > 0 {
                let _ = write!(zeile, " gesperrt={}", ziel.locked_items);
            }
            if ziel.denied_items > 0 {
                let _ = write!(zeile, " rechte={}", ziel.denied_items);
            }
        }

        // Fehlertexte können Dateinamen enthalten – deshalb nur die Anzahl und
        // die erste Meldung, gekürzt.
        if !ziel.errors.is_empty() {
            let erste: String = ziel.errors[0].chars().take(120).collect();
            let _ = write!(zeile, " fehler={} \"{}\"", ziel.errors.len(), erste);
        }

        schreibe(config_dir, &zeile);
    }

    if let Some(sicherung) = &bericht.registry_backup {
        schreibe(config_dir, &format!("  registry-sicherung={sicherung}"));
    }
}

/// Eine freie Meldung protokollieren (Start, Rechtewechsel, Abbruch).
pub fn notiz(config_dir: &Path, text: &str) {
    schreibe(config_dir, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::types::{Category, TargetClean};

    fn testordner(name: &str) -> PathBuf {
        let pfad = std::env::temp_dir().join(format!("plane_log_{name}"));
        let _ = fs::remove_dir_all(&pfad);
        pfad
    }

    fn bericht() -> CleanReport {
        CleanReport {
            success: true,
            targets: vec![TargetClean {
                key: "system.temp.user".into(),
                category: Category::System,
                ok: true,
                skipped: false,
                skip_reason: String::new(),
                freed: 2048,
                removed_items: 7,
                locked_items: 3,
                denied_items: 1,
                blocked_items: 0,
                errors: Vec::new(),
            }],
            total_freed: 2048,
            total_removed: 7,
            total_locked: 3,
            total_denied: 1,
            total_blocked: 0,
            duration_ms: 42,
            cancelled: false,
            registry_backup: None,
            error: String::new(),
        }
    }

    #[test]
    fn zeitstempel_hat_das_erwartete_format() {
        let t = zeitstempel();
        assert_eq!(t.len(), 19, "unerwartet: {t}");
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], " ");
        assert_eq!(&t[13..14], ":");
        assert!(t[..4].parse::<u32>().unwrap() >= 2024);
    }

    #[test]
    fn bereinigung_wird_mit_zahlen_protokolliert() {
        let dir = testordner("clean");
        clean(&dir, &bericht(), false);

        let inhalt = fs::read_to_string(log_path(&dir)).unwrap();
        assert!(inhalt.contains("BEREINIGUNG"));
        assert!(inhalt.contains("entfernt=7"));
        assert!(inhalt.contains("gesperrt=3"));
        assert!(inhalt.contains("rechte=1"));
        assert!(inhalt.contains("system.temp.user"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn trockenlauf_wird_als_simulation_gekennzeichnet() {
        let dir = testordner("dry");
        clean(&dir, &bericht(), true);
        let inhalt = fs::read_to_string(log_path(&dir)).unwrap();
        assert!(inhalt.contains("SIMULATION"));
        assert!(!inhalt.contains("BEREINIGUNG"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn protokoll_enthaelt_keine_einzelpfade() {
        // Ein Protokoll, das man in ein Issue kopiert, darf keine
        // persönlichen Pfade ausplaudern.
        let dir = testordner("privat");
        let mut b = bericht();
        b.targets[0].errors = vec![String::new()];
        clean(&dir, &b, false);

        let inhalt = fs::read_to_string(log_path(&dir)).unwrap();
        assert!(!inhalt.contains("\\Users\\"), "Pfad im Protokoll: {inhalt}");
        assert!(!inhalt.contains("AppData"), "Pfad im Protokoll: {inhalt}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zeilen_werden_angehaengt_nicht_ersetzt() {
        let dir = testordner("anhaengen");
        notiz(&dir, "erste");
        notiz(&dir, "zweite");

        let inhalt = fs::read_to_string(log_path(&dir)).unwrap();
        assert!(inhalt.contains("erste"));
        assert!(inhalt.contains("zweite"));
        assert_eq!(inhalt.lines().count(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn zu_grosses_protokoll_beginnt_von_vorn() {
        let dir = testordner("gross");
        fs::create_dir_all(&dir).unwrap();
        fs::write(log_path(&dir), vec![b'x'; (MAX_GROESSE + 10) as usize]).unwrap();

        notiz(&dir, "nach dem Zuruecksetzen");

        let inhalt = fs::read_to_string(log_path(&dir)).unwrap();
        assert!(inhalt.len() < 200, "Datei wurde nicht zurückgesetzt");
        assert!(inhalt.contains("nach dem Zuruecksetzen"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn nicht_schreibbares_verzeichnis_bricht_nichts_ab() {
        // Ein Protokollfehler darf einen Lauf niemals scheitern lassen.
        notiz(Path::new("\\\\?\\Z:\\gibt_es_nicht\\tief"), "egal");
    }

    #[test]
    fn abbruch_wird_vermerkt() {
        let dir = testordner("abbruch");
        let mut b = bericht();
        b.cancelled = true;
        clean(&dir, &b, false);
        assert!(fs::read_to_string(log_path(&dir))
            .unwrap()
            .contains("ABGEBROCHEN"));
        let _ = fs::remove_dir_all(&dir);
    }
}
