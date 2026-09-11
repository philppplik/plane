//! Anwendungszustand mit Persistenz.
//!
//! Der Zustand wird als JSON im App-Config-Verzeichnis abgelegt (unter Windows
//! `%APPDATA%\com.ppaul.plane\state.json`).
//!
//! Bewusst kein SQLite: für eine Handvoll Einstellungen wäre eine
//! C-Abhängigkeit im Build unverhältnismäßig. Wird später eine
//! Reinigungs-Historie gespeichert, ist der Wechsel der richtige Zeitpunkt.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Dateiname im App-Config-Verzeichnis.
const STATE_FILE: &str = "state.json";

/// Unterverzeichnis für Registry-Sicherungen.
pub const BACKUP_DIR: &str = "backups";

/// Zustände der Oberfläche. Ersetzt den früheren untypisierten `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Screen {
    Welcome,
    Home,
    About,
}

impl Screen {
    pub fn as_str(self) -> &'static str {
        match self {
            Screen::Welcome => "welcome",
            Screen::Home => "home",
            Screen::About => "about",
        }
    }

    /// Name des Events, das beim Wechsel emittiert wird.
    pub fn event(self) -> &'static str {
        match self {
            Screen::Welcome => "navigate-welcome",
            Screen::Home => "navigate-home",
            Screen::About => "navigate-about",
        }
    }
}

/// Erscheinungsbild der Oberfläche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Light,
    Dark,
}

impl Theme {
    pub fn as_str(self) -> &'static str {
        match self {
            Theme::System => "system",
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }
}

/// Benutzereinstellungen.
///
/// Alle Felder haben `serde(default)`, damit eine ältere Zustandsdatei nach
/// einem Update weiterhin lesbar bleibt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Sprachkürzel (`de`, `en`).
    pub language: String,
    pub theme: Theme,
    /// Vor riskanten Zielen nachfragen. Standard: an.
    pub confirm_risky: bool,
    /// Läufe standardmäßig nur simulieren.
    pub dry_run_default: bool,
    /// Zuletzt gewählte Ziele – wird beim nächsten Start wiederhergestellt.
    pub selected_targets: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: default_language(),
            theme: Theme::System,
            confirm_risky: true,
            dry_run_default: false,
            selected_targets: Vec::new(),
        }
    }
}

/// Systemsprache erraten, sonst Englisch.
fn default_language() -> String {
    for variable in ["LANG", "LC_ALL", "LANGUAGE"] {
        if let Ok(wert) = std::env::var(variable) {
            let normalisiert = crate::i18n::normalize(&wert);
            if normalisiert != crate::i18n::FALLBACK {
                return normalisiert;
            }
        }
    }

    #[cfg(windows)]
    {
        // Windows liefert die Anzeigesprache nicht über Umgebungsvariablen.
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(schluessel) = hkcu.open_subkey(r"Control Panel\International") {
            if let Ok(name) = schluessel.get_value::<String, _>("LocaleName") {
                return crate::i18n::normalize(&name);
            }
        }
    }

    crate::i18n::FALLBACK.to_string()
}

/// Anwendungszustand.
///
/// `current_screen` ist reiner Laufzeitzustand und wird nicht persistiert.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppState {
    pub has_seen_welcome: bool,
    pub settings: Settings,
    #[serde(skip, default = "default_screen")]
    pub current_screen: Screen,
}

fn default_screen() -> Screen {
    Screen::Welcome
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            has_seen_welcome: false,
            settings: Settings::default(),
            current_screen: Screen::Welcome,
        }
    }
}

impl AppState {
    /// Bildschirm, mit dem die App starten soll.
    pub fn start_screen(&self) -> Screen {
        if self.has_seen_welcome {
            Screen::Home
        } else {
            Screen::Welcome
        }
    }

    /// Zustand aus dem Verzeichnis laden.
    ///
    /// Fehlende oder beschädigte Dateien führen zum Standardzustand – ein
    /// kaputter Cache darf den Start nicht verhindern.
    pub fn load(config_dir: &Path) -> Self {
        let pfad = config_dir.join(STATE_FILE);
        match fs::read_to_string(&pfad) {
            Ok(inhalt) => serde_json::from_str(&inhalt).unwrap_or_else(|e| {
                eprintln!(
                    "Zustand {} nicht lesbar ({e}), verwende Standard",
                    pfad.display()
                );
                Self::default()
            }),
            Err(_) => Self::default(),
        }
    }

    /// Zustand in das Verzeichnis schreiben.
    pub fn save(&self, config_dir: &Path) -> Result<(), String> {
        fs::create_dir_all(config_dir)
            .map_err(|e| format!("Konfigurationsverzeichnis nicht anlegbar: {e}"))?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Zustand nicht serialisierbar: {e}"))?;
        fs::write(config_dir.join(STATE_FILE), json)
            .map_err(|e| format!("Zustand nicht speicherbar: {e}"))
    }
}

/// Pfad der Zustandsdatei – für Diagnose und Tests.
pub fn state_path(config_dir: &Path) -> PathBuf {
    config_dir.join(STATE_FILE)
}

/// Ordner für Registry-Sicherungen.
pub fn backup_path(config_dir: &Path) -> PathBuf {
    config_dir.join(BACKUP_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let pfad = std::env::temp_dir().join(format!("plane_state_{name}"));
        let _ = fs::remove_dir_all(&pfad);
        pfad
    }

    #[test]
    fn default_startet_auf_dem_welcome_screen() {
        let state = AppState::default();
        assert!(!state.has_seen_welcome);
        assert_eq!(state.current_screen, Screen::Welcome);
        assert_eq!(state.start_screen(), Screen::Welcome);
    }

    #[test]
    fn nach_erstem_start_beginnt_die_app_auf_home() {
        let state = AppState {
            has_seen_welcome: true,
            ..Default::default()
        };
        assert_eq!(state.start_screen(), Screen::Home);
    }

    #[test]
    fn screen_bezeichner_und_events_sind_stabil() {
        assert_eq!(Screen::Welcome.as_str(), "welcome");
        assert_eq!(Screen::Home.event(), "navigate-home");
        assert_eq!(Screen::About.event(), "navigate-about");
    }

    #[test]
    fn standardeinstellungen_sind_vorsichtig() {
        let s = Settings::default();
        assert!(s.confirm_risky, "Nachfragen muss standardmäßig an sein");
        assert!(!s.dry_run_default);
        assert!(crate::i18n::is_supported(&s.language));
    }

    #[test]
    fn zustand_ueberlebt_speichern_und_laden() {
        let dir = temp_dir("roundtrip");
        let mut state = AppState {
            has_seen_welcome: true,
            ..Default::default()
        };
        state.settings.language = "de".into();
        state.settings.theme = Theme::Dark;
        state.settings.dry_run_default = true;
        state.settings.selected_targets = vec!["system.temp.user".into()];
        state.current_screen = Screen::About;

        state.save(&dir).unwrap();
        let geladen = AppState::load(&dir);

        assert!(geladen.has_seen_welcome);
        assert_eq!(geladen.settings.language, "de");
        assert_eq!(geladen.settings.theme, Theme::Dark);
        assert!(geladen.settings.dry_run_default);
        assert_eq!(geladen.settings.selected_targets, vec!["system.temp.user"]);
        // Laufzeitzustand wird bewusst nicht persistiert.
        assert_eq!(geladen.current_screen, Screen::Welcome);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn fehlende_datei_ergibt_standardzustand() {
        assert!(!AppState::load(&temp_dir("fehlt")).has_seen_welcome);
    }

    #[test]
    fn beschaedigte_datei_verhindert_den_start_nicht() {
        let dir = temp_dir("kaputt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(state_path(&dir), "{kein json").unwrap();
        assert!(!AppState::load(&dir).has_seen_welcome);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn aeltere_zustandsdatei_bleibt_lesbar() {
        // Vorwärtskompatibilität: unbekannte/fehlende Felder dürfen nicht
        // dazu führen, dass der gesamte Zustand verworfen wird.
        let dir = temp_dir("alt");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            state_path(&dir),
            r#"{"has_seen_welcome": true, "settings": {"language": "de"}}"#,
        )
        .unwrap();

        let geladen = AppState::load(&dir);
        assert!(geladen.has_seen_welcome);
        assert_eq!(geladen.settings.language, "de");
        assert!(geladen.settings.confirm_risky, "Standardwert fehlt");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sicherungsordner_liegt_neben_dem_zustand() {
        let dir = Path::new("C:\\beliebig");
        assert_eq!(backup_path(dir), dir.join("backups"));
    }

    #[test]
    fn theme_serialisiert_kleingeschrieben() {
        assert_eq!(serde_json::to_string(&Theme::Dark).unwrap(), "\"dark\"");
        assert_eq!(Theme::System.as_str(), "system");
    }
}
