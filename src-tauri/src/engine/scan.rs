//! Analysephase: ermitteln, was bereinigt werden könnte – ohne etwas zu ändern.
//!
//! Der Scan ist garantiert nebenwirkungsfrei. Er ist die Grundlage dafür, dass
//! der Nutzer vor jeder Löschung sieht, worum es geht und wie viel es bringt.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

use super::catalog;
use super::fsutil;
use super::installers;
use super::recyclebin;
use super::registry;
use super::runtime::{running_processes, RunContext};
use super::types::{Progress, ScanItem, ScanReport, Target, TargetKind, TargetScan};

/// Höchstzahl gemeldeter Einzeltreffer je Ziel.
///
/// Ein Browser-Cache hat schnell 50 000 Dateien. Die Oberfläche zeigt sie
/// ohnehin nicht alle; `item_count` und `size` bleiben vollständig.
pub const MAX_ITEMS_PER_TARGET: usize = 200;

/// Alle angegebenen Ziele analysieren. Leere Liste = Katalogreihenfolge.
pub fn scan(keys: &[String], ctx: &RunContext) -> ScanReport {
    let start = Instant::now();

    let ziele: Vec<&'static Target> = if keys.is_empty() {
        catalog::TARGETS.iter().collect()
    } else {
        keys.iter()
            .filter_map(|k| catalog::target_by_key(k))
            .collect()
    };

    let gesamt = ziele.len();
    let mut ergebnisse = Vec::with_capacity(gesamt);
    let mut gefunden_bytes = 0u64;
    let mut abgebrochen = false;

    for (index, ziel) in ziele.iter().enumerate() {
        if ctx.cancelled() {
            abgebrochen = true;
            break;
        }

        ctx.report(Progress::new("scan", ziel, index, gesamt).with_bytes(gefunden_bytes));

        let ergebnis = scan_target(ziel, ctx);
        gefunden_bytes += ergebnis.size;

        ctx.report(
            Progress::new("scan", ziel, index, gesamt)
                .with_bytes(gefunden_bytes)
                .finished(),
        );

        ergebnisse.push(ergebnis);
    }

    ScanReport {
        total_size: ergebnisse.iter().map(|t| t.size).sum(),
        total_items: ergebnisse.iter().map(|t| t.item_count).sum(),
        targets: ergebnisse,
        duration_ms: start.elapsed().as_millis() as u64,
        cancelled: abgebrochen,
    }
}

/// Ein einzelnes Ziel analysieren.
pub fn scan_target(ziel: &'static Target, ctx: &RunContext) -> TargetScan {
    if ziel.requires_admin && !ctx.elevated {
        return TargetScan::skipped(ziel, "skip.needs_admin");
    }

    let mut ergebnis = TargetScan {
        key: ziel.key.to_string(),
        category: ziel.category,
        risk: ziel.risk,
        requires_admin: ziel.requires_admin,
        default_enabled: ziel.default_enabled,
        suggestion_only: ziel.is_suggestion_only(),
        item_count: 0,
        size: 0,
        items: Vec::new(),
        skipped: false,
        skip_reason: String::new(),
        warnings: Vec::new(),
    };

    // Laufende Programme sperren ihre Cache-Dateien – das ist kein Fehler,
    // aber der Nutzer sollte es wissen.
    let laufend = running_processes(ziel.blocking_processes);
    if !laufend.is_empty() {
        ergebnis
            .warnings
            .push(format!("warn.process_running|{}", laufend.join(", ")));
    }

    match ziel.kind {
        TargetKind::Files(regeln) => {
            let mut gesehen: HashSet<PathBuf> = HashSet::new();

            for regel in regeln {
                if ctx.cancelled() {
                    break;
                }

                // Bei rekursiven Mustern liefert der Glob Ordner UND Dateien.
                // Die Ordner zu vermessen wuerde denselben Baum mehrfach
                // durchlaufen (quadratische Laufzeit) - die Dateien sind
                // ohnehin alle einzeln enthalten.
                let nur_dateien = regel.pattern.contains("**");

                let pfade = match fsutil::resolve_pattern(regel.pattern) {
                    Ok(p) => p,
                    Err(e) => {
                        ergebnis.warnings.push(e);
                        continue;
                    }
                };

                for pfad in pfade {
                    if ctx.cancelled() {
                        break;
                    }
                    if gesehen.contains(&pfad) {
                        continue;
                    }
                    if !fsutil::path_is_allowed(&pfad) {
                        continue;
                    }
                    if !fsutil::is_older_than(&pfad, regel.min_age_days) {
                        continue;
                    }
                    if regel.is_excluded(&pfad.to_string_lossy()) {
                        continue;
                    }
                    if nur_dateien && pfad.is_dir() {
                        continue;
                    }

                    let groesse = fsutil::entry_size(&pfad);
                    ergebnis.size += groesse;
                    ergebnis.item_count += 1;

                    if ergebnis.items.len() < MAX_ITEMS_PER_TARGET {
                        ergebnis
                            .items
                            .push(ScanItem::new(pfad.to_string_lossy().to_string(), groesse));
                    }
                    gesehen.insert(pfad);
                }
            }
        }

        TargetKind::RecycleBin => {
            let (items, warnungen) = recyclebin::scan();
            ergebnis.size = items.iter().map(|i| i.size).sum();
            ergebnis.item_count = items.len();
            ergebnis.items = items;
            ergebnis.warnings.extend(warnungen);
        }

        TargetKind::Installers => {
            let (items, warnungen) = installers::find_installers(installers::SEARCH_PATTERNS);
            ergebnis.size = items.iter().map(|i| i.size).sum();
            ergebnis.item_count = items.len();
            ergebnis.items = items.into_iter().take(MAX_ITEMS_PER_TARGET).collect();
            ergebnis.warnings.extend(warnungen);
        }

        TargetKind::Registry(regeln) => {
            let (items, warnungen) = registry::scan_rules(regeln, &ctx.cancel);
            ergebnis.item_count = items.len();
            ergebnis.size = 0; // Registry-Einträge belegen keinen messbaren Platz.
            ergebnis.items = items.into_iter().take(MAX_ITEMS_PER_TARGET).collect();
            ergebnis.warnings.extend(warnungen);
        }

        TargetKind::Command(_) => {
            // Befehle lassen sich nicht analysieren, ohne sie auszuführen.
            // Sie werden als „vorhanden, Größe unbekannt“ gemeldet.
            ergebnis.item_count = 1;
            ergebnis.size = 0;
        }
    }

    ergebnis
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::runtime::CancelToken;
    use std::fs;

    fn kontext() -> RunContext {
        RunContext::new().with_elevated(false)
    }

    #[test]
    fn scan_liefert_ein_ergebnis_je_ziel() {
        let bericht = scan(&[], &kontext());
        assert_eq!(bericht.targets.len(), catalog::TARGETS.len());
    }

    #[test]
    fn scan_einzelner_ziele_beschraenkt_sich_darauf() {
        let bericht = scan(&["system.dns".to_string()], &kontext());
        assert_eq!(bericht.targets.len(), 1);
        assert_eq!(bericht.targets[0].key, "system.dns");
    }

    #[test]
    fn unbekannte_ziele_werden_ignoriert() {
        let bericht = scan(&["gibtesnicht".to_string()], &kontext());
        assert!(bericht.targets.is_empty());
        assert_eq!(bericht.total_size, 0);
    }

    #[test]
    fn adminziele_werden_ohne_rechte_uebersprungen() {
        let bericht = scan(&["system.temp.windows".to_string()], &kontext());
        assert!(bericht.targets[0].skipped);
        assert_eq!(bericht.targets[0].skip_reason, "skip.needs_admin");
    }

    #[test]
    fn adminziele_laufen_mit_rechten() {
        let ctx = RunContext::new().with_elevated(true);
        let bericht = scan(&["system.temp.windows".to_string()], &ctx);
        assert!(!bericht.targets[0].skipped);
    }

    #[test]
    fn scan_meldet_fortschritt_fuer_jedes_ziel() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let zaehler = Arc::new(AtomicUsize::new(0));
        let kopie = Arc::clone(&zaehler);
        let ctx = RunContext::new()
            .with_elevated(false)
            .with_progress(Box::new(move |p| {
                assert_eq!(p.phase, "scan");
                assert!(p.percent >= 0.0 && p.percent <= 100.0);
                if p.done {
                    kopie.fetch_add(1, Ordering::SeqCst);
                }
            }));

        let bericht = scan(&["system.dns".to_string(), "app.vscode".to_string()], &ctx);
        assert_eq!(bericht.targets.len(), 2);
        assert_eq!(zaehler.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn abbruch_beendet_den_scan_vorzeitig() {
        let token = CancelToken::new();
        token.cancel();
        let ctx = RunContext::new().with_cancel(token).with_elevated(false);

        let bericht = scan(&[], &ctx);
        assert!(bericht.cancelled);
        assert!(bericht.targets.is_empty());
    }

    #[test]
    fn scan_veraendert_nichts() {
        let ordner = std::env::temp_dir().join("plane_scan_unveraendert");
        let _ = fs::remove_dir_all(&ordner);
        fs::create_dir_all(&ordner).unwrap();
        fs::write(ordner.join("a.tmp"), vec![0u8; 1234]).unwrap();

        // %TEMP% enthält den Testordner – ein Scan darf ihn nicht anfassen.
        let _ = scan(&["system.temp.user".to_string()], &kontext());

        assert!(ordner.join("a.tmp").exists(), "Scan hat gelöscht");
        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn groessen_summieren_sich_zum_gesamtwert() {
        let bericht = scan(&[], &kontext());
        let summe: u64 = bericht.targets.iter().map(|t| t.size).sum();
        assert_eq!(bericht.total_size, summe);
        let anzahl: usize = bericht.targets.iter().map(|t| t.item_count).sum();
        assert_eq!(bericht.total_items, anzahl);
    }

    #[test]
    fn trefferliste_ist_begrenzt_die_zaehlung_nicht() {
        let ordner = std::env::temp_dir().join("plane_scan_limit");
        let _ = fs::remove_dir_all(&ordner);
        fs::create_dir_all(&ordner).unwrap();
        for i in 0..(MAX_ITEMS_PER_TARGET + 25) {
            fs::write(ordner.join(format!("f{i}.bin")), b"x").unwrap();
        }

        std::env::set_var("PLANE_SCAN_LIMIT_DIR", ordner.to_string_lossy().to_string());
        let treffer = fsutil::resolve_pattern("%PLANE_SCAN_LIMIT_DIR%/*").unwrap();
        assert!(treffer.len() > MAX_ITEMS_PER_TARGET);

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn dauer_wird_gemessen() {
        let bericht = scan(&["system.dns".to_string()], &kontext());
        assert!(bericht.duration_ms < 60_000);
    }

    #[test]
    fn registryziel_meldet_keine_groesse() {
        let bericht = scan(&["registry.orphans".to_string()], &kontext());
        assert_eq!(bericht.targets[0].size, 0);
    }
}
