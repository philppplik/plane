//! Katalog aller Reinigungsziele.
//!
//! Der Katalog ist die einzige Stelle, an der steht, *was* Plane bereinigt.
//! Scan und Clean sind vollständig generisch – ein neues Ziel besteht aus
//! einem Eintrag hier plus zwei Übersetzungen in `crate::i18n`.
//!
//! Regeln für neue Einträge:
//!
//! * `key` ist stabil, kleingeschrieben, mit `.` gegliedert (`browser.chrome.cache`).
//! * [`Risk::Safe`] nur, wenn der Inhalt garantiert neu erzeugt wird.
//! * Alles, was Anmeldungen, Verläufe oder Nutzerdaten betrifft, ist
//!   mindestens [`Risk::Notice`] und **nie** `default_enabled`.
//! * Ziele in Systemordnern brauchen `requires_admin`.
//! * Pfade immer mit `%VAR%` – nie ein Laufwerk fest verdrahten (ARM64- und
//!   Nicht-C:-Installationen).

use super::types::{Category, FileRule, RegistryCheck, RegistryRule, Risk, Target, TargetKind};

/// Kurzschreibweise für ein Dateiziel.
const fn files(
    key: &'static str,
    category: Category,
    risk: Risk,
    default_enabled: bool,
    rules: &'static [FileRule],
) -> Target {
    Target {
        key,
        category,
        kind: TargetKind::Files(rules),
        risk,
        requires_admin: false,
        default_enabled,
        services: &[],
        blocking_processes: &[],
    }
}

const fn admin(mut target: Target) -> Target {
    target.requires_admin = true;
    target
}

const fn with_services(mut target: Target, services: &'static [&'static str]) -> Target {
    target.services = services;
    target.requires_admin = true;
    target
}

const fn blocked_by(mut target: Target, prozesse: &'static [&'static str]) -> Target {
    target.blocking_processes = prozesse;
    target
}

// ---------------------------------------------------------------------------
// Dateiregeln
// ---------------------------------------------------------------------------

const TEMP_USER: &[FileRule] = &[FileRule::new("%TEMP%/*")
    // %TEMP%\\Low braucht Windows fuer Prozesse mit niedriger Integritaetsstufe.
    .excluding(&["\\Temp\\Low"])];
const TEMP_WINDOWS: &[FileRule] = &[FileRule::new("%WINDIR%/Temp/*")];
const PREFETCH: &[FileRule] = &[FileRule::new("%WINDIR%/Prefetch/*.pf")];
const WINDOWS_UPDATE: &[FileRule] = &[
    FileRule::new("%WINDIR%/SoftwareDistribution/Download/*"),
    // Nur die Logs - DataStore.edb ist die Update-Historie und bleibt.
    FileRule::new("%WINDIR%/SoftwareDistribution/DataStore/Logs/*"),
    FileRule::new("%WINDIR%/SoftwareDistribution/*.log"),
];
const DELIVERY_OPTIMIZATION: &[FileRule] = &[
    FileRule::new("%PROGRAMDATA%/Microsoft/Windows/DeliveryOptimization/Cache/*"),
    FileRule::new(
        "%WINDIR%/ServiceProfiles/NetworkService/AppData/Local/Microsoft/Windows/DeliveryOptimization/*",
    ),
];
const FONT_CACHE: &[FileRule] = &[FileRule::new(
    "%WINDIR%/ServiceProfiles/LocalService/AppData/Local/FontCache/*.dat",
)];
const THUMBNAILS: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/Explorer/thumbcache_*.db"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/Explorer/iconcache_*.db"),
];
const ERROR_REPORTS: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/WER/ReportArchive/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/WER/ReportQueue/*"),
    FileRule::new("%PROGRAMDATA%/Microsoft/Windows/WER/ReportArchive/*"),
    FileRule::new("%PROGRAMDATA%/Microsoft/Windows/WER/ReportQueue/*"),
];
const CRASH_DUMPS: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/CrashDumps/*"),
    FileRule::new("%WINDIR%/Minidump/*.dmp"),
    FileRule::new("%WINDIR%/MEMORY.DMP"),
    FileRule::new("%WINDIR%/LiveKernelReports/**/*.dmp"),
];
const WINDOWS_LOGS: &[FileRule] = &[
    // CBS.log ist die aktive Datei und wird von TrustedInstaller gehalten.
    FileRule::new("%WINDIR%/Logs/CBS/CbsPersist_*.log").older_than(7),
    FileRule::new("%WINDIR%/Logs/CBS/*.cab").older_than(7),
    FileRule::new("%WINDIR%/Logs/DISM/*.log").older_than(7),
    FileRule::new("%WINDIR%/Logs/WindowsUpdate/*").older_than(7),
    FileRule::new("%WINDIR%/Panther/*.log").older_than(30),
    FileRule::new("%WINDIR%/INF/setupapi.dev*.log").older_than(30),
    FileRule::new("%PROGRAMDATA%/USOShared/Logs/*").older_than(7),
];
const INTERNET_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/INetCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Windows/INetCache/IE/*"),
];
const RECENT_DOCS: &[FileRule] = &[
    FileRule::new("%APPDATA%/Microsoft/Windows/Recent/*.lnk"),
    FileRule::new("%APPDATA%/Microsoft/Windows/Recent/AutomaticDestinations/*"),
];
const CLIPBOARD_AND_SEARCH: &[FileRule] = &[FileRule::new(
    "%LOCALAPPDATA%/Packages/Microsoft.Windows.Search_cw5n1h2txyewy/LocalState/AppIconCache/**/*",
)];
const WINDOWS_OLD: &[FileRule] = &[
    FileRule::new("%SYSTEMDRIVE%/Windows.old"),
    FileRule::new("%SYSTEMDRIVE%/$Windows.~BT"),
    FileRule::new("%SYSTEMDRIVE%/$Windows.~WS"),
];

// -- Browser ---------------------------------------------------------------
// Chromium-Profile heißen "Default", "Profile 1", … – deshalb `*`.
// Firefox-Profile tragen Zufallsnamen – ebenfalls `*`.

const EDGE_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/Cache/Cache_Data/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/Code Cache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/GPUCache/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/Service Worker/CacheStorage/**/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/ShaderCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/GrShaderCache/**/*"),
];
const CHROME_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/Cache/Cache_Data/*"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/Code Cache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/GPUCache/*"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/Service Worker/CacheStorage/**/*"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/ShaderCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/GrShaderCache/**/*"),
];
// Firefox-Profile tragen Zufallsnamen (<zufall>.<name>); Profil liegt unter
// %APPDATA%, der Cache unter %LOCALAPPDATA%.
const FIREFOX_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/cache2/entries/*"),
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/startupCache/*"),
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/thumbnails/*"),
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/jumpListCache/*"),
    FileRule::new("%APPDATA%/Mozilla/Firefox/Crash Reports/**/*"),
];
const BRAVE_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/BraveSoftware/Brave-Browser/User Data/*/Cache/Cache_Data/*"),
    FileRule::new("%LOCALAPPDATA%/BraveSoftware/Brave-Browser/User Data/*/Code Cache/**/*"),
    FileRule::new("%LOCALAPPDATA%/BraveSoftware/Brave-Browser/User Data/*/GPUCache/*"),
];
// Opera trennt Profil (%APPDATA%) und Cache (%LOCALAPPDATA%); die Varianten
// heissen "Opera Stable", "Opera GX Stable", "Opera Crypto Stable".
const OPERA_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Opera Software/Opera*/Cache/Cache_Data/*"),
    FileRule::new("%LOCALAPPDATA%/Opera Software/Opera*/Code Cache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Opera Software/Opera*/GPUCache/*"),
];
const VIVALDI_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Vivaldi/User Data/*/Cache/Cache_Data/*"),
    FileRule::new("%LOCALAPPDATA%/Vivaldi/User Data/*/Code Cache/**/*"),
];

const BROWSER_COOKIES: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/Network/Cookies"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/Network/Cookies"),
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/cookies.sqlite"),
    FileRule::new("%LOCALAPPDATA%/BraveSoftware/Brave-Browser/User Data/*/Network/Cookies"),
];
const BROWSER_HISTORY: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/History"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/History"),
    FileRule::new("%LOCALAPPDATA%/Mozilla/Firefox/Profiles/*/places.sqlite"),
];
const BROWSER_DOWNLOAD_META: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Edge/User Data/*/Network Action Predictor"),
    FileRule::new("%LOCALAPPDATA%/Google/Chrome/User Data/*/Network Action Predictor"),
];

// -- Anwendungen -----------------------------------------------------------

const DISCORD_CACHE: &[FileRule] = &[
    FileRule::new("%APPDATA%/discord/Cache/Cache_Data/*"),
    FileRule::new("%APPDATA%/discord/Code Cache/**/*"),
    FileRule::new("%APPDATA%/discord/GPUCache/*"),
];
const TEAMS_CACHE: &[FileRule] = &[
    FileRule::new("%APPDATA%/Microsoft/Teams/Cache/*"),
    FileRule::new("%APPDATA%/Microsoft/Teams/Code Cache/**/*"),
    FileRule::new("%APPDATA%/Microsoft/Teams/GPUCache/*"),
    FileRule::new("%APPDATA%/Microsoft/Teams/blob_storage/**/*"),
    FileRule::new("%APPDATA%/Microsoft/Teams/tmp/*"),
    FileRule::new("%APPDATA%/Microsoft Teams/Logs/*").older_than(7),
    // Neues Teams als MSIX-Paket.
    FileRule::new("%LOCALAPPDATA%/Packages/MSTeams_8wekyb3d8bbwe/LocalCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Packages/MSTeams_8wekyb3d8bbwe/TempState/**/*"),
];
// Bewusst OHNE %LOCALAPPDATA%\\Spotify\\Storage: dort liegen heruntergeladene
// Titel fuer die Offline-Wiedergabe.
const SPOTIFY_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Spotify/Data/*"),
    FileRule::new("%LOCALAPPDATA%/Spotify/Browser/Cache/*"),
    FileRule::new("%APPDATA%/Spotify/*.log").older_than(7),
];
// Bewusst OHNE %APPDATA%\\Code\\User\\History: das ist die lokale
// Datei-Zeitleiste ("Local History") und damit ein Sicherungsnetz.
const VSCODE_CACHE: &[FileRule] = &[
    FileRule::new("%APPDATA%/Code/Cache/*"),
    FileRule::new("%APPDATA%/Code/CachedData/*"),
    FileRule::new("%APPDATA%/Code/CachedExtensionVSIXs/*"),
    FileRule::new("%APPDATA%/Code/Code Cache/**/*"),
    FileRule::new("%APPDATA%/Code/GPUCache/*"),
    FileRule::new("%APPDATA%/Code/logs/*").older_than(7),
    FileRule::new("%APPDATA%/Code/Crashpad/**/*"),
];
const GPU_SHADER_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/NVIDIA/DXCache/*"),
    FileRule::new("%LOCALAPPDATA%/NVIDIA/GLCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/NVIDIA Corporation/NV_Cache/*"),
    FileRule::new("%LOCALAPPDATA%/AMD/DxCache/*"),
    FileRule::new("%LOCALAPPDATA%/AMD/DxcCache/*"),
    FileRule::new("%LOCALAPPDATA%/AMD/GLCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/AMD/VkCache/**/*"),
    // Auf ARM64-Geraeten (Adreno) fehlen die Herstellerordner, D3DSCache nicht.
    FileRule::new("%LOCALAPPDATA%/D3DSCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Intel/ShaderCache/**/*"),
];
const STEAM_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Steam/htmlcache/**/*"),
    FileRule::new("%PROGRAMFILES(X86)%/Steam/appcache/httpcache/**/*"),
    FileRule::new("%PROGRAMFILES(X86)%/Steam/logs/*").older_than(7),
    FileRule::new("%PROGRAMFILES(X86)%/Steam/dumps/*").older_than(7),
];
const ADOBE_CACHE: &[FileRule] = &[
    FileRule::new("%APPDATA%/Adobe/Common/Media Cache Files/*"),
    FileRule::new("%APPDATA%/Adobe/Common/Media Cache/*"),
];
const JAVA_CACHE: &[FileRule] = &[FileRule::new(
    "%LOCALAPPDATA%/Sun/Java/Deployment/cache/**/*",
)];
// Bewusst OHNE OfficeFileCache: dort liegen noch nicht hochgeladene
// Aenderungen an Cloud-Dokumenten - Loeschen bedeutet Datenverlust.
const OFFICE_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/Microsoft/Office/*/WebServiceCache/**/*"),
    FileRule::new("%LOCALAPPDATA%/Microsoft/Office/OffDiag/*"),
];
const NPM_PIP_CACHE: &[FileRule] = &[
    FileRule::new("%LOCALAPPDATA%/npm-cache/_cacache/**/*"),
    FileRule::new("%LOCALAPPDATA%/pip/cache/**/*"),
];

// -- Registry --------------------------------------------------------------

// Microsoft rät von Registry-Cleaning ausdrücklich ab (KB2563254), und der
// messbare Nutzen ist null. Plane bietet deshalb nur die nachweislich
// risikoarme Teilmenge an und **verzichtet bewusst** auf die Bereiche, die
// dokumentiert Schäden verursachen: CLSID/ActiveX, TypeLib, Interface,
// MSI-Installer-Komponenten, Dienste, Firewall-Regeln und Dateizuordnungen.
// Begründung und Quellen: docs/REINIGUNGSZIELE.md.

/// Verwaiste Verweise – nur Schlüssel, deren Wert eindeutig ein Dateipfad ist.
const REGISTRY_ORPHAN_RULES: &[RegistryRule] = &[
    RegistryRule {
        hive: "HKLM",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\SharedDLLs",
        check: RegistryCheck::ValueNameIsPath,
    },
    RegistryRule {
        hive: "HKCU",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
        check: RegistryCheck::ValueIsPath { value: "" },
    },
    RegistryRule {
        hive: "HKLM",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run",
        check: RegistryCheck::ValueIsPath { value: "" },
    },
    RegistryRule {
        hive: "HKLM",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths",
        check: RegistryCheck::SubkeyProgId,
    },
    RegistryRule {
        hive: "HKCU",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        check: RegistryCheck::UninstallEntry,
    },
    RegistryRule {
        hive: "HKLM",
        path: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        check: RegistryCheck::UninstallEntry,
    },
];

/// Spurenreste: reine Caches und „Zuletzt verwendet“-Listen.
///
/// Das ist eine **Datenschutz**funktion, keine Reparatur – die Einträge
/// regenerieren sich und ihr Entfernen beschleunigt nichts.
const REGISTRY_PRIVACY_RULES: &[RegistryRule] = &[
    RegistryRule {
        hive: "HKCU",
        path: r"SOFTWARE\Microsoft\Windows\ShellNoRoam\MUICache",
        check: RegistryCheck::ValueNameIsPath,
    },
    RegistryRule {
        hive: "HKCU",
        path: r"SOFTWARE\Classes\Local Settings\Software\Microsoft\Windows\Shell\MuiCache",
        check: RegistryCheck::ValueNameIsPath,
    },
];

// ---------------------------------------------------------------------------
// Katalog
// ---------------------------------------------------------------------------

/// Alle bekannten Reinigungsziele.
///
/// Reihenfolge = Anzeigereihenfolge = Ausführungsreihenfolge.
pub static TARGETS: &[Target] = &[
    // --- System ---
    files(
        "system.temp.user",
        Category::System,
        Risk::Safe,
        true,
        TEMP_USER,
    ),
    admin(files(
        "system.temp.windows",
        Category::System,
        Risk::Safe,
        true,
        TEMP_WINDOWS,
    )),
    files(
        "system.thumbnails",
        Category::System,
        Risk::Safe,
        true,
        THUMBNAILS,
    ),
    files(
        "system.inetcache",
        Category::System,
        Risk::Safe,
        true,
        INTERNET_CACHE,
    ),
    files(
        "system.errorreports",
        Category::System,
        Risk::Safe,
        true,
        ERROR_REPORTS,
    ),
    files(
        "system.crashdumps",
        Category::System,
        Risk::Safe,
        true,
        CRASH_DUMPS,
    ),
    admin(files(
        "system.logs",
        Category::System,
        Risk::Safe,
        true,
        WINDOWS_LOGS,
    )),
    Target {
        key: "system.dns",
        category: Category::System,
        kind: TargetKind::Command(&["ipconfig", "/flushdns"]),
        risk: Risk::Safe,
        requires_admin: false,
        default_enabled: true,
        services: &[],
        blocking_processes: &[],
    },
    admin(files(
        "system.prefetch",
        Category::System,
        Risk::Notice,
        false,
        PREFETCH,
    )),
    with_services(
        files(
            "system.windowsupdate",
            Category::System,
            Risk::Notice,
            false,
            WINDOWS_UPDATE,
        ),
        &["wuauserv", "bits"],
    ),
    with_services(
        files(
            "system.deliveryoptimization",
            Category::System,
            Risk::Safe,
            false,
            DELIVERY_OPTIMIZATION,
        ),
        &["DoSvc"],
    ),
    with_services(
        files(
            "system.fontcache",
            Category::System,
            Risk::Safe,
            false,
            FONT_CACHE,
        ),
        &["FontCache"],
    ),
    files(
        "system.recentdocs",
        Category::System,
        Risk::Notice,
        false,
        RECENT_DOCS,
    ),
    files(
        "system.searchcache",
        Category::System,
        Risk::Safe,
        false,
        CLIPBOARD_AND_SEARCH,
    ),
    admin(files(
        "system.windowsold",
        Category::System,
        Risk::Caution,
        false,
        WINDOWS_OLD,
    )),
    // --- Papierkorb ---
    Target {
        key: "recyclebin.all",
        category: Category::RecycleBin,
        kind: TargetKind::RecycleBin,
        risk: Risk::Caution,
        requires_admin: false,
        default_enabled: false,
        services: &[],
        blocking_processes: &[],
    },
    // --- Browser ---
    blocked_by(
        files(
            "browser.edge.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            EDGE_CACHE,
        ),
        &["msedge.exe"],
    ),
    blocked_by(
        files(
            "browser.chrome.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            CHROME_CACHE,
        ),
        &["chrome.exe"],
    ),
    blocked_by(
        files(
            "browser.firefox.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            FIREFOX_CACHE,
        ),
        &["firefox.exe"],
    ),
    blocked_by(
        files(
            "browser.brave.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            BRAVE_CACHE,
        ),
        &["brave.exe"],
    ),
    blocked_by(
        files(
            "browser.opera.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            OPERA_CACHE,
        ),
        &["opera.exe"],
    ),
    blocked_by(
        files(
            "browser.vivaldi.cache",
            Category::Browsers,
            Risk::Safe,
            true,
            VIVALDI_CACHE,
        ),
        &["vivaldi.exe"],
    ),
    files(
        "browser.cookies",
        Category::Browsers,
        Risk::Caution,
        false,
        BROWSER_COOKIES,
    ),
    files(
        "browser.history",
        Category::Browsers,
        Risk::Notice,
        false,
        BROWSER_HISTORY,
    ),
    files(
        "browser.predictor",
        Category::Browsers,
        Risk::Safe,
        false,
        BROWSER_DOWNLOAD_META,
    ),
    // --- Anwendungen ---
    blocked_by(
        files(
            "app.discord",
            Category::Apps,
            Risk::Safe,
            true,
            DISCORD_CACHE,
        ),
        &["Discord.exe"],
    ),
    blocked_by(
        files("app.teams", Category::Apps, Risk::Safe, true, TEAMS_CACHE),
        &["ms-teams.exe", "Teams.exe"],
    ),
    files(
        "app.spotify",
        Category::Apps,
        Risk::Safe,
        true,
        SPOTIFY_CACHE,
    ),
    files("app.vscode", Category::Apps, Risk::Safe, true, VSCODE_CACHE),
    files(
        "app.gpushadercache",
        Category::Apps,
        Risk::Safe,
        true,
        GPU_SHADER_CACHE,
    ),
    files("app.steam", Category::Apps, Risk::Safe, false, STEAM_CACHE),
    files(
        "app.adobe",
        Category::Apps,
        Risk::Notice,
        false,
        ADOBE_CACHE,
    ),
    files("app.java", Category::Apps, Risk::Safe, false, JAVA_CACHE),
    files(
        "app.office",
        Category::Apps,
        Risk::Safe,
        false,
        OFFICE_CACHE,
    ),
    files(
        "app.packagemanagers",
        Category::Apps,
        Risk::Notice,
        false,
        NPM_PIP_CACHE,
    ),
    // --- Installationsdateien (nur Vorschlag) ---
    Target {
        key: "installers.downloads",
        category: Category::Installers,
        kind: TargetKind::Installers,
        risk: Risk::Notice,
        requires_admin: false,
        default_enabled: false,
        services: &[],
        blocking_processes: &[],
    },
    // --- Registry ---
    Target {
        key: "registry.privacy",
        category: Category::Registry,
        kind: TargetKind::Registry(REGISTRY_PRIVACY_RULES),
        risk: Risk::Notice,
        requires_admin: false,
        default_enabled: false,
        services: &[],
        blocking_processes: &[],
    },
    Target {
        key: "registry.orphans",
        category: Category::Registry,
        kind: TargetKind::Registry(REGISTRY_ORPHAN_RULES),
        risk: Risk::Caution,
        requires_admin: false,
        default_enabled: false,
        services: &[],
        blocking_processes: &[],
    },
];

/// Ziel zu einem Schlüssel.
pub fn target_by_key(key: &str) -> Option<&'static Target> {
    let key = key.trim().to_ascii_lowercase();
    TARGETS.iter().find(|t| t.key == key)
}

/// Alle Ziele einer Kategorie.
pub fn targets_in(category: Category) -> impl Iterator<Item = &'static Target> {
    TARGETS.iter().filter(move |t| t.category == category)
}

/// Standardauswahl: die vorausgewählten Ziele.
pub fn default_selection() -> Vec<&'static str> {
    TARGETS
        .iter()
        .filter(|t| t.default_enabled)
        .map(|t| t.key)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schluessel_sind_eindeutig() {
        let mut gesehen: Vec<&str> = Vec::new();
        for target in TARGETS {
            assert!(
                !gesehen.contains(&target.key),
                "Doppelter Schlüssel: {}",
                target.key
            );
            gesehen.push(target.key);
        }
    }

    #[test]
    fn schluessel_folgen_der_namenskonvention() {
        for target in TARGETS {
            assert_eq!(
                target.key,
                target.key.to_ascii_lowercase(),
                "{} ist nicht kleingeschrieben",
                target.key
            );
            assert!(
                !target.key.contains(' '),
                "{} enthält Leerzeichen",
                target.key
            );
            assert!(
                target.key.contains('.'),
                "{} sollte gegliedert sein (bereich.name)",
                target.key
            );
        }
    }

    #[test]
    fn jedes_ziel_hat_eine_wirkung() {
        for target in TARGETS {
            match target.kind {
                TargetKind::Files(regeln) => {
                    assert!(!regeln.is_empty(), "{} hat keine Regeln", target.key)
                }
                TargetKind::Command(argv) => {
                    assert!(!argv.is_empty(), "{} hat keinen Befehl", target.key)
                }
                TargetKind::Registry(regeln) => {
                    assert!(!regeln.is_empty(), "{} hat keine Regeln", target.key)
                }
                TargetKind::RecycleBin | TargetKind::Installers => {}
            }
        }
    }

    #[test]
    fn riskante_ziele_sind_nie_vorausgewaehlt() {
        for target in TARGETS {
            if target.risk == Risk::Caution {
                assert!(
                    !target.default_enabled,
                    "{} ist riskant und darf nicht vorausgewählt sein",
                    target.key
                );
            }
        }
    }

    #[test]
    fn dienststeuerung_erfordert_adminrechte() {
        for target in TARGETS {
            if !target.services.is_empty() {
                assert!(
                    target.requires_admin,
                    "{} steuert Dienste ohne requires_admin",
                    target.key
                );
            }
        }
    }

    #[test]
    fn pfade_verdrahten_kein_laufwerk() {
        for target in TARGETS {
            if let TargetKind::Files(regeln) = target.kind {
                for regel in regeln {
                    assert!(
                        regel.pattern.starts_with('%'),
                        "{}: Muster ohne Umgebungsvariable: {}",
                        target.key,
                        regel.pattern
                    );
                }
            }
        }
    }

    #[test]
    fn muster_verwenden_vorwaertsschraegstriche() {
        // Einheitliche Schreibweise; die Umwandlung passiert in expand_vars.
        for target in TARGETS {
            if let TargetKind::Files(regeln) = target.kind {
                for regel in regeln {
                    assert!(
                        !regel.pattern.contains('\\'),
                        "{}: Muster mit Backslash: {}",
                        target.key,
                        regel.pattern
                    );
                }
            }
        }
    }

    #[test]
    fn windows_old_ist_als_riskant_markiert() {
        let target = target_by_key("system.windowsold").unwrap();
        assert_eq!(target.risk, Risk::Caution);
        assert!(target.requires_admin);
        assert!(!target.default_enabled);
    }

    #[test]
    fn cookies_sind_riskant_und_nicht_vorausgewaehlt() {
        let target = target_by_key("browser.cookies").unwrap();
        assert_eq!(target.risk, Risk::Caution);
        assert!(!target.default_enabled);
    }

    #[test]
    fn papierkorb_ist_riskant_und_nicht_vorausgewaehlt() {
        let target = target_by_key("recyclebin.all").unwrap();
        assert_eq!(target.risk, Risk::Caution);
        assert!(!target.default_enabled);
    }

    #[test]
    fn registry_ist_riskant_und_nicht_vorausgewaehlt() {
        let target = target_by_key("registry.orphans").unwrap();
        assert_eq!(target.risk, Risk::Caution);
        assert!(!target.default_enabled);
    }

    #[test]
    fn installationsdateien_sind_nur_ein_vorschlag() {
        let target = target_by_key("installers.downloads").unwrap();
        assert!(target.is_suggestion_only());
        assert!(!target.default_enabled);
    }

    #[test]
    fn target_by_key_ignoriert_gross_klein() {
        assert!(target_by_key("  SYSTEM.TEMP.USER ").is_some());
        assert!(target_by_key("gibtesnicht").is_none());
    }

    #[test]
    fn jede_kategorie_hat_mindestens_ein_ziel() {
        for kategorie in Category::ALL {
            assert!(
                targets_in(*kategorie).next().is_some(),
                "Kategorie {} ist leer",
                kategorie.key()
            );
        }
    }

    #[test]
    fn standardauswahl_ist_ausschliesslich_unbedenklich() {
        for key in default_selection() {
            let target = target_by_key(key).unwrap();
            assert_ne!(
                target.risk,
                Risk::Caution,
                "{key} ist in der Standardauswahl, aber riskant"
            );
        }
    }

    #[test]
    fn standardauswahl_ist_nicht_leer() {
        assert!(default_selection().len() >= 10);
    }

    #[test]
    fn katalog_deckt_die_wichtigsten_bereiche_ab() {
        for key in [
            "system.temp.user",
            "system.thumbnails",
            "browser.edge.cache",
            "browser.chrome.cache",
            "browser.firefox.cache",
            "recyclebin.all",
            "installers.downloads",
            "registry.orphans",
        ] {
            assert!(target_by_key(key).is_some(), "Ziel fehlt: {key}");
        }
    }
}
