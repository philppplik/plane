//! Dateisystem-Hilfen: Pfadauflösung, Glob-Suche, Größenermittlung, sicheres
//! Löschen.
//!
//! Alle Löschvorgänge laufen über [`remove_entry`], damit die Schutzregeln aus
//! [`path_is_allowed`] an genau einer Stelle greifen.

use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Systemlaufwerk inklusive Trenner, z. B. `C:\`.
pub fn system_drive() -> String {
    if cfg!(windows) {
        let laufwerk = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        format!("{laufwerk}\\")
    } else {
        "/".to_string()
    }
}

/// Von Plane definierte Kürzel für Shell-Ordner.
///
/// `%USERPROFILE%\Downloads` wäre falsch, sobald OneDrive die Ordner
/// umgeleitet hat – der echte Pfad steht in der Shell-Ordnerdefinition.
const SHELL_FOLDERS: &[(&str, &str, &str)] = &[
    (
        "DOWNLOADS",
        "{374DE290-123F-4565-9164-39C4925E467B}",
        "Downloads",
    ),
    ("DESKTOP", "Desktop", "Desktop"),
    ("DOCUMENTS", "Personal", "Documents"),
];

/// Shell-Ordner auflösen. Fällt auf `%USERPROFILE%\<name>` zurück.
pub fn known_folder(kuerzel: &str) -> Option<String> {
    let (_, registry_name, fallback) = SHELL_FOLDERS
        .iter()
        .find(|(name, _, _)| name.eq_ignore_ascii_case(kuerzel))?;

    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        if let Ok(schluessel) = hkcu
            .open_subkey(r"SOFTWARE\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders")
        {
            if let Ok(wert) = schluessel.get_value::<String, _>(registry_name) {
                // Der Wert kann selbst Umgebungsvariablen enthalten.
                if let Some(aufgeloest) = expand_env_only(&wert) {
                    return Some(aufgeloest);
                }
            }
        }
    }

    let profil = std::env::var("USERPROFILE").ok()?;
    Some(format!(
        "{}{}{fallback}",
        profil.trim_end_matches(['\\', '/']),
        std::path::MAIN_SEPARATOR
    ))
}

/// Nur echte Umgebungsvariablen auflösen – ohne Shell-Ordner-Kürzel.
/// Getrennte Funktion, damit [`known_folder`] sich nicht selbst aufruft.
fn expand_env_only(pattern: &str) -> Option<String> {
    let mut ergebnis = String::with_capacity(pattern.len());
    let mut rest = pattern;

    while let Some(start) = rest.find('%') {
        ergebnis.push_str(&rest[..start]);
        let nach_start = &rest[start + 1..];
        let ende = nach_start.find('%')?;
        let name = &nach_start[..ende];
        let wert = std::env::var(name).ok()?;
        ergebnis.push_str(wert.trim_end_matches(['\\', '/']));
        rest = &nach_start[ende + 1..];
    }
    ergebnis.push_str(rest);
    Some(ergebnis)
}

/// `%VAR%` in einem Muster auflösen. `None`, wenn eine Variable fehlt.
///
/// Kennt neben den Windows-Umgebungsvariablen die Kürzel `%DOWNLOADS%`,
/// `%DESKTOP%` und `%DOCUMENTS%` (siehe [`known_folder`]).
pub fn expand_vars(pattern: &str) -> Option<String> {
    let mut ergebnis = String::with_capacity(pattern.len());
    let mut rest = pattern;

    while let Some(start) = rest.find('%') {
        ergebnis.push_str(&rest[..start]);
        let nach_start = &rest[start + 1..];
        let ende = nach_start.find('%')?;
        let name = &nach_start[..ende];

        let wert = match known_folder(name) {
            Some(pfad) => pfad,
            None => std::env::var(name).ok()?,
        };
        ergebnis.push_str(wert.trim_end_matches(['\\', '/']));
        rest = &nach_start[ende + 1..];
    }
    ergebnis.push_str(rest);

    Some(if cfg!(windows) {
        ergebnis.replace('/', "\\")
    } else {
        ergebnis
    })
}

/// `true`, wenn der Pfad ein Reparse-Point (Junction/Symlink) ist.
///
/// Entscheidend für die Sicherheit: ein rekursiver Lauf darf einer Junction
/// nicht folgen, sonst verlässt er den Zielbaum. `%TEMP%` und Cache-Ordner
/// enthalten regelmäßig solche Verweise.
pub fn is_reparse_point(pfad: &Path) -> bool {
    let Ok(meta) = fs::symlink_metadata(pfad) else {
        return false;
    };
    if meta.file_type().is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Junctions sind keine Symlinks, tragen aber dasselbe Attribut.
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    false
}

/// Alle Pfade zu einem Glob-Muster.
///
/// Unterstützt `*` je Segment und `**` für rekursiven Abstieg (Semantik der
/// `glob`-Kiste). Nicht lesbare Zweige werden übersprungen statt zu scheitern –
/// auf einem laufenden Windows ist immer irgendein Ordner gesperrt.
pub fn resolve_pattern(pattern: &str) -> Result<Vec<PathBuf>, String> {
    let aufgeloest = expand_vars(pattern)
        .ok_or_else(|| format!("Nicht auflösbare Umgebungsvariable: {pattern}"))?;

    let optionen = glob::MatchOptions {
        case_sensitive: false,
        require_literal_separator: true,
        require_literal_leading_dot: false,
    };

    let treffer = glob::glob_with(&aufgeloest, optionen)
        .map_err(|e| format!("Ungültiges Muster {pattern}: {e}"))?;

    Ok(treffer.filter_map(Result::ok).collect())
}

/// Schutz gegen versehentliches Löschen ganzer Laufwerke oder
/// Systemverzeichnisse.
///
/// Erlaubt sind nur Ziele mit mindestens zwei Pfadebenen unter dem
/// Laufwerksbuchstaben, die nicht auf einer Sperrliste stehen.
pub fn path_is_allowed(ziel: &Path) -> bool {
    let mut ebenen = 0usize;
    for teil in ziel.components() {
        if matches!(teil, Component::Normal(_)) {
            ebenen += 1;
        }
    }
    if ebenen < 2 {
        return false;
    }

    let text = ziel.to_string_lossy().to_ascii_lowercase();
    const GESPERRT: &[&str] = &[
        "\\windows\\system32",
        "\\windows\\syswow64",
        "\\windows\\winsxs",
        "\\program files\\",
        "\\program files (x86)\\",
        "\\program files (arm)\\",
        "\\users\\default\\",
    ];
    // Ausnahme: Unterhalb von System32 liegen keine Reinigungsziele, aber
    // %WINDIR%\Temp und die Dienstprofile sind ausdrücklich erlaubt.
    !GESPERRT.iter().any(|gesperrt| text.contains(gesperrt))
}

/// Größe eines Verzeichnisses in Bytes. Nicht lesbare Einträge zählen 0.
///
/// Junctions und Symlinks werden nicht verfolgt – sonst würde eine Umleitung
/// fremde Ordner in die Summe ziehen (und beim Löschen in Gefahr bringen).
pub fn dir_size(verzeichnis: &Path) -> u64 {
    if is_reparse_point(verzeichnis) {
        return 0;
    }
    walkdir::WalkDir::new(verzeichnis)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| !is_reparse_point(e.path()) || e.depth() == 0)
        .filter_map(Result::ok)
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

/// Größe eines Eintrags – Datei oder Verzeichnis.
pub fn entry_size(pfad: &Path) -> u64 {
    match fs::symlink_metadata(pfad) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => dir_size(pfad),
        Ok(meta) => meta.len(),
        Err(_) => 0,
    }
}

/// Alter eines Eintrags seit der letzten Änderung.
pub fn entry_age(pfad: &Path) -> Option<Duration> {
    let meta = fs::symlink_metadata(pfad).ok()?;
    let geaendert = meta.modified().ok()?;
    SystemTime::now().duration_since(geaendert).ok()
}

/// `true`, wenn der Eintrag mindestens `tage` alt ist. `tage == 0` ist immer wahr.
pub fn is_older_than(pfad: &Path, tage: u32) -> bool {
    if tage == 0 {
        return true;
    }
    match entry_age(pfad) {
        Some(alter) => alter.as_secs() >= u64::from(tage) * 86_400,
        // Kein Änderungsdatum lesbar: im Zweifel nicht anfassen.
        None => false,
    }
}

/// Einen Eintrag löschen. Verzeichnisse rekursiv.
///
/// Gibt die freigegebenen Bytes zurück. Die Größe wird **vor** dem Löschen
/// ermittelt, damit der gemeldete Wert der Realität entspricht.
/// Ausgang eines Löschversuchs.
///
/// Der wichtigste Unterschied gegenüber einem schlichten `Result`: **nicht
/// jeder nicht gelöschte Eintrag ist ein Fehler.** Auf einem laufenden Windows
/// ist immer irgendeine Cache-Datei von einem Programm geöffnet. Das als
/// Fehler zu melden, erzeugt eine Wand roter Zeilen für einen völlig normalen
/// Zustand – und lässt echte Fehler darin untergehen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Removal {
    /// Entfernt; enthält die freigegebenen Bytes.
    Removed(u64),
    /// War beim Zugriff schon nicht mehr da (zwischen Analyse und Bereinigung
    /// verschwunden). Kein Fehler.
    Vanished,
    /// Von einem anderen Prozess geöffnet. Bleibt erhalten, ist kein Fehler.
    /// Administratorrechte ändern daran **nichts** – eine exklusiv geöffnete
    /// Datei lässt sich auch als Administrator nicht löschen.
    InUse,
    /// Zugriff verweigert. Hier helfen erhöhte Rechte oft.
    Denied,
    /// Windows verweigert das Betreten grundsätzlich.
    ///
    /// Betrifft vor allem `INetCache\Content.IE5`: Windows legt dort eine
    /// Bereitstellung an, die es selbst als „nicht vertrauenswürdig"
    /// einstuft, und verweigert jedem Prozess das rekursive Durchlaufen —
    /// auch dem Administrator, auch dem System. Das ist kein Fehler, sondern
    /// eine Entscheidung von Windows, und der Ordner wird von Windows selbst
    /// aufgeräumt.
    Blocked,
}

/// Windows-Fehlercodes, die „gesperrt" bedeuten.
const ERROR_SHARING_VIOLATION: i32 = 32;
const ERROR_LOCK_VIOLATION: i32 = 33;
const ERROR_ACCESS_DENIED: i32 = 5;

/// `ERROR_UNTRUSTED_MOUNT_POINT` – Windows lässt den Pfad nicht durchlaufen.
const ERROR_UNTRUSTED_MOUNT_POINT: i32 = 448;

/// `ERROR_CANT_ACCESS_FILE` – dieselbe Familie, andere Ursache.
const ERROR_CANT_ACCESS_FILE: i32 = 1920;

/// E/A-Fehler einordnen: erwartbarer Zustand oder echter Fehler?
fn einordnen(fehler: &std::io::Error) -> Option<Removal> {
    match fehler.kind() {
        std::io::ErrorKind::NotFound => return Some(Removal::Vanished),
        std::io::ErrorKind::PermissionDenied => return Some(Removal::Denied),
        _ => {}
    }
    match fehler.raw_os_error() {
        Some(ERROR_SHARING_VIOLATION) | Some(ERROR_LOCK_VIOLATION) => Some(Removal::InUse),
        Some(ERROR_ACCESS_DENIED) => Some(Removal::Denied),
        Some(ERROR_UNTRUSTED_MOUNT_POINT) | Some(ERROR_CANT_ACCESS_FILE) => Some(Removal::Blocked),
        _ => None,
    }
}

pub fn remove_entry(pfad: &Path, dry_run: bool) -> Result<Removal, String> {
    if !path_is_allowed(pfad) {
        return Err(format!(
            "Aus Sicherheitsgründen abgelehnt: {}",
            pfad.display()
        ));
    }

    let meta = match fs::symlink_metadata(pfad) {
        Ok(m) => m,
        Err(e) => {
            return match einordnen(&e) {
                Some(ausgang) => Ok(ausgang),
                None => Err(format!("{}: {e}", pfad.display())),
            }
        }
    };

    let ist_verweis = is_reparse_point(pfad);
    let ist_echtes_verzeichnis = meta.is_dir() && !ist_verweis;

    let groesse = if ist_echtes_verzeichnis {
        dir_size(pfad)
    } else {
        meta.len()
    };

    if dry_run {
        return Ok(Removal::Removed(groesse));
    }

    let ergebnis = if ist_echtes_verzeichnis {
        fs::remove_dir_all(pfad)
    } else if ist_verweis && meta.is_dir() {
        // Nur den Verweis entfernen, niemals sein Ziel.
        fs::remove_dir(pfad)
    } else {
        entfernen_mit_schreibschutz(pfad)
    };

    match ergebnis {
        Ok(()) => Ok(Removal::Removed(groesse)),
        Err(e) => match einordnen(&e) {
            Some(ausgang) => Ok(ausgang),
            None => Err(format!("{}: {e}", dateiname(pfad))),
        },
    }
}

/// Schreibgeschützte Dateien lassen sich unter Windows nicht direkt löschen.
/// Cache-Dateien sind gelegentlich schreibgeschützt, deshalb ein zweiter Versuch.
fn entfernen_mit_schreibschutz(pfad: &Path) -> std::io::Result<()> {
    match fs::remove_file(pfad) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let mut rechte = fs::metadata(pfad)?.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            rechte.set_readonly(false);
            fs::set_permissions(pfad, rechte)?;
            fs::remove_file(pfad)
        }
        Err(e) => Err(e),
    }
}

fn dateiname(pfad: &Path) -> String {
    pfad.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| pfad.display().to_string())
}

/// Bytes menschenlesbar formatieren (für CLI und Protokoll).
pub fn format_bytes(bytes: u64) -> String {
    const EINHEITEN: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut wert = bytes as f64;
    let mut index = 0;
    while wert >= 1024.0 && index < EINHEITEN.len() - 1 {
        wert /= 1024.0;
        index += 1;
    }
    if index == 0 {
        format!("{bytes} B")
    } else {
        format!("{wert:.1} {}", EINHEITEN[index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testordner(name: &str) -> PathBuf {
        let pfad = std::env::temp_dir().join(format!("plane_fsutil_{name}"));
        let _ = fs::remove_dir_all(&pfad);
        fs::create_dir_all(&pfad).unwrap();
        pfad
    }

    fn datei(pfad: &Path, groesse: usize) {
        if let Some(eltern) = pfad.parent() {
            fs::create_dir_all(eltern).unwrap();
        }
        fs::write(pfad, vec![0u8; groesse]).unwrap();
    }

    #[test]
    fn expand_vars_loest_variablen_auf() {
        std::env::set_var("PLANE_FS_TEST", "wert");
        let erwartet = if cfg!(windows) { "wert\\x" } else { "wert/x" };
        assert_eq!(expand_vars("%PLANE_FS_TEST%/x").as_deref(), Some(erwartet));
        assert!(expand_vars("%GIBT_ES_NICHT_98765%/x").is_none());
        assert_eq!(expand_vars("ohne").as_deref(), Some("ohne"));
    }

    #[test]
    fn expand_vars_entfernt_doppelte_trenner() {
        std::env::set_var("PLANE_FS_SLASH", "C:\\");
        let ergebnis = expand_vars("%PLANE_FS_SLASH%/Temp").unwrap();
        assert!(
            !ergebnis.contains("\\\\"),
            "doppelter Trenner in {ergebnis}"
        );
    }

    #[test]
    fn resolve_pattern_findet_dateien() {
        let ordner = testordner("resolve");
        datei(&ordner.join("a.tmp"), 10);
        datei(&ordner.join("b.log"), 10);

        let treffer = resolve_pattern(&format!("{}/*.tmp", ordner.display())).unwrap();
        assert_eq!(treffer.len(), 1);
        assert!(treffer[0].ends_with("a.tmp"));

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn resolve_pattern_steigt_mit_doppelstern_ab() {
        let ordner = testordner("rekursiv");
        datei(&ordner.join("tief/tiefer/c.tmp"), 10);

        let treffer = resolve_pattern(&format!("{}/**/*.tmp", ordner.display())).unwrap();
        assert_eq!(treffer.len(), 1);

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn resolve_pattern_meldet_fehlende_variable() {
        let fehler = resolve_pattern("%GIBT_ES_NICHT_98765%/*").unwrap_err();
        assert!(fehler.contains("Nicht auflösbare"));
    }

    #[test]
    fn resolve_pattern_ohne_treffer_ist_leer() {
        let ordner = testordner("leer");
        let treffer = resolve_pattern(&format!("{}/*.nichts", ordner.display())).unwrap();
        assert!(treffer.is_empty());
        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn path_is_allowed_schuetzt_wurzeln_und_systemordner() {
        assert!(!path_is_allowed(Path::new("C:\\")));
        assert!(!path_is_allowed(Path::new("C:\\Windows")));
        assert!(!path_is_allowed(Path::new(
            "C:\\Windows\\System32\\drivers"
        )));
        assert!(!path_is_allowed(Path::new(
            "C:\\Program Files\\Plane\\app.exe"
        )));
        assert!(path_is_allowed(Path::new("C:\\Windows\\Temp\\x.tmp")));
        assert!(path_is_allowed(Path::new(
            "C:\\Users\\p\\AppData\\Local\\Temp\\x"
        )));
    }

    #[test]
    fn dir_size_summiert_rekursiv() {
        let ordner = testordner("groesse");
        datei(&ordner.join("a.bin"), 1000);
        datei(&ordner.join("tief/b.bin"), 24);

        assert_eq!(dir_size(&ordner), 1024);
        assert_eq!(entry_size(&ordner), 1024);
        assert_eq!(entry_size(&ordner.join("a.bin")), 1000);

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn entry_size_fuer_fehlenden_pfad_ist_null() {
        assert_eq!(entry_size(Path::new("C:\\gibt_es_nicht_98765\\x")), 0);
    }

    #[test]
    fn is_older_than_null_ist_immer_wahr() {
        let ordner = testordner("alter");
        let d = ordner.join("neu.tmp");
        datei(&d, 1);
        assert!(is_older_than(&d, 0));
        assert!(!is_older_than(&d, 1), "frische Datei gilt nicht als alt");
        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn is_older_than_ohne_datei_ist_falsch() {
        assert!(!is_older_than(Path::new("C:\\gibt_es_nicht_98765"), 1));
    }

    #[test]
    fn remove_entry_loescht_und_misst() {
        let ordner = testordner("loeschen");
        let d = ordner.join("a.bin");
        datei(&d, 2048);

        assert_eq!(remove_entry(&d, false).unwrap(), Removal::Removed(2048));
        assert!(!d.exists());

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn remove_entry_im_trockenlauf_loescht_nichts() {
        let ordner = testordner("trocken");
        let d = ordner.join("a.bin");
        datei(&d, 512);

        assert_eq!(remove_entry(&d, true).unwrap(), Removal::Removed(512));
        assert!(d.exists(), "Trockenlauf darf nicht löschen");

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn remove_entry_meldet_verschwundene_datei_als_kein_fehler() {
        // Zwischen Analyse und Bereinigung kann eine Datei verschwinden.
        // Das ist kein Fehler, sondern der Normalfall bei Cache-Ordnern.
        let ordner = testordner("verschwunden");
        let d = ordner.join("gibt_es_nicht.bin");
        assert_eq!(remove_entry(&d, false).unwrap(), Removal::Vanished);
        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn gesperrte_datei_ist_kein_fehler() {
        // Eine geöffnete Datei lässt sich unter Windows nicht löschen. Das
        // muss als InUse durchkommen, nicht als Err - sonst meldet jeder
        // normale Lauf Fehler.
        let ordner = testordner("gesperrt");
        let d = ordner.join("offen.bin");
        datei(&d, 64);

        let halter = fs::OpenOptions::new().read(true).open(&d).unwrap();
        let ausgang = remove_entry(&d, false);
        drop(halter);

        match ausgang {
            Ok(Removal::Removed(_)) => {
                // Manche Dateisysteme erlauben das Löschen offener Dateien.
            }
            Ok(Removal::InUse) => {}
            anderes => panic!("unerwarteter Ausgang: {anderes:?}"),
        }

        let _ = fs::remove_dir_all(&ordner);
    }

    #[test]
    fn einordnung_kennt_die_windows_fehlercodes() {
        use std::io::{Error, ErrorKind};

        assert_eq!(
            einordnen(&Error::from_raw_os_error(ERROR_SHARING_VIOLATION)),
            Some(Removal::InUse)
        );
        assert_eq!(
            einordnen(&Error::from_raw_os_error(ERROR_LOCK_VIOLATION)),
            Some(Removal::InUse)
        );
        assert_eq!(
            einordnen(&Error::from(ErrorKind::NotFound)),
            Some(Removal::Vanished)
        );
        assert_eq!(
            einordnen(&Error::from(ErrorKind::PermissionDenied)),
            Some(Removal::Denied)
        );
        // Unbekanntes bleibt ein echter Fehler.
        assert_eq!(einordnen(&Error::from(ErrorKind::InvalidData)), None);
    }

    #[test]
    fn remove_entry_lehnt_geschuetzte_pfade_ab() {
        let fehler = remove_entry(Path::new("C:\\Windows"), true).unwrap_err();
        assert!(fehler.contains("Sicherheitsgründen"));
    }

    #[test]
    fn format_bytes_ist_lesbar() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }
}
