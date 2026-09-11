//! Programme auflisten und deinstallieren.
//!
//! Plane startet hier **fremde** Deinstallationsprogramme. Das ist etwas
//! anderes als Dateien zu löschen: der Ausgang liegt nicht in unserer Hand.
//! Deshalb gelten drei Regeln:
//!
//! 1. **Plane deinstalliert nie von selbst.** Es gibt keine „alles
//!    aufräumen"-Funktion; jeder Eintrag wird einzeln bestätigt.
//! 2. **Geschützte Einträge werden gesperrt, nicht versteckt.** Wer sein
//!    Visual C++ Redistributable sucht, soll sehen, *dass* es da ist und
//!    *warum* Plane es nicht anfasst.
//! 3. **Kein Raten bei stillen Schaltern.** Nur `QuietUninstallString` wird
//!    für eine stille Deinstallation verwendet. Ansonsten läuft der
//!    Deinstaller sichtbar — halb entfernte Software ist schlimmer als ein
//!    Klick mehr.
//!
//! # Warum kein WMI
//!
//! `Win32_Product` aufzuzählen löst für **jedes** installierte MSI-Produkt
//! eine Neukonfiguration aus, dauert Minuten und kann Software beschädigen.
//! Die Registry ist die richtige Quelle.

use serde::{Deserialize, Serialize};

use super::process;

/// Ein installiertes Programm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Programm {
    /// Registry-Schlüsselname bzw. Paketname – eindeutig je Quelle.
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub publisher: String,
    /// Installationsdatum als `YYYYMMDD`, falls hinterlegt.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub install_date: String,
    /// Geschätzte Größe in Bytes. 0 = unbekannt.
    pub size: u64,
    pub source: Quelle,
    /// Kann Plane den Eintrag entfernen?
    pub removable: bool,
    /// Übersetzungsschlüssel, warum nicht – leer, wenn entfernbar.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub protection: String,
    /// Läuft die Deinstallation ohne Dialog?
    pub quiet: bool,
    /// Braucht die Deinstallation Administratorrechte?
    pub requires_admin: bool,
    /// Rohwert von `DisplayIcon` – Quelle für das Programmsymbol.
    ///
    /// Wird nicht hier aufgelöst: das Lesen von 150 Symbolen aus ebenso
    /// vielen Dateien dauert spürbar, und die Liste soll sofort stehen.
    /// Die Oberfläche holt die Bilder in einem zweiten Schritt nach,
    /// siehe [`super::icons`].
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub icon: String,
}

/// Woher der Eintrag stammt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Quelle {
    /// `HKLM\...\Uninstall`, 64-Bit-Sicht.
    Machine,
    /// `HKLM\...\Uninstall`, 32-Bit-Sicht.
    Machine32,
    /// `HKCU\...\Uninstall` – betrifft nur den angemeldeten Nutzer.
    User,
    /// Store-/MSIX-Paket.
    Store,
}

impl Quelle {
    pub fn key(self) -> &'static str {
        match self {
            Quelle::Machine => "machine",
            Quelle::Machine32 => "machine32",
            Quelle::User => "user",
            Quelle::Store => "store",
        }
    }
}

/// Ergebnis einer Deinstallation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UninstallResult {
    pub id: String,
    pub name: String,
    pub ok: bool,
    /// Exitcode des Deinstallers; `-1`, wenn keiner ermittelbar war.
    pub exit_code: i32,
    /// Windows verlangt einen Neustart, um fertig zu werden.
    pub reboot_required: bool,
    /// Der Registry-Eintrag ist nach dem Lauf verschwunden.
    pub verified: bool,
    /// Übersetzungsschlüssel der Meldung.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub message: String,
}

// ---------------------------------------------------------------------------
// Schutzregeln
// ---------------------------------------------------------------------------

/// Namensmuster, die niemals zur Deinstallation angeboten werden.
///
/// Kleingeschrieben; geprüft wird per `contains`. Die Liste stammt aus der
/// Zielrecherche: das sind die Pakete, deren Entfernen regelmäßig andere
/// Software oder das System beschädigt.
const GESCHUETZTE_NAMEN: &[(&str, &str)] = &[
    ("microsoft visual c++", "uninstall.protected.runtime"),
    ("microsoft .net", "uninstall.protected.runtime"),
    (".net framework", "uninstall.protected.runtime"),
    ("webview2", "uninstall.protected.runtime"),
    ("windows app runtime", "uninstall.protected.runtime"),
    (
        "windows software development kit",
        "uninstall.protected.runtime",
    ),
    ("microsoft defender", "uninstall.protected.security"),
    ("windows defender", "uninstall.protected.security"),
    ("security update", "uninstall.protected.update"),
    ("update for windows", "uninstall.protected.update"),
    ("windows-treiberpaket", "uninstall.protected.driver"),
    ("windows driver package", "uninstall.protected.driver"),
    ("chipset", "uninstall.protected.driver"),
    ("graphics driver", "uninstall.protected.driver"),
    ("audio driver", "uninstall.protected.driver"),
    ("plane", "uninstall.protected.self"),
];

/// Herausgeber, deren Treiberpakete geschützt sind.
const TREIBER_HERSTELLER: &[&str] = &[
    "intel",
    "nvidia",
    "advanced micro devices",
    "realtek",
    "qualcomm",
    "synaptics",
    "mediatek",
];

/// Store-Pakete, die zur Shell gehören.
const GESCHUETZTE_PAKETE: &[&str] = &[
    "microsoft.windowsstore",
    "microsoft.desktopappinstaller",
    "microsoft.ui.xaml",
    "microsoft.vclibs",
    "microsoft.net.native",
    "microsoft.windows.shellexperiencehost",
    "microsoft.windows.startmenuexperiencehost",
    "microsoftwindows.client",
    "microsoft.sechealthui",
    "microsoft.windows.search",
    "windows.immersivecontrolpanel",
];

/// Prüft einen Eintrag gegen die Schutzregeln.
///
/// Gibt den Übersetzungsschlüssel des Grundes zurück, oder `None`, wenn der
/// Eintrag entfernt werden darf.
pub fn schutzgrund(name: &str, publisher: &str, quelle: Quelle) -> Option<&'static str> {
    let name_klein = name.to_ascii_lowercase();

    if quelle == Quelle::Store {
        if GESCHUETZTE_PAKETE
            .iter()
            .any(|paket| name_klein.contains(paket))
        {
            return Some("uninstall.protected.system");
        }
        return None;
    }

    for (muster, grund) in GESCHUETZTE_NAMEN {
        if name_klein.contains(muster) {
            return Some(grund);
        }
    }

    // Treiberpakete erkennt man erst aus Hersteller UND Name zusammen –
    // „Intel Driver & Support Assistant" ist kein Treiber, „Intel Chipset
    // Device Software" schon.
    let publisher_klein = publisher.to_ascii_lowercase();
    if TREIBER_HERSTELLER
        .iter()
        .any(|h| publisher_klein.contains(h))
        && (name_klein.contains("driver") || name_klein.contains("treiber"))
    {
        return Some("uninstall.protected.driver");
    }

    None
}

// ---------------------------------------------------------------------------
// Kommandozeilen
// ---------------------------------------------------------------------------

/// Eine MSI-Produkt-GUID aus einem Schlüsselnamen oder einer Kommandozeile.
///
/// Bei MSI ist der Registry-Schlüssel die Produkt-GUID – daraus lässt sich der
/// Aufruf sicher rekonstruieren, statt die Zeichenkette zu zerlegen.
pub fn msi_guid(schluessel: &str, uninstall_string: &str) -> Option<String> {
    let ist_guid = |s: &str| {
        s.len() == 38
            && s.starts_with('{')
            && s.ends_with('}')
            && s[1..37].chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    };

    if ist_guid(schluessel) {
        return Some(schluessel.to_string());
    }

    // Sonst aus der Kommandozeile herauslösen: MsiExec.exe /X{GUID}
    let text = uninstall_string;
    let start = text.find('{')?;
    let ende = text[start..].find('}')? + start + 1;
    let kandidat = &text[start..ende];
    ist_guid(kandidat).then(|| kandidat.to_string())
}

/// Eine Kommandozeile in Programm und Argumente zerlegen.
///
/// Behandelt die drei Formen, die in `UninstallString` vorkommen:
/// `"C:\P\u.exe" /S`, `C:\P\u.exe /S` und `MsiExec.exe /X{GUID}`.
/// Bewusst **kein** Umweg über `cmd.exe` – der wäre eine Injektionsfläche.
pub fn zerlege_befehl(zeile: &str) -> Option<(String, Vec<String>)> {
    let text = zeile.trim();
    if text.is_empty() {
        return None;
    }

    if let Some(rest) = text.strip_prefix('"') {
        let ende = rest.find('"')?;
        let programm = rest[..ende].to_string();
        let argumente = rest[ende + 1..]
            .split_whitespace()
            .map(str::to_string)
            .collect();
        return Some((programm, argumente));
    }

    // Ohne Anführungszeichen: bis zur ersten ausführbaren Endung suchen.
    // `C:\Program Files\X\uninst.exe -mode` hat Leerzeichen im Pfad, ein
    // schlichtes split_whitespace() würde ihn zerreißen.
    let klein = text.to_ascii_lowercase();
    for endung in [".exe", ".com", ".bat", ".cmd"] {
        if let Some(i) = klein.find(endung) {
            let schnitt = i + endung.len();
            let programm = text[..schnitt].to_string();
            let argumente = text[schnitt..]
                .split_whitespace()
                .map(str::to_string)
                .collect();
            return Some((programm, argumente));
        }
    }

    let mut teile = text.split_whitespace();
    let programm = teile.next()?.to_string();
    Some((programm, teile.map(str::to_string).collect()))
}

/// Exitcodes des Windows Installers deuten.
///
/// Der häufigste Integrationsfehler fremder Werkzeuge ist, `3010` als
/// Fehlschlag zu behandeln – dabei bedeutet er Erfolg mit Neustartbedarf.
pub fn deute_exitcode(code: i32) -> (bool, bool, &'static str) {
    match code {
        0 => (true, false, ""),
        // Erfolg, Neustart nötig bzw. bereits eingeleitet.
        3010 => (true, true, "uninstall.reboot_required"),
        1641 => (true, true, "uninstall.reboot_started"),
        // Produkt war gar nicht installiert – nichts zu tun ist kein Fehler.
        1605 => (true, false, "uninstall.not_installed"),
        1602 => (false, false, "uninstall.user_cancelled"),
        1618 => (false, false, "uninstall.installer_busy"),
        1625 => (false, false, "uninstall.blocked_by_policy"),
        _ => (false, false, "uninstall.failed"),
    }
}

/// Argumente für eine stille MSI-Deinstallation.
pub fn msi_argumente(guid: &str) -> Vec<String> {
    vec![
        "/x".to_string(),
        guid.to_string(),
        "/qn".to_string(),
        // Ohne /norestart startet msiexec den Rechner eigenmächtig neu.
        "/norestart".to_string(),
    ]
}

// ---------------------------------------------------------------------------
// Auflisten
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod windows_impl {
    use super::*;
    use winreg::enums::*;
    use winreg::RegKey;

    /// Handle-Typ der Registry-Wurzeln.
    type RegistryWurzel = windows_sys::Win32::System::Registry::HKEY;

    const UNINSTALL_PFAD: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";

    /// Update-Typen, die Windows selbst nicht in „Apps & Features" zeigt.
    const UPDATE_TYPEN: &[&str] = &[
        "update",
        "hotfix",
        "security update",
        "update rollup",
        "servicepack",
    ];

    /// Alle installierten Programme.
    pub fn liste() -> Vec<Programm> {
        let mut gefunden = Vec::new();

        // Die logischen Pfade mit ausdrücklicher Sicht öffnen statt
        // `WOW6432Node` hart zu verdrahten – der Knotenname ist reserviert.
        for (hive, sicht, quelle) in [
            (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY, Quelle::Machine),
            (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY, Quelle::Machine32),
            (HKEY_CURRENT_USER, 0, Quelle::User),
        ] {
            lies_hive(hive, sicht, quelle, &mut gefunden);
        }

        gefunden.extend(store_pakete());

        // Dubletten: derselbe Eintrag kann in beiden Sichten stehen.
        gefunden.sort_by_key(|p| p.name.to_lowercase());
        gefunden.dedup_by(|a, b| a.name == b.name && a.version == b.version);
        gefunden
    }

    fn lies_hive(hive: RegistryWurzel, sicht: u32, quelle: Quelle, ziel: &mut Vec<Programm>) {
        let wurzel = RegKey::predef(hive);
        let Ok(uninstall) = wurzel.open_subkey_with_flags(UNINSTALL_PFAD, KEY_READ | sicht) else {
            return;
        };

        for name in uninstall.enum_keys().flatten() {
            let Ok(eintrag) = uninstall.open_subkey_with_flags(&name, KEY_READ | sicht) else {
                continue;
            };
            if let Some(programm) = aus_eintrag(&eintrag, &name, quelle) {
                ziel.push(programm);
            }
        }
    }

    fn lies(schluessel: &RegKey, name: &str) -> String {
        schluessel.get_value::<String, _>(name).unwrap_or_default()
    }

    fn lies_zahl(schluessel: &RegKey, name: &str) -> u32 {
        schluessel.get_value::<u32, _>(name).unwrap_or(0)
    }

    /// Woher das Programmsymbol kommt.
    ///
    /// `DisplayIcon` ist der vorgesehene Weg, fehlt aber bei etwa jedem
    /// dritten Eintrag. Dann taugt die Deinstallationsdatei als Ersatz —
    /// allerdings nur, wenn es eine echte `.exe` ist: bei MSI-Paketen steht
    /// dort `msiexec.exe`, und dessen Symbol vor jedem zweiten Programm wäre
    /// schlechter als gar keins.
    fn symbolquelle(schluessel: &RegKey, uninstall_string: &str) -> String {
        let angabe = lies(schluessel, "DisplayIcon");
        if !angabe.trim().is_empty() {
            return angabe;
        }

        // Die Deinstallationszeile trägt fast immer Argumente
        // (`"…\unins000.exe" /SILENT`). Ohne sauberes Zerlegen bliebe hier
        // fast nichts übrig — vor dieser Korrektur hatten von 136 Einträgen
        // nur 24 überhaupt eine Symbolquelle.
        if let Some((programm, _)) = zerlege_befehl(uninstall_string) {
            let klein = programm.to_ascii_lowercase();
            if klein.ends_with(".exe") && !klein.contains("msiexec") {
                return programm;
            }
        }

        // MSI-Pakete verweisen auf `msiexec.exe`; dessen Symbol vor jedem
        // zweiten Programm wäre schlechter als gar keins. Stattdessen die
        // Hauptanwendung im Installationsordner suchen.
        symbol_im_ordner(&lies(schluessel, "InstallLocation"))
    }

    /// Die wahrscheinlichste Programmdatei in einem Installationsordner.
    ///
    /// Ein Ordner enthält oft mehrere `.exe` — Hilfsprogramme, Updater,
    /// Deinstaller. Genommen wird die **größte**, denn das ist in aller Regel
    /// die Hauptanwendung, und nur die trägt ein aussagekräftiges Symbol.
    /// Gesucht wird bewusst nur eine Ebene tief: ein rekursiver Durchlauf
    /// über 100 Installationsordner würde die Liste spürbar verzögern.
    fn symbol_im_ordner(ordner: &str) -> String {
        let pfad = ordner.trim().trim_matches('"');
        if pfad.is_empty() {
            return String::new();
        }

        let Ok(eintraege) = std::fs::read_dir(pfad) else {
            return String::new();
        };

        let mut beste: Option<(u64, String)> = None;
        for eintrag in eintraege.flatten() {
            let name = eintrag.file_name().to_string_lossy().to_ascii_lowercase();
            if !name.ends_with(".exe") || name.starts_with("unins") {
                continue;
            }
            let groesse = eintrag.metadata().map(|m| m.len()).unwrap_or(0);
            if beste.as_ref().is_none_or(|(bisher, _)| groesse > *bisher) {
                beste = Some((groesse, eintrag.path().to_string_lossy().to_string()));
            }
        }

        beste.map(|(_, pfad)| pfad).unwrap_or_default()
    }

    /// Einen Registry-Eintrag in ein [`Programm`] übersetzen – oder ihn
    /// verwerfen.
    ///
    /// Die Filterregeln bilden nach, was Windows selbst in „Apps & Features"
    /// zeigt. Es gibt dafür keinen dokumentierten Vertrag; das hier ist die
    /// etablierte Annäherung.
    fn aus_eintrag(schluessel: &RegKey, key_name: &str, quelle: Quelle) -> Option<Programm> {
        let name = lies(schluessel, "DisplayName");
        if name.trim().is_empty() {
            return None;
        }
        if lies_zahl(schluessel, "SystemComponent") == 1 {
            return None;
        }
        if !lies(schluessel, "ParentKeyName").is_empty() {
            return None;
        }

        let release = lies(schluessel, "ReleaseType").to_ascii_lowercase();
        if UPDATE_TYPEN.iter().any(|t| release == *t) {
            return None;
        }
        // Update-Altlasten heißen KB gefolgt von Ziffern.
        if key_name.len() > 2
            && key_name.starts_with("KB")
            && key_name[2..].chars().all(|c| c.is_ascii_digit())
        {
            return None;
        }

        let uninstall_string = lies(schluessel, "UninstallString");
        let quiet_string = lies(schluessel, "QuietUninstallString");
        let ist_msi = lies_zahl(schluessel, "WindowsInstaller") == 1
            || uninstall_string.to_ascii_lowercase().contains("msiexec");

        // Ohne Deinstallationsweg nur informativ.
        let entfernbar_technisch = !uninstall_string.is_empty() || !quiet_string.is_empty();

        let publisher = lies(schluessel, "Publisher");
        let mut schutz = schutzgrund(&name, &publisher, quelle).unwrap_or_default();

        // Windows selbst bietet diese Einträge nicht zum Entfernen an.
        if lies_zahl(schluessel, "NoRemove") == 1 {
            schutz = "uninstall.protected.noremove";
        }
        if !entfernbar_technisch && schutz.is_empty() {
            schutz = "uninstall.protected.no_uninstaller";
        }

        Some(Programm {
            id: key_name.to_string(),
            name,
            version: lies(schluessel, "DisplayVersion"),
            publisher,
            install_date: lies(schluessel, "InstallDate"),
            // EstimatedSize steht in KiB.
            size: u64::from(lies_zahl(schluessel, "EstimatedSize")) * 1024,
            source: quelle,
            removable: schutz.is_empty(),
            protection: schutz.to_string(),
            quiet: !quiet_string.is_empty() || ist_msi,
            requires_admin: quelle != Quelle::User,
            icon: symbolquelle(schluessel, &uninstall_string),
        })
    }

    /// Store-Pakete des angemeldeten Nutzers.
    ///
    /// Über PowerShell statt der WinRT-API: letztere wäre robuster, verlangt
    /// aber die schwergewichtige `windows`-Kiste mit
    /// `Management_Deployment`-Feature. Für eine Liste, die der Nutzer ohnehin
    /// durchsieht, ist der Aufruf vertretbar.
    fn store_pakete() -> Vec<Programm> {
        let ausgabe = process::run(&[
            "powershell",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "Get-AppxPackage | Where-Object { $_.SignatureKind -ne 'System' -and \
             -not $_.IsFramework -and -not $_.IsResourcePackage } | \
             ForEach-Object { \"$($_.Name)|$($_.PackageFullName)|$($_.Version)|$($_.Publisher)\" }",
        ]);

        if !ausgabe.ok() {
            return Vec::new();
        }

        ausgabe
            .stdout
            .lines()
            .filter_map(|zeile| {
                let teile: Vec<&str> = zeile.trim().split('|').collect();
                if teile.len() < 4 || teile[0].is_empty() {
                    return None;
                }
                let name = teile[0].to_string();
                let schutz = schutzgrund(&name, teile[3], Quelle::Store).unwrap_or_default();

                Some(Programm {
                    id: teile[1].to_string(),
                    name,
                    version: teile[2].to_string(),
                    publisher: kurzer_herausgeber(teile[3]),
                    install_date: String::new(),
                    size: 0,
                    source: Quelle::Store,
                    removable: schutz.is_empty(),
                    protection: schutz.to_string(),
                    quiet: true,
                    requires_admin: false,
                    // Store-Pakete legen ihr Symbol im Paketmanifest ab, nicht
                    // in der Registry. Das auszulesen verlangte die WinRT-API;
                    // die Oberfläche zeigt hier ein Ersatzbild.
                    icon: String::new(),
                })
            })
            .collect()
    }

    /// `CN=Microsoft Corporation, O=…` → `Microsoft Corporation`.
    fn kurzer_herausgeber(dn: &str) -> String {
        dn.split(',')
            .find_map(|teil| teil.trim().strip_prefix("CN="))
            .unwrap_or(dn)
            .to_string()
    }

    /// Ein Programm deinstallieren.
    pub fn deinstalliere(programm: &Programm, still: bool) -> UninstallResult {
        let mut ergebnis = UninstallResult {
            id: programm.id.clone(),
            name: programm.name.clone(),
            ok: false,
            exit_code: -1,
            reboot_required: false,
            verified: false,
            message: String::new(),
        };

        if !programm.removable {
            ergebnis.message = "uninstall.protected".to_string();
            return ergebnis;
        }

        let ausgabe = match programm.source {
            Quelle::Store => process::run(&[
                "powershell",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("Remove-AppxPackage -Package '{}'", programm.id),
            ]),
            _ => match befehl_fuer(programm, still) {
                Some((programmpfad, argumente)) => {
                    let mut argv: Vec<&str> = vec![programmpfad.as_str()];
                    argv.extend(argumente.iter().map(String::as_str));
                    process::run_with_timeout(&argv, std::time::Duration::from_secs(1800))
                }
                None => {
                    ergebnis.message = "uninstall.no_command".to_string();
                    return ergebnis;
                }
            },
        };

        ergebnis.exit_code = ausgabe.code;
        let (erfolg, neustart, meldung) = deute_exitcode(ausgabe.code);
        ergebnis.ok = erfolg;
        ergebnis.reboot_required = neustart;
        ergebnis.message = meldung.to_string();

        // Nicht-MSI-Deinstaller liefern beliebige Exitcodes, oft immer 0. Der
        // verlässliche Test ist, ob der Registry-Eintrag verschwunden ist.
        if programm.source != Quelle::Store {
            ergebnis.verified = !eintrag_existiert(&programm.id, programm.source);
            if ergebnis.verified {
                ergebnis.ok = true;
                if ergebnis.message == "uninstall.failed" {
                    ergebnis.message = String::new();
                }
            } else if ergebnis.ok && ergebnis.message.is_empty() {
                // Erfolg gemeldet, Eintrag aber noch da: manche Deinstaller
                // räumen verzögert auf.
                ergebnis.message = "uninstall.unverified".to_string();
            }
        } else {
            ergebnis.verified = erfolg;
        }

        ergebnis
    }

    /// Kommandozeile für die Deinstallation zusammenstellen.
    fn befehl_fuer(programm: &Programm, still: bool) -> Option<(String, Vec<String>)> {
        let (hive, sicht) = match programm.source {
            Quelle::Machine => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            Quelle::Machine32 => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
            _ => (HKEY_CURRENT_USER, 0),
        };

        let wurzel = RegKey::predef(hive);
        let eintrag = wurzel
            .open_subkey_with_flags(
                format!("{UNINSTALL_PFAD}\\{}", programm.id),
                KEY_READ | sicht,
            )
            .ok()?;

        let uninstall_string = lies(&eintrag, "UninstallString");
        let quiet_string = lies(&eintrag, "QuietUninstallString");

        // MSI: den Aufruf aus der GUID rekonstruieren statt die Zeichenkette
        // zu zerlegen. Das ist der einzige Fall, in dem ein stiller Schalter
        // sicher bekannt ist.
        if still {
            if let Some(guid) = msi_guid(&programm.id, &uninstall_string) {
                return Some(("msiexec".to_string(), msi_argumente(&guid)));
            }
            if !quiet_string.is_empty() {
                return zerlege_befehl(&quiet_string);
            }
        }

        zerlege_befehl(&uninstall_string)
    }

    /// Existiert der Uninstall-Schlüssel noch?
    fn eintrag_existiert(id: &str, quelle: Quelle) -> bool {
        let (hive, sicht) = match quelle {
            Quelle::Machine => (HKEY_LOCAL_MACHINE, KEY_WOW64_64KEY),
            Quelle::Machine32 => (HKEY_LOCAL_MACHINE, KEY_WOW64_32KEY),
            _ => (HKEY_CURRENT_USER, 0),
        };
        RegKey::predef(hive)
            .open_subkey_with_flags(format!("{UNINSTALL_PFAD}\\{id}"), KEY_READ | sicht)
            .is_ok()
    }
}

#[cfg(not(windows))]
mod windows_impl {
    use super::*;

    pub fn liste() -> Vec<Programm> {
        Vec::new()
    }

    pub fn deinstalliere(programm: &Programm, _still: bool) -> UninstallResult {
        UninstallResult {
            id: programm.id.clone(),
            name: programm.name.clone(),
            ok: false,
            exit_code: -1,
            reboot_required: false,
            verified: false,
            message: "uninstall.windows_only".to_string(),
        }
    }
}

pub use windows_impl::{deinstalliere, liste};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laufzeitpakete_sind_geschuetzt() {
        for name in [
            "Microsoft Visual C++ 2015-2022 Redistributable (x64)",
            "Microsoft .NET Runtime - 8.0.11 (x64)",
            "Microsoft Edge WebView2 Runtime",
        ] {
            assert!(
                schutzgrund(name, "Microsoft Corporation", Quelle::Machine).is_some(),
                "nicht geschützt: {name}"
            );
        }
    }

    #[test]
    fn sicherheitssoftware_ist_geschuetzt() {
        assert_eq!(
            schutzgrund("Microsoft Defender", "Microsoft", Quelle::Machine),
            Some("uninstall.protected.security")
        );
    }

    #[test]
    fn plane_deinstalliert_sich_nicht_selbst() {
        assert_eq!(
            schutzgrund("Plane", "Philipp Paulik", Quelle::Machine),
            Some("uninstall.protected.self")
        );
    }

    #[test]
    fn treiber_nur_bei_hersteller_und_name() {
        // Beides zusammen: geschützt.
        assert!(schutzgrund(
            "Intel Chipset Device Software",
            "Intel Corporation",
            Quelle::Machine
        )
        .is_some());
        assert!(schutzgrund(
            "NVIDIA Graphics Driver",
            "NVIDIA Corporation",
            Quelle::Machine
        )
        .is_some());

        // Hersteller allein reicht nicht – sonst wäre jedes Intel-Werkzeug
        // gesperrt.
        assert!(
            schutzgrund("Intel Unison", "Intel Corporation", Quelle::Machine).is_none(),
            "gewöhnliche Herstellersoftware darf nicht gesperrt sein"
        );
    }

    #[test]
    fn normale_programme_sind_entfernbar() {
        for name in ["7-Zip 24.09", "Notepad++", "Steam", "Spotify"] {
            assert!(
                schutzgrund(name, "Irgendwer", Quelle::Machine).is_none(),
                "fälschlich gesperrt: {name}"
            );
        }
    }

    #[test]
    fn shell_pakete_sind_geschuetzt() {
        for paket in [
            "Microsoft.WindowsStore",
            "Microsoft.UI.Xaml.2.8",
            "MicrosoftWindows.Client.WebExperience",
        ] {
            assert!(
                schutzgrund(paket, "CN=Microsoft", Quelle::Store).is_some(),
                "nicht geschützt: {paket}"
            );
        }
        assert!(schutzgrund("SpotifyAB.SpotifyMusic", "CN=Spotify", Quelle::Store).is_none());
    }

    #[test]
    fn msi_guid_aus_dem_schluesselnamen() {
        let guid = "{90160000-000F-0000-0000-0000000FF1CE}";
        assert_eq!(msi_guid(guid, "").as_deref(), Some(guid));
    }

    #[test]
    fn msi_guid_aus_der_kommandozeile() {
        let guid = "{90160000-000F-0000-0000-0000000FF1CE}";
        assert_eq!(
            msi_guid("EgalName", &format!("MsiExec.exe /X{guid}")).as_deref(),
            Some(guid)
        );
    }

    #[test]
    fn keine_guid_wo_keine_ist() {
        assert!(msi_guid("7-Zip", r#""C:\P\uninstall.exe" /S"#).is_none());
    }

    #[test]
    fn befehl_mit_anfuehrungszeichen() {
        let (programm, argumente) =
            zerlege_befehl(r#""C:\Program Files\App\unins000.exe" /SILENT"#).unwrap();
        assert_eq!(programm, r"C:\Program Files\App\unins000.exe");
        assert_eq!(argumente, vec!["/SILENT"]);
    }

    #[test]
    fn befehl_ohne_anfuehrungszeichen_mit_leerzeichen_im_pfad() {
        // Der klassische Stolperstein: split_whitespace() würde den Pfad
        // zerreißen.
        let (programm, argumente) =
            zerlege_befehl(r"C:\Program Files\App\uninst.exe -mode silent").unwrap();
        assert_eq!(programm, r"C:\Program Files\App\uninst.exe");
        assert_eq!(argumente, vec!["-mode", "silent"]);
    }

    #[test]
    fn befehl_ohne_argumente() {
        let (programm, argumente) = zerlege_befehl(r"C:\P\unins000.exe").unwrap();
        assert_eq!(programm, r"C:\P\unins000.exe");
        assert!(argumente.is_empty());
    }

    #[test]
    fn leerer_befehl_ergibt_nichts() {
        assert!(zerlege_befehl("").is_none());
        assert!(zerlege_befehl("   ").is_none());
    }

    #[test]
    fn neustartcodes_gelten_als_erfolg() {
        // Der häufigste Integrationsfehler fremder Werkzeuge.
        assert!(
            deute_exitcode(3010).0,
            "3010 bedeutet Erfolg mit Neustartbedarf"
        );
        assert!(deute_exitcode(3010).1);
        assert!(deute_exitcode(1641).0);
        assert!(deute_exitcode(1641).1);
    }

    #[test]
    fn nicht_installiert_ist_kein_fehler() {
        let (ok, neustart, meldung) = deute_exitcode(1605);
        assert!(ok);
        assert!(!neustart);
        assert_eq!(meldung, "uninstall.not_installed");
    }

    #[test]
    fn abbruch_und_fehler_werden_unterschieden() {
        assert_eq!(deute_exitcode(1602).2, "uninstall.user_cancelled");
        assert_eq!(deute_exitcode(1618).2, "uninstall.installer_busy");
        assert_eq!(deute_exitcode(1625).2, "uninstall.blocked_by_policy");
        assert_eq!(deute_exitcode(1603).2, "uninstall.failed");
        assert!(!deute_exitcode(1603).0);
    }

    #[test]
    fn erfolg_ist_stumm() {
        assert_eq!(deute_exitcode(0), (true, false, ""));
    }

    #[test]
    fn msi_argumente_verhindern_eigenmaechtigen_neustart() {
        let argumente = msi_argumente("{ABC}");
        assert!(argumente.contains(&"/norestart".to_string()));
        assert!(argumente.contains(&"/qn".to_string()));
        assert_eq!(argumente[0], "/x");
    }

    #[test]
    fn quellen_haben_stabile_schluessel() {
        assert_eq!(Quelle::Machine.key(), "machine");
        assert_eq!(Quelle::Store.key(), "store");
    }

    #[test]
    fn liste_liefert_plausible_eintraege() {
        let programme = liste();
        if !cfg!(windows) {
            return;
        }
        assert!(
            programme.len() > 3,
            "auf einem Windows-System sollten Programme gefunden werden"
        );
        for p in &programme {
            assert!(!p.name.trim().is_empty(), "Eintrag ohne Namen");
            assert!(!p.id.trim().is_empty(), "Eintrag ohne Kennung");
            if !p.removable {
                assert!(!p.protection.is_empty(), "gesperrt ohne Grund: {}", p.name);
            }
        }
    }

    #[test]
    fn geschuetzte_eintraege_werden_nicht_deinstalliert() {
        let geschuetzt = Programm {
            icon: String::new(),
            id: "X".into(),
            name: "Microsoft Visual C++ 2022".into(),
            version: String::new(),
            publisher: String::new(),
            install_date: String::new(),
            size: 0,
            source: Quelle::Machine,
            removable: false,
            protection: "uninstall.protected.runtime".into(),
            quiet: false,
            requires_admin: true,
        };

        let ergebnis = deinstalliere(&geschuetzt, true);
        assert!(!ergebnis.ok);
        assert_eq!(ergebnis.message, "uninstall.protected");
        assert_eq!(
            ergebnis.exit_code, -1,
            "es darf nichts gestartet worden sein"
        );
    }
}
