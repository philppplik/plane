//! Bereinigungsphase: löschen, was der Nutzer ausgewählt hat.
//!
//! Regeln:
//!
//! * Es wird nur bereinigt, was ausdrücklich angefordert wurde – es gibt kein
//!   „alles“ ohne Zielliste.
//! * Vor der Registry-Bereinigung wird gesichert. Scheitert die Sicherung,
//!   wird die Registry **nicht** angefasst.
//! * Scheitert der Dienststopp, wird das Ziel übersprungen statt einen halb
//!   geleerten Cache zu hinterlassen.
//! * `dry_run` meldet exakt dieselben Zahlen, löscht aber nichts.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::catalog;
use super::fsutil;
use super::fsutil::Removal;
use super::process;
use super::recyclebin;
use super::registry;
use super::runtime::RunContext;
use super::types::{CleanReport, CleanRequest, Progress, Target, TargetClean, TargetKind};

/// Auswahl bereinigen.
///
/// `backup_dir` ist der Ort für die Registry-Sicherung.
pub fn clean(anfrage: &CleanRequest, backup_dir: &Path, ctx: &RunContext) -> CleanReport {
    let start = Instant::now();

    let ziele: Vec<&'static Target> = anfrage
        .targets
        .iter()
        .filter_map(|k| catalog::target_by_key(k))
        .collect();

    let gesamt = ziele.len();
    let mut ergebnisse: Vec<TargetClean> = Vec::with_capacity(gesamt);
    let mut freigegeben = 0u64;
    let mut abgebrochen = false;
    let mut sicherung: Option<String> = None;

    for (index, ziel) in ziele.iter().enumerate() {
        if ctx.cancelled() {
            abgebrochen = true;
            break;
        }

        ctx.report(Progress::new("clean", ziel, index, gesamt).with_bytes(freigegeben));

        let ergebnis = clean_target(ziel, anfrage, backup_dir, ctx, &mut sicherung);
        freigegeben += ergebnis.freed;

        ctx.report(
            Progress::new("clean", ziel, index, gesamt)
                .with_bytes(freigegeben)
                .finished(),
        );

        ergebnisse.push(ergebnis);
    }

    let fehlgeschlagen: Vec<&str> = ergebnisse
        .iter()
        .filter(|t| !t.ok && !t.skipped)
        .map(|t| t.key.as_str())
        .collect();

    CleanReport {
        success: fehlgeschlagen.is_empty() && !abgebrochen,
        total_freed: ergebnisse.iter().map(|t| t.freed).sum(),
        total_removed: ergebnisse.iter().map(|t| t.removed_items).sum(),
        total_locked: ergebnisse.iter().map(|t| t.locked_items).sum(),
        total_denied: ergebnisse.iter().map(|t| t.denied_items).sum(),
        total_blocked: ergebnisse.iter().map(|t| t.blocked_items).sum(),
        error: if fehlgeschlagen.is_empty() {
            String::new()
        } else {
            format!("clean.failed|{}", fehlgeschlagen.join(", "))
        },
        targets: ergebnisse,
        duration_ms: start.elapsed().as_millis() as u64,
        cancelled: abgebrochen,
        registry_backup: sicherung,
    }
}

fn clean_target(
    ziel: &'static Target,
    anfrage: &CleanRequest,
    backup_dir: &Path,
    ctx: &RunContext,
    sicherung: &mut Option<String>,
) -> TargetClean {
    if ziel.requires_admin && !ctx.elevated {
        return TargetClean::skipped(ziel.key, ziel.category, "skip.needs_admin");
    }

    let mut ergebnis = TargetClean {
        key: ziel.key.to_string(),
        category: ziel.category,
        ok: true,
        skipped: false,
        skip_reason: String::new(),
        freed: 0,
        removed_items: 0,
        locked_items: 0,
        denied_items: 0,
        blocked_items: 0,
        errors: Vec::new(),
    };

    // Dienste anhalten – scheitert das, wird nichts gelöscht.
    let mut dienste_gestoppt = false;
    if !ziel.services.is_empty() && !anfrage.dry_run {
        let ausgabe = process::service("Stop", ziel.services);
        if !ausgabe.ok() {
            ergebnis.ok = false;
            ergebnis
                .errors
                .push(format!("error.service_stop|{}", ausgabe.message()));
            return ergebnis;
        }
        dienste_gestoppt = true;
    }

    match ziel.kind {
        TargetKind::Files(regeln) => {
            let mut gesehen: HashSet<PathBuf> = HashSet::new();

            for regel in regeln {
                if ctx.cancelled() {
                    break;
                }

                // Siehe scan.rs: bei rekursiven Mustern nur Dateien anfassen.
                // Die Ordnerstruktur bleibt dadurch erhalten - Anwendungen
                // legen ihre Cache-Ordner sonst nicht zuverlaessig neu an.
                let nur_dateien = regel.pattern.contains("**");

                let pfade = match fsutil::resolve_pattern(regel.pattern) {
                    Ok(p) => p,
                    Err(e) => {
                        ergebnis.errors.push(e);
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
                    if !fsutil::is_older_than(&pfad, regel.min_age_days) {
                        continue;
                    }
                    if regel.is_excluded(&pfad.to_string_lossy()) {
                        continue;
                    }
                    if nur_dateien && pfad.is_dir() {
                        continue;
                    }
                    if !path_selected(&pfad, anfrage) {
                        continue;
                    }

                    ctx.report(
                        Progress::new("clean", ziel, 0, 1)
                            .with_path(pfad.to_string_lossy().to_string()),
                    );

                    verbuche(&mut ergebnis, fsutil::remove_entry(&pfad, anfrage.dry_run));
                    gesehen.insert(pfad);
                }
            }
        }

        TargetKind::RecycleBin => {
            let (bytes, fehler) = recyclebin::empty(anfrage.dry_run);
            ergebnis.freed = bytes;
            ergebnis.removed_items = if bytes > 0 { 1 } else { 0 };
            ergebnis.errors.extend(fehler);
        }

        TargetKind::Command(argv) => {
            if anfrage.dry_run {
                ergebnis.removed_items = 1;
            } else {
                let ausgabe = process::run(argv);
                if ausgabe.ok() {
                    ergebnis.removed_items = 1;
                } else {
                    ergebnis.errors.push(ausgabe.message());
                }
            }
        }

        TargetKind::Installers => {
            // Nur ausdrücklich ausgewählte Dateien – niemals pauschal.
            if anfrage.only_paths.is_empty() {
                return TargetClean::skipped(
                    ziel.key,
                    ziel.category,
                    "skip.needs_explicit_selection",
                );
            }
            for pfad in &anfrage.only_paths {
                if ctx.cancelled() {
                    break;
                }
                let p = PathBuf::from(pfad);
                if !p.is_file() {
                    continue;
                }
                ctx.report(Progress::new("clean", ziel, 0, 1).with_path(pfad.clone()));
                verbuche(&mut ergebnis, fsutil::remove_entry(&p, anfrage.dry_run));
            }
        }

        TargetKind::Registry(regeln) => {
            // Ohne erfolgreiche Sicherung wird die Registry nicht angefasst.
            if !anfrage.dry_run && sicherung.is_none() {
                match registry::backup(regeln, backup_dir) {
                    Ok(pfad) => *sicherung = Some(pfad.to_string_lossy().to_string()),
                    Err(e) => {
                        ergebnis.ok = false;
                        ergebnis.errors.push(format!("error.registry_backup|{e}"));
                        return ergebnis;
                    }
                }
            }

            let (treffer, warnungen) = registry::scan_rules(regeln, &ctx.cancel);
            ergebnis.errors.extend(warnungen);

            let auswahl: Vec<_> = treffer
                .into_iter()
                .filter(|item| {
                    anfrage.only_paths.is_empty()
                        || anfrage.only_paths.iter().any(|p| p == &item.path)
                })
                .collect();

            let (entfernt, fehler) = registry::delete_items(&auswahl, anfrage.dry_run);
            ergebnis.removed_items = entfernt;
            ergebnis.errors.extend(fehler);
        }
    }

    if dienste_gestoppt {
        let ausgabe = process::service("Start", ziel.services);
        if !ausgabe.ok() {
            ergebnis
                .errors
                .push(format!("error.service_start|{}", ausgabe.message()));
        }
    }

    ergebnis.ok = ergebnis.errors.is_empty();
    ergebnis
}

/// Ausgang eines Löschversuchs im Ergebnis verbuchen.
///
/// Der Kern der Fehlerbehandlung: gesperrte Dateien und fehlende Rechte werden
/// **gezählt**, nicht als Fehler gemeldet. Andernfalls erzeugt jeder normale
/// Lauf eine Wand roter Zeilen für einen völlig erwartbaren Zustand – und
/// echte Fehler gehen darin unter.
fn verbuche(ergebnis: &mut TargetClean, ausgang: Result<Removal, String>) {
    match ausgang {
        Ok(Removal::Removed(bytes)) => {
            ergebnis.freed += bytes;
            ergebnis.removed_items += 1;
        }
        Ok(Removal::InUse) => ergebnis.locked_items += 1,
        Ok(Removal::Denied) => ergebnis.denied_items += 1,
        Ok(Removal::Blocked) => ergebnis.blocked_items += 1,
        // War schon weg – zwischen Analyse und Bereinigung verschwunden.
        Ok(Removal::Vanished) => {}
        Err(fehler) => ergebnis.errors.push(fehler),
    }
}

/// `true`, wenn der Pfad zur Auswahl gehört. Leere Auswahl = alles.
fn path_selected(pfad: &Path, anfrage: &CleanRequest) -> bool {
    if anfrage.only_paths.is_empty() {
        return true;
    }
    let text = pfad.to_string_lossy();
    anfrage
        .only_paths
        .iter()
        .any(|p| p.eq_ignore_ascii_case(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::runtime::CancelToken;
    use std::fs;

    fn ctx() -> RunContext {
        RunContext::new().with_elevated(false)
    }

    fn backup_dir() -> PathBuf {
        std::env::temp_dir().join("plane_clean_backup")
    }

    fn testordner(name: &str) -> PathBuf {
        let pfad = std::env::temp_dir().join(format!("plane_clean_{name}"));
        let _ = fs::remove_dir_all(&pfad);
        fs::create_dir_all(&pfad).unwrap();
        pfad
    }

    #[test]
    fn leere_auswahl_bereinigt_nichts() {
        let bericht = clean(&CleanRequest::default(), &backup_dir(), &ctx());
        assert!(bericht.success);
        assert!(bericht.targets.is_empty());
        assert_eq!(bericht.total_freed, 0);
    }

    #[test]
    fn unbekannte_ziele_werden_ignoriert() {
        let anfrage = CleanRequest {
            targets: vec!["gibtesnicht".into()],
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());
        assert!(bericht.targets.is_empty());
    }

    #[test]
    fn adminziele_werden_ohne_rechte_uebersprungen() {
        let anfrage = CleanRequest {
            targets: vec!["system.temp.windows".into()],
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());
        assert!(bericht.targets[0].skipped);
        assert_eq!(bericht.targets[0].skip_reason, "skip.needs_admin");
        assert!(bericht.success, "Übersprungen ist kein Fehlschlag");
    }

    #[test]
    fn trockenlauf_loescht_nichts_meldet_aber_bytes() {
        let ordner = testordner("trocken");
        let datei = ordner.join("gross.tmp");
        fs::write(&datei, vec![0u8; 4096]).unwrap();

        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            only_paths: vec![datei.to_string_lossy().to_string()],
            dry_run: true,
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());

        assert_eq!(bericht.total_freed, 4096);
        assert_eq!(bericht.total_removed, 1);
        assert!(datei.exists(), "Trockenlauf darf nicht löschen");

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn installationsdateien_ohne_auswahl_werden_uebersprungen() {
        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());
        assert!(bericht.targets[0].skipped);
        assert_eq!(
            bericht.targets[0].skip_reason,
            "skip.needs_explicit_selection"
        );
    }

    #[test]
    fn ausgewaehlte_installationsdatei_wird_geloescht() {
        let ordner = testordner("installer");
        let datei = ordner.join("setup.exe");
        fs::write(&datei, vec![0u8; 2048]).unwrap();

        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            only_paths: vec![datei.to_string_lossy().to_string()],
            dry_run: false,
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());

        assert!(bericht.success, "{}", bericht.error);
        assert_eq!(bericht.total_freed, 2048);
        assert!(!datei.exists());

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn nur_ausgewaehlte_pfade_werden_angefasst() {
        let ordner = testordner("auswahl");
        let a = ordner.join("a-setup.exe");
        let b = ordner.join("b-setup.exe");
        fs::write(&a, vec![0u8; 1000]).unwrap();
        fs::write(&b, vec![0u8; 1000]).unwrap();

        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            only_paths: vec![a.to_string_lossy().to_string()],
            dry_run: false,
        };
        clean(&anfrage, &backup_dir(), &ctx());

        assert!(!a.exists());
        assert!(b.exists(), "nicht ausgewählte Datei wurde gelöscht");

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn abbruch_stoppt_die_bereinigung() {
        let token = CancelToken::new();
        token.cancel();
        let kontext = RunContext::new().with_cancel(token).with_elevated(false);

        let anfrage = CleanRequest {
            targets: vec!["system.dns".into()],
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &kontext);

        assert!(bericht.cancelled);
        assert!(!bericht.success);
        assert!(bericht.targets.is_empty());
    }

    #[test]
    fn fortschritt_wird_je_ziel_gemeldet() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        let fertig = Arc::new(AtomicUsize::new(0));
        let kopie = Arc::clone(&fertig);
        let kontext = RunContext::new()
            .with_elevated(false)
            .with_progress(Box::new(move |p| {
                assert_eq!(p.phase, "clean");
                if p.done {
                    kopie.fetch_add(1, Ordering::SeqCst);
                }
            }));

        let anfrage = CleanRequest {
            targets: vec!["installers.downloads".into()],
            dry_run: true,
            ..Default::default()
        };
        clean(&anfrage, &backup_dir(), &kontext);
        assert_eq!(fertig.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn registry_im_trockenlauf_sichert_nicht() {
        let anfrage = CleanRequest {
            targets: vec!["registry.orphans".into()],
            dry_run: true,
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());
        assert!(
            bericht.registry_backup.is_none(),
            "Trockenlauf braucht keine Sicherung"
        );
    }

    #[test]
    fn pfadauswahl_ist_gross_klein_unabhaengig() {
        let anfrage = CleanRequest {
            targets: Vec::new(),
            only_paths: vec![r"C:\Temp\A.TMP".into()],
            dry_run: true,
        };
        assert!(path_selected(Path::new(r"c:\temp\a.tmp"), &anfrage));
        assert!(!path_selected(Path::new(r"c:\temp\b.tmp"), &anfrage));
    }

    #[test]
    fn leere_pfadauswahl_erfasst_alles() {
        let anfrage = CleanRequest::default();
        assert!(path_selected(Path::new(r"C:\beliebig\x.tmp"), &anfrage));
    }

    #[test]
    fn befehl_im_trockenlauf_wird_nicht_ausgefuehrt() {
        let anfrage = CleanRequest {
            targets: vec!["system.dns".into()],
            dry_run: true,
            ..Default::default()
        };
        let bericht = clean(&anfrage, &backup_dir(), &ctx());
        assert!(bericht.success);
        assert_eq!(bericht.targets[0].removed_items, 1);
    }
}
