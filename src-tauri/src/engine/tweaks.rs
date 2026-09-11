//! Windows-Einstellungen ändern („Plane Tweaker").
//!
//! # Haltung
//!
//! Es gibt viele Werkzeuge, die Dutzende Registry-Werte auf einmal umstellen.
//! Einige davon haben dokumentiert Systeme beschädigt: das dauerhafte
//! Abschalten von Windows Update bricht nachweislich den Microsoft Store und
//! `winget`, und in Berichten ließ es sich nachträglich nicht wieder
//! aktivieren. Plane bietet deshalb eine **kuratierte, kleine Auswahl** an —
//! jeder Eintrag umkehrbar, jeder mit benannter Nebenwirkung.
//!
//! Ausdrücklich **nicht** enthalten und auch nicht erwünscht:
//! Windows Update deaktivieren, Defender abschalten, Edge oder OneDrive
//! entfernen, BitLocker deaktivieren, Dienste massenhaft abschalten, IPv6
//! global abschalten.
//!
//! # Warum nicht einfach Standardwerte zurückschreiben
//!
//! Verbreitete Werkzeuge hinterlegen je Tweak einen festen „Originalwert".
//! Das ist falsch: wer `MenuShowDelay` selbst auf `0` gestellt hatte, bekommt
//! beim Zurücknehmen den Windows-Standard `400` — nicht seinen Wert. Plane
//! liest deshalb vor jeder Änderung den **tatsächlichen** Ist-Zustand und
//! legt ihn in einem Journal ab. Zurücknehmen heißt: genau diesen Zustand
//! wiederherstellen, inklusive „der Wert existierte vorher gar nicht".

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Gruppierung in der Oberfläche.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakGroup {
    Privacy,
    Explorer,
    Taskbar,
    Performance,
    System,
}

impl TweakGroup {
    pub const ALL: &'static [TweakGroup] = &[
        TweakGroup::Privacy,
        TweakGroup::Explorer,
        TweakGroup::Taskbar,
        TweakGroup::Performance,
        TweakGroup::System,
    ];

    pub fn key(self) -> &'static str {
        match self {
            TweakGroup::Privacy => "privacy",
            TweakGroup::Explorer => "explorer",
            TweakGroup::Taskbar => "taskbar",
            TweakGroup::Performance => "performance",
            TweakGroup::System => "system",
        }
    }

    pub fn i18n_key(self) -> String {
        format!("tweakgroup.{}", self.key())
    }
}

/// Was nach dem Anwenden nötig ist, damit die Änderung greift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Apply {
    /// Wirkt sofort.
    Immediate,
    /// Explorer muss neu starten.
    Explorer,
    /// Anmeldung oder Neustart nötig.
    Restart,
}

/// Ein einzelner Registry-Wert, den ein Tweak setzt.
#[derive(Debug, Clone, Copy)]
pub struct RegValue {
    /// `"HKCU"` oder `"HKLM"`.
    pub hive: &'static str,
    pub path: &'static str,
    /// Wertname; leerer String bedeutet den Standardwert des Schlüssels.
    pub name: &'static str,
    /// Gewünschter Zustand, wenn der Tweak aktiv ist.
    pub on: RegData,
    /// Zustand, wenn der Tweak *nicht* aktiv ist – nur für die
    /// Zustandserkennung, **nicht** zum Zurücknehmen. Dafür dient das Journal.
    pub off: RegData,
}

/// Ein Registry-Wert mitsamt Typ.
///
/// Der Typ gehört dazu: `"0"` als Zeichenkette und `0` als Zahl sind zwei
/// verschiedene Dinge, und ein Vergleich ohne Typ liefert zufällige
/// Ergebnisse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegData {
    Dword(u32),
    Text(&'static str),
    /// Der Wert soll nicht existieren.
    Absent,
}

/// Ein Tweak.
#[derive(Debug, Clone, Copy)]
pub struct Tweak {
    pub key: &'static str,
    pub group: TweakGroup,
    pub values: &'static [RegValue],
    pub requires_admin: bool,
    pub apply: Apply,
    /// Nur unter Windows 11 wirksam (Buildnummer >= 22000).
    pub windows11_only: bool,
}

impl Tweak {
    pub fn i18n_name(&self) -> String {
        format!("tweak.{}.name", self.key)
    }

    pub fn i18n_description(&self) -> String {
        format!("tweak.{}.description", self.key)
    }

    /// Übersetzungsschlüssel der Nebenwirkung.
    pub fn i18n_effect(&self) -> String {
        format!("tweak.{}.effect", self.key)
    }
}

/// Zustand eines Tweaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TweakState {
    /// Alle Werte stehen auf „an".
    On,
    /// Alle Werte stehen auf „aus".
    Off,
    /// Teils/teils, oder nicht lesbar.
    ///
    /// Verbreitete Werkzeuge melden hier schlicht „aus" und verleiten damit
    /// zum wiederholten Anwenden. Der dritte Zustand ist ehrlicher.
    Mixed,
}

/// Zustandsbericht eines Tweaks für die Oberfläche.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TweakStatus {
    pub key: String,
    pub group: TweakGroup,
    pub state: TweakState,
    pub requires_admin: bool,
    pub apply: Apply,
    /// Auf diesem System wirkungslos (z. B. Windows-11-Tweak unter Win10).
    pub unsupported: bool,
    /// Von einer Gruppenrichtlinie überlagert – eine Änderung hier hätte
    /// keinen dauerhaften Effekt.
    pub managed: bool,
}

// ---------------------------------------------------------------------------
// Katalog
// ---------------------------------------------------------------------------

const fn hkcu(path: &'static str, name: &'static str, on: RegData, off: RegData) -> RegValue {
    RegValue {
        hive: "HKCU",
        path,
        name,
        on,
        off,
    }
}

const fn hklm(path: &'static str, name: &'static str, on: RegData, off: RegData) -> RegValue {
    RegValue {
        hive: "HKLM",
        path,
        name,
        on,
        off,
    }
}

const EXPLORER_ADVANCED: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";

// --- Datenschutz ---
const WERBE_ID: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
    "Enabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const ZUGESCHNITTEN: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Windows\CurrentVersion\Privacy",
    "TailoredExperiencesWithDiagnosticDataEnabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const APP_START_VERFOLGUNG: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "Start_TrackProgs",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const EINGABE_TELEMETRIE: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Input\TIPC",
    "Enabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const FEEDBACK: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Siuf\Rules",
    "NumberOfSIUFInPeriod",
    RegData::Dword(0),
    RegData::Absent,
)];
const VORGESCHLAGENE_APPS: &[RegValue] = &[hklm(
    r"SOFTWARE\Policies\Microsoft\Windows\CloudContent",
    "DisableWindowsConsumerFeatures",
    RegData::Dword(1),
    RegData::Absent,
)];

// --- Explorer ---
const DATEIENDUNGEN: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "HideFileExt",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const VERSTECKTE_DATEIEN: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "Hidden",
    RegData::Dword(1),
    RegData::Dword(2),
)];
const KLASSISCHES_KONTEXTMENUE: &[RegValue] = &[hkcu(
    r"Software\Classes\CLSID\{86ca1aa0-34aa-4e8b-a509-50c905bae2a2}\InprocServer32",
    "",
    RegData::Text(""),
    RegData::Absent,
)];
const EXPLORER_ZU_DIESEM_PC: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "LaunchTo",
    RegData::Dword(1),
    RegData::Dword(2),
)];
const TASK_BEENDEN: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced\TaskbarDeveloperSettings",
    "TaskbarEndTask",
    RegData::Dword(1),
    RegData::Absent,
)];

// --- Taskleiste ---
const TASKLEISTE_LINKS: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "TaskbarAl",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const SUCHFELD_AUS: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Windows\CurrentVersion\Search",
    "SearchboxTaskbarMode",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const TASKANSICHT_AUS: &[RegValue] = &[hkcu(
    EXPLORER_ADVANCED,
    "ShowTaskViewButton",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const BING_SUCHE_AUS: &[RegValue] = &[hkcu(
    r"Software\Microsoft\Windows\CurrentVersion\Search",
    "BingSearchEnabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];

// --- Leistung ---
const GAME_DVR: &[RegValue] = &[hkcu(
    r"System\GameConfigStore",
    "GameDVR_Enabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];
const SCHNELLSTART: &[RegValue] = &[hklm(
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Power",
    "HiberbootEnabled",
    RegData::Dword(0),
    RegData::Dword(1),
)];

// --- System ---
const LANGE_PFADE: &[RegValue] = &[hklm(
    r"SYSTEM\CurrentControlSet\Control\FileSystem",
    "LongPathsEnabled",
    RegData::Dword(1),
    RegData::Dword(0),
)];
const AUSFUEHRLICHER_STATUS: &[RegValue] = &[hklm(
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System",
    "VerboseStatus",
    RegData::Dword(1),
    RegData::Absent,
)];

/// Alle angebotenen Tweaks.
pub static TWEAKS: &[Tweak] = &[
    // --- Datenschutz ---
    Tweak {
        key: "privacy.advertising_id",
        group: TweakGroup::Privacy,
        values: WERBE_ID,
        requires_admin: false,
        apply: Apply::Immediate,
        windows11_only: false,
    },
    Tweak {
        key: "privacy.tailored",
        group: TweakGroup::Privacy,
        values: ZUGESCHNITTEN,
        requires_admin: false,
        apply: Apply::Immediate,
        windows11_only: false,
    },
    Tweak {
        key: "privacy.app_tracking",
        group: TweakGroup::Privacy,
        values: APP_START_VERFOLGUNG,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "privacy.input_telemetry",
        group: TweakGroup::Privacy,
        values: EINGABE_TELEMETRIE,
        requires_admin: false,
        apply: Apply::Immediate,
        windows11_only: false,
    },
    Tweak {
        key: "privacy.feedback",
        group: TweakGroup::Privacy,
        values: FEEDBACK,
        requires_admin: false,
        apply: Apply::Immediate,
        windows11_only: false,
    },
    Tweak {
        key: "privacy.suggested_apps",
        group: TweakGroup::Privacy,
        values: VORGESCHLAGENE_APPS,
        requires_admin: true,
        apply: Apply::Restart,
        windows11_only: false,
    },
    // --- Explorer ---
    Tweak {
        key: "explorer.file_extensions",
        group: TweakGroup::Explorer,
        values: DATEIENDUNGEN,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "explorer.hidden_files",
        group: TweakGroup::Explorer,
        values: VERSTECKTE_DATEIEN,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "explorer.classic_menu",
        group: TweakGroup::Explorer,
        values: KLASSISCHES_KONTEXTMENUE,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: true,
    },
    Tweak {
        key: "explorer.launch_to_pc",
        group: TweakGroup::Explorer,
        values: EXPLORER_ZU_DIESEM_PC,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "explorer.end_task",
        group: TweakGroup::Explorer,
        values: TASK_BEENDEN,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: true,
    },
    // --- Taskleiste ---
    Tweak {
        key: "taskbar.align_left",
        group: TweakGroup::Taskbar,
        values: TASKLEISTE_LINKS,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: true,
    },
    Tweak {
        key: "taskbar.hide_search",
        group: TweakGroup::Taskbar,
        values: SUCHFELD_AUS,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "taskbar.hide_taskview",
        group: TweakGroup::Taskbar,
        values: TASKANSICHT_AUS,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    Tweak {
        key: "taskbar.no_bing",
        group: TweakGroup::Taskbar,
        values: BING_SUCHE_AUS,
        requires_admin: false,
        apply: Apply::Explorer,
        windows11_only: false,
    },
    // --- Leistung ---
    Tweak {
        key: "performance.game_dvr",
        group: TweakGroup::Performance,
        values: GAME_DVR,
        requires_admin: false,
        apply: Apply::Restart,
        windows11_only: false,
    },
    Tweak {
        key: "performance.fast_startup",
        group: TweakGroup::Performance,
        values: SCHNELLSTART,
        requires_admin: true,
        apply: Apply::Restart,
        windows11_only: false,
    },
    // --- System ---
    Tweak {
        key: "system.long_paths",
        group: TweakGroup::System,
        values: LANGE_PFADE,
        requires_admin: true,
        apply: Apply::Restart,
        windows11_only: false,
    },
    Tweak {
        key: "system.verbose_status",
        group: TweakGroup::System,
        values: AUSFUEHRLICHER_STATUS,
        requires_admin: true,
        apply: Apply::Restart,
        windows11_only: false,
    },
];

/// Tweak zu einem Schlüssel.
pub fn tweak_by_key(key: &str) -> Option<&'static Tweak> {
    let key = key.trim().to_ascii_lowercase();
    TWEAKS.iter().find(|t| t.key == key)
}

// ---------------------------------------------------------------------------
// Journal
// ---------------------------------------------------------------------------

/// Ein gesicherter Wert, so wie er **vor** der Änderung war.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub hive: String,
    pub path: String,
    pub name: String,
    /// Existierte der Wert vorher überhaupt?
    pub existed: bool,
    /// Registry-Typ als Zahl (`REG_DWORD` = 4, `REG_SZ` = 1).
    pub value_type: u32,
    /// Zahlenwert, falls DWORD.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dword: Option<u32>,
    /// Textwert, falls REG_SZ.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Ein Journaleintrag: alles, was ein Tweak verändert hat.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    pub tweak: String,
    /// Unix-Zeit in Sekunden.
    pub timestamp: u64,
    /// Wurde der Tweak ein- oder ausgeschaltet?
    pub enabled: bool,
    pub before: Vec<Snapshot>,
}

/// Dateiname des Journals im Konfigurationsverzeichnis.
pub const JOURNAL_FILE: &str = "tweaks.json";

pub fn journal_path(config_dir: &Path) -> PathBuf {
    config_dir.join(JOURNAL_FILE)
}

/// Journal laden. Eine beschädigte Datei ergibt ein leeres Journal – sie darf
/// die Anwendung nicht blockieren.
pub fn journal_laden(config_dir: &Path) -> Vec<JournalEntry> {
    std::fs::read_to_string(journal_path(config_dir))
        .ok()
        .and_then(|inhalt| serde_json::from_str(&inhalt).ok())
        .unwrap_or_default()
}

/// Journal speichern.
pub fn journal_speichern(config_dir: &Path, eintraege: &[JournalEntry]) -> Result<(), String> {
    std::fs::create_dir_all(config_dir)
        .map_err(|e| format!("Konfigurationsverzeichnis nicht anlegbar: {e}"))?;
    let json = serde_json::to_string_pretty(eintraege)
        .map_err(|e| format!("Journal nicht serialisierbar: {e}"))?;
    std::fs::write(journal_path(config_dir), json)
        .map_err(|e| format!("Journal nicht speicherbar: {e}"))
}

/// Den jüngsten Journaleintrag eines Tweaks finden.
pub fn letzter_eintrag<'a>(journal: &'a [JournalEntry], tweak: &str) -> Option<&'a JournalEntry> {
    journal
        .iter()
        .filter(|e| e.tweak == tweak)
        .max_by_key(|e| e.timestamp)
}

fn jetzt() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use winreg::enums::*;
    use winreg::types::FromRegValue;
    use winreg::RegKey;

    fn wurzel(hive: &str) -> Option<RegKey> {
        Some(match hive {
            "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
            "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
            _ => return None,
        })
    }

    /// Den aktuellen Zustand eines Werts festhalten.
    pub fn lies_snapshot(wert: &RegValue) -> Snapshot {
        let mut schnappschuss = Snapshot {
            hive: wert.hive.to_string(),
            path: wert.path.to_string(),
            name: wert.name.to_string(),
            existed: false,
            value_type: 0,
            dword: None,
            text: None,
        };

        let Some(hive) = wurzel(wert.hive) else {
            return schnappschuss;
        };
        let Ok(schluessel) = hive.open_subkey_with_flags(wert.path, KEY_READ) else {
            return schnappschuss;
        };
        let Ok(roh) = schluessel.get_raw_value(wert.name) else {
            return schnappschuss;
        };

        schnappschuss.existed = true;
        schnappschuss.value_type = roh.vtype.clone() as u32;
        match roh.vtype {
            REG_DWORD => {
                schnappschuss.dword = u32::from_reg_value(&roh).ok();
            }
            REG_SZ | REG_EXPAND_SZ => {
                schnappschuss.text = String::from_reg_value(&roh).ok();
            }
            _ => {}
        }
        schnappschuss
    }

    /// Den aktuellen Zustand mit dem Sollzustand vergleichen.
    ///
    /// Typkonform: `"0"` (Text) und `0` (Zahl) sind verschiedene Dinge.
    fn entspricht(schnappschuss: &Snapshot, soll: RegData) -> bool {
        match soll {
            RegData::Absent => !schnappschuss.existed,
            RegData::Dword(zahl) => schnappschuss.dword == Some(zahl),
            RegData::Text(text) => schnappschuss.text.as_deref() == Some(text),
        }
    }

    /// Zustand eines Tweaks ermitteln.
    pub fn zustand(tweak: &Tweak) -> TweakState {
        let mut an = 0usize;
        let mut aus = 0usize;

        for wert in tweak.values {
            let schnappschuss = lies_snapshot(wert);
            if entspricht(&schnappschuss, wert.on) {
                an += 1;
            } else if entspricht(&schnappschuss, wert.off) || !schnappschuss.existed {
                // Fehlt der Wert, gilt der Windows-Standard – also „aus".
                aus += 1;
            }
        }

        if an == tweak.values.len() {
            TweakState::On
        } else if aus == tweak.values.len() {
            TweakState::Off
        } else {
            TweakState::Mixed
        }
    }

    fn schreibe(wert: &RegValue, daten: RegData) -> Result<(), String> {
        let hive = wurzel(wert.hive).ok_or_else(|| format!("Unbekannte Wurzel: {}", wert.hive))?;

        if daten == RegData::Absent {
            // Fehlt der Schlüssel schon, ist nichts zu tun.
            if let Ok(schluessel) = hive.open_subkey_with_flags(wert.path, KEY_SET_VALUE) {
                let _ = schluessel.delete_value(wert.name);
            }
            return Ok(());
        }

        let (schluessel, _) = hive
            .create_subkey(wert.path)
            .map_err(|e| format!("{}\\{}: {e}", wert.hive, wert.path))?;

        match daten {
            RegData::Dword(zahl) => schluessel
                .set_value(wert.name, &zahl)
                .map_err(|e| format!("{}: {e}", wert.name)),
            RegData::Text(text) => schluessel
                .set_value(wert.name, &text.to_string())
                .map_err(|e| format!("{}: {e}", wert.name)),
            RegData::Absent => unreachable!("oben behandelt"),
        }
    }

    /// Einen gesicherten Zustand wiederherstellen.
    pub fn stelle_wieder_her(schnappschuss: &Snapshot) -> Result<(), String> {
        let hive = wurzel(&schnappschuss.hive)
            .ok_or_else(|| format!("Unbekannte Wurzel: {}", schnappschuss.hive))?;

        if !schnappschuss.existed {
            // Der Wert existierte vorher nicht – also wieder entfernen.
            if let Ok(schluessel) = hive.open_subkey_with_flags(&schnappschuss.path, KEY_SET_VALUE)
            {
                let _ = schluessel.delete_value(&schnappschuss.name);
            }
            return Ok(());
        }

        let (schluessel, _) = hive
            .create_subkey(&schnappschuss.path)
            .map_err(|e| format!("{}: {e}", schnappschuss.path))?;

        if let Some(zahl) = schnappschuss.dword {
            return schluessel
                .set_value(&schnappschuss.name, &zahl)
                .map_err(|e| format!("{}: {e}", schnappschuss.name));
        }
        if let Some(text) = &schnappschuss.text {
            return schluessel
                .set_value(&schnappschuss.name, text)
                .map_err(|e| format!("{}: {e}", schnappschuss.name));
        }

        // Unbekannter Typ: unverändert lassen ist besser als raten.
        Ok(())
    }

    /// Einen Tweak anwenden oder zurücknehmen.
    pub fn setze(tweak: &Tweak, ein: bool) -> Result<Vec<Snapshot>, String> {
        let vorher: Vec<Snapshot> = tweak.values.iter().map(lies_snapshot).collect();

        for wert in tweak.values {
            let ziel = if ein { wert.on } else { wert.off };
            schreibe(wert, ziel)?;
        }

        Ok(vorher)
    }

    /// `true`, wenn eine Gruppenrichtlinie denselben Bereich belegt.
    ///
    /// Richtlinienwerte haben Vorrang; eine Änderung daneben wäre wirkungslos
    /// und würde beim nächsten `gpupdate` ohnehin überschrieben.
    pub fn ist_verwaltet(tweak: &Tweak) -> bool {
        tweak.values.iter().any(|wert| {
            if wert.path.contains("Policies") {
                return false; // Der Tweak schreibt selbst die Richtlinie.
            }
            let policy_pfad = format!(
                r"SOFTWARE\Policies\Microsoft\Windows\{}",
                wert.path.rsplit('\\').next().unwrap_or("")
            );
            RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(&policy_pfad, KEY_READ)
                .map(|k| k.get_raw_value(wert.name).is_ok())
                .unwrap_or(false)
        })
    }
}

#[cfg(not(windows))]
mod windows_impl {
    use super::*;

    pub fn lies_snapshot(wert: &RegValue) -> Snapshot {
        Snapshot {
            hive: wert.hive.to_string(),
            path: wert.path.to_string(),
            name: wert.name.to_string(),
            existed: false,
            value_type: 0,
            dword: None,
            text: None,
        }
    }

    pub fn zustand(_tweak: &Tweak) -> TweakState {
        TweakState::Off
    }

    pub fn setze(_tweak: &Tweak, _ein: bool) -> Result<Vec<Snapshot>, String> {
        Err("Tweaks gibt es nur unter Windows".to_string())
    }

    pub fn stelle_wieder_her(_schnappschuss: &Snapshot) -> Result<(), String> {
        Err("Tweaks gibt es nur unter Windows".to_string())
    }

    pub fn ist_verwaltet(_tweak: &Tweak) -> bool {
        false
    }
}

pub use windows_impl::{ist_verwaltet, lies_snapshot, setze, stelle_wieder_her, zustand};

/// Windows-11-Buildnummer erreicht?
fn ist_windows11() -> bool {
    sysinfo::System::kernel_version()
        .and_then(|k| k.split('.').next_back().and_then(|b| b.parse::<u32>().ok()))
        .map(|build| build >= 22_000)
        .unwrap_or(false)
}

/// Zustand aller Tweaks.
pub fn status_aller() -> Vec<TweakStatus> {
    let win11 = ist_windows11();

    TWEAKS
        .iter()
        .map(|tweak| TweakStatus {
            key: tweak.key.to_string(),
            group: tweak.group,
            state: zustand(tweak),
            requires_admin: tweak.requires_admin,
            apply: tweak.apply,
            unsupported: tweak.windows11_only && !win11,
            managed: ist_verwaltet(tweak),
        })
        .collect()
}

/// Einen Tweak setzen und den Vorzustand im Journal ablegen.
pub fn anwenden(key: &str, ein: bool, config_dir: &Path) -> Result<TweakState, String> {
    let tweak = tweak_by_key(key).ok_or_else(|| format!("Unbekannter Tweak: {key}"))?;

    if tweak.requires_admin && !super::runtime::is_admin() {
        return Err("tweak.needs_admin".to_string());
    }

    let vorher = setze(tweak, ein)?;

    let mut journal = journal_laden(config_dir);
    journal.push(JournalEntry {
        tweak: tweak.key.to_string(),
        timestamp: jetzt(),
        enabled: ein,
        before: vorher,
    });
    // Das Journal darf nicht unbegrenzt wachsen.
    if journal.len() > 500 {
        let ueberschuss = journal.len() - 500;
        journal.drain(..ueberschuss);
    }
    journal_speichern(config_dir, &journal)?;

    Ok(zustand(tweak))
}

/// Die letzte Änderung eines Tweaks zurücknehmen.
///
/// Stellt den **tatsächlich vorgefundenen** Zustand wieder her, nicht einen
/// angenommenen Windows-Standard.
pub fn zuruecknehmen(key: &str, config_dir: &Path) -> Result<TweakState, String> {
    let tweak = tweak_by_key(key).ok_or_else(|| format!("Unbekannter Tweak: {key}"))?;

    if tweak.requires_admin && !super::runtime::is_admin() {
        return Err("tweak.needs_admin".to_string());
    }

    let journal = journal_laden(config_dir);
    let eintrag = letzter_eintrag(&journal, tweak.key)
        .ok_or_else(|| "tweak.no_journal".to_string())?
        .clone();

    for schnappschuss in &eintrag.before {
        stelle_wieder_her(schnappschuss)?;
    }

    // Den verbrauchten Eintrag entfernen, damit ein zweites Zurücknehmen
    // nicht denselben Zustand erneut schreibt.
    let rest: Vec<JournalEntry> = journal
        .into_iter()
        .filter(|e| !(e.tweak == eintrag.tweak && e.timestamp == eintrag.timestamp))
        .collect();
    journal_speichern(config_dir, &rest)?;

    Ok(zustand(tweak))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testordner(name: &str) -> PathBuf {
        let pfad = std::env::temp_dir().join(format!("plane_tweaks_{name}"));
        let _ = std::fs::remove_dir_all(&pfad);
        pfad
    }

    #[test]
    fn schluessel_sind_eindeutig() {
        let mut gesehen: Vec<&str> = Vec::new();
        for tweak in TWEAKS {
            assert!(!gesehen.contains(&tweak.key), "doppelt: {}", tweak.key);
            gesehen.push(tweak.key);
        }
    }

    #[test]
    fn jeder_tweak_aendert_etwas() {
        for tweak in TWEAKS {
            assert!(!tweak.values.is_empty(), "{} ist wirkungslos", tweak.key);
            for wert in tweak.values {
                assert_ne!(wert.on, wert.off, "{}: an und aus gleich", tweak.key);
                assert!(
                    wert.hive == "HKCU" || wert.hive == "HKLM",
                    "{}: unbekannte Wurzel {}",
                    tweak.key,
                    wert.hive
                );
            }
        }
    }

    #[test]
    fn gefaehrliche_bereiche_werden_nicht_angefasst() {
        // Diese Bereiche haben dokumentiert Systeme beschädigt.
        const VERBOTEN: &[&str] = &[
            "WindowsUpdate",
            "Windows Defender",
            "Services",
            "BitLocker",
            "Winlogon",
        ];
        for tweak in TWEAKS {
            for wert in tweak.values {
                for verboten in VERBOTEN {
                    assert!(
                        !wert.path.contains(verboten),
                        "{} fasst {verboten} an",
                        tweak.key
                    );
                }
            }
        }
    }

    #[test]
    fn hklm_tweaks_verlangen_adminrechte() {
        for tweak in TWEAKS {
            if tweak.values.iter().any(|w| w.hive == "HKLM") {
                assert!(
                    tweak.requires_admin,
                    "{} schreibt nach HKLM ohne requires_admin",
                    tweak.key
                );
            }
        }
    }

    #[test]
    fn hkcu_tweaks_brauchen_keine_adminrechte() {
        for tweak in TWEAKS {
            if tweak.values.iter().all(|w| w.hive == "HKCU") {
                assert!(
                    !tweak.requires_admin,
                    "{} schreibt nur nach HKCU, verlangt aber Adminrechte",
                    tweak.key
                );
            }
        }
    }

    #[test]
    fn jede_gruppe_hat_tweaks() {
        for gruppe in TweakGroup::ALL {
            assert!(
                TWEAKS.iter().any(|t| t.group == *gruppe),
                "Gruppe {} ist leer",
                gruppe.key()
            );
        }
    }

    #[test]
    fn tweak_by_key_ignoriert_gross_klein() {
        assert!(tweak_by_key("  TASKBAR.ALIGN_LEFT ").is_some());
        assert!(tweak_by_key("gibtesnicht").is_none());
    }

    #[test]
    fn status_deckt_alle_tweaks_ab() {
        let status = status_aller();
        assert_eq!(status.len(), TWEAKS.len());
        for eintrag in &status {
            assert!(tweak_by_key(&eintrag.key).is_some());
        }
    }

    #[test]
    fn windows11_tweaks_sind_auf_win10_als_nicht_unterstuetzt_markiert() {
        let status = status_aller();
        for eintrag in &status {
            let tweak = tweak_by_key(&eintrag.key).unwrap();
            if !tweak.windows11_only {
                assert!(!eintrag.unsupported, "{} fälschlich gesperrt", tweak.key);
            }
        }
    }

    #[test]
    fn journal_ueberlebt_speichern_und_laden() {
        let dir = testordner("journal");
        let eintraege = vec![JournalEntry {
            tweak: "explorer.file_extensions".into(),
            timestamp: 1_700_000_000,
            enabled: true,
            before: vec![Snapshot {
                hive: "HKCU".into(),
                path: "Software\\Test".into(),
                name: "Wert".into(),
                existed: true,
                value_type: 4,
                dword: Some(1),
                text: None,
            }],
        }];

        journal_speichern(&dir, &eintraege).unwrap();
        let geladen = journal_laden(&dir);

        assert_eq!(geladen.len(), 1);
        assert_eq!(geladen[0].before[0].dword, Some(1));
        assert!(geladen[0].before[0].existed);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn beschaedigtes_journal_blockiert_nicht() {
        let dir = testordner("kaputt");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(journal_path(&dir), "{kein json").unwrap();
        assert!(journal_laden(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fehlendes_journal_ist_leer() {
        assert!(journal_laden(&testordner("fehlt")).is_empty());
    }

    #[test]
    fn letzter_eintrag_ist_der_juengste() {
        let journal = vec![
            JournalEntry {
                tweak: "a".into(),
                timestamp: 100,
                enabled: true,
                before: Vec::new(),
            },
            JournalEntry {
                tweak: "a".into(),
                timestamp: 300,
                enabled: false,
                before: Vec::new(),
            },
            JournalEntry {
                tweak: "b".into(),
                timestamp: 500,
                enabled: true,
                before: Vec::new(),
            },
        ];
        assert_eq!(letzter_eintrag(&journal, "a").unwrap().timestamp, 300);
        assert_eq!(letzter_eintrag(&journal, "b").unwrap().timestamp, 500);
        assert!(letzter_eintrag(&journal, "c").is_none());
    }

    #[test]
    fn snapshot_haelt_den_nicht_vorhandenen_zustand_fest() {
        // Der wichtigste Fall: ein Wert, den es vorher nicht gab, muss beim
        // Zurücknehmen wieder verschwinden – nicht auf einen angenommenen
        // Standard gesetzt werden.
        let wert = RegValue {
            hive: "HKCU",
            path: r"Software\PlaneGibtEsNicht12345",
            name: "Test",
            on: RegData::Dword(1),
            off: RegData::Absent,
        };
        let schnappschuss = lies_snapshot(&wert);
        assert!(!schnappschuss.existed);
        assert_eq!(schnappschuss.dword, None);
    }

    #[test]
    fn unbekannter_tweak_wird_abgelehnt() {
        let dir = testordner("unbekannt");
        assert!(anwenden("gibtesnicht", true, &dir).is_err());
        assert!(zuruecknehmen("gibtesnicht", &dir).is_err());
    }

    #[test]
    fn zuruecknehmen_ohne_journal_meldet_das() {
        let dir = testordner("ohne_journal");
        // Ein Tweak ohne Adminrechte, damit der Test nicht an der
        // Rechteprüfung hängenbleibt.
        let fehler = zuruecknehmen("explorer.file_extensions", &dir).unwrap_err();
        assert_eq!(fehler, "tweak.no_journal");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adminpflichtige_tweaks_scheitern_ohne_rechte() {
        if super::super::runtime::is_admin() {
            return;
        }
        let dir = testordner("ohne_rechte");
        let fehler = anwenden("system.long_paths", true, &dir).unwrap_err();
        assert_eq!(fehler, "tweak.needs_admin");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn gruppen_haben_stabile_schluessel() {
        assert_eq!(TweakGroup::Privacy.key(), "privacy");
        assert_eq!(TweakGroup::Taskbar.i18n_key(), "tweakgroup.taskbar");
    }

    #[test]
    fn anwenden_und_zuruecknehmen_stellt_den_ist_zustand_wieder_her() {
        if !cfg!(windows) {
            return;
        }
        let dir = testordner("rundlauf");

        // Ein reiner HKCU-Tweak ohne Nebenwirkung auf das Testsystem.
        let key = "privacy.feedback";
        let vorher = zustand(tweak_by_key(key).unwrap());

        anwenden(key, true, &dir).unwrap();
        assert_eq!(zustand(tweak_by_key(key).unwrap()), TweakState::On);

        zuruecknehmen(key, &dir).unwrap();
        assert_eq!(
            zustand(tweak_by_key(key).unwrap()),
            vorher,
            "der ursprüngliche Zustand muss exakt wiederhergestellt sein"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
