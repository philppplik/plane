//! Argumentauswertung und Exitcodes.
//!
//! Die Struktur der Kommandozeile steckt in `clap`-Ableitungen, die *Bedeutung*
//! der Werte dagegen in freien Funktionen. Nur so lassen sich Sprachwahl,
//! Zielprüfung und Exitcode-Ableitung ohne Prozessstart testen.

use clap::{Parser, Subcommand};

use crate::engine::{self, Category, CleanReport, ScanReport};
use crate::i18n;

/// Exitcode des Programms.
///
/// Die Werte sind für Skripte gedacht: `0` heißt „nichts zu tun übrig“,
/// `1` „mindestens ein Ziel ist gescheitert“, `2` „falsch bedient“ und
/// `130` folgt der Unix-Konvention für Abbruch durch den Nutzer (SIGINT).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exitcode {
    Erfolg = 0,
    Fehlgeschlagen = 1,
    Bedienfehler = 2,
    Abgebrochen = 130,
}

impl Exitcode {
    pub fn wert(self) -> i32 {
        self as i32
    }
}

/// Bedienfehler mit fertiger Meldung für `stderr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bedienfehler(pub String);

impl std::fmt::Display for Bedienfehler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Bedienfehler {}

/// Kommandozeile von `plane-cli`.
#[derive(Debug, Parser)]
#[command(
    name = "plane-cli",
    version,
    about = "Plane - lightweight PC cleaner for Windows",
    long_about = None,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// Sprache der Ausgabe: `de` oder `en`.
    #[arg(long, global = true, value_name = "de|en")]
    pub lang: Option<String>,

    /// Keine Farben und keine Fortschrittsanimation verwenden.
    #[arg(long = "no-color", global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Befehl>,
}

/// Unterbefehle.
#[derive(Debug, Subcommand)]
pub enum Befehl {
    /// Alle Reinigungsziele auflisten.
    List {
        /// Nur eine Kategorie zeigen (system, browsers, apps, installers,
        /// recyclebin, registry).
        #[arg(long, value_name = "NAME")]
        category: Option<String>,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Analysieren, ohne etwas zu verändern.
    Scan {
        /// Zielschlüssel; ohne Angabe werden alle Ziele analysiert.
        #[arg(value_name = "ZIEL")]
        targets: Vec<String>,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
        /// Auch Ziele ohne Treffer anzeigen.
        #[arg(long)]
        all: bool,
    },
    /// Bereinigen. Ohne Zielangabe wird die empfohlene Auswahl verwendet.
    Clean {
        /// Zielschlüssel.
        #[arg(value_name = "ZIEL")]
        targets: Vec<String>,
        /// Nichts löschen, nur berichten, was passieren würde.
        #[arg(long = "dry-run")]
        dry_run: bool,
        /// Ohne Rückfrage ausführen.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Systeminformationen, Rechtestatus und Laufwerksbelegung.
    Info {
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Installierte Programme auflisten.
    Programs {
        /// Nur Namen, die diesen Text enthalten.
        #[arg(long, value_name = "TEXT")]
        filter: Option<String>,
        /// Auch geschützte Einträge zeigen.
        #[arg(long)]
        all: bool,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Ein Programm deinstallieren.
    ///
    /// Die Kennung stammt aus `plane-cli programs`. Plane startet den
    /// Deinstaller des Herstellers – es löscht nichts selbst.
    Uninstall {
        /// Kennung des Programms.
        #[arg(value_name = "KENNUNG")]
        id: String,
        /// Ohne Rückfrage ausführen.
        #[arg(short = 'y', long)]
        yes: bool,
        /// Nach Möglichkeit ohne Dialog deinstallieren.
        #[arg(long)]
        quiet: bool,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Windows-Einstellungen anzeigen und ändern.
    Tweaks {
        /// Einen Tweak einschalten.
        #[arg(long, value_name = "SCHLUESSEL")]
        on: Option<String>,
        /// Einen Tweak ausschalten.
        #[arg(long, value_name = "SCHLUESSEL")]
        off: Option<String>,
        /// Die letzte Änderung eines Tweaks zurücknehmen.
        #[arg(long, value_name = "SCHLUESSEL")]
        revert: Option<String>,
        /// Ausgabe als JSON.
        #[arg(long)]
        json: bool,
    },
    /// Die textbasierte Oberfläche starten.
    Tui,
}

/// Sprache bestimmen.
///
/// Reihenfolge: `--lang` schlägt alles, danach die Umgebung (`LANG`), zuletzt
/// die Rückfallsprache. Alles läuft durch [`i18n::normalize`], damit `de_AT`
/// und `de-DE` dasselbe ergeben und Unbekanntes auf Englisch landet statt
/// die Ausgabe zu leeren.
pub fn sprache_bestimmen(explizit: Option<&str>, umgebung: Option<&str>) -> String {
    if let Some(wunsch) = explizit {
        return i18n::normalize(wunsch);
    }
    match umgebung {
        Some(wert) if !wert.trim().is_empty() => i18n::normalize(wert),
        _ => i18n::FALLBACK.to_string(),
    }
}

/// Sprache aus der Prozessumgebung ableiten.
///
/// Windows setzt `LANG` nicht immer; `PLANE_LANG` erlaubt eine gezielte
/// Vorgabe, ohne die globale Umgebung zu verbiegen.
pub fn sprache_aus_umgebung() -> Option<String> {
    std::env::var("PLANE_LANG")
        .or_else(|_| std::env::var("LANG"))
        .ok()
}

/// Kategoriename in eine [`Category`] übersetzen.
///
/// Akzeptiert die stabilen Schlüssel des Katalogs, unabhängig von
/// Groß-/Kleinschreibung. Lokalisierte Namen bewusst nicht: Skripte müssen
/// unabhängig von der Systemsprache funktionieren.
pub fn kategorie_aus_name(name: &str) -> Result<Category, Bedienfehler> {
    let gesucht = name.trim().to_ascii_lowercase();
    Category::ALL
        .iter()
        .copied()
        .find(|k| k.key() == gesucht)
        .ok_or_else(|| {
            let bekannt: Vec<&str> = Category::ALL.iter().map(|k| k.key()).collect();
            Bedienfehler(format!(
                "Unbekannte Kategorie: {name}. Bekannt: {}",
                bekannt.join(", ")
            ))
        })
}

/// Zielschlüssel prüfen und normalisieren.
///
/// Ein Tippfehler darf nicht dazu führen, dass stillschweigend weniger
/// bereinigt wird als gewollt – deshalb bricht schon ein einziger unbekannter
/// Schlüssel den Lauf ab.
pub fn ziele_pruefen(keys: &[String]) -> Result<Vec<String>, Bedienfehler> {
    let mut geprueft = Vec::with_capacity(keys.len());
    for key in keys {
        let normalisiert = key.trim().to_ascii_lowercase();
        if engine::target_by_key(&normalisiert).is_none() {
            return Err(Bedienfehler(format!(
                "Unbekanntes Ziel: {key}. `plane-cli list` zeigt alle Schlüssel."
            )));
        }
        if !geprueft.contains(&normalisiert) {
            geprueft.push(normalisiert);
        }
    }
    Ok(geprueft)
}

/// Zielauswahl für die Analyse: leer bedeutet „alles“.
pub fn scanauswahl(keys: &[String]) -> Result<Vec<String>, Bedienfehler> {
    let geprueft = ziele_pruefen(keys)?;
    if geprueft.is_empty() {
        Ok(engine::TARGETS.iter().map(|z| z.key.to_string()).collect())
    } else {
        Ok(geprueft)
    }
}

/// Zielauswahl für die Bereinigung: leer bedeutet „empfohlene Auswahl“.
///
/// Bewusst **nicht** „alles“ – ein `plane-cli clean` ohne Argumente darf
/// niemals riskante Ziele mitnehmen.
pub fn cleanauswahl(keys: &[String]) -> Result<Vec<String>, Bedienfehler> {
    let geprueft = ziele_pruefen(keys)?;
    if geprueft.is_empty() {
        Ok(engine::default_selection()
            .iter()
            .map(|k| k.to_string())
            .collect())
    } else {
        Ok(geprueft)
    }
}

/// Exitcode aus einem Bereinigungsbericht ableiten.
pub fn exitcode_fuer_clean(bericht: &CleanReport) -> Exitcode {
    if bericht.cancelled {
        return Exitcode::Abgebrochen;
    }
    let ziel_gescheitert = bericht.targets.iter().any(|z| !z.ok);
    if !bericht.success || ziel_gescheitert || !bericht.error.is_empty() {
        Exitcode::Fehlgeschlagen
    } else {
        Exitcode::Erfolg
    }
}

/// Exitcode aus einem Analysebericht ableiten.
///
/// Eine Analyse kann nur abgebrochen werden – übersprungene Ziele sind kein
/// Fehler, sonst würde jeder Lauf ohne Administratorrechte CI-Pipelines rot
/// färben.
pub fn exitcode_fuer_scan(bericht: &ScanReport) -> Exitcode {
    if bericht.cancelled {
        Exitcode::Abgebrochen
    } else {
        Exitcode::Erfolg
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Category, TargetClean};
    use clap::CommandFactory;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    fn clean_bericht(success: bool, cancelled: bool, ziel_ok: bool) -> CleanReport {
        CleanReport {
            success,
            targets: vec![TargetClean {
                key: "system.temp.user".into(),
                category: Category::System,
                ok: ziel_ok,
                skipped: false,
                skip_reason: String::new(),
                freed: 0,
                removed_items: 0,
                locked_items: 0,
                denied_items: 0,
                errors: Vec::new(),
            }],
            total_freed: 0,
            total_removed: 0,
            total_locked: 0,
            total_denied: 0,
            duration_ms: 0,
            cancelled,
            registry_backup: None,
            error: String::new(),
        }
    }

    #[test]
    fn kommandozeilendefinition_ist_in_sich_stimmig() {
        // Deckt doppelte Kurzoptionen und fehlende Hilfetexte auf.
        Cli::command().debug_assert();
    }

    #[test]
    fn ohne_unterbefehl_bleibt_das_kommando_offen() {
        let cli = parse(&["plane-cli"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn globale_optionen_gelten_auch_hinter_dem_unterbefehl() {
        let cli = parse(&["plane-cli", "scan", "--lang", "de", "--no-color"]).unwrap();
        assert_eq!(cli.lang.as_deref(), Some("de"));
        assert!(cli.no_color);
    }

    #[test]
    fn clean_kennt_trockenlauf_und_zustimmung() {
        let cli = parse(&["plane-cli", "clean", "--dry-run", "-y"]).unwrap();
        match cli.command {
            Some(Befehl::Clean { dry_run, yes, .. }) => {
                assert!(dry_run);
                assert!(yes);
            }
            andere => panic!("falscher Unterbefehl: {andere:?}"),
        }
    }

    #[test]
    fn clean_nimmt_mehrere_ziele_entgegen() {
        let cli = parse(&[
            "plane-cli",
            "clean",
            "system.temp.user",
            "browser.chrome.cache",
        ])
        .unwrap();
        match cli.command {
            Some(Befehl::Clean { targets, .. }) => assert_eq!(targets.len(), 2),
            andere => panic!("falscher Unterbefehl: {andere:?}"),
        }
    }

    #[test]
    fn unbekannter_unterbefehl_ist_ein_bedienfehler() {
        assert!(parse(&["plane-cli", "putzen"]).is_err());
    }

    #[test]
    fn unbekannte_option_ist_ein_bedienfehler() {
        assert!(parse(&["plane-cli", "scan", "--gibtsnicht"]).is_err());
    }

    #[test]
    fn sprache_folgt_der_reihenfolge_argument_umgebung_rueckfall() {
        assert_eq!(sprache_bestimmen(Some("de-DE"), Some("en_US")), "de");
        assert_eq!(sprache_bestimmen(None, Some("de_AT")), "de");
        assert_eq!(sprache_bestimmen(None, None), i18n::FALLBACK);
        assert_eq!(sprache_bestimmen(None, Some("   ")), i18n::FALLBACK);
        // Unbekannte Sprachen dürfen nicht zu leerer Ausgabe führen.
        assert_eq!(sprache_bestimmen(Some("fr"), None), i18n::FALLBACK);
    }

    #[test]
    fn kategorien_werden_ueber_ihren_schluessel_erkannt() {
        assert_eq!(kategorie_aus_name("browsers").unwrap(), Category::Browsers);
        assert_eq!(
            kategorie_aus_name(" RecycleBin ").unwrap(),
            Category::RecycleBin
        );
    }

    #[test]
    fn unbekannte_kategorie_nennt_die_bekannten() {
        let fehler = kategorie_aus_name("musik").unwrap_err();
        assert!(fehler.0.contains("musik"));
        assert!(fehler.0.contains("browsers"));
    }

    #[test]
    fn unbekanntes_ziel_bricht_die_auswertung_ab() {
        let fehler =
            ziele_pruefen(&["system.temp.user".into(), "gibt.es.nicht".into()]).unwrap_err();
        assert!(fehler.0.contains("gibt.es.nicht"));
        assert!(fehler.0.contains("list"));
    }

    #[test]
    fn zielpruefung_entfernt_doppelte_und_normalisiert() {
        let geprueft =
            ziele_pruefen(&["SYSTEM.TEMP.USER".into(), " system.temp.user ".into()]).unwrap();
        assert_eq!(geprueft, vec!["system.temp.user".to_string()]);
    }

    #[test]
    fn scan_ohne_ziele_nimmt_alles() {
        assert_eq!(scanauswahl(&[]).unwrap().len(), engine::TARGETS.len());
    }

    #[test]
    fn clean_ohne_ziele_nimmt_die_empfohlene_auswahl() {
        let auswahl = cleanauswahl(&[]).unwrap();
        assert_eq!(auswahl.len(), engine::default_selection().len());
        assert!(auswahl.len() < engine::TARGETS.len());
        // Nichts Riskantes in der Standardauswahl.
        assert!(super::super::table::riskante_ziele(&auswahl).is_empty());
    }

    #[test]
    fn exitcode_null_bei_erfolg() {
        assert_eq!(
            exitcode_fuer_clean(&clean_bericht(true, false, true)),
            Exitcode::Erfolg
        );
    }

    #[test]
    fn exitcode_eins_wenn_ein_ziel_scheitert() {
        assert_eq!(
            exitcode_fuer_clean(&clean_bericht(true, false, false)),
            Exitcode::Fehlgeschlagen
        );
        assert_eq!(
            exitcode_fuer_clean(&clean_bericht(false, false, true)),
            Exitcode::Fehlgeschlagen
        );
    }

    #[test]
    fn exitcode_hundertdreissig_bei_abbruch() {
        // Abbruch schlägt Fehlschlag: der Nutzer hat entschieden, nicht ein Bug.
        assert_eq!(
            exitcode_fuer_clean(&clean_bericht(false, true, false)),
            Exitcode::Abgebrochen
        );
        assert_eq!(Exitcode::Abgebrochen.wert(), 130);
        assert_eq!(Exitcode::Bedienfehler.wert(), 2);
    }

    #[test]
    fn analyse_ohne_abbruch_meldet_erfolg() {
        let mut bericht = ScanReport {
            targets: Vec::new(),
            total_size: 0,
            total_items: 0,
            duration_ms: 0,
            cancelled: false,
        };
        assert_eq!(exitcode_fuer_scan(&bericht), Exitcode::Erfolg);
        bericht.cancelled = true;
        assert_eq!(exitcode_fuer_scan(&bericht), Exitcode::Abgebrochen);
    }
}
