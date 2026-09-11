//! Registry-Analyse und -Bereinigung.
//!
//! # Haltung zu Registry-Cleaning
//!
//! Microsoft empfiehlt Registry-Cleaner ausdrücklich **nicht**, und der
//! messbare Geschwindigkeitsgewinn ist praktisch null. Plane bietet die
//! Funktion trotzdem an, weil Nutzer sie von anderen Werkzeugen kennen – aber
//! mit vier harten Regeln:
//!
//! 1. Das Ziel ist [`Risk::Caution`](super::types::Risk::Caution) und **nie**
//!    vorausgewählt.
//! 2. Vor jeder Löschung wird eine `.reg`-Sicherung geschrieben. Ohne
//!    erfolgreiche Sicherung wird nichts gelöscht.
//! 3. Als verwaist gilt nur, was zweifelsfrei verwaist ist: ein **absoluter
//!    Pfad mit Laufwerksbuchstaben und Dateiendung**, der nicht existiert.
//!    Alles Mehrdeutige (URLs, `rundll32`-Aufrufe, MSI-Platzhalter, UNC-Pfade)
//!    bleibt unangetastet.
//! 4. Bekannte Systemeinträge stehen auf einer Sperrliste.
//!
//! Die Pfadauswertung ist von der Registry-API getrennt, damit sie ohne
//! Windows testbar ist.

use std::path::{Path, PathBuf};

use super::fsutil;
use super::runtime::CancelToken;
use super::types::{RegistryCheck, RegistryRule, ScanItem};

/// Einträge, die nie angetastet werden – auch dann nicht, wenn das Ziel fehlt.
///
/// Sicherheitssoftware, COM-Starthelfer und Windows-eigene Dienste dürfen nicht
/// entfernt werden, selbst wenn ihr Pfad gerade nicht auflösbar ist.
const SPERRLISTE: &[&str] = &[
    "onedrive",
    "securityhealth",
    "windowsdefender",
    "windows defender",
    "msiexec",
    "rundll32",
    "explorer.exe",
    "systemroot",
    "dllhost",
    "svchost",
];

/// Schlüsselpfade, die niemals gelöscht werden – unabhängig vom Regelwerk.
///
/// Die WinRT-/MSIX-Registrierungen (`AppModel`, `PackagedCom`,
/// `ActivatableClasses`) sehen für eine naive Existenzprüfung immer „verwaist“
/// aus, weil ihre Ziele erst bei Bedarf bereitgestellt werden.
/// `Winlogon\Userinit` und `Shell` zu entfernen macht Windows unbenutzbar.
const GESPERRTE_SCHLUESSEL: &[&str] = &[
    r"\system\currentcontrolset\services",
    r"\windows nt\currentversion\winlogon",
    r"\currentversion\appmodel",
    r"\classes\packagedcom",
    r"\classes\activatableclasses",
    r"\classes\installer",
    r"\currentversion\installer",
    r"\classes\clsid",
    r"\classes\interface",
    r"\classes\typelib",
    r"firewallpolicy",
];

/// `true`, wenn der Schlüsselpfad grundsätzlich gesperrt ist.
pub fn is_protected_key(schluessel: &str) -> bool {
    let klein = schluessel.to_ascii_lowercase();
    GESPERRTE_SCHLUESSEL
        .iter()
        .any(|gesperrt| klein.contains(gesperrt))
}

/// Einen Dateipfad aus einer Befehlszeile herauslösen.
///
/// Behandelt: `"C:\P\app.exe" --arg`, `C:\P\app.exe --arg`,
/// `C:\P\app.exe,0` (Icon-Referenzen).
/// Gibt `None` zurück, wenn kein eindeutiger absoluter Pfad erkennbar ist.
pub fn extract_path(befehlszeile: &str) -> Option<String> {
    let text = befehlszeile.trim();
    if text.is_empty() {
        return None;
    }

    let pfad = if let Some(rest) = text.strip_prefix('"') {
        // In Anführungszeichen: bis zum schließenden Zeichen.
        rest.split('"').next()?.to_string()
    } else {
        // Ohne Anführungszeichen: heuristisch bis zur ersten Endung, danach
        // beginnen die Argumente.
        let klein = text.to_ascii_lowercase();
        let ende = [".exe", ".dll", ".msi", ".bat", ".cmd", ".cpl", ".scr"]
            .iter()
            .filter_map(|endung| klein.find(endung).map(|i| i + endung.len()))
            .min();
        match ende {
            Some(i) => text[..i].to_string(),
            None => text.split_whitespace().next()?.to_string(),
        }
    };

    let pfad = pfad.trim().trim_end_matches(',').trim().to_string();
    if pfad.is_empty() {
        None
    } else {
        Some(pfad)
    }
}

/// `true`, wenn ein Wert überhaupt als Dateipfad geprüft werden darf.
///
/// Bewusst streng: alles, was nicht eindeutig ein lokaler Dateipfad ist, wird
/// nie als verwaist gemeldet.
pub fn is_checkable_path(pfad: &str) -> bool {
    let klein = pfad.to_ascii_lowercase();

    if klein.len() < 4 {
        return false;
    }
    // UNC-Pfade: Netzlaufwerk könnte nur gerade nicht verbunden sein.
    if klein.starts_with("\\\\") {
        return false;
    }
    // URLs, Protokolle, GUID-Platzhalter des Windows Installers.
    if klein.contains("://") || klein.starts_with('{') {
        return false;
    }
    // Muss ein Laufwerksbuchstabe sein: "C:\..."
    let bytes = klein.as_bytes();
    if !(bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
    {
        return false;
    }
    // Ohne Dateiendung ist es vermutlich ein Ordner – zu unsicher.
    Path::new(&klein).extension().is_some()
}

/// `true`, wenn der Eintrag auf der Sperrliste steht.
pub fn is_protected(name: &str, wert: &str) -> bool {
    let text = format!("{name} {wert}").to_ascii_lowercase();
    SPERRLISTE.iter().any(|eintrag| text.contains(eintrag))
}

/// Kernentscheidung: Ist dieser Eintrag verwaist?
///
/// `exists` wird injiziert, damit die Regel ohne echtes Dateisystem prüfbar ist.
pub fn is_orphaned(name: &str, wert: &str, exists: &dyn Fn(&str) -> bool) -> bool {
    if is_protected(name, wert) {
        return false;
    }
    let Some(pfad) = extract_path(wert) else {
        return false;
    };
    let Some(aufgeloest) = fsutil::expand_vars(&pfad) else {
        // Nicht auflösbare Variable: im Zweifel behalten.
        return false;
    };
    if !is_checkable_path(&aufgeloest) {
        return false;
    }
    !exists(&aufgeloest)
}

/// Vollständiger Schlüsselpfad zur Anzeige.
pub fn full_key(hive: &str, path: &str) -> String {
    let lang = match hive {
        "HKCU" => "HKEY_CURRENT_USER",
        "HKLM" => "HKEY_LOCAL_MACHINE",
        "HKCR" => "HKEY_CLASSES_ROOT",
        anderes => anderes,
    };
    format!("{lang}\\{path}")
}

// ---------------------------------------------------------------------------
// Windows-Implementierung
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use winreg::enums::*;
    use winreg::RegKey;

    fn hive_key(hive: &str) -> Option<RegKey> {
        Some(match hive {
            "HKCU" => RegKey::predef(HKEY_CURRENT_USER),
            "HKLM" => RegKey::predef(HKEY_LOCAL_MACHINE),
            "HKCR" => RegKey::predef(HKEY_CLASSES_ROOT),
            _ => return None,
        })
    }

    fn datei_existiert(pfad: &str) -> bool {
        Path::new(pfad).exists()
    }

    /// Eine Regel auswerten.
    pub fn scan_rule(regel: &RegistryRule, cancel: &CancelToken) -> (Vec<ScanItem>, Vec<String>) {
        let mut treffer = Vec::new();
        let mut warnungen = Vec::new();

        let Some(hive) = hive_key(regel.hive) else {
            warnungen.push(format!("Unbekannte Registry-Wurzel: {}", regel.hive));
            return (treffer, warnungen);
        };

        let anzeige = full_key(regel.hive, regel.path);
        if is_protected_key(&anzeige) {
            warnungen.push(format!("Gesperrter Bereich übersprungen: {anzeige}"));
            return (treffer, warnungen);
        }

        let Ok(schluessel) = hive.open_subkey_with_flags(regel.path, KEY_READ) else {
            // Nicht vorhandene Schlüssel sind normal (je nach Windows-Version).
            return (treffer, warnungen);
        };

        match regel.check {
            RegistryCheck::ValueNameIsPath => {
                for (name, _wert) in schluessel.enum_values().flatten() {
                    if cancel.is_cancelled() {
                        break;
                    }
                    if is_orphaned(&name, &name, &datei_existiert) {
                        treffer.push(ScanItem {
                            path: anzeige.clone(),
                            size: 0,
                            detail: name.clone(),
                            value_name: Some(name),
                        });
                    }
                }
            }
            RegistryCheck::ValueIsPath { value } => {
                for (name, wert) in schluessel.enum_values().flatten() {
                    if cancel.is_cancelled() {
                        break;
                    }
                    if !value.is_empty() && !name.eq_ignore_ascii_case(value) {
                        continue;
                    }
                    let text = wert.to_string();
                    if is_orphaned(&name, &text, &datei_existiert) {
                        treffer.push(ScanItem {
                            path: anzeige.clone(),
                            size: 0,
                            detail: format!("{name} → {text}"),
                            value_name: Some(name),
                        });
                    }
                }
            }
            RegistryCheck::SubkeyProgId => {
                for unterschluessel in schluessel.enum_keys().flatten() {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let Ok(unter) = schluessel.open_subkey(&unterschluessel) else {
                        continue;
                    };
                    let standard: String = unter.get_value("").unwrap_or_default();
                    if is_orphaned(&unterschluessel, &standard, &datei_existiert) {
                        treffer.push(ScanItem {
                            path: format!("{anzeige}\\{unterschluessel}"),
                            size: 0,
                            detail: standard,
                            value_name: None,
                        });
                    }
                }
            }
            RegistryCheck::UninstallEntry => {
                for unterschluessel in schluessel.enum_keys().flatten() {
                    if cancel.is_cancelled() {
                        break;
                    }
                    let Ok(unter) = schluessel.open_subkey(&unterschluessel) else {
                        continue;
                    };
                    let anzeigename: String = unter.get_value("DisplayName").unwrap_or_default();
                    let ort: String = unter.get_value("InstallLocation").unwrap_or_default();
                    let icon: String = unter.get_value("DisplayIcon").unwrap_or_default();

                    // Nur eindeutige Fälle: DisplayIcon zeigt auf eine Datei,
                    // die es nicht mehr gibt. InstallLocation ist zu oft leer
                    // oder ein Ordner ohne Endung.
                    if icon.is_empty() && ort.is_empty() {
                        continue;
                    }
                    if is_orphaned(&unterschluessel, &icon, &datei_existiert) {
                        let name = if anzeigename.is_empty() {
                            unterschluessel.clone()
                        } else {
                            anzeigename
                        };
                        treffer.push(ScanItem {
                            path: format!("{anzeige}\\{unterschluessel}"),
                            size: 0,
                            detail: name,
                            value_name: None,
                        });
                    }
                }
            }
        }

        (treffer, warnungen)
    }

    /// Sicherung der betroffenen Schlüssel als `.reg`-Datei.
    ///
    /// Nutzt `reg.exe export`, weil das Format damit garantiert wieder
    /// importierbar ist – eine selbstgebaute `.reg`-Datei wäre fehleranfällig.
    pub fn backup(regeln: &[RegistryRule], ziel: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(ziel)
            .map_err(|e| format!("Sicherungsordner nicht anlegbar: {e}"))?;

        let stempel = zeitstempel();
        let datei = ziel.join(format!("plane-registry-{stempel}.reg"));
        let mut inhalt = String::from("Windows Registry Editor Version 5.00\r\n\r\n");
        let mut gesichert = 0usize;

        for regel in regeln {
            let schluessel = full_key(regel.hive, regel.path);
            let temp = std::env::temp_dir().join(format!("plane-reg-{gesichert}.tmp.reg"));

            let ausgabe = std::process::Command::new("reg")
                .args(["export", &schluessel, &temp.to_string_lossy(), "/y"])
                .output();

            match ausgabe {
                Ok(o) if o.status.success() => {
                    if let Ok(rohdaten) = std::fs::read(&temp) {
                        let text = dekodiere_utf16_oder_utf8(&rohdaten);
                        // Kopfzeile nur einmal.
                        for zeile in text.lines().skip(1) {
                            inhalt.push_str(zeile);
                            inhalt.push_str("\r\n");
                        }
                        gesichert += 1;
                    }
                    let _ = std::fs::remove_file(&temp);
                }
                // Nicht existierende Schlüssel müssen nicht gesichert werden.
                _ => {
                    let _ = std::fs::remove_file(&temp);
                }
            }
        }

        if gesichert == 0 {
            return Err("Keiner der Registry-Schlüssel ließ sich sichern".to_string());
        }

        std::fs::write(&datei, inhalt).map_err(|e| format!("Sicherung nicht schreibbar: {e}"))?;
        Ok(datei)
    }

    /// Gefundene Einträge entfernen.
    pub fn delete_items(items: &[ScanItem], dry_run: bool) -> (usize, Vec<String>) {
        let mut entfernt = 0usize;
        let mut fehler = Vec::new();

        for item in items {
            if dry_run {
                entfernt += 1;
                continue;
            }

            if is_protected_key(&item.path) {
                fehler.push(format!("Gesperrter Bereich: {}", item.path));
                continue;
            }

            let Some((hive, unterpfad)) = zerlege(&item.path) else {
                fehler.push(format!("Unlesbarer Schlüsselpfad: {}", item.path));
                continue;
            };
            let Some(wurzel) = hive_key(&hive) else {
                fehler.push(format!("Unbekannte Wurzel: {hive}"));
                continue;
            };

            let ergebnis = match &item.value_name {
                Some(name) => wurzel
                    .open_subkey_with_flags(&unterpfad, KEY_SET_VALUE)
                    .and_then(|k| k.delete_value(name)),
                None => wurzel.delete_subkey_all(&unterpfad),
            };

            match ergebnis {
                Ok(()) => entfernt += 1,
                Err(e) => fehler.push(format!("{}: {e}", item.path)),
            }
        }

        (entfernt, fehler)
    }

    /// `HKEY_CURRENT_USER\A\B` → `("HKCU", "A\B")`
    fn zerlege(voll: &str) -> Option<(String, String)> {
        let (wurzel, rest) = voll.split_once('\\')?;
        let kurz = match wurzel {
            "HKEY_CURRENT_USER" => "HKCU",
            "HKEY_LOCAL_MACHINE" => "HKLM",
            "HKEY_CLASSES_ROOT" => "HKCR",
            _ => return None,
        };
        Some((kurz.to_string(), rest.to_string()))
    }

    fn dekodiere_utf16_oder_utf8(rohdaten: &[u8]) -> String {
        // reg.exe schreibt UTF-16LE mit BOM.
        if rohdaten.len() >= 2 && rohdaten[0] == 0xFF && rohdaten[1] == 0xFE {
            let mut worte: Vec<u16> = Vec::with_capacity(rohdaten.len() / 2);
            let mut rest = &rohdaten[2..];
            while rest.len() >= 2 {
                worte.push(u16::from_le_bytes([rest[0], rest[1]]));
                rest = &rest[2..];
            }
            String::from_utf16_lossy(&worte)
        } else {
            String::from_utf8_lossy(rohdaten).to_string()
        }
    }

    fn zeitstempel() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let sekunden = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{sekunden}")
    }
}

#[cfg(not(windows))]
mod windows_impl {
    use super::*;

    pub fn scan_rule(_regel: &RegistryRule, _cancel: &CancelToken) -> (Vec<ScanItem>, Vec<String>) {
        (Vec::new(), Vec::new())
    }

    pub fn backup(_regeln: &[RegistryRule], _ziel: &Path) -> Result<PathBuf, String> {
        Err("Registry gibt es nur unter Windows".to_string())
    }

    pub fn delete_items(_items: &[ScanItem], _dry_run: bool) -> (usize, Vec<String>) {
        (0, Vec::new())
    }
}

pub use windows_impl::{backup, delete_items, scan_rule};

/// Alle Regeln auswerten.
pub fn scan_rules(regeln: &[RegistryRule], cancel: &CancelToken) -> (Vec<ScanItem>, Vec<String>) {
    let mut treffer = Vec::new();
    let mut warnungen = Vec::new();

    for regel in regeln {
        if cancel.is_cancelled() {
            break;
        }
        let (mut t, mut w) = scan_rule(regel, cancel);
        treffer.append(&mut t);
        warnungen.append(&mut w);
    }

    (treffer, warnungen)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrahiert_pfad_aus_anfuehrungszeichen() {
        assert_eq!(
            extract_path(r#""C:\Programme\App\app.exe" --start"#).as_deref(),
            Some(r"C:\Programme\App\app.exe")
        );
    }

    #[test]
    fn extrahiert_pfad_ohne_anfuehrungszeichen() {
        assert_eq!(
            extract_path(r"C:\Tools\app.exe -q").as_deref(),
            Some(r"C:\Tools\app.exe")
        );
    }

    #[test]
    fn extrahiert_pfad_aus_icon_referenz() {
        assert_eq!(
            extract_path(r"C:\Tools\app.exe,0").as_deref(),
            Some(r"C:\Tools\app.exe")
        );
    }

    #[test]
    fn leerer_wert_ergibt_keinen_pfad() {
        assert!(extract_path("").is_none());
        assert!(extract_path("   ").is_none());
    }

    #[test]
    fn pruefbar_nur_bei_lokalem_dateipfad_mit_endung() {
        assert!(is_checkable_path(r"C:\Tools\app.exe"));
        assert!(is_checkable_path(r"D:\x\y.dll"));

        assert!(!is_checkable_path(r"\\server\freigabe\app.exe"));
        assert!(!is_checkable_path("https://example.com/app.exe"));
        assert!(!is_checkable_path("{12345678-1234-1234-1234-123456789012}"));
        assert!(!is_checkable_path(r"C:\Programme\Ordner"));
        assert!(!is_checkable_path("app.exe"));
        assert!(!is_checkable_path("abc"));
    }

    #[test]
    fn sperrliste_schuetzt_systemeintraege() {
        assert!(is_protected("OneDrive", r"C:\x\OneDrive.exe"));
        assert!(is_protected("X", r"C:\Windows\System32\rundll32.exe foo"));
        assert!(!is_protected("MeinTool", r"C:\Tools\mein.exe"));
    }

    #[test]
    fn verwaist_nur_wenn_datei_wirklich_fehlt() {
        let existiert_nie = |_: &str| false;
        let existiert_immer = |_: &str| true;

        assert!(is_orphaned("MeinTool", r"C:\Tools\weg.exe", &existiert_nie));
        assert!(!is_orphaned(
            "MeinTool",
            r"C:\Tools\da.exe",
            &existiert_immer
        ));
    }

    #[test]
    fn mehrdeutige_eintraege_gelten_nie_als_verwaist() {
        let existiert_nie = |_: &str| false;
        for wert in [
            r"\\server\share\app.exe",
            "https://example.com/start",
            "{90140000-0011-0000-0000-0000000FF1CE}",
            r"C:\Programme\NurEinOrdner",
            "",
        ] {
            assert!(
                !is_orphaned("Test", wert, &existiert_nie),
                "fälschlich als verwaist erkannt: {wert}"
            );
        }
    }

    #[test]
    fn geschuetzte_eintraege_gelten_nie_als_verwaist() {
        let existiert_nie = |_: &str| false;
        assert!(!is_orphaned(
            "OneDrive",
            r"C:\x\OneDrive.exe",
            &existiert_nie
        ));
    }

    #[test]
    fn unaufloesbare_variable_gilt_nicht_als_verwaist() {
        let existiert_nie = |_: &str| false;
        assert!(!is_orphaned(
            "Test",
            r"%GIBT_ES_NICHT_98765%\app.exe",
            &existiert_nie
        ));
    }

    #[test]
    fn voller_schluesselname_wird_ausgeschrieben() {
        assert_eq!(
            full_key("HKCU", r"SOFTWARE\X"),
            r"HKEY_CURRENT_USER\SOFTWARE\X"
        );
        assert_eq!(full_key("HKLM", "A"), r"HKEY_LOCAL_MACHINE\A");
        assert_eq!(full_key("HKCR", "A"), r"HKEY_CLASSES_ROOT\A");
    }

    #[test]
    fn abgebrochener_scan_liefert_nichts() {
        let cancel = CancelToken::new();
        cancel.cancel();
        let regeln = &[RegistryRule {
            hive: "HKCU",
            path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
            check: RegistryCheck::ValueIsPath { value: "" },
        }];
        let (treffer, _) = scan_rules(regeln, &cancel);
        assert!(treffer.is_empty());
    }

    #[test]
    fn trockenlauf_loescht_nichts_und_zaehlt_korrekt() {
        let items = vec![ScanItem {
            path: r"HKEY_CURRENT_USER\SOFTWARE\PlaneTestGibtEsNicht".into(),
            size: 0,
            detail: String::new(),
            value_name: None,
        }];
        let (entfernt, fehler) = delete_items(&items, true);
        assert_eq!(entfernt, 1);
        assert!(fehler.is_empty());
    }
}
