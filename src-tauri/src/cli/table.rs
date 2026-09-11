//! Tabellen- und Zusammenfassungsaufbereitung.
//!
//! Dieses Modul erzeugt ausschließlich Zeichenketten und `Vec`s – es schreibt
//! nichts. Dadurch lässt sich die gesamte Ausgabe ohne Terminal prüfen, und
//! CLI wie TUI teilen sich dieselbe Aufbereitung.

use std::collections::BTreeMap;

use crate::engine::fsutil::format_bytes;
use crate::engine::{self, Category, CleanReport, Risk, ScanReport, TargetScan};

use super::text;

/// Ausrichtung einer Tabellenspalte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ausrichtung {
    Links,
    Rechts,
}

/// Einfache Textttabelle mit automatischer Spaltenbreite.
///
/// Bewusst keine Rahmenzeichen: die Ausgabe soll sich mit `awk`, `cut` und
/// `grep` weiterverarbeiten lassen. Zwei Leerzeichen trennen die Spalten.
#[derive(Debug, Clone)]
pub struct Tabelle {
    kopf: Vec<String>,
    ausrichtung: Vec<Ausrichtung>,
    zeilen: Vec<Vec<String>>,
}

impl Tabelle {
    /// Neue Tabelle mit Kopfzeile und Ausrichtung je Spalte.
    pub fn neu(kopf: &[&str], ausrichtung: &[Ausrichtung]) -> Self {
        Self {
            kopf: kopf.iter().map(|s| s.to_string()).collect(),
            ausrichtung: ausrichtung.to_vec(),
            zeilen: Vec::new(),
        }
    }

    /// Datenzeile anhängen. Zu kurze Zeilen werden mit Leerfeldern aufgefüllt.
    pub fn zeile(&mut self, felder: Vec<String>) {
        let mut felder = felder;
        while felder.len() < self.kopf.len() {
            felder.push(String::new());
        }
        self.zeilen.push(felder);
    }

    /// Anzahl der Datenzeilen (ohne Kopf und Trennlinie).
    pub fn zeilenzahl(&self) -> usize {
        self.zeilen.len()
    }

    /// Spaltenbreiten aus Kopf und Inhalt ermitteln.
    fn breiten(&self) -> Vec<usize> {
        let mut breiten: Vec<usize> = self.kopf.iter().map(|k| k.chars().count()).collect();
        for zeile in &self.zeilen {
            for (i, feld) in zeile.iter().enumerate() {
                if i < breiten.len() {
                    breiten[i] = breiten[i].max(feld.chars().count());
                }
            }
        }
        breiten
    }

    /// Tabelle als Zeilenliste rendern – inklusive Kopf und Trennlinie.
    pub fn rendern(&self) -> Vec<String> {
        let breiten = self.breiten();
        let mut ausgabe = Vec::with_capacity(self.zeilen.len() + 2);
        ausgabe.push(self.formatiere(&self.kopf, &breiten));
        ausgabe.push(
            breiten
                .iter()
                .map(|b| "-".repeat(*b))
                .collect::<Vec<_>>()
                .join("  "),
        );
        for zeile in &self.zeilen {
            ausgabe.push(self.formatiere(zeile, &breiten));
        }
        ausgabe
    }

    fn formatiere(&self, zeile: &[String], breiten: &[usize]) -> String {
        let felder: Vec<String> = zeile
            .iter()
            .enumerate()
            .map(|(i, feld)| {
                let breite = breiten.get(i).copied().unwrap_or(0);
                let fehlend = breite.saturating_sub(feld.chars().count());
                match self
                    .ausrichtung
                    .get(i)
                    .copied()
                    .unwrap_or(Ausrichtung::Links)
                {
                    Ausrichtung::Links => format!("{feld}{}", " ".repeat(fehlend)),
                    Ausrichtung::Rechts => format!("{}{feld}", " ".repeat(fehlend)),
                }
            })
            .collect();
        // Nachlaufende Leerzeichen stören beim Kopieren aus dem Terminal.
        felder.join("  ").trim_end().to_string()
    }
}

/// Tabelle aller Ziele für `plane-cli list`.
///
/// Optional auf eine Kategorie eingeschränkt. Die Reihenfolge folgt dem
/// Katalog, damit dieselbe Eingabe immer dieselbe Ausgabe liefert.
pub fn zielliste(lang: &str, kategorie: Option<Category>) -> Tabelle {
    let mut tabelle = Tabelle::neu(
        &["KEY", "KATEGORIE", "RISIKO", "NAME"],
        &[
            Ausrichtung::Links,
            Ausrichtung::Links,
            Ausrichtung::Links,
            Ausrichtung::Links,
        ],
    );
    for ziel in engine::TARGETS {
        if kategorie.is_some_and(|k| k != ziel.category) {
            continue;
        }
        tabelle.zeile(vec![
            ziel.key.to_string(),
            text::kategoriename(lang, ziel.category),
            format!(
                "{} {}",
                text::risikosymbol(ziel.risk),
                text::risikoname(lang, ziel.risk)
            ),
            text::zielname(lang, ziel.key),
        ]);
    }
    tabelle
}

/// Eine Zeile der nach Kategorien gruppierten Analyseübersicht.
#[derive(Debug, Clone, PartialEq)]
pub struct Kategoriesumme {
    pub kategorie: Category,
    pub size: u64,
    pub items: usize,
    pub ziele: usize,
}

/// Analyseergebnis nach Kategorie zusammenfassen.
///
/// Warum gruppiert: eine Liste aus 40 Einzelzielen erschlägt die Konsole. Die
/// Kategorieebene beantwortet die eigentliche Frage – wo liegt der Platz.
pub fn kategoriesummen(bericht: &ScanReport) -> Vec<Kategoriesumme> {
    let mut nach_kategorie: BTreeMap<Category, Kategoriesumme> = BTreeMap::new();
    for ziel in &bericht.targets {
        let eintrag = nach_kategorie
            .entry(ziel.category)
            .or_insert(Kategoriesumme {
                kategorie: ziel.category,
                size: 0,
                items: 0,
                ziele: 0,
            });
        eintrag.size += ziel.size;
        eintrag.items += ziel.item_count;
        eintrag.ziele += 1;
    }
    nach_kategorie.into_values().collect()
}

/// Ziele eines Analyseberichts, sortiert nach Größe (absteigend).
///
/// `alle = false` blendet leere und übersprungene Ziele aus – das ist der
/// Normalfall, weil sie nichts zur Entscheidung beitragen.
pub fn ziele_nach_groesse(bericht: &ScanReport, alle: bool) -> Vec<&TargetScan> {
    let mut ziele: Vec<&TargetScan> = bericht
        .targets
        .iter()
        .filter(|z| alle || (z.size > 0 || z.item_count > 0 || z.skipped))
        .collect();
    ziele.sort_by(|a, b| b.size.cmp(&a.size).then_with(|| a.key.cmp(&b.key)));
    ziele
}

/// Tabelle der Analyseergebnisse: Kategorien mit ihren Zielen.
pub fn scan_tabelle(lang: &str, bericht: &ScanReport, alle: bool) -> Tabelle {
    let mut tabelle = Tabelle::neu(
        &["", "ZIEL", "KATEGORIE", "GRÖSSE", "ANZAHL", "HINWEIS"],
        &[
            Ausrichtung::Links,
            Ausrichtung::Links,
            Ausrichtung::Links,
            Ausrichtung::Rechts,
            Ausrichtung::Rechts,
            Ausrichtung::Links,
        ],
    );
    for ziel in ziele_nach_groesse(bericht, alle) {
        let hinweis = if ziel.skipped {
            text::meldung(lang, &ziel.skip_reason)
        } else {
            text::meldungen(lang, &ziel.warnings).join("; ")
        };
        tabelle.zeile(vec![
            text::risikosymbol(ziel.risk).to_string(),
            text::zielname(lang, &ziel.key),
            text::kategoriename(lang, ziel.category),
            format_bytes(ziel.size),
            ziel.item_count.to_string(),
            hinweis,
        ]);
    }
    tabelle
}

/// Tabelle der Kategoriesummen.
pub fn kategorie_tabelle(lang: &str, bericht: &ScanReport) -> Tabelle {
    let mut tabelle = Tabelle::neu(
        &["KATEGORIE", "GRÖSSE", "ANZAHL"],
        &[Ausrichtung::Links, Ausrichtung::Rechts, Ausrichtung::Rechts],
    );
    for summe in kategoriesummen(bericht) {
        tabelle.zeile(vec![
            text::kategoriename(lang, summe.kategorie),
            format_bytes(summe.size),
            summe.items.to_string(),
        ]);
    }
    tabelle.zeile(vec![
        crate::i18n::t(lang, "cli.total"),
        format_bytes(bericht.total_size),
        bericht.total_items.to_string(),
    ]);
    tabelle
}

/// Tabelle des Bereinigungsergebnisses.
pub fn clean_tabelle(lang: &str, bericht: &CleanReport) -> Tabelle {
    let mut tabelle = Tabelle::neu(
        &["", "ZIEL", "FREIGEGEBEN", "EINTRÄGE", "HINWEIS"],
        &[
            Ausrichtung::Links,
            Ausrichtung::Links,
            Ausrichtung::Rechts,
            Ausrichtung::Rechts,
            Ausrichtung::Links,
        ],
    );
    for ziel in &bericht.targets {
        let mut hinweis = text::meldung(lang, &ziel.skip_reason);
        let fehler = text::meldungen(lang, &ziel.errors).join("; ");
        if !fehler.is_empty() {
            if hinweis.is_empty() {
                hinweis = fehler;
            } else {
                hinweis = format!("{hinweis}; {fehler}");
            }
        }
        tabelle.zeile(vec![
            text::statuszeichen(ziel.ok, ziel.skipped).to_string(),
            text::zielname(lang, &ziel.key),
            format_bytes(ziel.freed),
            ziel.removed_items.to_string(),
            hinweis,
        ]);
    }
    tabelle
}

/// Abschlusssatz eines Bereinigungslaufs.
///
/// Der Trockenlauf bekommt bewusst einen eigenen Satz: „freigegeben“ wäre
/// dort schlicht gelogen.
pub fn clean_zusammenfassung(lang: &str, bericht: &CleanReport, trockenlauf: bool) -> String {
    let bytes = format_bytes(bericht.total_freed);
    let anzahl = bericht.total_removed.to_string();
    let schluessel = if trockenlauf {
        "clean.dry_run_summary"
    } else {
        "clean.summary"
    };
    crate::i18n::format(lang, schluessel, &[&bytes, &anzahl])
}

/// Satz mit Gesamtergebnis der Analyse.
pub fn scan_zusammenfassung(lang: &str, bericht: &ScanReport) -> String {
    let bytes = format_bytes(bericht.total_size);
    let anzahl = bericht.total_items.to_string();
    crate::i18n::format(lang, "home.found_total", &[&bytes, &anzahl])
}

/// Zielschlüssel mit Risikostufe [`Risk::Caution`] aus einer Auswahl.
///
/// Diese Ziele müssen vor dem Löschen einzeln benannt werden – ein pauschales
/// „ja“ auf eine Sammelfrage ist hier zu wenig.
pub fn riskante_ziele(keys: &[String]) -> Vec<&'static str> {
    keys.iter()
        .filter_map(|k| engine::target_by_key(k))
        .filter(|z| z.risk == Risk::Caution)
        .map(|z| z.key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{TargetClean, TargetScan};

    fn scan_ziel(key: &str, kategorie: Category, size: u64, items: usize) -> TargetScan {
        TargetScan {
            key: key.into(),
            category: kategorie,
            risk: Risk::Safe,
            requires_admin: false,
            default_enabled: true,
            suggestion_only: false,
            item_count: items,
            size,
            items: Vec::new(),
            skipped: false,
            skip_reason: String::new(),
            warnings: Vec::new(),
        }
    }

    fn bericht(ziele: Vec<TargetScan>) -> ScanReport {
        let total_size = ziele.iter().map(|z| z.size).sum();
        let total_items = ziele.iter().map(|z| z.item_count).sum();
        ScanReport {
            targets: ziele,
            total_size,
            total_items,
            duration_ms: 0,
            cancelled: false,
        }
    }

    #[test]
    fn tabelle_richtet_spalten_aus() {
        let mut tabelle = Tabelle::neu(&["A", "B"], &[Ausrichtung::Links, Ausrichtung::Rechts]);
        tabelle.zeile(vec!["lang".into(), "1".into()]);
        let zeilen = tabelle.rendern();
        assert_eq!(zeilen[0], "A     B");
        assert_eq!(zeilen[1], "----  -");
        assert_eq!(zeilen[2], "lang  1");
    }

    #[test]
    fn tabelle_fuellt_fehlende_felder_auf() {
        let mut tabelle = Tabelle::neu(&["A", "B", "C"], &[Ausrichtung::Links; 3]);
        tabelle.zeile(vec!["x".into()]);
        assert_eq!(tabelle.zeilenzahl(), 1);
        // Kein Absturz und keine nachlaufenden Leerzeichen.
        assert_eq!(tabelle.rendern()[2], "x");
    }

    #[test]
    fn zielliste_enthaelt_alle_ziele() {
        let tabelle = zielliste("de", None);
        assert_eq!(tabelle.zeilenzahl(), engine::TARGETS.len());
    }

    #[test]
    fn zielliste_laesst_sich_auf_eine_kategorie_einschraenken() {
        let tabelle = zielliste("de", Some(Category::Browsers));
        let erwartet = engine::targets_in(Category::Browsers).count();
        assert_eq!(tabelle.zeilenzahl(), erwartet);
        assert!(erwartet > 0);
        assert!(erwartet < engine::TARGETS.len());
    }

    #[test]
    fn kategoriesummen_addieren_je_kategorie() {
        let b = bericht(vec![
            scan_ziel("a", Category::System, 100, 2),
            scan_ziel("b", Category::System, 50, 1),
            scan_ziel("c", Category::Browsers, 25, 5),
        ]);
        let summen = kategoriesummen(&b);
        assert_eq!(summen.len(), 2);
        let system = summen
            .iter()
            .find(|s| s.kategorie == Category::System)
            .unwrap();
        assert_eq!(system.size, 150);
        assert_eq!(system.items, 3);
        assert_eq!(system.ziele, 2);
    }

    #[test]
    fn ziele_werden_absteigend_nach_groesse_sortiert() {
        let b = bericht(vec![
            scan_ziel("klein", Category::System, 10, 1),
            scan_ziel("gross", Category::System, 900, 1),
        ]);
        let sortiert = ziele_nach_groesse(&b, false);
        assert_eq!(sortiert[0].key, "gross");
        assert_eq!(sortiert[1].key, "klein");
    }

    #[test]
    fn leere_ziele_werden_nur_mit_alle_gezeigt() {
        let b = bericht(vec![
            scan_ziel("voll", Category::System, 10, 1),
            scan_ziel("leer", Category::System, 0, 0),
        ]);
        assert_eq!(ziele_nach_groesse(&b, false).len(), 1);
        assert_eq!(ziele_nach_groesse(&b, true).len(), 2);
    }

    #[test]
    fn uebersprungene_ziele_bleiben_auch_ohne_treffer_sichtbar() {
        // Ein übersprungenes Ziel ist eine Information, kein leeres Ergebnis.
        let mut ziel = scan_ziel("gesperrt", Category::System, 0, 0);
        ziel.skipped = true;
        ziel.skip_reason = "skip.needs_admin".into();
        let b = bericht(vec![ziel]);
        assert_eq!(ziele_nach_groesse(&b, false).len(), 1);
    }

    #[test]
    fn kategorietabelle_endet_mit_der_gesamtsumme() {
        let b = bericht(vec![scan_ziel("a", Category::System, 2048, 4)]);
        let zeilen = kategorie_tabelle("de", &b).rendern();
        let letzte = zeilen.last().unwrap();
        assert!(letzte.starts_with("Gesamt"));
        assert!(letzte.contains("4"));
    }

    #[test]
    fn scan_tabelle_loest_uebersprungene_gruende_auf() {
        let mut ziel = scan_ziel("x", Category::System, 0, 0);
        ziel.skipped = true;
        ziel.skip_reason = "skip.needs_admin".into();
        let zeilen = scan_tabelle("de", &bericht(vec![ziel]), true).rendern();
        assert!(zeilen.iter().any(|z| z.contains("Administratorrechte")));
        assert!(!zeilen.iter().any(|z| z.contains("skip.needs_admin")));
    }

    #[test]
    fn clean_tabelle_zeigt_fehler_und_statuszeichen() {
        let b = CleanReport {
            success: false,
            targets: vec![TargetClean {
                key: "system.temp.user".into(),
                category: Category::System,
                ok: false,
                skipped: false,
                skip_reason: String::new(),
                freed: 0,
                removed_items: 0,
                errors: vec!["clean.failed|Zugriff verweigert".into()],
            }],
            total_freed: 0,
            total_removed: 0,
            duration_ms: 0,
            cancelled: false,
            registry_backup: None,
            error: String::new(),
        };
        let zeilen = clean_tabelle("de", &b).rendern();
        assert!(zeilen[2].starts_with('x'));
        assert!(zeilen[2].contains("Zugriff verweigert"));
    }

    #[test]
    fn zusammenfassung_unterscheidet_trockenlauf() {
        let b = CleanReport {
            success: true,
            targets: Vec::new(),
            total_freed: 1024,
            total_removed: 3,
            duration_ms: 0,
            cancelled: false,
            registry_backup: None,
            error: String::new(),
        };
        let echt = clean_zusammenfassung("de", &b, false);
        let trocken = clean_zusammenfassung("de", &b, true);
        assert_ne!(echt, trocken);
        assert!(trocken.contains("Simulation"));
        assert!(echt.contains("freigegeben"));
    }

    #[test]
    fn riskante_ziele_werden_erkannt() {
        let riskant: Vec<String> = engine::TARGETS
            .iter()
            .filter(|z| z.risk == Risk::Caution)
            .map(|z| z.key.to_string())
            .collect();
        assert!(!riskant.is_empty(), "Katalog ohne riskante Ziele?");
        assert_eq!(riskante_ziele(&riskant).len(), riskant.len());
        assert!(riskante_ziele(&["system.temp.user".to_string()]).is_empty());
        // Unbekannte Schlüssel werden hier stillschweigend ignoriert – sie
        // scheitern bereits bei der Argumentauswertung.
        assert!(riskante_ziele(&["gibt.es.nicht".to_string()]).is_empty());
    }
}
