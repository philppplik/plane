//! Datentypen der Reinigungs-Engine.
//!
//! Die Engine arbeitet zweistufig, wie professionelle Cleaner es tun:
//!
//! 1. **Analyse** ([`ScanReport`]) – es wird nichts verändert, nur ermittelt,
//!    was löschbar wäre und wie viel Platz das bringt.
//! 2. **Bereinigung** ([`CleanReport`]) – der Nutzer wählt aus dem
//!    Analyseergebnis aus, erst dann wird gelöscht.
//!
//! Diese Trennung ist der Grund, weshalb Plane niemals ungefragt löscht.

use serde::{Deserialize, Serialize};

/// Oberkategorie eines Reinigungsziels. Bestimmt die Gruppierung in UI und CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Windows selbst: Temp, Prefetch, Update-Cache, Absturzberichte …
    System,
    /// Browser-Caches und -Daten.
    Browsers,
    /// Caches einzelner Anwendungen.
    Apps,
    /// Gefundene Installationsdateien – werden nur vorgeschlagen.
    Installers,
    /// Papierkorb.
    RecycleBin,
    /// Registry-Einträge.
    Registry,
}

impl Category {
    pub const ALL: &'static [Category] = &[
        Category::System,
        Category::Browsers,
        Category::Apps,
        Category::Installers,
        Category::RecycleBin,
        Category::Registry,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Category::System => "system",
            Category::Browsers => "browsers",
            Category::Apps => "apps",
            Category::Installers => "installers",
            Category::RecycleBin => "recyclebin",
            Category::Registry => "registry",
        }
    }

    /// Übersetzungsschlüssel des Anzeigenamens.
    pub fn i18n_key(self) -> String {
        format!("category.{}", self.key())
    }
}

/// Risikostufe eines Ziels. Steuert Vorauswahl und Warnhinweise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    /// Reiner Cache. Wird vom System bei Bedarf neu erzeugt.
    Safe,
    /// Spürbare Nebenwirkung (Abmeldungen, längere Startzeiten, Verlauf weg).
    Notice,
    /// Datenverlust möglich. Niemals vorausgewählt.
    Caution,
}

impl Risk {
    pub fn key(self) -> &'static str {
        match self {
            Risk::Safe => "safe",
            Risk::Notice => "notice",
            Risk::Caution => "caution",
        }
    }
}

/// Was beim Löschen eines Dateiziels passieren soll.
#[derive(Debug, Clone, Copy)]
pub struct FileRule {
    /// Glob-Muster mit `%VAR%`-Platzhaltern. `*` gilt je Pfadsegment,
    /// `**` steigt rekursiv ab.
    ///
    /// Neben den Umgebungsvariablen von Windows kennt Plane die Kürzel
    /// `%DOWNLOADS%`, `%DESKTOP%` und `%DOCUMENTS%`. Diese werden über die
    /// Shell-Ordnerdefinition aufgelöst – `%USERPROFILE%\Downloads` wäre
    /// falsch, sobald OneDrive die Ordner umgeleitet hat.
    pub pattern: &'static str,
    /// Nur Einträge löschen, die mindestens so alt sind (0 = egal).
    /// Schützt z. B. Installer, die gerade heruntergeladen wurden.
    pub min_age_days: u32,
    /// Teilzeichenketten, die einen Treffer ausschließen (ohne Groß-/
    /// Kleinschreibung). Beispiel: `%TEMP%\Low` muss als Ordner bestehen
    /// bleiben, weil Programme mit niedriger Integritätsstufe ihn brauchen.
    pub exclude: &'static [&'static str],
}

impl FileRule {
    pub const fn new(pattern: &'static str) -> Self {
        Self {
            pattern,
            min_age_days: 0,
            exclude: &[],
        }
    }

    pub const fn older_than(mut self, tage: u32) -> Self {
        self.min_age_days = tage;
        self
    }

    pub const fn excluding(mut self, muster: &'static [&'static str]) -> Self {
        self.exclude = muster;
        self
    }

    /// `true`, wenn der Pfad durch [`FileRule::exclude`] geschützt ist.
    pub fn is_excluded(&self, pfad: &str) -> bool {
        if self.exclude.is_empty() {
            return false;
        }
        let klein = pfad.to_ascii_lowercase();
        self.exclude
            .iter()
            .any(|muster| klein.contains(&muster.to_ascii_lowercase()))
    }
}

/// Registry-Prüfung: welcher Schlüssel wird wie auf Verwaisung geprüft.
#[derive(Debug, Clone, Copy)]
pub struct RegistryRule {
    /// Wurzel: `"HKCU"` oder `"HKLM"` oder `"HKCR"`.
    pub hive: &'static str,
    /// Unterschlüssel.
    pub path: &'static str,
    /// Art der Verwaisungsprüfung.
    pub check: RegistryCheck,
}

/// Wie ein Registry-Eintrag auf Gültigkeit geprüft wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryCheck {
    /// Der **Wertname** ist ein Dateipfad; existiert die Datei nicht, ist der
    /// Eintrag verwaist (z. B. MUICache, SharedDLLs).
    ValueNameIsPath,
    /// Der **Wert** enthält einen Dateipfad (ggf. mit Argumenten).
    ValueIsPath { value: &'static str },
    /// Unterschlüssel, dessen Standardwert auf eine ProgID zeigt, die in
    /// HKCR nicht existiert (ungültige Dateizuordnung).
    SubkeyProgId,
    /// Uninstall-Eintrag, dessen Zielprogramm fehlt.
    UninstallEntry,
}

/// Konkretes Reinigungsziel.
///
/// Namen und Beschreibungen sind **nicht** enthalten – sie werden über
/// [`Target::i18n_name`] aus dem Sprachkatalog geholt, damit es sie genau
/// einmal gibt (siehe `crate::i18n`).
#[derive(Debug, Clone, Copy)]
pub struct Target {
    pub key: &'static str,
    pub category: Category,
    pub kind: TargetKind,
    pub risk: Risk,
    /// Ohne Administratorrechte nicht durchführbar.
    pub requires_admin: bool,
    /// Bei einer Standardanalyse vorausgewählt.
    pub default_enabled: bool,
    /// Dienste, die vor dem Löschen gestoppt und danach gestartet werden.
    pub services: &'static [&'static str],
    /// Prozesse, die das Ziel sperren. Laufen sie, wird gewarnt.
    pub blocking_processes: &'static [&'static str],
}

/// Art des Ziels.
#[derive(Debug, Clone, Copy)]
pub enum TargetKind {
    /// Dateien und Ordner nach Glob-Regeln.
    Files(&'static [FileRule]),
    /// Papierkorb aller Laufwerke (über die Shell-API).
    RecycleBin,
    /// Externer Befehl ohne Shell (z. B. `ipconfig /flushdns`).
    Command(&'static [&'static str]),
    /// Suche nach Installationsdateien in den üblichen Ablageorten.
    Installers,
    /// Registry-Prüfungen.
    Registry(&'static [RegistryRule]),
}

impl Target {
    pub fn i18n_name(&self) -> String {
        format!("target.{}.name", self.key)
    }

    pub fn i18n_description(&self) -> String {
        format!("target.{}.description", self.key)
    }

    /// `true`, wenn das Ziel ohne ausdrückliche Bestätigung laufen darf.
    pub fn is_safe(&self) -> bool {
        self.risk == Risk::Safe && !self.requires_admin
    }

    /// `true`, wenn nur analysiert und vorgeschlagen wird (nie automatisch
    /// gelöscht).
    pub fn is_suggestion_only(&self) -> bool {
        matches!(self.kind, TargetKind::Installers)
    }
}

/// Ein einzelner gefundener Eintrag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanItem {
    /// Anzeige- und Löschpfad. Bei Registry-Treffern der Schlüsselpfad.
    pub path: String,
    /// Belegter Platz in Bytes. Registry-Einträge zählen 0.
    pub size: u64,
    /// Zusatzinformation für die UI (z. B. „vor 214 Tagen geändert“).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub detail: String,
    /// Registry-Wertname, falls ein Wert (nicht ein Schlüssel) entfernt wird.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_name: Option<String>,
}

impl ScanItem {
    pub fn new(path: impl Into<String>, size: u64) -> Self {
        Self {
            path: path.into(),
            size,
            detail: String::new(),
            value_name: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }
}

/// Analyseergebnis eines einzelnen Ziels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetScan {
    pub key: String,
    pub category: Category,
    pub risk: Risk,
    pub requires_admin: bool,
    pub default_enabled: bool,
    pub suggestion_only: bool,
    /// Anzahl gefundener Einträge.
    pub item_count: usize,
    /// Summe der Größen in Bytes.
    pub size: u64,
    /// Gefundene Einträge. Aus Speichergründen begrenzt (siehe
    /// [`super::scan::MAX_ITEMS_PER_TARGET`]); `item_count` bleibt vollständig.
    pub items: Vec<ScanItem>,
    /// Ziel konnte nicht analysiert werden.
    pub skipped: bool,
    /// Grund für `skipped` als Übersetzungsschlüssel.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skip_reason: String,
    /// Nicht lesbare Pfade o. Ä. – blockieren die Analyse nicht.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl TargetScan {
    pub fn skipped(target: &Target, reason: impl Into<String>) -> Self {
        Self {
            key: target.key.to_string(),
            category: target.category,
            risk: target.risk,
            requires_admin: target.requires_admin,
            default_enabled: target.default_enabled,
            suggestion_only: target.is_suggestion_only(),
            item_count: 0,
            size: 0,
            items: Vec::new(),
            skipped: true,
            skip_reason: reason.into(),
            warnings: Vec::new(),
        }
    }
}

/// Gesamtergebnis der Analyse.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub targets: Vec<TargetScan>,
    /// Summe über alle Ziele.
    pub total_size: u64,
    pub total_items: usize,
    pub duration_ms: u64,
    /// Analyse wurde vom Nutzer abgebrochen.
    pub cancelled: bool,
}

impl ScanReport {
    /// Summe der Ziele, die standardmäßig vorausgewählt sind.
    pub fn selectable_size(&self) -> u64 {
        self.targets
            .iter()
            .filter(|t| t.default_enabled && !t.skipped)
            .map(|t| t.size)
            .sum()
    }
}

/// Ergebnis der Bereinigung eines Ziels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetClean {
    pub key: String,
    pub category: Category,
    pub ok: bool,
    pub skipped: bool,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skip_reason: String,
    /// Tatsächlich freigegebene Bytes – gemessen, nicht geschätzt.
    pub freed: u64,
    /// Anzahl entfernter Einträge.
    pub removed_items: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl TargetClean {
    pub fn skipped(key: &str, category: Category, reason: impl Into<String>) -> Self {
        Self {
            key: key.to_string(),
            category,
            ok: true,
            skipped: true,
            skip_reason: reason.into(),
            freed: 0,
            removed_items: 0,
            errors: Vec::new(),
        }
    }
}

/// Gesamtergebnis der Bereinigung.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanReport {
    pub success: bool,
    pub targets: Vec<TargetClean>,
    pub total_freed: u64,
    pub total_removed: usize,
    pub duration_ms: u64,
    pub cancelled: bool,
    /// Pfad der angelegten Registry-Sicherung, falls Registry bereinigt wurde.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_backup: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub error: String,
}

/// Fortschrittsmeldung. Wird während Analyse und Bereinigung laufend an die
/// Oberfläche geschickt, damit der Nutzer sieht, was gerade passiert.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progress {
    /// `"scan"` oder `"clean"`.
    pub phase: &'static str,
    /// Schlüssel des aktuellen Ziels.
    pub target_key: String,
    /// Übersetzungsschlüssel des Anzeigenamens.
    pub target_i18n: String,
    /// Nulbasierter Index des aktuellen Ziels.
    pub index: usize,
    /// Gesamtzahl der Ziele in diesem Lauf.
    pub total: usize,
    /// Fortschritt in Prozent (0–100), über den gesamten Lauf.
    pub percent: f64,
    /// Bisher gefundene bzw. freigegebene Bytes.
    pub bytes: u64,
    /// Aktuell bearbeiteter Pfad – für die Detailzeile in der UI.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_path: String,
    /// Ziel ist abgeschlossen.
    pub done: bool,
}

impl Progress {
    pub fn new(phase: &'static str, target: &Target, index: usize, total: usize) -> Self {
        let percent = if total == 0 {
            100.0
        } else {
            (index as f64 / total as f64) * 100.0
        };
        Self {
            phase,
            target_key: target.key.to_string(),
            target_i18n: target.i18n_name(),
            index,
            total,
            percent: (percent * 10.0).round() / 10.0,
            bytes: 0,
            current_path: String::new(),
            done: false,
        }
    }

    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = bytes;
        self
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.current_path = path.into();
        self
    }

    pub fn finished(mut self) -> Self {
        self.done = true;
        self.percent = if self.total == 0 {
            100.0
        } else {
            (((self.index + 1) as f64 / self.total as f64) * 1000.0).round() / 10.0
        };
        self
    }
}

/// Auswahl für einen Bereinigungslauf.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CleanRequest {
    /// Zu bereinigende Zielschlüssel.
    pub targets: Vec<String>,
    /// Nur diese Pfade bereinigen (leer = alle Treffer des Ziels).
    /// Ermöglicht die Einzelauswahl bei Installationsdateien.
    #[serde(default)]
    pub only_paths: Vec<String>,
    /// Nichts löschen, nur berichten, was passieren würde.
    #[serde(default)]
    pub dry_run: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kategorien_haben_eindeutige_schluessel() {
        let mut gesehen = Vec::new();
        for kategorie in Category::ALL {
            assert!(!gesehen.contains(&kategorie.key()));
            gesehen.push(kategorie.key());
        }
    }

    #[test]
    fn kategorie_serialisiert_kleingeschrieben() {
        let json = serde_json::to_string(&Category::RecycleBin).unwrap();
        assert_eq!(json, "\"recyclebin\"");
    }

    #[test]
    fn i18n_schluessel_folgen_dem_schema() {
        assert_eq!(Category::System.i18n_key(), "category.system");
    }

    #[test]
    fn filerule_baut_sich_verkettet_auf() {
        let regel = FileRule::new("%TEMP%/*")
            .older_than(7)
            .excluding(&["\\Low"]);
        assert_eq!(regel.min_age_days, 7);
        assert_eq!(regel.exclude, &["\\Low"]);
    }

    #[test]
    fn ausschluss_ignoriert_gross_klein() {
        let regel = FileRule::new("%TEMP%/*").excluding(&["\\Low"]);
        assert!(regel.is_excluded(r"C:\Users\p\AppData\Local\Temp\low"));
        assert!(regel.is_excluded(r"C:\Users\p\AppData\Local\Temp\LOW\x"));
        assert!(!regel.is_excluded(r"C:\Users\p\AppData\Local\Temp\andere"));
    }

    #[test]
    fn ohne_ausschlussliste_ist_nichts_ausgeschlossen() {
        assert!(!FileRule::new("%TEMP%/*").is_excluded(r"C:\beliebig\x"));
    }

    #[test]
    fn fortschritt_rechnet_prozente() {
        let ziel = Target {
            key: "x",
            category: Category::System,
            kind: TargetKind::Files(&[]),
            risk: Risk::Safe,
            requires_admin: false,
            default_enabled: true,
            services: &[],
            blocking_processes: &[],
        };

        let start = Progress::new("scan", &ziel, 0, 4);
        assert_eq!(start.percent, 0.0);
        assert!(!start.done);

        let ende = Progress::new("scan", &ziel, 3, 4).finished();
        assert_eq!(ende.percent, 100.0);
        assert!(ende.done);

        let mitte = Progress::new("clean", &ziel, 1, 4).finished();
        assert_eq!(mitte.percent, 50.0);
    }

    #[test]
    fn fortschritt_ohne_ziele_ist_vollstaendig() {
        let ziel = Target {
            key: "x",
            category: Category::System,
            kind: TargetKind::Files(&[]),
            risk: Risk::Safe,
            requires_admin: false,
            default_enabled: true,
            services: &[],
            blocking_processes: &[],
        };
        assert_eq!(Progress::new("scan", &ziel, 0, 0).percent, 100.0);
    }

    #[test]
    fn scanreport_summiert_nur_vorausgewaehlte_ziele() {
        let mache = |key: &str, default_enabled: bool, size: u64| TargetScan {
            key: key.into(),
            category: Category::System,
            risk: Risk::Safe,
            requires_admin: false,
            default_enabled,
            suggestion_only: false,
            item_count: 1,
            size,
            items: Vec::new(),
            skipped: false,
            skip_reason: String::new(),
            warnings: Vec::new(),
        };

        let report = ScanReport {
            targets: vec![mache("a", true, 100), mache("b", false, 900)],
            total_size: 1000,
            total_items: 2,
            duration_ms: 0,
            cancelled: false,
        };
        assert_eq!(report.selectable_size(), 100);
    }

    #[test]
    fn risikostufe_ordnet_sich_aufsteigend() {
        assert!(Risk::Safe < Risk::Notice);
        assert!(Risk::Notice < Risk::Caution);
    }
}
