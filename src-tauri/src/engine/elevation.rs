//! Neustart mit Administratorrechten.
//!
//! Plane fordert **nie** von sich aus erhöhte Rechte an. Ein Aufräumwerkzeug,
//! das grundsätzlich als Administrator läuft, ist eine größere Angriffsfläche
//! als eines, das ohne auskommt — deshalb bleibt die Elevation eine bewusste
//! Handlung des Nutzers.
//!
//! # Was Adminrechte bringen — und was nicht
//!
//! | Fall | Hilft Elevation? |
//! |------|------------------|
//! | Ziel liegt in `%WINDIR%` oder `%PROGRAMDATA%` | **ja** |
//! | Dienste stoppen (`wuauserv`, `FontCache`) | **ja** |
//! | Zugriff verweigert (Fehler 5) | meistens |
//! | Datei von einem Programm geöffnet (Fehler 32) | **nein** |
//!
//! Der letzte Punkt wird regelmäßig missverstanden: eine exklusiv geöffnete
//! Datei lässt sich auch als Administrator nicht löschen. Dagegen hilft nur,
//! das Programm zu schließen.

/// Ergebnis eines Elevationsversuchs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Elevation {
    /// Der neue Prozess wurde gestartet; der aufrufende soll sich beenden.
    Gestartet,
    /// Läuft bereits erhöht – nichts zu tun.
    Bereits,
    /// Nutzer hat die Rückfrage abgelehnt oder der Start scheiterte.
    Abgelehnt(String),
}

/// Prüft, ob ein Neustart überhaupt sinnvoll wäre.
pub fn neustart_sinnvoll() -> bool {
    !super::runtime::is_admin()
}

/// Plane mit Administratorrechten neu starten.
///
/// Windows zeigt dabei die UAC-Rückfrage. Lehnt der Nutzer ab, kommt
/// [`Elevation::Abgelehnt`] zurück und der laufende Prozess bleibt unverändert
/// bestehen — es passiert also nichts Unerwartetes.
///
/// Der Aufrufer ist dafür zuständig, sich nach [`Elevation::Gestartet`] zu
/// beenden. Täte er es nicht, liefen zwei Instanzen nebeneinander.
#[cfg(windows)]
pub fn neu_starten_als_admin(argumente: &[String]) -> Elevation {
    use std::os::windows::ffi::OsStrExt;

    if super::runtime::is_admin() {
        return Elevation::Bereits;
    }

    let Ok(programm) = std::env::current_exe() else {
        return Elevation::Abgelehnt("Eigener Programmpfad nicht ermittelbar".into());
    };

    /// `OsStr` in eine nullterminierte UTF-16-Folge, wie sie die Win32-API
    /// erwartet.
    fn weit(text: &std::ffi::OsStr) -> Vec<u16> {
        text.encode_wide().chain(std::iter::once(0)).collect()
    }

    let verb = weit(std::ffi::OsStr::new("runas"));
    let datei = weit(programm.as_os_str());
    let parameter = weit(std::ffi::OsStr::new(&argumente.join(" ")));

    // SAFETY: Alle übergebenen Zeiger verweisen auf nullterminierte Puffer,
    // die bis zum Ende des Aufrufs am Leben bleiben. ShellExecuteW schreibt
    // nicht in sie hinein.
    let ergebnis = unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            std::ptr::null_mut(),
            verb.as_ptr(),
            datei.as_ptr(),
            if argumente.is_empty() {
                std::ptr::null()
            } else {
                parameter.as_ptr()
            },
            std::ptr::null(),
            windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        )
    };

    // ShellExecuteW gibt bei Erfolg einen Wert größer 32 zurück. 5 bedeutet
    // „Zugriff verweigert" – das ist die abgelehnte UAC-Rückfrage.
    let code = ergebnis as isize;
    if code > 32 {
        Elevation::Gestartet
    } else if code == 5 {
        Elevation::Abgelehnt("UAC-Rückfrage abgelehnt".into())
    } else {
        Elevation::Abgelehnt(format!("ShellExecuteW meldete {code}"))
    }
}

#[cfg(not(windows))]
pub fn neu_starten_als_admin(_argumente: &[String]) -> Elevation {
    Elevation::Abgelehnt("Nur unter Windows verfügbar".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neustart_ist_nur_ohne_rechte_sinnvoll() {
        assert_eq!(neustart_sinnvoll(), !super::super::runtime::is_admin());
    }

    #[test]
    fn bereits_erhoehter_prozess_startet_nicht_neu() {
        // Läuft die Testsuite als Administrator, muss der Aufruf sofort
        // `Bereits` melden statt ein zweites Fenster zu öffnen.
        if super::super::runtime::is_admin() {
            assert_eq!(neu_starten_als_admin(&[]), Elevation::Bereits);
        }
    }

    #[test]
    fn elevation_ist_vergleichbar_und_beschreibbar() {
        let abgelehnt = Elevation::Abgelehnt("Grund".into());
        assert_ne!(abgelehnt, Elevation::Gestartet);
        assert!(format!("{abgelehnt:?}").contains("Grund"));
    }
}
